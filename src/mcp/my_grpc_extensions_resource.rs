use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyGrpcExtensionsResource;

impl MyGrpcExtensionsResource {
    pub const FILENAME: &'static str = "my-grpc-extensions.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-grpc-extensions/main/README.md";
    pub const TOOL_FN: &'static str = "get_my_grpc_extensions_readme";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch my-grpc-extensions README resource content";
}

impl ResourceDefinition for MyGrpcExtensionsResource {
    const RESOURCE_URI: &'static str = "resource://my-grpc-extensions.md";
    const RESOURCE_NAME: &'static str = "Grpc extensions";
    const DESCRIPTION: &'static str = "Utilities and macros for building gRPC clients and servers";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyGrpcExtensionsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
