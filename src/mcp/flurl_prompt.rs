use std::collections::HashMap;

use flurl::FlUrl;
use mcp_server_middleware::*;

pub struct FlUrlPrompt;

impl PromptDefinition for FlUrlPrompt {
    const PROMPT_NAME: &'static str = "flurl_usage";
    const DESCRIPTION: &'static str = "How to use FlUrl library";
    fn get_argument_descriptions() -> Vec<PromptArgumentDescription> {
        vec![]
    }
}

#[async_trait::async_trait]
impl McpPromptService for FlUrlPrompt {
    async fn execute_prompt(
        &self,
        _model: &HashMap<String, String>,
    ) -> Result<PromptExecutionResult, String> {
        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/fl-url/refs/heads/main/README.md";

        // Fetch the README content using FlUrl
        let mut response = FlUrl::new(README_URL)
            .get()
            .await
            .map_err(|e| format!("Failed to fetch FlUrl README: {:?}", e))?;

        let content = response
            .get_body_as_str()
            .await
            .map_err(|e| format!("Failed to read response body: {:?}", e))?;

        let result = PromptExecutionResult {
            description: "How to use FlUrl library".to_string(),
            message: content.to_string(),
        };

        Ok(result)
    }
}
