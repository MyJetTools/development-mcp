use std::{collections::HashMap, sync::Arc};

use mcp_server_middleware::*;
use serde::{Deserialize, Serialize};

use crate::app::AppContext;
use crate::mcp::{fetch_resource_text, scripts::load_resource_by_http, ResourceToolResponse};

pub struct RustExtensionsResource;

impl RustExtensionsResource {
    pub const FILENAME: &'static str = "rust-extensions.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/rust-extensions/main/README.md";
    pub const TOOL_FN: &'static str = "get_rust_extensions_readme";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch rust-extensions docs. Without `topic` - the README: what the crate is and the list \
         of topics. With `topic` (e.g. `events-loop`, `date-time`) - that topic in depth, with \
         examples for every case";
}

impl ResourceDefinition for RustExtensionsResource {
    const RESOURCE_URI: &'static str = "resource://rust-extensions";
    const RESOURCE_NAME: &'static str = "rust-extensions for each project";
    const DESCRIPTION: &'static str =
        "Low-level utils, queues and other helpers to glue together Rust code. The README is the \
         index of topics; each topic is resource://rust-extensions/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for RustExtensionsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

/// `resource://rust-extensions/{topic}` - one document of `docs/` in the
/// rust-extensions repo.
pub struct RustExtensionsTopicResource;

impl ResourceTemplateDefinition for RustExtensionsTopicResource {
    const URI_TEMPLATE: &'static str = "resource://rust-extensions/{topic}";
    const TEMPLATE_NAME: &'static str = "rust-extensions topic";
    const DESCRIPTION: &'static str =
        "One topic of rust-extensions in depth: contracts and examples for every case. The \
         topics are listed in resource://rust-extensions";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceTemplateService for RustExtensionsTopicResource {
    async fn read_resource(
        &self,
        uri: &str,
        variables: &HashMap<String, String>,
    ) -> Result<ResourceReadResult, ResourceTemplateReadError> {
        let text = load_topic(variables["topic"].as_str()).await?;

        Ok(ResourceReadResult {
            contents: vec![ResourceContent {
                uri: uri.to_string(),
                mime_type: Self::MIME_TYPE.to_string(),
                text: Some(text),
                blob: None,
            }],
        })
    }
}

const DOCS_URL: &str = "https://raw.githubusercontent.com/MyJetTools/rust-extensions/main/docs";

const SEE_INDEX: &str = "The topics are listed in the rust-extensions README: \
     resource://rust-extensions, or get_rust_extensions_readme without `topic`.";

/// Loads `docs/{topic}.md`. The list of topics lives in the rust-extensions
/// README only, so a new topic needs no release of this server.
async fn load_topic(topic: &str) -> Result<String, ResourceTemplateReadError> {
    // The topic goes into a URL path - nothing but a plain name may get there.
    let is_name = !topic.is_empty()
        && topic.len() <= 64
        && topic
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');

    if !is_name {
        return Err(ResourceTemplateReadError::NotFound(format!(
            "`{}` is not a topic name. {}",
            topic, SEE_INDEX
        )));
    }

    let url = format!("{}/{}.md", DOCS_URL, topic);

    let mut response = flurl::FlUrl::new(url.as_str())
        .get()
        .await
        .map_err(|e| format!("Failed to fetch {}: {:?}", url, e))?;

    match response.get_status_code() {
        200 => {}
        404 => {
            return Err(ResourceTemplateReadError::NotFound(format!(
                "There is no rust-extensions topic `{}`. {}",
                topic, SEE_INDEX
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

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct RustExtensionsDocsInput {
    #[property(
        description = "Topic to read, e.g. `events-loop` or `date-time` - the README lists them all. Omit it to get the README"
    )]
    pub topic: Option<String>,
}

pub struct RustExtensionsReadmeTool {
    _app: Arc<AppContext>,
}

impl RustExtensionsReadmeTool {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

impl ToolDefinition for RustExtensionsReadmeTool {
    const FUNC_NAME: &'static str = RustExtensionsResource::TOOL_FN;
    const DESCRIPTION: &'static str = RustExtensionsResource::TOOL_DESCRIPTION;
}

#[async_trait::async_trait]
impl McpToolCall<RustExtensionsDocsInput, ResourceToolResponse> for RustExtensionsReadmeTool {
    async fn execute_tool_call(
        &self,
        model: RustExtensionsDocsInput,
    ) -> Result<ResourceToolResponse, String> {
        let Some(topic) = model.topic.as_deref().filter(|topic| !topic.is_empty()) else {
            return fetch_resource_text(
                RustExtensionsResource::RESOURCE_URI,
                RustExtensionsResource::RESOURCE_NAME,
                RustExtensionsResource::DESCRIPTION,
                RustExtensionsResource::MIME_TYPE,
                RustExtensionsResource::URL,
            )
            .await;
        };

        let text = load_topic(topic).await.map_err(|err| match err {
            ResourceTemplateReadError::NotFound(message) => message,
            ResourceTemplateReadError::Internal(message) => message,
        })?;

        Ok(ResourceToolResponse {
            uri: format!("resource://rust-extensions/{}", topic),
            resource_name: RustExtensionsTopicResource::TEMPLATE_NAME.to_string(),
            resource_description: RustExtensionsTopicResource::DESCRIPTION.to_string(),
            mime_type: RustExtensionsTopicResource::MIME_TYPE.to_string(),
            text,
        })
    }
}
