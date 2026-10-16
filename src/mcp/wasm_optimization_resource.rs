use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct WasmOptimizationResource;

impl WasmOptimizationResource {
    pub const FILENAME: &'static str = "wasm-optimization.md";
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/wasm-optimization.md";
    pub const TOOL_FN: &'static str = "get_wasm_optimization_guide";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch the WASM optimization guide for Dioxus web bundles (client-side and the web half of fullstack): dx build --debug-symbols false, [profile.wasm-release] with opt-level \"z\" / lto / codegen-units 1 (not [profile.release]), no strip and no panic = \"abort\", wasm-opt -Oz and its out-of-memory case on CI, the latest stable toolchain, a build script that fails on a failed wasm-opt, how to verify the size. Load it before creating or changing the release build of any Dioxus project, and before every production release of one";
}

impl ResourceDefinition for WasmOptimizationResource {
    const RESOURCE_URI: &'static str = "resource://wasm-optimization";
    const RESOURCE_NAME: &'static str = "WASM Optimization";
    const DESCRIPTION: &'static str =
        "How to ship the smallest .wasm from a Dioxus project: the dx flags, the cargo profile dx really uses for the web build, wasm-opt, the toolchain, the build script and how to verify it - with measured numbers";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for WasmOptimizationResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
