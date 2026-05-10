use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyAiAgentResource;

impl MyAiAgentResource {
    pub const FILENAME: &'static str = "my-ai-agent-readme.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/my-ai-utils/my-ai-agent/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_my_ai_agent_readme";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch my-ai-agent README: building AI agents with chat completions, local and remote tool execution, streaming, multi-vendor LLM support";
}

impl ResourceDefinition for MyAiAgentResource {
    const RESOURCE_URI: &'static str = "resource://my-ai-agent-readme";
    const RESOURCE_NAME: &'static str = "my-ai-agent";
    const DESCRIPTION: &'static str =
        "Rust toolkit for building AI agents: chat completions, local and remote tool execution, streaming, multi-vendor LLM support (OpenAI, Nebius, Z.ai, Fireworks, Cerebras)";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyAiAgentResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
