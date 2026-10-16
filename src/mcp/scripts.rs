use flurl::{FlUrl, FlUrlError, FlUrlResponse};
use mcp_server_middleware::{ResourceContent, ResourceReadResult};

/// The largest doc read into memory. A doc is a few dozen KB - a body this big is not one.
pub const MAX_DOC_SIZE: usize = 4 * 1024 * 1024;

/// Reads the whole body of a doc response as text.
pub async fn read_doc_body(response: &mut FlUrlResponse) -> Result<String, FlUrlError> {
    response.get_body()?.into_string(MAX_DOC_SIZE).await
}

/// Fetches a text resource over HTTP and wraps it into an MCP `ResourceReadResult`.
pub async fn load_resource_by_http(
    uri: &str,
    mime_type: &str,
    url: &str,
) -> Result<ResourceReadResult, String> {
    let mut response = FlUrl::new(url)
        .get()
        .await
        .map_err(|e| format!("Failed to fetch resource from {}: {:?}", url, e))?;

    let content_str = read_doc_body(&mut response)
        .await
        .map_err(|e| format!("Failed to read response body from {}: {:?}", url, e))?;

    let content = ResourceContent {
        uri: uri.to_string(),
        mime_type: mime_type.to_string(),
        text: Some(content_str),
        blob: None,
    };

    Ok(ResourceReadResult {
        contents: vec![content],
    })
}
