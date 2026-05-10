use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MySshResource;

impl MySshResource {
    pub const FILENAME: &'static str = "my-ssh-readme.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-ssh/main/README.md";
    pub const TOOL_FN: &'static str = "get_my_ssh_readme";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch my-ssh README resource content";
}

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
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
