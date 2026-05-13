use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyJsonResource;

impl MyJsonResource {
    pub const FILENAME: &'static str = "my-json-readme.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-json/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_my_json_readme";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch the my-json README: a higher-performance alternative to serde_json with zero-copy \
         reads over byte slices via JsonValueRef and lazy on-demand path resolution (no full \
         pre-parse). JSON path queries (get_value, get_value_as_vec, j_update), the my_json! \
         declarative macro (object vs array by bracket type), fluent JsonObjectWriter / \
         JsonArrayWriter APIs, conditional writes, async streaming for arrays and JSONL, \
         optional decimal support.";
}

impl ResourceDefinition for MyJsonResource {
    const RESOURCE_URI: &'static str = "resource://my-json-readme";
    const RESOURCE_NAME: &'static str = "my-json JSON Processing Library";
    const DESCRIPTION: &'static str =
        "Higher-performance alternative to serde_json: zero-copy JsonValueRef reads over byte \
         slices, lazy on-demand path resolution, path queries, j_update path-based edits, \
         my_json! macro, fluent JsonObjectWriter / JsonArrayWriter, async streaming.";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyJsonResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
