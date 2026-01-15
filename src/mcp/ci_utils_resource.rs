use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct CiUtilsResource;

impl ResourceDefinition for CiUtilsResource {
    const RESOURCE_URI: &'static str = "resource://ci-utils-readme";
    const RESOURCE_NAME: &'static str = "ci-utils for each project";
    const DESCRIPTION: &'static str = "Utility crate for build-time helpers";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for CiUtilsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/ci-utils/refs/heads/main/README.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
