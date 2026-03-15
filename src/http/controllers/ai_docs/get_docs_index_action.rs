use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::AppContext;

struct DocEntry {
    filename: &'static str,
    name: &'static str,
    description: &'static str,
}

const DOCS: &[DocEntry] = &[
    DocEntry {
        filename: "mcp-development-guide.md",
        name: "MCP Development Guide",
        description: "Guide for creating Prompts and Tool Calls",
    },
    DocEntry {
        filename: "flurl-usage-guide.md",
        name: "FlUrl Usage Guide",
        description: "How to use FlUrl library",
    },
    DocEntry {
        filename: "http-actions-design-guide.md",
        name: "HTTP Actions Design Guide",
        description: "Guide for HTTP action architecture and patterns",
    },
    DocEntry {
        filename: "app-bootstrap.md",
        name: "App Bootstrap Guide",
        description: "Step-by-step instructions for bootstrapping a new project",
    },
    DocEntry {
        filename: "dioxus-bootstrap.md",
        name: "Dioxus Fullstack Bootstrap Guide",
        description: "Step-by-step instructions for bootstrapping a new empty Dioxus fullstack web application",
    },
    DocEntry {
        filename: "cargo-dependencies-guide.md",
        name: "Cargo Dependencies Guide",
        description: "How to add dependencies to Cargo.toml",
    },
    DocEntry {
        filename: "my-ssh-readme.md",
        name: "Ssh connections design library",
        description: "Async SSH helpers for commands, file transfer, and port forwarding",
    },
    DocEntry {
        filename: "my-tcp-sockets-readme.md",
        name: "TcpSockets design library",
        description: "Async TCP server/client building blocks with ping/pong and TLS options",
    },
    DocEntry {
        filename: "rust-extensions.md",
        name: "rust-extensions",
        description: "Low-level utils, queues and other helpers to glue together Rust code",
    },
    DocEntry {
        filename: "dioxus-fullstack-design-patterns.md",
        name: "Dioxus Fullstack Design Patterns",
        description: "Project playbook for dialogs, forms, lists, and server functions",
    },
    DocEntry {
        filename: "my-no-sql-entity-design-patterns.md",
        name: "MyNoSql Entity Design Patterns",
        description: "Design patterns for MyNoSql entities and enums",
    },
    DocEntry {
        filename: "my-grpc-extensions.md",
        name: "Grpc extensions",
        description: "Utilities and macros for building gRPC clients and servers",
    },
    DocEntry {
        filename: "dioxus-utils-readme.md",
        name: "dioxus-utils Usage Cases Guide",
        description: "Utilities for Dioxus apps: data state, dialogs, JS helpers",
    },
    DocEntry {
        filename: "ci-utils-readme.md",
        name: "ci-utils",
        description: "Utility crate for build-time helpers",
    },
    DocEntry {
        filename: "my-postgres-readme.md",
        name: "Postgres Design Library",
        description: "Documentation for my-postgres library",
    },
    DocEntry {
        filename: "dioxus-admin-ui-kit.md",
        name: "Dioxus Admin UI Kit",
        description: "UI components for Dioxus admin apps: typed inputs, table rendering, enum selectors",
    },
];

fn build_html() -> String {
    let mut rows = String::new();
    for doc in DOCS {
        rows.push_str(&format!(
            r#"<tr>
          <td><a href="/ai-docs/{filename}">{filename}</a></td>
          <td>{name}</td>
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
      max-width: 960px;
      margin: 40px auto;
      padding: 0 20px;
      background: #f8f9fa;
      color: #212529;
    }}
    h1 {{ margin-bottom: 8px; }}
    p.subtitle {{ color: #6c757d; margin-top: 0; margin-bottom: 28px; }}
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
    }}
    td {{
      padding: 11px 16px;
      border-bottom: 1px solid #e9ecef;
      vertical-align: top;
    }}
    tr:last-child td {{ border-bottom: none; }}
    tr:hover td {{ background: #f1f3f5; }}
    a {{ color: #0d6efd; text-decoration: none; font-family: monospace; font-size: .9em; }}
    a:hover {{ text-decoration: underline; }}
    td:nth-child(2) {{ font-weight: 500; white-space: nowrap; }}
    td:nth-child(3) {{ color: #495057; }}
  </style>
</head>
<body>
  <h1>AI Docs</h1>
  <p class="subtitle">Development best-practice documents served by this MCP server</p>
  <table>
    <thead>
      <tr>
        <th>URL</th>
        <th>Name</th>
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
    description: "Returns an HTML page listing all available markdown documents",
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
    HttpOutput::as_html(build_html()).into_ok_result(true).into()
}
