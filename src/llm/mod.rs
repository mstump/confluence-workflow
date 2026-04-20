pub mod types;

use async_trait::async_trait;

use crate::error::LlmError;
use crate::merge::{CommentDecision, CommentMarker};
use types::{
    ContentBlock, EvaluateCommentInput, Message, MessageRequest, MessageResponse, ToolChoice,
    ToolDefinition,
};

/// Trait for LLM-based comment evaluation.
///
/// The trait boundary enables unit testing the merge engine without HTTP calls.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Evaluate whether an inline comment should survive a content update.
    ///
    /// Returns `CommentDecision::Keep` or `CommentDecision::Drop` based on
    /// whether the comment is still relevant to the updated content.
    async fn evaluate_comment(
        &self,
        old_section: &str,
        new_section: Option<&str>,
        marker: &CommentMarker,
    ) -> Result<CommentDecision, LlmError>;
}

/// Maximum number of retry attempts for transient errors.
const MAX_RETRIES: u32 = 5;

/// Initial backoff delay in milliseconds.
const INITIAL_BACKOFF_MS: u64 = 1000;

/// Maximum backoff delay in milliseconds.
const MAX_BACKOFF_MS: u64 = 32000;

/// HTTP status codes that trigger a retry.
const RETRYABLE_STATUS_CODES: &[u16] = &[429, 500, 502, 503, 529];

/// Anthropic Messages API client with retry and backoff.
pub struct AnthropicClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
    endpoint: String,
}

impl AnthropicClient {
    /// Create a new client pointing at the production Anthropic API.
    ///
    /// Honours the `ANTHROPIC_BASE_URL` env var (test-infrastructure affordance,
    /// D-03) when set to a non-empty value; otherwise falls back to the production
    /// URL. `with_endpoint` (below) remains the direct seam for unit tests that
    /// prefer not to mutate process env.
    pub fn new(api_key: String, model: String) -> Self {
        let endpoint = std::env::var("ANTHROPIC_BASE_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());
        Self::with_endpoint(api_key, model, endpoint)
    }

    /// Create a new client with a custom endpoint (for testing with wiremock).
    pub fn with_endpoint(api_key: String, model: String, endpoint: String) -> Self {
        let client = reqwest::Client::builder()
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    "application/json".parse().unwrap(),
                );
                headers.insert(
                    reqwest::header::HeaderName::from_static("anthropic-version"),
                    "2023-06-01".parse().unwrap(),
                );
                headers
            })
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client,
            api_key,
            model,
            endpoint,
        }
    }

    /// Send a request with exponential backoff retry on transient errors.
    ///
    /// Retries on 429, 500, 502, 503, 529. Respects retry-after header.
    /// Returns `LlmError::RateLimitExhausted` after MAX_RETRIES attempts.
    /// Non-retryable errors (400, 401, 403) fail immediately.
    async fn request_with_retry(
        &self,
        body: &MessageRequest,
    ) -> Result<MessageResponse, LlmError> {
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        for attempt in 0..=MAX_RETRIES {
            let response = self
                .client
                .post(&self.endpoint)
                .header("x-api-key", &self.api_key)
                .json(body)
                .send()
                .await?;

            // Read status and headers BEFORE consuming body
            let status = response.status().as_u16();
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<f64>().ok());

            if status == 200 {
                let msg: MessageResponse = response
                    .json()
                    .await
                    .map_err(LlmError::Deserialize)?;
                return Ok(msg);
            }

            let response_body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable>".to_string());

            if !RETRYABLE_STATUS_CODES.contains(&status) {
                return Err(LlmError::ApiError {
                    status,
                    body: response_body,
                });
            }

            // Last attempt exhausted -- do not retry
            if attempt == MAX_RETRIES {
                return Err(LlmError::RateLimitExhausted {
                    max_retries: MAX_RETRIES,
                });
            }

            // Calculate delay: use retry-after if present, otherwise exponential backoff
            let delay_ms = if let Some(retry_secs) = retry_after {
                (retry_secs * 1000.0) as u64
            } else {
                backoff_ms
            };

            // Apply jitter: +/-25%
            let jitter = {
                use rand::Rng;
                let mut rng = rand::rng();
                rng.random_range(0.75..=1.25)
            };
            let delay_ms = (delay_ms as f64 * jitter) as u64;

            tracing::warn!(
                status = status,
                attempt = attempt + 1,
                max_retries = MAX_RETRIES,
                delay_ms = delay_ms,
                "Anthropic API transient error, retrying"
            );

            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

            // Exponential backoff (capped)
            backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
        }

        // Should not be reached due to the loop logic, but just in case
        Err(LlmError::RateLimitExhausted {
            max_retries: MAX_RETRIES,
        })
    }

    /// Build the tool definition for evaluate_comment.
    fn evaluate_comment_tool() -> ToolDefinition {
        ToolDefinition {
            name: "evaluate_comment".to_string(),
            description: "Evaluate whether an inline comment should be kept or dropped after a content update".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "decision": {
                        "type": "string",
                        "enum": ["KEEP", "DROP"],
                        "description": "Whether to keep or drop the inline comment"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Brief explanation for the decision (used for debugging)"
                    }
                },
                "required": ["decision"]
            }),
        }
    }
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn evaluate_comment(
        &self,
        old_section: &str,
        new_section: Option<&str>,
        marker: &CommentMarker,
    ) -> Result<CommentDecision, LlmError> {
        let new_section_text = new_section
            .unwrap_or("This section has been deleted from the new content.");

        let prompt = format!(
            "You are evaluating whether an inline comment on a Confluence page should survive a content update.\n\n\
             ## Old Section\n{old_section}\n\n\
             ## New Section\n{new_section_text}\n\n\
             ## Comment\n\
             The comment marker wraps the text: \"{anchor_text}\"\n\n\
             Should this comment be KEPT (still relevant to the updated content) or DROPPED (no longer applicable)?",
            anchor_text = marker.anchor_text,
        );

        let request = MessageRequest {
            model: self.model.clone(),
            max_tokens: 256,
            tools: vec![Self::evaluate_comment_tool()],
            tool_choice: ToolChoice {
                choice_type: "tool".to_string(),
                name: "evaluate_comment".to_string(),
            },
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let response = self.request_with_retry(&request).await?;

        // Find the tool_use content block
        for block in &response.content {
            if let ContentBlock::ToolUse { input, .. } = block {
                let eval: EvaluateCommentInput = serde_json::from_value(input.clone())
                    .map_err(|e| LlmError::MalformedResponse(e.to_string()))?;

                return match eval.decision.as_str() {
                    "KEEP" => Ok(CommentDecision::Keep),
                    "DROP" => Ok(CommentDecision::Drop),
                    other => {
                        tracing::warn!(
                            decision = other,
                            "Unexpected decision value, defaulting to KEEP"
                        );
                        Ok(CommentDecision::Keep)
                    }
                };
            }
        }

        // No tool_use block found -- fail-safe to KEEP
        tracing::warn!("No tool_use block in response, defaulting to KEEP");
        Ok(CommentDecision::Keep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::*;

    // -- MockLlmClient for downstream testing --

    struct MockLlmClient {
        decision: CommentDecision,
    }

    impl MockLlmClient {
        fn new(decision: CommentDecision) -> Self {
            Self { decision }
        }
    }

    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn evaluate_comment(
            &self,
            _old_section: &str,
            _new_section: Option<&str>,
            _marker: &CommentMarker,
        ) -> Result<CommentDecision, LlmError> {
            Ok(self.decision)
        }
    }

    // -- Serde round-trip tests --

    #[test]
    fn test_message_request_serializes_correctly() {
        let req = MessageRequest {
            model: "claude-haiku-4-5-20251001".to_string(),
            max_tokens: 256,
            tools: vec![ToolDefinition {
                name: "evaluate_comment".to_string(),
                description: "Evaluate a comment".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "decision": { "type": "string", "enum": ["KEEP", "DROP"] },
                        "reason": { "type": "string" }
                    },
                    "required": ["decision"]
                }),
            }],
            tool_choice: ToolChoice {
                choice_type: "tool".to_string(),
                name: "evaluate_comment".to_string(),
            },
            messages: vec![Message {
                role: "user".to_string(),
                content: "Test prompt".to_string(),
            }],
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "claude-haiku-4-5-20251001");
        assert_eq!(json["max_tokens"], 256);
        assert_eq!(json["tools"][0]["name"], "evaluate_comment");
        assert_eq!(json["tool_choice"]["type"], "tool");
        assert_eq!(json["tool_choice"]["name"], "evaluate_comment");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Test prompt");
    }

    #[test]
    fn test_message_response_with_tool_use_deserializes() {
        let json = serde_json::json!({
            "id": "msg_123",
            "model": "claude-haiku-4-5-20251001",
            "stop_reason": "tool_use",
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_123",
                    "name": "evaluate_comment",
                    "input": {
                        "decision": "KEEP",
                        "reason": "Comment still relevant"
                    }
                }
            ]
        });

        let resp: MessageResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.id, "msg_123");
        assert_eq!(resp.stop_reason, "tool_use");
        assert_eq!(resp.content.len(), 1);

        match &resp.content[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_123");
                assert_eq!(name, "evaluate_comment");
                let eval: EvaluateCommentInput = serde_json::from_value(input.clone()).unwrap();
                assert_eq!(eval.decision, "KEEP");
                assert_eq!(eval.reason.as_deref(), Some("Comment still relevant"));
            }
            other => panic!("Expected ToolUse, got: {:?}", other),
        }
    }

    #[test]
    fn test_message_response_with_text_only_deserializes() {
        let json = serde_json::json!({
            "id": "msg_456",
            "model": "claude-haiku-4-5-20251001",
            "stop_reason": "end_turn",
            "content": [
                {
                    "type": "text",
                    "text": "I'll keep this comment."
                }
            ]
        });

        let resp: MessageResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.content.len(), 1);
        match &resp.content[0] {
            ContentBlock::Text { text } => {
                assert_eq!(text, "I'll keep this comment.");
            }
            other => panic!("Expected Text, got: {:?}", other),
        }
    }

    #[test]
    fn test_evaluate_comment_input_deserializes_keep() {
        let json = serde_json::json!({ "decision": "KEEP" });
        let input: EvaluateCommentInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.decision, "KEEP");
        assert!(input.reason.is_none());
    }

    #[test]
    fn test_evaluate_comment_input_deserializes_drop() {
        let json = serde_json::json!({ "decision": "DROP", "reason": "Section removed" });
        let input: EvaluateCommentInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.decision, "DROP");
        assert_eq!(input.reason.as_deref(), Some("Section removed"));
    }

    #[test]
    fn test_content_block_tags_correctly() {
        let text_json = serde_json::json!({"type": "text", "text": "hello"});
        let tool_json = serde_json::json!({
            "type": "tool_use",
            "id": "t1",
            "name": "eval",
            "input": {}
        });

        let text: ContentBlock = serde_json::from_value(text_json).unwrap();
        let tool: ContentBlock = serde_json::from_value(tool_json).unwrap();

        assert!(matches!(text, ContentBlock::Text { .. }));
        assert!(matches!(tool, ContentBlock::ToolUse { .. }));
    }

    #[tokio::test]
    async fn test_mock_llm_client_returns_keep() {
        let client = MockLlmClient::new(CommentDecision::Keep);
        let marker = CommentMarker {
            full_match: "<ac:inline-comment-marker ac:ref=\"abc\">text</ac:inline-comment-marker>"
                .to_string(),
            ac_ref: "abc".to_string(),
            anchor_text: "text".to_string(),
            position: 0,
        };

        let result = client
            .evaluate_comment("old section", Some("new section"), &marker)
            .await
            .unwrap();
        assert_eq!(result, CommentDecision::Keep);
    }

    #[tokio::test]
    async fn test_mock_llm_client_returns_drop() {
        let client = MockLlmClient::new(CommentDecision::Drop);
        let marker = CommentMarker {
            full_match: "<ac:inline-comment-marker ac:ref=\"abc\">text</ac:inline-comment-marker>"
                .to_string(),
            ac_ref: "abc".to_string(),
            anchor_text: "text".to_string(),
            position: 0,
        };

        let result = client
            .evaluate_comment("old section", None, &marker)
            .await
            .unwrap();
        assert_eq!(result, CommentDecision::Drop);
    }
}
