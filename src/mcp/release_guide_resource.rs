use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct ReleaseGuideResource;

impl ReleaseGuideResource {
    pub const FILENAME: &'static str = "release-guide.md";
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/release-guide.md";
    pub const TOOL_FN: &'static str = "get_release_guide";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch the release guide: single-repo and monorepo release flows, gh commands, re-deploy, troubleshooting";
}

impl ResourceDefinition for ReleaseGuideResource {
    const RESOURCE_URI: &'static str = "resource://release-guide";
    const RESOURCE_NAME: &'static str = "Release Guide";
    const DESCRIPTION: &'static str =
        "How to create releases and deploy services: single-repo and monorepo, gh commands, re-deploy, troubleshooting";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for ReleaseGuideResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
