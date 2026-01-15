use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MySshResource;

impl ResourceDefinition for MySshResource {
    const RESOURCE_URI: &'static str = "resource://my-ssh-readme";
    const RESOURCE_NAME: &'static str = "Ssh connections design library";
    const DESCRIPTION: &'static str =
        "Async SSH helpers for commands, file transfer, and port forwarding.";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MySshResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/my-ssh/main/README.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
