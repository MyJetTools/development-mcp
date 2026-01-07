use mcp_server_middleware::*;

pub struct McpResource;

impl ResourceDefinition for McpResource {
    const RESOURCE_URI: &'static str = "resource://mcp-development-guide";
    const RESOURCE_NAME: &'static str = "MCP Development Guide";
    const DESCRIPTION: &'static str = "Guide for creating Prompts and Tool Calls";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for McpResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        let content = ResourceContent {
            uri: Self::RESOURCE_URI.to_string(),
            mime_type: Self::MIME_TYPE.to_string(),
            text: Some(PROMPT_DATA.to_string()),
            blob: None,
        };

        Ok(ResourceReadResult {
            contents: vec![content],
        })
    }
}

const PROMPT_DATA: &'static str = r##"
# MCP Middleware Guide: Creating Prompts and Tool Calls

This guide explains how to create prompts and tool calls for the MCPMiddleware based on the existing codebase structure.

## Overview

The MCPMiddleware uses two main types of components:
1. **Tool Calls** - Executable functions that can be called by the MCP client
2. **Prompts** - Predefined prompt templates that provide information or instructions

## Architecture

```
src/mcp/
├── mod.rs                                    # Exports all MCP components
├── prompt_definition.rs                      # Example prompt implementation
└── {tool_name}_tool_call.rs                  # Example tool call files
```

Registration happens in `src/http/start_up.rs` where all tools and prompts are registered with the middleware.

---

## Creating a Tool Call

### Step 1: Create the Tool Call File

Create a new file in `src/mcp/` following the naming pattern: `{tool_name}_tool_call.rs`

### Step 2: Define Input and Output Structures

Use `ApplyJsonSchema` derive macro to automatically generate JSON schema for the MCP protocol:

```rust
use mcp_server_middleware::*;
use my_ai_agent::{macros::*, *};
use serde::*;
use std::sync::Arc;
use crate::app::AppContext;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct YourToolInputData {
    #[property(description = "Description of the parameter")]
    pub parameter_name: String,
    
    #[property(description = "Another parameter")]
    pub another_param: Option<i32>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct YourToolResponse {
    #[property(description = "Result description")]
    pub result: String,
    
    #[property(description = "Status code")]
    pub status: i32,
}
```

**Key Points:**
- Use `#[property(description = "...")]` to document each field (this becomes part of the MCP schema)
- Use `Option<T>` for optional parameters
- Both input and output must derive `ApplyJsonSchema`, `Serialize`, `Deserialize`, and `Debug`

### Step 3: Create the Handler Struct

```rust
pub struct YourToolHandler {
    app: Arc<AppContext>,  // Use _app if you don't need it
}

impl YourToolHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}
```

### Step 4: Implement ToolDefinition Trait

```rust
impl ToolDefinition for YourToolHandler {
    const FUNC_NAME: &'static str = "your_tool_name";
    const DESCRIPTION: &'static str = "Clear description of what this tool does";
}
```

**Key Points:**
- `FUNC_NAME` is the identifier used by MCP clients to call your tool
- `DESCRIPTION` should be clear and concise

### Step 5: Implement McpToolCall Trait

```rust
#[async_trait::async_trait]
impl McpToolCall<YourToolInputData, YourToolResponse> for YourToolHandler {
    async fn execute_tool_call(
        &self,
        model: YourToolInputData,
    ) -> Result<YourToolResponse, String> {
        println!("Executing tool with params: {:?}", model);
        
        // Your implementation here
        // Access app context: self.app.app_states, etc.
        
        let result = YourToolResponse {
            result: "Success".to_string(),
            status: 200,
        };
        
        Ok(result)
    }
}
```

**Key Points:**
- The trait is async, so you can perform async operations
- Return `Ok(YourToolResponse)` on success
- Return `Err(String)` on failure (the error message will be sent to the client)
- You have access to `self.app` for accessing app state, etc.

### Step 6: Export in mod.rs

Add to `src/mcp/mod.rs`:

```rust
mod your_tool_call;
pub use your_tool_call::*;
```

### Step 7: Register in start_up.rs

Add to `src/http/start_up.rs`:

```rust
use crate::mcp::{YourToolHandler, /* ... other imports */};

// In the start function:
mcp.register_tool_call(Arc::new(YourToolHandler::new(app.clone())))
    .await;
```

---

## Creating a Prompt

### Step 1: Create the Prompt File

Create a new file in `src/mcp/` following the naming pattern: `{prompt_name}_prompt.rs` or add to `prompt_definition.rs`

### Step 2: Implement PromptDefinition Trait

```rust
use mcp_server_middleware::*;
use std::collections::HashMap;

pub struct YourPromptHandler;

impl PromptDefinition for YourPromptHandler {
    const PROMPT_NAME: &'static str = "your_prompt_name";
    const DESCRIPTION: &'static str = "Description of what this prompt provides";
    
    fn get_argument_descriptions() -> Vec<PromptArgumentDescription> {
        vec![
            PromptArgumentDescription {
                name: "param1".to_string(),
                description: "Description of param1".to_string(),
                required: true,
            },
            PromptArgumentDescription {
                name: "param2".to_string(),
                description: "Description of param2".to_string(),
                required: false,
            },
        ]
    }
}
```

**Key Points:**
- `PROMPT_NAME` is the identifier used by MCP clients
- `DESCRIPTION` explains what information/instructions the prompt provides
- `get_argument_descriptions()` defines optional parameters the prompt can accept

### Step 3: Implement McpPromptService Trait

```rust
#[async_trait::async_trait]
impl McpPromptService for YourPromptHandler {
    async fn execute_prompt(
        &self,
        model: &HashMap<String, String>,
    ) -> Result<PromptExecutionResult, String> {
        // Access arguments from model if needed
        let param1 = model.get("param1");
        
        // Build your prompt content
        let prompt_content = format!(
            r#"
# Your Prompt Title

## Section 1
Content here...

## Section 2
More content...
"#
        );
        
        let result = PromptExecutionResult {
            description: "What this prompt provides".to_string(),
            message: prompt_content,
        };
        
        Ok(result)
    }
}
```

**Key Points:**
- `model` contains the arguments passed by the client (if any)
- Return a `PromptExecutionResult` with:
  - `description`: Brief description
  - `message`: The actual prompt content (can be markdown formatted)

### Step 4: Export in mod.rs

Add to `src/mcp/mod.rs`:

```rust
mod your_prompt;
pub use your_prompt::*;
```

### Step 5: Register in start_up.rs

Add to `src/http/start_up.rs`:

```rust
use crate::mcp::{YourPromptHandler, /* ... other imports */};

// In the start function:
mcp.register_prompt(Arc::new(YourPromptHandler)).await;
```

---

## Complete Example: Tool Call

Here's a complete example of a simple tool call:

```rust
// src/mcp/get_user_info_tool_call.rs
use std::sync::Arc;
use mcp_server_middleware::*;
use my_ai_agent::{macros::*, *};
use serde::*;
use crate::app::AppContext;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetUserInfoInputData {
    #[property(description = "User ID to retrieve information for")]
    pub user_id: String,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct GetUserInfoResponse {
    #[property(description = "User name")]
    pub name: String,
    
    #[property(description = "User email")]
    pub email: String,
    
    #[property(description = "Account creation date")]
    pub created_at: String,
}

pub struct GetUserInfoHandler {
    app: Arc<AppContext>,
}

impl GetUserInfoHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for GetUserInfoHandler {
    const FUNC_NAME: &'static str = "get_user_info";
    const DESCRIPTION: &'static str = "Retrieves user information by user ID";
}

#[async_trait::async_trait]
impl McpToolCall<GetUserInfoInputData, GetUserInfoResponse> for GetUserInfoHandler {
    async fn execute_tool_call(
        &self,
        model: GetUserInfoInputData,
    ) -> Result<GetUserInfoResponse, String> {
        println!("Getting user info for: {}", model.user_id);
        
        // Your implementation here
        // Example: query database, call API, etc.
        
        let result = GetUserInfoResponse {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        
        Ok(result)
    }
}
```

---

## Complete Example: Prompt

Here's a complete example of a prompt:

```rust
// src/mcp/user_guide_prompt.rs
use std::collections::HashMap;
use mcp_server_middleware::*;

pub struct UserGuidePromptHandler;

impl PromptDefinition for UserGuidePromptHandler {
    const PROMPT_NAME: &'static str = "user_guide";
    const DESCRIPTION: &'static str = "Provides a comprehensive guide for users";
    
    fn get_argument_descriptions() -> Vec<PromptArgumentDescription> {
        vec![
            PromptArgumentDescription {
                name: "topic".to_string(),
                description: "Specific topic to focus on (optional)".to_string(),
                required: false,
            },
        ]
    }
}

#[async_trait::async_trait]
impl McpPromptService for UserGuidePromptHandler {
    async fn execute_prompt(
        &self,
        model: &HashMap<String, String>,
    ) -> Result<PromptExecutionResult, String> {
        let topic = model.get("topic");
        
        let content = if let Some(topic) = topic {
            format!(
                r#"
# User Guide: {}

## Overview
This guide covers {} in detail.

## Getting Started
1. First step
2. Second step
3. Third step

## Advanced Topics
- Advanced feature 1
- Advanced feature 2
"#,
                topic, topic
            )
        } else {
            r#"
# User Guide

## General Information
This is a comprehensive user guide covering all topics.

## Available Topics
- Topic 1
- Topic 2
- Topic 3
"#
            .to_string()
        };
        
        let result = PromptExecutionResult {
            description: "Comprehensive user guide".to_string(),
            message: content,
        };
        
        Ok(result)
    }
}
```

---

## Registration Order

In `start_up.rs`, the registration order is:

1. Create `McpMiddleware` instance
2. Register all tool calls using `register_tool_call()`
3. Register all prompts using `register_prompt()`
4. Add middleware to HTTP server

```rust
let mut mcp = McpMiddleware::new(
    "/",
    crate::app::APP_NAME,
    crate::app::APP_VERSION,
    "Server description",
);

// Register tools
mcp.register_tool_call(Arc::new(Tool1Handler::new(app.clone()))).await;
mcp.register_tool_call(Arc::new(Tool2Handler::new(app.clone()))).await;

// Register prompts
mcp.register_prompt(Arc::new(Prompt1Handler)).await;
mcp.register_prompt(Arc::new(Prompt2Handler)).await;

http_server.add_middleware(Arc::new(mcp));
```

---

## Best Practices

1. **Naming Conventions:**
   - Tool files: `{snake_case_name}_tool_call.rs`
   - Prompt files: `{snake_case_name}_prompt.rs` or add to `prompt_definition.rs`
   - Handler structs: `{PascalCaseName}Handler`
   - Input/Output structs: `{PascalCaseName}InputData` / `{PascalCaseName}Response`

2. **Error Handling:**
   - Always return descriptive error messages
   - Use `Result<T, String>` - the String will be sent to the client

3. **Documentation:**
   - Use `#[property(description = "...")]` for all fields
   - Write clear `DESCRIPTION` constants
   - Document complex logic in comments

4. **Accessing App Context:**
   - Use `self.app.app_states` for application state
   - If you don't need app context, use `_app` to avoid warnings

5. **Async Operations:**
   - All tool calls and prompts are async
   - You can perform HTTP requests, database queries, file I/O, etc.

6. **Testing:**
   - Test your tool calls independently
   - Verify JSON schema generation
   - Test error cases

---

## Dependencies

Make sure these dependencies are in `Cargo.toml`:

```toml
mcp-server-middleware = { tag = "0.8.3", git = "https://github.com/my-ai-utils/mcp-server-middleware.git" }
my-ai-agent = { tag = "0.1.0", git = "https://github.com/my-ai-utils/my-ai-agent.git", features = ["agent"] }
async-trait = "*"
serde = { version = "*", features = ["derive"] }
serde_json = "*"
```

---

## Summary Checklist

For a new **Tool Call**:
- [ ] Create `{name}_tool_call.rs` file
- [ ] Define input struct with `ApplyJsonSchema`
- [ ] Define output struct with `ApplyJsonSchema`
- [ ] Create handler struct
- [ ] Implement `ToolDefinition` trait
- [ ] Implement `McpToolCall` trait
- [ ] Export in `mod.rs`
- [ ] Register in `start_up.rs`

For a new **Prompt**:
- [ ] Create `{name}_prompt.rs` file (or add to existing)
- [ ] Create handler struct
- [ ] Implement `PromptDefinition` trait
- [ ] Implement `McpPromptService` trait
- [ ] Export in `mod.rs`
- [ ] Register in `start_up.rs`
"##;
