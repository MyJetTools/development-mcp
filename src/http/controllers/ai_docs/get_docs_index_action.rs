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
        filename: "app-bootstrap.md",
        name: "App Bootstrap Guide",
        description: "Step-by-step instructions for bootstrapping a new project using service-sdk: settings, AppContext, gRPC, HTTP, Service Bus, MyNoSql reader/writer",
    },
    DocEntry {
        filename: "cargo-dependencies-guide.md",
        name: "Cargo Dependencies Guide",
        description: "How to add dependencies to Cargo.toml",
    },
    DocEntry {
        filename: "mcp-development-guide.md",
        name: "MCP Development Guide",
        description: "Guide for creating MCP resources and Tool Calls",
    },
    DocEntry {
        filename: "http-actions-design-guide.md",
        name: "HTTP Actions Design Guide",
        description: "HTTP action architecture and patterns for my-http-server",
    },
    DocEntry {
        filename: "flurl-usage-guide.md",
        name: "FlUrl Usage Guide",
        description: "How to use the FlUrl HTTP client library",
    },
    DocEntry {
        filename: "dioxus-bootstrap.md",
        name: "Dioxus Fullstack Bootstrap Guide",
        description: "Bootstrap a new empty Dioxus fullstack web application",
    },
    DocEntry {
        filename: "dioxus-client-side-bootstrap.md",
        name: "Dioxus Client-Side Bootstrap Guide",
        description: "Bootstrap a new Dioxus client-side (WASM-only) web application with WebSocket and API calls",
    },
    DocEntry {
        filename: "dioxus-fullstack-design-patterns.md",
        name: "Dioxus Fullstack Design Patterns",
        description: "Playbook for dialogs, forms, lists, and server functions",
    },
    DocEntry {
        filename: "dioxus-utils-readme.md",
        name: "dioxus-utils Usage Cases Guide",
        description: "Utilities for Dioxus apps: data state, dialogs, JS helpers",
    },
    DocEntry {
        filename: "dioxus-admin-ui-kit.md",
        name: "Dioxus Admin UI Kit",
        description: "Ready-made UI components for Dioxus admin panels",
    },
    DocEntry {
        filename: "my-postgres-readme.md",
        name: "Postgres Design Library",
        description: "Documentation for my-postgres library",
    },
    DocEntry {
        filename: "my-no-sql-entity-design-patterns.md",
        name: "MyNoSql Entity Design Patterns",
        description: "Design patterns for MyNoSql entities and enums",
    },
    DocEntry {
        filename: "my-grpc-extensions.md",
        name: "gRPC extensions",
        description: "Utilities and macros for building gRPC clients and servers",
    },
    DocEntry {
        filename: "my-ssh-readme.md",
        name: "SSH connections design library",
        description: "Async SSH helpers for commands, file transfer, and port forwarding",
    },
    DocEntry {
        filename: "my-tcp-sockets-readme.md",
        name: "TcpSockets design library",
        description: "Async TCP server/client building blocks with ping/pong and TLS options",
    },
    DocEntry {
        filename: "my-web-socket-client-guide.md",
        name: "WebSocket Client Guide",
        description: "WebSocket client for non-WASM apps: auto-reconnect, heartbeat, callbacks, compression",
    },
    DocEntry {
        filename: "rust-extensions.md",
        name: "rust-extensions",
        description: "Low-level utils, queues and other helpers",
    },
    DocEntry {
        filename: "ci-utils-readme.md",
        name: "ci-utils",
        description: "Build-time helper crate",
    },
    DocEntry {
        filename: "release-guide.md",
        name: "Release Guide",
        description: "How to create releases and deploy services: single-repo and monorepo, gh commands, re-deploy, troubleshooting",
    },
    DocEntry {
        filename: "dioxus-design-patterns.md",
        name: "Dioxus Design Patterns",
        description: "Common patterns for all Dioxus projects: naming conventions, ComponentState, signals, forms, CSS",
    },
    DocEntry {
        filename: "application-architecture-best-practices.md",
        name: "Application Architecture Best Practices",
        description: "Complete coding standards: project structure, flows/scripts, gRPC, Postgres, Service Bus, HTTP actions, mappers, MyNoSql, settings, logging, error handling",
    },
];

pub(super) fn build_yaml(scheme: &str, host: &str) -> String {
    let mut yaml = String::from("resources:\n");
    for doc in DOCS {
        yaml.push_str(&format!(
            "  - url: \"{}://{}/ai-docs/{}\"\n    name: \"{}\"\n    description: \"{}\"\n",
            scheme, host, doc.filename, doc.name, doc.description
        ));
    }
    yaml
}

fn build_html() -> String {
    let mut rows = String::new();
    for doc in DOCS {
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
