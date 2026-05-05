use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct DioxusAdminUiKitResource;

impl DioxusAdminUiKitResource {
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/dioxus-admin-ui-kit/main/README.md";
    pub const TOOL_FN: &'static str = "get_dioxus_admin_ui_kit";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch Dioxus Admin UI Kit README resource content";
}

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
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
