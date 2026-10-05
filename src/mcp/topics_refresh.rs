use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
    time::Duration,
};

use mcp_server_middleware::*;
use my_logger::LogEventCtx;
use rust_extensions::{MyTimer, MyTimerTick, RepeatTimerIteration};

use crate::mcp::{
    get_topic_name, get_topic_uri, is_topic_name, load_topic, TopicsDocDefinition, TopicsDocTool,
};

/// How often the topics table of every index is read again.
const REFRESH_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// One row of the topics table of an index:
/// ``| [`events-loop`](events-loop.md) | Single-consumer async message loop | `EventsLoop` |``.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocTopic {
    pub topic: String,
    pub description: String,
    /// The third cell of the row - the `Startable` types of rust-extensions;
    /// `None` when the index has `—` (or no such column).
    pub note: Option<String>,
}

impl DocTopic {
    /// The index description, plus the third cell under the header of its
    /// column, when the doc names one.
    pub fn get_description(&self, note_column: Option<&str>) -> String {
        match (note_column, &self.note) {
            (Some(column), Some(note)) => format!("{}. {}: {}", self.description, column, note),
            _ => self.description.clone(),
        }
    }
}

/// The topics the index table lists, in its order. Anything that is not a
/// topic row is skipped.
pub fn parse_topics(index: &str) -> Vec<DocTopic> {
    index.lines().filter_map(parse_topic_row).collect()
}

/// `| [`topic`](topic.md) | description | note |` - the last cell is
/// optional, and a `docs/topic.md` link (relative to the repo root) is fine too.
fn parse_topic_row(line: &str) -> Option<DocTopic> {
    let cells: Vec<&str> = line
        .trim()
        .strip_prefix('|')?
        .strip_suffix('|')?
        .split('|')
        .map(str::trim)
        .collect();

    let (topic, link) = cells.first()?.strip_prefix("[`")?.split_once("`](")?;
    let link = link.strip_prefix("docs/").unwrap_or(link);
    let description = *cells.get(1)?;

    if !is_topic_name(topic) || link != format!("{}.md)", topic) || description.is_empty() {
        return None;
    }

    let note = cells
        .get(2)
        .filter(|cell| !matches!(**cell, "" | "—" | "-"))
        .map(|cell| cell.to_string());

    Some(DocTopic {
        topic: topic.to_string(),
        description: description.to_string(),
        note,
    })
}

/// What a refresh changed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TopicsUpdate {
    /// Gone from the index.
    pub removed: Vec<DocTopic>,
    /// New in the index, or with a new description.
    pub upserted: Vec<DocTopic>,
}

impl TopicsUpdate {
    pub fn is_empty(&self) -> bool {
        self.removed.is_empty() && self.upserted.is_empty()
    }
}

/// The topics an index listed at the last refresh. Owned by the
/// `TopicsDocTool` of the doc, filled by [`TopicsRefreshTimer`].
pub struct DocTopics {
    items: RwLock<Vec<DocTopic>>,
}

impl DocTopics {
    pub fn new() -> Self {
        Self {
            items: RwLock::new(Vec::new()),
        }
    }

    pub fn get(&self, topic: &str) -> Option<DocTopic> {
        self.items
            .read()
            .unwrap()
            .iter()
            .find(|item| item.topic == topic)
            .cloned()
    }

    /// Stores the fresh list and tells what changed.
    pub fn update(&self, fresh: Vec<DocTopic>) -> TopicsUpdate {
        let mut items = self.items.write().unwrap();

        let removed = items
            .iter()
            .filter(|item| !fresh.iter().any(|fresh| fresh.topic == item.topic))
            .cloned()
            .collect();

        let upserted = fresh
            .iter()
            .filter(|fresh| !items.contains(fresh))
            .cloned()
            .collect();

        *items = fresh;

        TopicsUpdate { removed, upserted }
    }
}

/// Refreshes the topics of every doc split into topics on one timer - one
/// tick per doc. Every topic of an index becomes a resource of its own.
pub struct TopicsRefreshTimer {
    timer: MyTimer,
    mcp: Arc<McpMiddleware>,
}

impl TopicsRefreshTimer {
    pub fn new(mcp: &Arc<McpMiddleware>) -> Self {
        let mut timer = MyTimer::new(REFRESH_INTERVAL, my_logger::LOGGER.clone());
        timer.set_first_tick_before_delay();

        Self {
            timer,
            mcp: mcp.clone(),
        }
    }

    /// Before [`Self::start`]: the timer takes no ticks once it is started.
    pub fn register<TDoc: TopicsDocDefinition>(&mut self, tool: &TopicsDocTool<TDoc>) {
        self.timer.register_timer(
            format!("{}-topics", TDoc::TOPIC_NAME_PREFIX).as_str(),
            Arc::new(TopicsRefresh::<TDoc> {
                topics: tool.get_topics(),
                mcp: self.mcp.clone(),
                _doc: PhantomData,
            }),
        );
    }

    pub fn start(&self) {
        self.timer.start();
    }
}

/// Keeps one MCP resource per topic of the index of `TDoc`, so every topic
/// shows up in `resources/list` with its own name and description. Topics the
/// index does not list yet are still served through the `{topic}` template.
struct TopicsRefresh<TDoc: TopicsDocDefinition> {
    topics: Arc<DocTopics>,
    mcp: Arc<McpMiddleware>,
    _doc: PhantomData<TDoc>,
}

#[async_trait::async_trait]
impl<TDoc: TopicsDocDefinition> MyTimerTick for TopicsRefresh<TDoc> {
    async fn tick(&self) -> RepeatTimerIteration {
        let index = match fetch_index::<TDoc>().await {
            Ok(index) => index,
            Err(err) => {
                my_logger::LOGGER.write_error(
                    "TopicsRefresh",
                    err,
                    LogEventCtx::new().add("index", TDoc::URL),
                );
                return RepeatTimerIteration::WithInterval;
            }
        };

        let fresh = parse_topics(index.as_str());

        // An index without the table is a broken read, not "every topic is gone".
        if fresh.is_empty() {
            my_logger::LOGGER.write_warning(
                "TopicsRefresh",
                "The index has no topics table - the topics are kept as they are",
                LogEventCtx::new().add("index", TDoc::URL),
            );
            return RepeatTimerIteration::WithInterval;
        }

        let update = self.topics.update(fresh);

        for topic in update.removed.iter() {
            self.mcp
                .unregister_dynamic_resource(get_topic_uri::<TDoc>(topic.topic.as_str()).as_str())
                .await;
        }

        for topic in update.upserted.iter() {
            self.mcp
                .register_dynamic_resource(
                    get_topic_uri::<TDoc>(topic.topic.as_str()),
                    get_topic_name::<TDoc>(topic.topic.as_str()),
                    topic.get_description(TDoc::NOTE_COLUMN),
                    TDoc::MIME_TYPE.to_string(),
                    Arc::new(TopicDoc::<TDoc> {
                        topic: topic.topic.clone(),
                        _doc: PhantomData,
                    }),
                )
                .await;
        }

        if !update.is_empty() {
            self.mcp.notify_resources_changed().await;
        }

        RepeatTimerIteration::WithInterval
    }
}

async fn fetch_index<TDoc: TopicsDocDefinition>() -> Result<String, String> {
    let url = TDoc::URL;

    let mut response = flurl::FlUrl::new(url)
        .get()
        .await
        .map_err(|e| format!("Failed to fetch {}: {:?}", url, e))?;

    let status_code = response.get_status_code();
    if status_code != 200 {
        return Err(format!(
            "Failed to fetch {}: status code {}",
            url, status_code
        ));
    }

    let text = response
        .get_body_as_str()
        .await
        .map_err(|e| format!("Failed to read the body of {}: {:?}", url, e))?;

    Ok(text.to_string())
}

/// `resources/read` of one listed topic.
struct TopicDoc<TDoc: TopicsDocDefinition> {
    topic: String,
    _doc: PhantomData<TDoc>,
}

#[async_trait::async_trait]
impl<TDoc: TopicsDocDefinition> McpResourceService for TopicDoc<TDoc> {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        let text = load_topic::<TDoc>(self.topic.as_str())
            .await
            .map_err(|err| match err {
                ResourceTemplateReadError::NotFound(message) => message,
                ResourceTemplateReadError::Internal(message) => message,
            })?;

        Ok(ResourceReadResult {
            contents: vec![ResourceContent {
                uri: get_topic_uri::<TDoc>(self.topic.as_str()),
                mime_type: TDoc::MIME_TYPE.to_string(),
                text: Some(text),
                blob: None,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(topic: &str, description: &str) -> DocTopic {
        DocTopic {
            topic: topic.to_string(),
            description: description.to_string(),
            note: None,
        }
    }

    fn with_note(topic: &str, description: &str, note: &str) -> DocTopic {
        DocTopic {
            topic: topic.to_string(),
            description: description.to_string(),
            note: Some(note.to_string()),
        }
    }

    #[test]
    fn parses_the_rows_of_the_topics_table() {
        let index = "\
# rust-extensions — index

| Feature | Enables |
| --- | --- |
| `with-tokio` | Timers, events loop |

| Topic | What is inside | Startable |
| --- | --- | --- |
| [`date-time`](date-time.md) | `DateTimeAsMicroseconds` — UTC µs timestamp, serde | — |
| [`timers`](timers.md) | Periodic work | `MyTimer`, `MyExactTimer` |
";

        assert_eq!(
            parse_topics(index),
            vec![
                topic(
                    "date-time",
                    "`DateTimeAsMicroseconds` — UTC µs timestamp, serde"
                ),
                with_note("timers", "Periodic work", "`MyTimer`, `MyExactTimer`"),
            ]
        );
    }

    /// The README of the repo root used to be the index - its rows link `docs/`.
    #[test]
    fn the_third_column_and_the_docs_prefix_are_optional() {
        let index = "| [`events-loop`](docs/events-loop.md) | Single-consumer async message loop |";

        assert_eq!(
            parse_topics(index),
            vec![topic("events-loop", "Single-consumer async message loop")]
        );
    }

    #[test]
    fn the_description_names_the_third_column() {
        let events_loop = with_note(
            "events-loop",
            "Single-consumer async message loop",
            "`EventsLoop`",
        );

        assert_eq!(
            events_loop.get_description(Some("Startable")),
            "Single-consumer async message loop. Startable: `EventsLoop`"
        );
        assert_eq!(
            events_loop.get_description(None),
            "Single-consumer async message loop"
        );
        assert_eq!(
            topic("misc", "Paths").get_description(Some("Startable")),
            "Paths"
        );
    }

    #[test]
    fn skips_rows_which_are_not_topics() {
        let index = "\
| `with-tokio` | Async primitives |
| [`Bad Name`](Bad Name.md) | x |
| [`mismatch`](other.md) | x |
| [`empty`](empty.md) |  |
";

        assert!(parse_topics(index).is_empty());
    }

    #[test]
    fn an_update_tells_what_was_removed_added_and_changed() {
        let topics = DocTopics::new();

        let first = topics.update(vec![topic("a", "one"), topic("b", "two")]);
        assert_eq!(first.upserted, vec![topic("a", "one"), topic("b", "two")]);
        assert!(first.removed.is_empty());

        let same = topics.update(vec![topic("a", "one"), topic("b", "two")]);
        assert!(same.is_empty());

        let changed = topics.update(vec![topic("b", "two, longer"), topic("c", "three")]);
        assert_eq!(changed.removed, vec![topic("a", "one")]);
        assert_eq!(
            changed.upserted,
            vec![topic("b", "two, longer"), topic("c", "three")]
        );

        assert_eq!(topics.get("c"), Some(topic("c", "three")));
        assert_eq!(topics.get("a"), None);
    }
}
