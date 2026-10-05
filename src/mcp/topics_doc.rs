use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use mcp_server_middleware::*;
use serde::{Deserialize, Serialize};

use crate::app::AppContext;
use crate::mcp::{fetch_resource_text, DocTopics, ResourceToolResponse};

/// A doc split into topics: an index which lists the topics in a table, and
/// one `{topic}.md` per topic next to it.
///
/// The index is the resource itself; every topic is `{RESOURCE_URI}/{topic}`,
/// served by [`TopicResource`] and by [`TopicsDocTool`] with a `topic`. The
/// list of topics lives in the index only, so a new topic needs no release of
/// this server.
pub trait TopicsDocDefinition: ResourceDefinition + Send + Sync + 'static {
    /// The name of the doc in `/ai-docs`.
    const FILENAME: &'static str;
    /// The index - the page with the topics table.
    const URL: &'static str;
    /// The folder of the topics: a topic is `{DOCS_URL}/{topic}.md`.
    const DOCS_URL: &'static str;
    const TOOL_FN: &'static str;
    const TOOL_DESCRIPTION: &'static str;
    /// `{RESOURCE_URI}/{topic}`.
    const TOPIC_URI_TEMPLATE: &'static str;
    const TOPIC_TEMPLATE_NAME: &'static str;
    const TOPIC_DESCRIPTION: &'static str;
    /// A topic resource is named `{TOPIC_NAME_PREFIX}: {topic}`.
    const TOPIC_NAME_PREFIX: &'static str;
    /// The header of the third column of the topics table, when the doc uses
    /// one: its cell goes into the description of the topic.
    const NOTE_COLUMN: Option<&'static str> = None;
}

pub fn get_topic_uri<TDoc: TopicsDocDefinition>(topic: &str) -> String {
    format!("{}/{}", TDoc::RESOURCE_URI, topic)
}

/// The resource name of a topic - every topic has its own.
pub fn get_topic_name<TDoc: TopicsDocDefinition>(topic: &str) -> String {
    format!("{}: {}", TDoc::TOPIC_NAME_PREFIX, topic)
}

/// The topic goes into a URL path - nothing but a plain name may get there.
pub fn is_topic_name(topic: &str) -> bool {
    !topic.is_empty()
        && topic.len() <= 64
        && topic
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn see_index<TDoc: TopicsDocDefinition>() -> String {
    format!(
        "The topics are listed in the {} index: {}, or {} without `topic`.",
        TDoc::TOPIC_NAME_PREFIX,
        TDoc::RESOURCE_URI,
        TDoc::TOOL_FN
    )
}

/// Loads `{DOCS_URL}/{topic}.md`.
pub async fn load_topic<TDoc: TopicsDocDefinition>(
    topic: &str,
) -> Result<String, ResourceTemplateReadError> {
    if !is_topic_name(topic) {
        return Err(ResourceTemplateReadError::NotFound(format!(
            "`{}` is not a topic name. {}",
            topic,
            see_index::<TDoc>()
        )));
    }

    let url = format!("{}/{}.md", TDoc::DOCS_URL, topic);

    let mut response = flurl::FlUrl::new(url.as_str())
        .get()
        .await
        .map_err(|e| format!("Failed to fetch {}: {:?}", url, e))?;

    match response.get_status_code() {
        200 => {}
        404 => {
            return Err(ResourceTemplateReadError::NotFound(format!(
                "There is no {} topic `{}`. {}",
                TDoc::TOPIC_NAME_PREFIX,
                topic,
                see_index::<TDoc>()
            )));
        }
        status_code => {
            return Err(format!("Failed to fetch {}: status code {}", url, status_code).into());
        }
    }

    let text = response
        .get_body_as_str()
        .await
        .map_err(|e| format!("Failed to read the body of {}: {:?}", url, e))?;

    Ok(text.to_string())
}

/// `{RESOURCE_URI}/{topic}` - one topic of the doc.
pub struct TopicResource<TDoc: TopicsDocDefinition> {
    _doc: PhantomData<TDoc>,
}

impl<TDoc: TopicsDocDefinition> Default for TopicResource<TDoc> {
    fn default() -> Self {
        Self { _doc: PhantomData }
    }
}

impl<TDoc: TopicsDocDefinition> ResourceTemplateDefinition for TopicResource<TDoc> {
    const URI_TEMPLATE: &'static str = TDoc::TOPIC_URI_TEMPLATE;
    const TEMPLATE_NAME: &'static str = TDoc::TOPIC_TEMPLATE_NAME;
    const DESCRIPTION: &'static str = TDoc::TOPIC_DESCRIPTION;
    const MIME_TYPE: &'static str = TDoc::MIME_TYPE;
}

#[async_trait::async_trait]
impl<TDoc: TopicsDocDefinition> McpResourceTemplateService for TopicResource<TDoc> {
    async fn read_resource(
        &self,
        uri: &str,
        variables: &HashMap<String, String>,
    ) -> Result<ResourceReadResult, ResourceTemplateReadError> {
        let text = load_topic::<TDoc>(variables["topic"].as_str()).await?;

        Ok(ResourceReadResult {
            contents: vec![ResourceContent {
                uri: uri.to_string(),
                mime_type: TDoc::MIME_TYPE.to_string(),
                text: Some(text),
                blob: None,
            }],
        })
    }
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct TopicsDocInput {
    #[property(
        description = "Topic to read - the index lists them all, the tool description names a few. Omit it to get the index"
    )]
    pub topic: Option<String>,
}

/// The `get_*` tool of the doc: the index without `topic`, the topic with it.
pub struct TopicsDocTool<TDoc: TopicsDocDefinition> {
    _app: Arc<AppContext>,
    /// The topics of the index - the name and the description of every topic.
    /// Kept fresh by the refresh timer, see [`Self::get_topics`].
    topics: Arc<DocTopics>,
    _doc: PhantomData<TDoc>,
}

impl<TDoc: TopicsDocDefinition> TopicsDocTool<TDoc> {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self {
            _app: app,
            topics: Arc::new(DocTopics::new()),
            _doc: PhantomData,
        }
    }

    /// For [`crate::mcp::TopicsRefreshTimer`], which fills the index.
    pub fn get_topics(&self) -> Arc<DocTopics> {
        self.topics.clone()
    }
}

impl<TDoc: TopicsDocDefinition> ToolDefinition for TopicsDocTool<TDoc> {
    const FUNC_NAME: &'static str = TDoc::TOOL_FN;
    const DESCRIPTION: &'static str = TDoc::TOOL_DESCRIPTION;
}

#[async_trait::async_trait]
impl<TDoc: TopicsDocDefinition> McpToolCall<TopicsDocInput, ResourceToolResponse>
    for TopicsDocTool<TDoc>
{
    async fn execute_tool_call(
        &self,
        model: TopicsDocInput,
    ) -> Result<ResourceToolResponse, String> {
        let Some(topic) = model.topic.as_deref().filter(|topic| !topic.is_empty()) else {
            return fetch_index::<TDoc>().await;
        };

        let text = match load_topic::<TDoc>(topic).await {
            Ok(text) => text,
            // A topic the doc does not have is a normal request, not a failure:
            // the answer is the index, whose table lists every topic.
            Err(ResourceTemplateReadError::NotFound(_)) => {
                let mut index = fetch_index::<TDoc>().await?;
                let note = format!(
                    "> There is no topic `{}` in {} - this is its index instead: the table \
                     lists every topic.",
                    topic,
                    TDoc::TOPIC_NAME_PREFIX
                );
                index.text = prepend_note(index.text.as_str(), note.as_str());
                return Ok(index);
            }
            Err(ResourceTemplateReadError::Internal(message)) => return Err(message),
        };

        // The index row of the topic gives its description. A topic the index
        // did not list at the last refresh still gets its own name.
        let resource_description = match self.topics.get(topic) {
            Some(listed) => listed.get_description(TDoc::NOTE_COLUMN),
            None => TDoc::TOPIC_DESCRIPTION.to_string(),
        };

        Ok(ResourceToolResponse {
            uri: get_topic_uri::<TDoc>(topic),
            resource_name: get_topic_name::<TDoc>(topic),
            resource_description,
            mime_type: TDoc::MIME_TYPE.to_string(),
            text,
        })
    }
}

async fn fetch_index<TDoc: TopicsDocDefinition>() -> Result<ResourceToolResponse, String> {
    fetch_resource_text(
        TDoc::RESOURCE_URI,
        TDoc::RESOURCE_NAME,
        TDoc::DESCRIPTION,
        TDoc::MIME_TYPE,
        TDoc::URL,
    )
    .await
}

/// Puts `note` in front of `doc` - after its front matter, which has to stay
/// the first thing of the doc.
fn prepend_note(doc: &str, note: &str) -> String {
    if let Some(end) = doc
        .strip_prefix("---\n")
        .and_then(|rest| rest.find("\n---\n"))
    {
        let (front_matter, body) = doc.split_at("---\n".len() + end + "\n---\n".len());
        return format!("{}{}\n\n{}", front_matter, note, body);
    }

    format!("{}\n\n{}", note, doc)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::mcp::{
        parse_topics, AppBootstrapResource, ApplicationArchitectureResource,
        ArchitectSkillResource, DioxusClientSideBootstrapResource, FlUrlResource,
        HttpActionsResource, RustExtensionsResource,
    };

    /// The docs of this repo are served from its `main` branch.
    const THIS_REPO: &str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/";

    struct Doc {
        resource_uri: &'static str,
        url: &'static str,
        docs_url: &'static str,
        tool_fn: &'static str,
        topic_uri_template: &'static str,
        topic_name_prefix: &'static str,
    }

    fn doc<TDoc: TopicsDocDefinition>() -> Doc {
        Doc {
            resource_uri: TDoc::RESOURCE_URI,
            url: TDoc::URL,
            docs_url: TDoc::DOCS_URL,
            tool_fn: TDoc::TOOL_FN,
            topic_uri_template: TDoc::TOPIC_URI_TEMPLATE,
            topic_name_prefix: TDoc::TOPIC_NAME_PREFIX,
        }
    }

    fn all_docs() -> Vec<Doc> {
        vec![
            doc::<FlUrlResource>(),
            doc::<HttpActionsResource>(),
            doc::<AppBootstrapResource>(),
            doc::<RustExtensionsResource>(),
            doc::<ArchitectSkillResource>(),
            doc::<DioxusClientSideBootstrapResource>(),
            doc::<ApplicationArchitectureResource>(),
        ]
    }

    #[test]
    fn every_doc_is_defined_consistently() {
        let docs = all_docs();

        for doc in docs.iter() {
            assert_eq!(
                doc.topic_uri_template,
                format!("{}/{{topic}}", doc.resource_uri)
            );
            assert_eq!(doc.url, format!("{}/index_resource.md", doc.docs_url));
        }

        for (i, doc) in docs.iter().enumerate() {
            for other in docs.iter().skip(i + 1) {
                assert_ne!(doc.resource_uri, other.resource_uri);
                assert_ne!(doc.tool_fn, other.tool_fn);
                // Names the refresh tick of the doc - the timer refuses a duplicate.
                assert_ne!(doc.topic_name_prefix, other.topic_name_prefix);
            }
        }
    }

    #[test]
    fn the_note_goes_after_the_front_matter() {
        assert_eq!(
            prepend_note("---\nalwaysApply: true\n---\n# Guide\n", "> note"),
            "---\nalwaysApply: true\n---\n> note\n\n# Guide\n"
        );
        assert_eq!(prepend_note("# Guide\n", "> note"), "> note\n\n# Guide\n");
        // A rule, not front matter: the doc does not start with it.
        assert_eq!(
            prepend_note("# Guide\n---\nx\n---\n", "> note"),
            "> note\n\n# Guide\n---\nx\n---\n"
        );
    }

    /// The index of a doc of this repo lists exactly the files of its folder,
    /// and a topic does not link into another file by an anchor.
    #[test]
    fn the_indexes_of_this_repo_list_exactly_their_topic_files() {
        let local_docs: Vec<Doc> = all_docs()
            .into_iter()
            .filter(|doc| doc.docs_url.starts_with(THIS_REPO))
            .collect();
        assert_eq!(local_docs.len(), 4);

        for doc in local_docs {
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(&doc.docs_url[THIS_REPO.len()..]);

            let index = std::fs::read_to_string(dir.join("index_resource.md")).unwrap();
            let mut listed: Vec<String> = parse_topics(index.as_str())
                .into_iter()
                .map(|topic| topic.topic)
                .collect();
            assert!(
                !listed.is_empty(),
                "{}: the index has no topics",
                doc.docs_url
            );

            let mut files: Vec<String> = std::fs::read_dir(&dir)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .filter(|name| name != "index_resource.md")
                .collect();

            for file in files.iter() {
                let text = std::fs::read_to_string(dir.join(file)).unwrap();
                assert!(
                    text.starts_with("# "),
                    "{}/{} has no title",
                    doc.docs_url,
                    file
                );
                assert!(
                    !text.contains("](#"),
                    "{}/{} links an anchor",
                    doc.docs_url,
                    file
                );
            }

            listed.sort();
            files.sort();
            let files: Vec<String> = files
                .into_iter()
                .map(|file| file.strip_suffix(".md").unwrap().to_string())
                .collect();
            assert_eq!(listed, files, "{}", doc.docs_url);
        }
    }
}
