use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct DioxusUtilsResource;

impl ResourceDefinition for DioxusUtilsResource {
    const RESOURCE_URI: &'static str = "resource://dioxus-utils-readme";
    const RESOURCE_NAME: &'static str = "dioxus-utils Usage Cases Guide";
    const DESCRIPTION: &'static str = "Utilities for Dioxus apps: data state, dialogs, JS helpers";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusUtilsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/dioxus-utils/refs/heads/main/README.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
