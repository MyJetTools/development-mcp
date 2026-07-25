use std::sync::Arc;

use mcp_server_middleware::*;
use serde::{Deserialize, Serialize};

use crate::app::AppContext;
use crate::mcp::{all_doc_entries, get_doc_url_by_filename};
use crate::rag::fetch_doc;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetDocInputData {
    #[property(
        description = "Document filename as returned by search_docs or list_resource_tools, e.g. 'release-guide.md'"
    )]
    pub filename: String,

    #[property(
        description = "Optional heading to narrow to, e.g. 'Creating a Tool Call'. Matched case-insensitively against headings; the whole subtree under it is returned. Omit to get the full document."
    )]
    pub section: Option<String>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetDocResponse {
    #[property(description = "Document filename")]
    pub filename: String,

    #[property(description = "Markdown content of the document, or of the requested section")]
    pub text: String,

    #[property(description = "Set when the requested section could not be matched")]
    pub warning: Option<String>,
}

pub struct GetDocHandler {
    _app: Arc<AppContext>,
}

impl GetDocHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

impl ToolDefinition for GetDocHandler {
    const FUNC_NAME: &'static str = "get_doc";

    const DESCRIPTION: &'static str = "Read a MyJetTools development guide in full, by filename. \
         This is the depth companion to search_docs: search first to find out WHERE something is, \
         then call this when the returned fragment is not enough and you need the surrounding \
         rules, the full example, or the exceptions that did not make it into the fragment.\n\n\
         Pass `section` to get just one heading and everything nested under it. \
         Call list_resource_tools to see every available filename.";
}

#[async_trait::async_trait]
impl McpToolCall<GetDocInputData, GetDocResponse> for GetDocHandler {
    async fn execute_tool_call(&self, model: GetDocInputData) -> Result<GetDocResponse, String> {
        let filename = model.filename.trim();

        let Some(url) = get_doc_url_by_filename(filename) else {
            let available: Vec<&str> = all_doc_entries().iter().map(|it| it.filename).collect();

            return Err(format!(
                "Unknown document '{}'. Available: {}",
                filename,
                available.join(", ")
            ));
        };

        let text = fetch_doc(url).await?;

        let Some(section) = model.section else {
            return Ok(GetDocResponse {
                filename: filename.to_string(),
                text,
                warning: None,
            });
        };

        match extract_section(&text, &section) {
            Some(extracted) => Ok(GetDocResponse {
                filename: filename.to_string(),
                text: extracted,
                warning: None,
            }),
            None => Ok(GetDocResponse {
                filename: filename.to_string(),
                text,
                warning: Some(format!(
                    "Section '{}' was not found - returning the whole document instead.",
                    section
                )),
            }),
        }
    }
}

/// Returns the requested heading together with everything nested under it, up
/// to the next heading of the same or higher level.
fn extract_section(content: &str, section: &str) -> Option<String> {
    let needle = section.trim().to_lowercase();

    let mut result: Vec<&str> = Vec::new();
    let mut capturing_at_level: Option<usize> = None;
    let mut inside_fence = false;

    for line in content.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            inside_fence = !inside_fence;

            if capturing_at_level.is_some() {
                result.push(line);
            }

            continue;
        }

        if inside_fence {
            if capturing_at_level.is_some() {
                result.push(line);
            }

            continue;
        }

        let level = heading_level(trimmed);

        if let Some(level) = level {
            let title = trimmed
                .trim_start_matches('#')
                .trim()
                .trim_end_matches('#')
                .trim()
                .to_lowercase();

            if let Some(open_level) = capturing_at_level {
                if level <= open_level {
                    break;
                }
            } else if title == needle || title.contains(&needle) {
                capturing_at_level = Some(level);
                result.push(line);
                continue;
            }
        }

        if capturing_at_level.is_some() {
            result.push(line);
        }
    }

    if result.is_empty() {
        return None;
    }

    Some(result.join("\n"))
}

fn heading_level(trimmed: &str) -> Option<usize> {
    if !trimmed.starts_with('#') {
        return None;
    }

    let level = trimmed.chars().take_while(|c| *c == '#').count();

    if level == 0 || level > 6 {
        return None;
    }

    match trimmed.chars().nth(level) {
        Some(' ') => Some(level),
        _ => None,
    }
}
