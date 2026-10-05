use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct AppBootstrapResource;

impl ResourceDefinition for AppBootstrapResource {
    const RESOURCE_URI: &'static str = "resource://app-bootstrap";
    const RESOURCE_NAME: &'static str = "App Bootstrap Guide";
    const DESCRIPTION: &'static str =
        "Wiring up a service: CI / GitHub Actions (ci-utils + build.rs for single-repo, \
         hand-written release-{service-name}.yaml plus the pre-baked builder image \
         build-{service-name}-docker.yaml for monorepo), service-sdk feature flags, MyNoSql \
         reader/writer wiring, TLS. This resource is the index: the decisions to ask the user \
         about and the list of topics; each topic is resource://app-bootstrap/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for AppBootstrapResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for AppBootstrapResource {
    const FILENAME: &'static str = "app-bootstrap.md";
    const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/app-bootstrap/index_resource.md";
    const DOCS_URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/app-bootstrap";
    const TOOL_FN: &'static str = "get_app_bootstrap_guide";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch the app bootstrap guide. Without `topic` - the index: the CI and TLS decisions to \
         ask the user about and the list of topics. With `topic` - `ci-single-repo` (ci-utils + \
         build.rs), `ci-monorepo` (hand-written release workflows plus the pre-baked builder \
         image that cuts a release from ~10 min to ~2 min), `service-sdk-features`, \
         `my-no-sql` (reader/writer wiring), `tls`. Load the CI topic before creating or editing \
         any release workflow, Dockerfile or build.rs.";
    const TOPIC_URI_TEMPLATE: &'static str = "resource://app-bootstrap/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "app-bootstrap topic";
    const TOPIC_DESCRIPTION: &'static str =
        "One part of wiring up a service, with the exact templates and rules. The topics are \
         listed in resource://app-bootstrap";
    const TOPIC_NAME_PREFIX: &'static str = "app-bootstrap";
}

pub type AppBootstrapTopicResource = TopicResource<AppBootstrapResource>;
pub type AppBootstrapGuideTool = TopicsDocTool<AppBootstrapResource>;
