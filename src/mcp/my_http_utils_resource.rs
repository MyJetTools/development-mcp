use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyHttpUtilsResource;

impl MyHttpUtilsResource {
    pub const FILENAME: &'static str = "my-http-utils-readme.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-http-utils/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_my_http_utils_readme";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch the my-http-utils README: describe an HTTP model once with derive macros and reuse \
         it across client and server. Wasm-compatible (no hyper/tokio). Derive macros \
         (MyHttpInput, MyHttpObjectStructure, MyHttpStringEnum / MyHttpIntegerEnum) and field \
         location markers (#[http_path], #[http_query], #[http_header], #[http_body], \
         #[http_form_data], #[http_body_raw]) generate client request builders \
         (THttpRequestBuilder, UrlBuilder, HttpRequestBody) and, behind the `server` feature, \
         server-side request parsing with OpenAPI/Swagger schema generation (THttpRequest, \
         HttpInputValue, HttpParseError).";
}

impl ResourceDefinition for MyHttpUtilsResource {
    const RESOURCE_URI: &'static str = "resource://my-http-utils-readme";
    const RESOURCE_NAME: &'static str = "my-http-utils HTTP Models Library";
    const DESCRIPTION: &'static str =
        "Library used to create HTTP models that are shared between client and server. Describe an \
         HTTP model once with derive macros and reuse the same contract to build client requests \
         and to parse server-side requests (with OpenAPI/Swagger schema generation). \
         Wasm-compatible, no hyper/tokio dependencies.";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyHttpUtilsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
