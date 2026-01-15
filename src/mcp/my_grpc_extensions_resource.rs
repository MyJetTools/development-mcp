use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyGrpcExtensionsResource;

impl ResourceDefinition for MyGrpcExtensionsResource {
    const RESOURCE_URI: &'static str = "resource://my-grpc-extensions.md";
    const RESOURCE_NAME: &'static str = "Grpc extensions";
    const DESCRIPTION: &'static str = "Utilities and macros for building gRPC clients and servers";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyGrpcExtensionsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/my-grpc-extensions/main/README.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
