use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct CiUtilsResource;

impl CiUtilsResource {
    pub const FILENAME: &'static str = "ci-utils-readme.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/ci-utils/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_ci_utils_readme";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch ci-utils README resource content";
}

impl ResourceDefinition for CiUtilsResource {
    const RESOURCE_URI: &'static str = "resource://ci-utils-readme";
    const RESOURCE_NAME: &'static str = "ci-utils for each project";
    const DESCRIPTION: &'static str = "Utility crate for build-time helpers";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for CiUtilsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
