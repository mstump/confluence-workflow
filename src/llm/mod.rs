pub mod types;

use async_trait::async_trait;

use crate::error::LlmError;
use crate::merge::{CommentDecision, CommentMarker};

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
