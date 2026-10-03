# CLAUDE.md

## MCP Resources — MANDATORY

Before writing any code, you MUST list the available resource tools of the MyJetTools documentation MCP server — the server this repository builds. Call its `list_resource_tools` tool. The full tool name depends on how the server is connected in the session: with the project `.mcp.json` entry `dev-best-practices` it is `mcp__dev-best-practices__list_resource_tools`; through a claude.ai connector the prefix differs. Look for the tool whose name ends in `list_resource_tools`.

Identify which resources are relevant to your task, then call the matching `get_*` tool(s) of the same server to read them before proceeding. Never rely on memory for API signatures or patterns — always read fresh documentation.
