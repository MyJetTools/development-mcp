use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::AppContext;

struct DocEntry {
    filename: &'static str,
    tool: &'static str,
    name: &'static str,
    description: &'static str,
    when_to_use: &'static str,
}

const DOCS: &[DocEntry] = &[
    DocEntry {
        filename: "app-bootstrap.md",
        tool: "get_app_bootstrap_guide",
        name: "App Bootstrap Guide",
        description: "Step-by-step instructions for bootstrapping a new project",
        when_to_use: "Создание нового Rust микросервиса — layout проекта, AppContext, settings reader, startup wiring.",
    },
    DocEntry {
        filename: "cargo-dependencies-guide.md",
        tool: "get_cargo_dependencies_guide",
        name: "Cargo Dependencies Guide",
        description: "How to add dependencies to Cargo.toml",
        when_to_use: "Добавление или обновление зависимостей из экосистемы MyJetTools — git tags, feature flags, workspace patterns.",
    },
    DocEntry {
        filename: "mcp-development-guide.md",
        tool: "get_mcp_development_guide",
        name: "MCP Development Guide",
        description: "Guide for creating MCP resources and Tool Calls",
        when_to_use: "Разработка или расширение MCP сервера — ресурсы, tool calls, регистрация.",
    },
    DocEntry {
        filename: "http-actions-design-guide.md",
        tool: "get_http_actions_design_guide",
        name: "HTTP Actions Design Guide",
        description: "HTTP action architecture and patterns for my-http-server",
        when_to_use: "Добавление HTTP контроллеров — макрос #[http_route], input/output модели, типы ошибок, регистрация.",
    },
    DocEntry {
        filename: "flurl-usage-guide.md",
        tool: "get_flurl_usage_guide",
        name: "FlUrl Usage Guide",
        description: "How to use the FlUrl HTTP client library",
        when_to_use: "Исходящие HTTP/HTTPS запросы — FlUrl — стандартный async HTTP клиент MyJetTools.",
    },
    DocEntry {
        filename: "dioxus-bootstrap.md",
        tool: "get_dioxus_bootstrap_guide",
        name: "Dioxus Fullstack Bootstrap Guide",
        description: "Bootstrap a new empty Dioxus fullstack web application",
        when_to_use: "Создание нового Dioxus fullstack проекта — workspace, feature flags, entry points, routing.",
    },
    DocEntry {
        filename: "dioxus-fullstack-design-patterns.md",
        tool: "get_dioxus_fullstack_design_patterns",
        name: "Dioxus Fullstack Design Patterns",
        description: "Playbook for dialogs, forms, lists, and server functions",
        when_to_use: "Dioxus: диалоги, формы, списки, server functions в существующем приложении.",
    },
    DocEntry {
        filename: "dioxus-utils-readme.md",
        tool: "get_dioxus_utils_readme",
        name: "dioxus-utils Usage Cases Guide",
        description: "Utilities for Dioxus apps: data state, dialogs, JS helpers",
        when_to_use: "Dioxus: конкретный API библиотеки dioxus-utils — state wrappers, dialog hooks, JS interop.",
    },
    DocEntry {
        filename: "dioxus-admin-ui-kit.md",
        tool: "get_dioxus_admin_ui_kit",
        name: "Dioxus Admin UI Kit",
        description: "Ready-made UI components for Dioxus admin panels",
        when_to_use: "Dioxus: админ-панель — готовые typed inputs, sortable tables, enum selectors.",
    },
    DocEntry {
        filename: "my-postgres-readme.md",
        tool: "get_my_postgres_readme",
        name: "Postgres Design Library",
        description: "Documentation for my-postgres library",
        when_to_use: "PostgreSQL — connection pooling, query macros, bulk operations, entity model conventions.",
    },
    DocEntry {
        filename: "my-no-sql-entity-design-patterns.md",
        tool: "get_my_no_sql_entity_patterns",
        name: "MyNoSql Entity Design Patterns",
        description: "Design patterns for MyNoSql entities and enums",
        when_to_use: "MyNoSql таблицы — partition/row key, сериализация enum, версионирование.",
    },
    DocEntry {
        filename: "my-grpc-extensions.md",
        tool: "get_my_grpc_extensions_readme",
        name: "gRPC extensions",
        description: "Utilities and macros for building gRPC clients and servers",
        when_to_use: "gRPC клиент или сервер — макросы my-grpc-extensions, retry policies, channel management.",
    },
    DocEntry {
        filename: "my-ssh-readme.md",
        tool: "get_my_ssh_readme",
        name: "SSH connections design library",
        description: "Async SSH helpers for commands, file transfer, and port forwarding",
        when_to_use: "SSH — удалённые команды, SCP, SSH туннели программно.",
    },
    DocEntry {
        filename: "my-tcp-sockets-readme.md",
        tool: "get_my_tcp_sockets_readme",
        name: "TcpSockets design library",
        description: "Async TCP server/client building blocks with ping/pong and TLS options",
        when_to_use: "TCP сервер или клиент — lifecycle, ping/pong keep-alive, TLS.",
    },
    DocEntry {
        filename: "rust-extensions.md",
        tool: "get_rust_extensions_readme",
        name: "rust-extensions",
        description: "Low-level utils, queues and other helpers",
        when_to_use: "Background queues, lazy-init, date/time helpers и другие low-level примитивы MyJetTools.",
    },
    DocEntry {
        filename: "ci-utils-readme.md",
        tool: "get_ci_utils_readme",
        name: "ci-utils",
        description: "Build-time helper crate",
        when_to_use: "Build scripts (build.rs) — version stamping, git-hash, protobuf compilation.",
    },
];

pub(super) fn build_yaml() -> String {
    let mut yaml = String::from("resources:\n");
    for doc in DOCS {
        yaml.push_str(&format!(
            "  - tool: {}\n    when: \"{}\"\n",
            doc.tool, doc.when_to_use
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
    HttpOutput::as_html(build_html())
        .add_header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
        .into_ok_result(true)
        .into()
}
