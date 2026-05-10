use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::AppContext;
use crate::mcp::all_doc_entries;

pub(super) fn build_yaml(scheme: &str, host: &str) -> String {
    let mut yaml = String::from("resources:\n");
    for doc in all_doc_entries() {
        yaml.push_str(&format!(
            "  - url: \"{}://{}/ai-docs/{}\"\n    name: \"{}\"\n    description: \"{}\"\n",
            scheme, host, doc.filename, doc.name, doc.description
        ));
    }
    yaml
}

fn build_html() -> String {
    let mut rows = String::new();
    for doc in all_doc_entries() {
        rows.push_str(&format!(
            r#"<tr>
          <td><a href="/ai-docs/{filename}">{filename}</a></td>
          <td><strong>{name}</strong></td>
          <td>{description}</td>
        </tr>"#,
            filename = doc.filename,
            name = doc.name,
            description = doc.description,
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>AI Docs</title>
  <style>
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      max-width: 1100px;
      margin: 40px auto;
      padding: 0 20px;
      background: #f8f9fa;
      color: #212529;
    }}
    h1 {{ margin-bottom: 6px; }}
    p.subtitle {{ color: #6c757d; margin-top: 0; margin-bottom: 28px; font-size: .97em; }}
    table {{
      width: 100%;
      border-collapse: collapse;
      background: #fff;
      border-radius: 8px;
      overflow: hidden;
      box-shadow: 0 1px 4px rgba(0,0,0,.1);
    }}
    th {{
      background: #343a40;
      color: #fff;
      text-align: left;
      padding: 12px 16px;
      font-weight: 600;
      font-size: .88em;
      text-transform: uppercase;
      letter-spacing: .04em;
    }}
    td {{
      padding: 12px 16px;
      border-bottom: 1px solid #e9ecef;
      vertical-align: top;
      font-size: .93em;
    }}
    tr:last-child td {{ border-bottom: none; }}
    tr:hover td {{ background: #f1f3f5; }}
    a {{
      color: #0d6efd;
      text-decoration: none;
      font-family: "SFMono-Regular", Consolas, monospace;
      font-size: .85em;
      white-space: nowrap;
    }}
    a:hover {{ text-decoration: underline; }}
    td:last-child {{ color: #495057; line-height: 1.55; }}
    col.col-url  {{ width: 30%; }}
    col.col-name {{ width: 25%; }}
    col.col-desc {{ width: 45%; }}
  </style>
</head>
<body>
  <h1>AI Docs</h1>
  <p class="subtitle">Development best-practice documents served by this MCP server.</p>
  <table>
    <colgroup>
      <col class="col-url">
      <col class="col-name">
      <col class="col-desc">
    </colgroup>
    <thead>
      <tr>
        <th>URL</th>
        <th>Document</th>
        <th>Description</th>
      </tr>
    </thead>
    <tbody>
      {rows}
    </tbody>
  </table>
</body>
</html>"#,
        rows = rows
    )
}

#[http_route(
    method: "GET",
    route: "/ai-docs",
    controller: "AiDocs",
    summary: "List all AI documentation",
    description: "Returns an HTML page listing all available markdown documents with usage guidance",
    result: [
        {status_code: 200, description: "HTML index page"},
    ]
)]
pub struct GetAiDocsIndexAction {
    _app: Arc<AppContext>,
}

impl GetAiDocsIndexAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

async fn handle_request(
    _action: &GetAiDocsIndexAction,
    _ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    HttpOutput::as_html(build_html())
        .add_header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
        .into_ok_result(true)
        .into()
}
