use flurl::FlUrl;
use mcp_server_middleware::{ResourceContent, ResourceReadResult};

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

    let content_str = response
        .get_body_as_str()
        .await
        .map_err(|e| format!("Failed to read response body from {}: {:?}", url, e))?;

    let content = ResourceContent {
        uri: uri.to_string(),
        mime_type: mime_type.to_string(),
        text: Some(content_str.to_string()),
        blob: None,
    };

    Ok(ResourceReadResult {
        contents: vec![content],
    })
}
