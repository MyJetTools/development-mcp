use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::AppContext;

struct DocEntry {
    filename: &'static str,
    name: &'static str,
    description: &'static str,
    when_to_use: &'static str,
}

const DOCS: &[DocEntry] = &[
    DocEntry {
        filename: "app-bootstrap.md",
        name: "App Bootstrap Guide",
        description: "Step-by-step instructions for bootstrapping a new project",
        when_to_use: "Read this first when starting a brand-new Rust microservice. Covers project layout, AppContext, settings reader, and startup wiring.",
    },
    DocEntry {
        filename: "cargo-dependencies-guide.md",
        name: "Cargo Dependencies Guide",
        description: "How to add dependencies to Cargo.toml",
        when_to_use: "Consult whenever you need to add or update a crate from the MyJetTools ecosystem — correct git tags, feature flags, and workspace patterns.",
    },
    DocEntry {
        filename: "mcp-development-guide.md",
        name: "MCP Development Guide",
        description: "Guide for creating MCP resources and Tool Calls",
        when_to_use: "Read when building or extending this MCP server itself — how to define resources, tool calls, and register them.",
    },
    DocEntry {
        filename: "http-actions-design-guide.md",
        name: "HTTP Actions Design Guide",
        description: "HTTP action architecture and patterns for my-http-server",
        when_to_use: "Read when adding HTTP controllers: the #[http_route] macro, input models (MyHttpInput), output models (MyHttpObjectStructure), error types, and controller registration.",
    },
    DocEntry {
        filename: "flurl-usage-guide.md",
        name: "FlUrl Usage Guide",
        description: "How to use the FlUrl HTTP client library",
        when_to_use: "Read when making outbound HTTP/HTTPS requests — FlUrl is the standard async HTTP client used across MyJetTools projects.",
    },
    DocEntry {
        filename: "dioxus-bootstrap.md",
        name: "Dioxus Fullstack Bootstrap Guide",
        description: "Bootstrap a new empty Dioxus fullstack web application",
        when_to_use: "Read at the start of a new Dioxus fullstack project. Covers workspace setup, feature flags, entry points, and initial routing.",
    },
    DocEntry {
        filename: "dioxus-fullstack-design-patterns.md",
        name: "Dioxus Fullstack Design Patterns",
        description: "Playbook for dialogs, forms, lists, and server functions",
        when_to_use: "Read when implementing UI features in an existing Dioxus app — standard patterns for server functions, dialogs, reactive lists, and form handling.",
    },
    DocEntry {
        filename: "dioxus-utils-readme.md",
        name: "dioxus-utils Usage Cases Guide",
        description: "Utilities for Dioxus apps: data state, dialogs, JS helpers",
        when_to_use: "Read when you need the concrete helper crate behind Dioxus patterns — use-state wrappers, dialog open/close hooks, and JS interop calls. Complements dioxus-fullstack-design-patterns, which covers the patterns; this covers the library API that implements them.",
    },
    DocEntry {
        filename: "dioxus-admin-ui-kit.md",
        name: "Dioxus Admin UI Kit",
        description: "Ready-made UI components for Dioxus admin panels",
        when_to_use: "Read when building an internal admin or back-office UI — provides pre-built typed text inputs, sortable table components, and enum dropdown selectors. Use instead of hand-rolling form controls described in dioxus-fullstack-design-patterns.",
    },
    DocEntry {
        filename: "my-postgres-readme.md",
        name: "Postgres Design Library",
        description: "Documentation for my-postgres library",
        when_to_use: "Read when integrating PostgreSQL — covers connection pooling, query macros, bulk operations, and the entity model conventions used in MyJetTools projects.",
    },
    DocEntry {
        filename: "my-no-sql-entity-design-patterns.md",
        name: "MyNoSql Entity Design Patterns",
        description: "Design patterns for MyNoSql entities and enums",
        when_to_use: "Read when designing or modifying MyNoSql table entities — partition key / row key conventions, enum serialization, and versioning patterns.",
    },
    DocEntry {
        filename: "my-grpc-extensions.md",
        name: "gRPC extensions",
        description: "Utilities and macros for building gRPC clients and servers",
        when_to_use: "Read when adding a gRPC client or server — covers the my-grpc-extensions macros, retry policies, and channel management.",
    },
    DocEntry {
        filename: "my-ssh-readme.md",
        name: "SSH connections design library",
        description: "Async SSH helpers for commands, file transfer, and port forwarding",
        when_to_use: "Read when the service needs to run remote shell commands, transfer files via SCP, or open SSH tunnels programmatically.",
    },
    DocEntry {
        filename: "my-tcp-sockets-readme.md",
        name: "TcpSockets design library",
        description: "Async TCP server/client building blocks with ping/pong and TLS options",
        when_to_use: "Read when implementing a raw TCP server or client — covers connection lifecycle, ping/pong keep-alive, and optional TLS wrapping.",
    },
    DocEntry {
        filename: "rust-extensions.md",
        name: "rust-extensions",
        description: "Low-level utils, queues and other helpers",
        when_to_use: "Read when you need background queues, lazy-init containers, date/time helpers, or other low-level primitives from the MyJetTools base layer.",
    },
    DocEntry {
        filename: "ci-utils-readme.md",
        name: "ci-utils",
        description: "Build-time helper crate",
        when_to_use: "Read when setting up build scripts (build.rs) — provides version stamping, git-hash embedding, and protobuf compilation helpers used in CI pipelines.",
    },
];

fn build_html() -> String {
    let mut rows = String::new();
    for doc in DOCS {
        rows.push_str(&format!(
            r#"<tr>
          <td><a href="/ai-docs/{filename}">{filename}</a></td>
          <td><strong>{name}</strong><br><span class="desc">{description}</span></td>
          <td>{when_to_use}</td>
        </tr>"#,
            filename = doc.filename,
            name = doc.name,
            description = doc.description,
            when_to_use = doc.when_to_use,
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
    .desc {{ color: #868e96; font-size: .88em; margin-top: 2px; display: block; }}
    td:last-child {{ color: #495057; line-height: 1.55; }}
    col.col-url  {{ width: 26%; }}
    col.col-name {{ width: 26%; }}
    col.col-when {{ width: 48%; }}
  </style>
</head>
<body>
  <h1>AI Docs</h1>
  <p class="subtitle">Development best-practice documents served by this MCP server.<br>
  Use the <em>When to use</em> column to pick the right document before fetching it.</p>
  <table>
    <colgroup>
      <col class="col-url">
      <col class="col-name">
      <col class="col-when">
    </colgroup>
    <thead>
      <tr>
        <th>URL</th>
        <th>Document</th>
        <th>When to use</th>
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
    HttpOutput::as_html(build_html()).into_ok_result(true).into()
}
