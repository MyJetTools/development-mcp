use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct SingleVmUnixSocketResource;

impl SingleVmUnixSocketResource {
    pub const FILENAME: &'static str = "single-vm-unix-socket-setup.md";
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/single-vm-unix-socket-setup.md";
    pub const TOOL_FN: &'static str = "get_single_vm_unix_socket_setup";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch the Single-VM Unix-Socket Setup guide: how microservices on one host are wired together via unix sockets — system vs product scopes, the 4 standard volume mounts, SETTINGS_URL convention, compose templates for background-worker and gateway services, multi-product isolation rules.";
}

impl ResourceDefinition for SingleVmUnixSocketResource {
    const RESOURCE_URI: &'static str = "resource://single-vm-unix-socket-setup";
    const RESOURCE_NAME: &'static str = "Single-VM Unix-Socket Setup";
    const DESCRIPTION: &'static str =
        "Deployment convention for wiring microservices via unix sockets when the system runs on one host: directory layout (~/unix-sockets/system + ~/unix-sockets/<product>), the 4 standard volume mounts, SETTINGS_URL over unix, UNIX_SOCKET env, compose templates, and anti-patterns.";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for SingleVmUnixSocketResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
