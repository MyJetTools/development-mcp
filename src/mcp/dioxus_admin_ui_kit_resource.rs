use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct DioxusAdminUiKitResource;

impl ResourceDefinition for DioxusAdminUiKitResource {
    const RESOURCE_URI: &'static str = "resource://dioxus-admin-ui-kit";
    const RESOURCE_NAME: &'static str = "Dioxus Admin UI Kit";
    const DESCRIPTION: &'static str =
        "UI components for Dioxus admin apps: typed inputs, table rendering, enum selectors";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusAdminUiKitResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/dioxus-admin-ui-kit/main/README.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
