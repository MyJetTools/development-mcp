use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use mcp_server_middleware::*;
use my_logger::LogEventCtx;
use rust_extensions::{MyTimer, MyTimerTick, RepeatTimerIteration};

use crate::mcp::{
    get_topic_uri, is_topic_name, load_topic, RustExtensionsResource, RustExtensionsTopicResource,
};

/// How often the topics table of the rust-extensions index is read again.
const REFRESH_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// One row of the topics table of the rust-extensions index (`docs/index_resource.md`):
/// ``| [`events-loop`](events-loop.md) | Single-consumer async message loop | `EventsLoop` |``.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustExtensionsTopic {
    pub topic: String,
    pub description: String,
    /// The types of the topic which implement `Startable`; `None` when the
    /// index has `—` (or no such column).
    pub startable: Option<String>,
}

impl RustExtensionsTopic {
    pub fn get_name(&self) -> String {
        get_topic_name(self.topic.as_str())
    }

    /// The index description, plus the `Startable` types when the topic has any.
    pub fn get_description(&self) -> String {
        match &self.startable {
            Some(startable) => format!("{}. Startable: {}", self.description, startable),
            None => self.description.clone(),
        }
    }
}

/// The resource name of a topic - every topic has its own.
pub fn get_topic_name(topic: &str) -> String {
    format!("rust-extensions: {}", topic)
}

/// The topics the index table lists, in its order. Anything that is not a
/// topic row is skipped.
pub fn parse_topics(index: &str) -> Vec<RustExtensionsTopic> {
    index.lines().filter_map(parse_topic_row).collect()
}

/// `| [`topic`](topic.md) | description | startable |` - the last cell is
/// optional, and a `docs/topic.md` link (relative to the repo root) is fine too.
fn parse_topic_row(line: &str) -> Option<RustExtensionsTopic> {
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

    let startable = cells
        .get(2)
        .filter(|cell| !matches!(**cell, "" | "—" | "-"))
        .map(|cell| cell.to_string());

    Some(RustExtensionsTopic {
        topic: topic.to_string(),
        description: description.to_string(),
        startable,
    })
}

/// What a refresh changed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TopicsUpdate {
    /// Gone from the index.
    pub removed: Vec<RustExtensionsTopic>,
    /// New in the index, or with a new description.
    pub upserted: Vec<RustExtensionsTopic>,
}

impl TopicsUpdate {
    pub fn is_empty(&self) -> bool {
        self.removed.is_empty() && self.upserted.is_empty()
    }
}

/// The topics the index listed at the last refresh. Owned by
/// `RustExtensionsReadmeTool`, filled by [`RustExtensionsTopicsRefresh`].
pub struct RustExtensionsTopics {
    items: RwLock<Vec<RustExtensionsTopic>>,
}

impl RustExtensionsTopics {
    pub fn new() -> Self {
        Self {
            items: RwLock::new(Vec::new()),
        }
    }

    pub fn get(&self, topic: &str) -> Option<RustExtensionsTopic> {
        self.items
            .read()
            .unwrap()
            .iter()
            .find(|item| item.topic == topic)
            .cloned()
    }

    /// Stores the fresh list and tells what changed.
    pub fn update(&self, fresh: Vec<RustExtensionsTopic>) -> TopicsUpdate {
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

/// Keeps one MCP resource per topic of the index, so every topic shows up in
/// `resources/list` with its own name and description. Topics the index does
/// not list yet are still served through the `{topic}` template.
pub struct RustExtensionsTopicsRefresh {
    topics: Arc<RustExtensionsTopics>,
    mcp: Arc<McpMiddleware>,
}

#[async_trait::async_trait]
impl MyTimerTick for RustExtensionsTopicsRefresh {
    async fn tick(&self) -> RepeatTimerIteration {
        let index = match fetch_index().await {
            Ok(index) => index,
            Err(err) => {
                my_logger::LOGGER.write_error(
                    "RustExtensionsTopicsRefresh",
                    err,
                    LogEventCtx::new(),
                );
                return RepeatTimerIteration::WithInterval;
            }
        };

        let fresh = parse_topics(index.as_str());

        // An index without the table is a broken read, not "every topic is gone".
        if fresh.is_empty() {
            my_logger::LOGGER.write_warning(
                "RustExtensionsTopicsRefresh",
                "The rust-extensions index has no topics table - the topics are kept as they are",
                LogEventCtx::new(),
            );
            return RepeatTimerIteration::WithInterval;
        }

        let update = self.topics.update(fresh);

        for topic in update.removed.iter() {
            self.mcp
                .unregister_dynamic_resource(get_topic_uri(topic.topic.as_str()).as_str())
                .await;
        }

        for topic in update.upserted.iter() {
            self.mcp
                .register_dynamic_resource(
                    get_topic_uri(topic.topic.as_str()),
                    topic.get_name(),
                    topic.get_description(),
                    RustExtensionsTopicResource::MIME_TYPE.to_string(),
                    Arc::new(RustExtensionsTopicDoc {
                        topic: topic.topic.clone(),
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

pub fn start_rust_extensions_topics_refresh(
    topics: Arc<RustExtensionsTopics>,
    mcp: &Arc<McpMiddleware>,
) {
    let mut timer = MyTimer::new(REFRESH_INTERVAL, my_logger::LOGGER.clone());
    timer.set_first_tick_before_delay();
    timer.register_timer(
        "rust-extensions-topics",
        Arc::new(RustExtensionsTopicsRefresh {
            topics,
            mcp: mcp.clone(),
        }),
    );
    timer.start();
}

async fn fetch_index() -> Result<String, String> {
    let url = RustExtensionsResource::URL;

    let mut response = flurl::FlUrl::new(url)
        .get()
        .await
        .map_err(|e| format!("Failed to fetch {}: {:?}", url, e))?;

    let status_code = response.get_status_code();
    if status_code != 200 {
        return Err(format!("Failed to fetch {}: status code {}", url, status_code));
    }

    let text = response
        .get_body_as_str()
        .await
        .map_err(|e| format!("Failed to read the body of {}: {:?}", url, e))?;

    Ok(text.to_string())
}

/// `resources/read` of one listed topic.
struct RustExtensionsTopicDoc {
    topic: String,
}

#[async_trait::async_trait]
impl McpResourceService for RustExtensionsTopicDoc {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        let text = load_topic(self.topic.as_str()).await.map_err(|err| match err {
            ResourceTemplateReadError::NotFound(message) => message,
            ResourceTemplateReadError::Internal(message) => message,
        })?;

        Ok(ResourceReadResult {
            contents: vec![ResourceContent {
                uri: get_topic_uri(self.topic.as_str()),
                mime_type: RustExtensionsTopicResource::MIME_TYPE.to_string(),
                text: Some(text),
                blob: None,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(topic: &str, description: &str) -> RustExtensionsTopic {
        RustExtensionsTopic {
            topic: topic.to_string(),
            description: description.to_string(),
            startable: None,
        }
    }

    fn startable(topic: &str, description: &str, startable: &str) -> RustExtensionsTopic {
        RustExtensionsTopic {
            topic: topic.to_string(),
            description: description.to_string(),
            startable: Some(startable.to_string()),
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
                topic("date-time", "`DateTimeAsMicroseconds` — UTC µs timestamp, serde"),
                startable("timers", "Periodic work", "`MyTimer`, `MyExactTimer`"),
            ]
        );
    }

    /// The README of the repo root used to be the index - its rows link `docs/`.
    #[test]
    fn the_startable_column_and_the_docs_prefix_are_optional() {
        let index = "| [`events-loop`](docs/events-loop.md) | Single-consumer async message loop |";

        assert_eq!(
            parse_topics(index),
            vec![topic("events-loop", "Single-consumer async message loop")]
        );
    }

    #[test]
    fn the_description_names_the_startable_types() {
        assert_eq!(
            startable("events-loop", "Single-consumer async message loop", "`EventsLoop`")
                .get_description(),
            "Single-consumer async message loop. Startable: `EventsLoop`"
        );
        assert_eq!(topic("misc", "Paths").get_description(), "Paths");
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
        let topics = RustExtensionsTopics::new();

        let first = topics.update(vec![topic("a", "one"), topic("b", "two")]);
        assert_eq!(first.upserted, vec![topic("a", "one"), topic("b", "two")]);
        assert!(first.removed.is_empty());

        let same = topics.update(vec![topic("a", "one"), topic("b", "two")]);
        assert!(same.is_empty());

        let changed = topics.update(vec![topic("b", "two, longer"), topic("c", "three")]);
        assert_eq!(changed.removed, vec![topic("a", "one")]);
        assert_eq!(changed.upserted, vec![topic("b", "two, longer"), topic("c", "three")]);

        assert_eq!(topics.get("c"), Some(topic("c", "three")));
        assert_eq!(topics.get("a"), None);
    }
}
