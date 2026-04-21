use confluence_workflow::llm::{AnthropicClient, LlmClient};
use confluence_workflow::merge::{CommentDecision, CommentMarker};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a test CommentMarker.
fn test_marker() -> CommentMarker {
    CommentMarker {
        full_match:
            "<ac:inline-comment-marker ac:ref=\"test-uuid\">important text</ac:inline-comment-marker>"
                .to_string(),
        ac_ref: "test-uuid".to_string(),
        anchor_text: "important text".to_string(),
        position: 42,
    }
}

/// Helper to build a successful tool_use response body.
fn tool_use_response(decision: &str, reason: Option<&str>) -> serde_json::Value {
    let mut input = serde_json::json!({ "decision": decision });
    if let Some(r) = reason {
        input["reason"] = serde_json::Value::String(r.to_string());
    }
    serde_json::json!({
        "id": "msg_test",
        "model": "claude-haiku-4-5-20251001",
        "stop_reason": "tool_use",
        "content": [
            {
                "type": "tool_use",
                "id": "toolu_test",
                "name": "evaluate_comment",
                "input": input
            }
        ]
    })
}

/// Helper to build a text-only response (no tool_use block).
fn text_only_response() -> serde_json::Value {
    serde_json::json!({
        "id": "msg_text",
        "model": "claude-haiku-4-5-20251001",
        "stop_reason": "end_turn",
        "content": [
            {
                "type": "text",
                "text": "I think this comment should be kept."
            }
        ]
    })
}

#[tokio::test]
async fn test_sends_correct_headers() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(header("x-api-key", "test-api-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("content-type", "application/json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(tool_use_response("KEEP", None)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "test-api-key".to_string(),
        "claude-haiku-4-5-20251001".to_string(),
        server.uri(),
    )
    .unwrap();

    let result = client
        .evaluate_comment("old section", Some("new section"), &test_marker())
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sends_tool_use_schema() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(tool_use_response("KEEP", None)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "test-key".to_string(),
        "claude-haiku-4-5-20251001".to_string(),
        server.uri(),
    )
    .unwrap();

    client
        .evaluate_comment("old", Some("new"), &test_marker())
        .await
        .unwrap();

    // Verify the request body structure
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);

    let body: serde_json::Value =
        serde_json::from_slice(&requests[0].body).unwrap();

    // Verify tool_choice forces tool use
    assert_eq!(body["tool_choice"]["type"], "tool");
    assert_eq!(body["tool_choice"]["name"], "evaluate_comment");

    // Verify tool definition
    assert_eq!(body["tools"][0]["name"], "evaluate_comment");
    assert!(body["tools"][0]["input_schema"]["properties"]["decision"].is_object());

    // Verify model and max_tokens
    assert_eq!(body["model"], "claude-haiku-4-5-20251001");
    assert_eq!(body["max_tokens"], 256);

    // Verify message content contains the prompt
    let content = body["messages"][0]["content"].as_str().unwrap();
    assert!(content.contains("important text"));
    assert!(content.contains("old"));
    assert!(content.contains("new"));
}

#[tokio::test]
async fn test_keep_response_returns_keep() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(tool_use_response("KEEP", Some("Still relevant"))),
        )
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "key".to_string(),
        "model".to_string(),
        server.uri(),
    )
    .unwrap();

    let result = client
        .evaluate_comment("old", Some("new"), &test_marker())
        .await
        .unwrap();
    assert_eq!(result, CommentDecision::Keep);
}

#[tokio::test]
async fn test_drop_response_returns_drop() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(tool_use_response("DROP", Some("Section removed"))),
        )
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "key".to_string(),
        "model".to_string(),
        server.uri(),
    )
    .unwrap();

    let result = client
        .evaluate_comment("old", Some("new"), &test_marker())
        .await
        .unwrap();
    assert_eq!(result, CommentDecision::Drop);
}

#[tokio::test]
async fn test_no_tool_use_block_defaults_to_keep() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(text_only_response()),
        )
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "key".to_string(),
        "model".to_string(),
        server.uri(),
    )
    .unwrap();

    let result = client
        .evaluate_comment("old", Some("new"), &test_marker())
        .await
        .unwrap();
    assert_eq!(result, CommentDecision::Keep);
}

#[tokio::test]
async fn test_429_triggers_retry_then_succeeds() {
    let server = MockServer::start().await;

    // First request returns 429, second succeeds
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(tool_use_response("DROP", None)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "key".to_string(),
        "model".to_string(),
        server.uri(),
    )
    .unwrap();

    let result = client
        .evaluate_comment("old", Some("new"), &test_marker())
        .await
        .unwrap();
    assert_eq!(result, CommentDecision::Drop);
}

#[tokio::test]
async fn test_529_overloaded_triggers_retry() {
    let server = MockServer::start().await;

    // First request returns 529 (overloaded), second succeeds
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(529).set_body_string("overloaded"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(tool_use_response("KEEP", None)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "key".to_string(),
        "model".to_string(),
        server.uri(),
    )
    .unwrap();

    let result = client
        .evaluate_comment("old", Some("new"), &test_marker())
        .await
        .unwrap();
    assert_eq!(result, CommentDecision::Keep);
}

#[tokio::test]
async fn test_400_does_not_retry() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(400).set_body_string("bad request"),
        )
        .expect(1) // Only 1 request -- no retry
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "key".to_string(),
        "model".to_string(),
        server.uri(),
    )
    .unwrap();

    let result = client
        .evaluate_comment("old", Some("new"), &test_marker())
        .await;

    match result {
        Err(confluence_workflow::error::LlmError::ApiError { status, body }) => {
            assert_eq!(status, 400);
            assert_eq!(body, "bad request");
        }
        other => panic!("Expected ApiError, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_five_consecutive_429s_returns_rate_limit_exhausted() {
    let server = MockServer::start().await;

    // 6 requests total: initial + 5 retries, all return 429
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
        .expect(6)
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "key".to_string(),
        "model".to_string(),
        server.uri(),
    )
    .unwrap();

    let result = client
        .evaluate_comment("old", Some("new"), &test_marker())
        .await;

    match result {
        Err(confluence_workflow::error::LlmError::RateLimitExhausted { max_retries }) => {
            assert_eq!(max_retries, 5);
        }
        other => panic!("Expected RateLimitExhausted, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_retry_after_header_is_respected() {
    let server = MockServer::start().await;

    // First request returns 429 with retry-after: 1 (1 second)
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("rate limited")
                .insert_header("retry-after", "1"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(tool_use_response("KEEP", None)),
        )
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "key".to_string(),
        "model".to_string(),
        server.uri(),
    )
    .unwrap();

    let start = std::time::Instant::now();
    let result = client
        .evaluate_comment("old", Some("new"), &test_marker())
        .await
        .unwrap();

    let elapsed = start.elapsed();
    assert_eq!(result, CommentDecision::Keep);
    // With retry-after: 1 and jitter (0.75-1.25), delay should be at least 750ms
    assert!(
        elapsed.as_millis() >= 700,
        "Expected delay >= 700ms from retry-after header, got {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_deleted_section_prompt() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(tool_use_response("DROP", None)),
        )
        .mount(&server)
        .await;

    let client = AnthropicClient::with_endpoint(
        "key".to_string(),
        "model".to_string(),
        server.uri(),
    )
    .unwrap();

    // Pass None for new_section (deleted section)
    client
        .evaluate_comment("old section content", None, &test_marker())
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&requests[0].body).unwrap();
    let content = body["messages"][0]["content"].as_str().unwrap();
    assert!(
        content.contains("This section has been deleted from the new content."),
        "Prompt should indicate deleted section when new_section is None"
    );
}

#[test]
fn test_api_key_not_in_debug_output() {
    let _client = AnthropicClient::with_endpoint(
        "sk-secret-key-12345".to_string(),
        "model".to_string(),
        "http://localhost:1234".to_string(),
    )
    .unwrap();

    // The AnthropicClient struct does not derive Debug, so the api_key
    // cannot leak through Debug formatting. Verify the struct fields
    // are not exposed via any public debug representation.
    // The key is only used in the header() call, never in tracing macros.

    // Verify the source code does not log the api key in tracing macros
    let source = include_str!("../src/llm/mod.rs");
    // Find all tracing macro invocations and ensure none reference api_key
    for line in source.lines() {
        if (line.contains("tracing::info!") || line.contains("tracing::debug!"))
            && line.contains("api_key")
        {
            panic!(
                "Found api_key reference in tracing output: {}",
                line.trim()
            );
        }
    }
}
