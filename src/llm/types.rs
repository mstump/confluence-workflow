use serde::{Deserialize, Serialize};

/// Request body for the Anthropic Messages API.
#[derive(Debug, Serialize)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub tools: Vec<ToolDefinition>,
    pub tool_choice: ToolChoice,
    pub messages: Vec<Message>,
}

/// A tool definition for the Anthropic Messages API.
#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Forces the model to use a specific tool.
#[derive(Debug, Serialize)]
pub struct ToolChoice {
    #[serde(rename = "type")]
    pub choice_type: String,
    pub name: String,
}

/// A single message in the conversation.
#[derive(Debug, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Response from the Anthropic Messages API.
#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub model: String,
    pub stop_reason: String,
    pub content: Vec<ContentBlock>,
}

/// A content block in the response -- either text or tool_use.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// The structured input from the evaluate_comment tool call.
#[derive(Debug, Deserialize)]
pub struct EvaluateCommentInput {
    pub decision: String,
    pub reason: Option<String>,
}
