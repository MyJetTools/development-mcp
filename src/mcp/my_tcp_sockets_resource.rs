use flurl::FlUrl;
use mcp_server_middleware::*;

pub struct MyTcpSocketsResource;

impl ResourceDefinition for MyTcpSocketsResource {
    const RESOURCE_URI: &'static str = "resource://tcp-sockets-design-library";
    const RESOURCE_NAME: &'static str = "TcpSockets design library";
    const DESCRIPTION: &'static str =
        "Async TCP server/client building blocks with ping/pong and TLS options";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyTcpSocketsResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/my-tcp-sockets/refs/heads/main/README.md";

        // Fetch the My TCP Sockets README content using FlUrl
        let mut response = FlUrl::new(README_URL)
            .get()
            .await
            .map_err(|e| format!("Failed to fetch My TCP Sockets README: {:?}", e))?;

        let content_str = response
            .get_body_as_str()
            .await
            .map_err(|e| format!("Failed to read response body: {:?}", e))?;

        let result = ResourceContent {
            uri: Self::RESOURCE_URI.to_string(),
            mime_type: Self::MIME_TYPE.to_string(),
            text: Some(content_str.to_string()),
            blob: None,
        };

        Ok(ResourceReadResult {
            contents: vec![result],
        })
    }
}
