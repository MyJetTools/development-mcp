use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct CiUtilsResource;

impl CiUtilsResource {
    pub const FILENAME: &'static str = "ci-utils-readme.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/ci-utils/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_ci_utils_readme";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch the ci-utils README: the build.rs helper crate — CiGenerator (generates Dockerfile \
         and .github/workflows/release.yaml for SINGLE-REPO services), ProtoFileBuilder (syncs and \
         compiles proto files) and CssCompiler. Monorepo services do NOT use CiGenerator — their \
         workflows are hand-written, see get_app_bootstrap_guide.";
}

impl ResourceDefinition for CiUtilsResource {
    const RESOURCE_URI: &'static str = "resource://ci-utils-readme";
    const RESOURCE_NAME: &'static str = "ci-utils for each project";
    const DESCRIPTION: &'static str =
        "build.rs helper crate: CiGenerator (Dockerfile + GitHub CI for single-repo services), \
         ProtoFileBuilder (proto sync + compile), CssCompiler. Not used for monorepo CI.";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for CiUtilsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
