use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct AppBootstrapResource;

impl AppBootstrapResource {
    pub const FILENAME: &'static str = "app-bootstrap.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/app-bootstrap.md";
    pub const TOOL_FN: &'static str = "get_app_bootstrap_guide";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch the app bootstrap guide: bootstrapping a new project, and the CI / GitHub Actions \
         setup — ci-utils + build.rs for single-repo, hand-written release workflows for monorepo, \
         plus the pre-baked builder image that cuts a monorepo release from ~10 min to ~2 min. \
         Load this before creating or editing any release workflow, Dockerfile or build.rs. Also \
         covers service-sdk feature flags, MyNoSql reader/writer wiring and the TLS feature.";
}

impl ResourceDefinition for AppBootstrapResource {
    const RESOURCE_URI: &'static str = "resource://app-bootstrap";
    const RESOURCE_NAME: &'static str = "App Bootstrap Guide";
    const DESCRIPTION: &'static str =
        "Bootstrapping a new project plus CI / GitHub Actions: ci-utils + build.rs for single-repo, \
         hand-written release-{service-name}.yaml for monorepo, and the pre-baked builder image \
         (build-{service-name}-docker.yaml) that makes releases ~2 minutes instead of ~10. Also \
         service-sdk feature flags, MyNoSql reader/writer wiring, TLS feature.";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for AppBootstrapResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
