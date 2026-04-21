//! Integration tests for CLI command wiring (CLI-01, CLI-02, CLI-03, CLI-05).

use assert_cmd::Command;
use serde_json::json;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temporary markdown file with minimal content and return the dir +
/// absolute path to the file.
fn temp_markdown(content: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let md_path = dir.path().join("doc.md");
    fs::write(&md_path, content).expect("write temp markdown");
    (dir, md_path)
}

/// Build a Confluence GET-page response body. Used by both happy-path tests.
///
/// The body contains one inline-comment-marker inside an `<h2>Context</h2>`
/// section; the matching section in the converted markdown has different
/// paragraph text, so the merge classifier produces an ambiguous marker that
/// fans out to the LLM. Deviation from plan 10-01: the original helper had
/// no heading, which caused the marker to be deterministically DROPPED (no
/// LLM call) and broke the `received_requests()` assertion. Using a heading
/// that matches the markdown forces the LLM path.
fn page_json_with_comment(id: &str, version: u32) -> serde_json::Value {
    json!({
        "id": id,
        "title": "Happy Path Test Page",
        "body": {
            "storage": {
                "value": "<h2>Context</h2><p>Before <ac:inline-comment-marker ac:ref=\"abc-123\">important</ac:inline-comment-marker> after.</p>",
                "representation": "storage"
            }
        },
        "version": { "number": version }
    })
}

/// Build a Confluence GET-page response body with no inline-comment-markers.
/// Used by the upload happy-path test (upload bypasses the merge pipeline
/// entirely, so the body content does not matter for correctness — this
/// helper keeps the test body minimal).
fn page_json_plain(id: &str, version: u32) -> serde_json::Value {
    json!({
        "id": id,
        "title": "Upload Test",
        "body": {
            "storage": {
                "value": "<p>old content</p>",
                "representation": "storage"
            }
        },
        "version": { "number": version }
    })
}

/// Copy of `tests/llm_integration.rs::tool_use_response` — a KEEP decision
/// response shaped for the merge engine's evaluate_comment tool.
fn anthropic_tool_use_keep_response() -> serde_json::Value {
    json!({
        "id": "msg_test",
        "model": "claude-haiku-4-5-20251001",
        "stop_reason": "tool_use",
        "content": [
            {
                "type": "tool_use",
                "id": "toolu_test",
                "name": "evaluate_comment",
                "input": { "decision": "KEEP" }
            }
        ]
    })
}

// ---------------------------------------------------------------------------
// CLI-03: convert command — fully automated, no Confluence needed
// ---------------------------------------------------------------------------

/// convert command writes page.xml to the output directory (CLI-03).
#[test]
fn test_convert_command() {
    let (md_dir, md_path) = temp_markdown("# Hello World\n\nSome content.\n");
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("convert")
        .arg(&md_path)
        .arg(out_dir.path())
        // Ensure no stray env vars interfere
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN");

    let output = cmd.output().expect("run command");

    // Exit code 0
    assert!(
        output.status.success(),
        "convert should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // stdout contains "Converted to:" with the output dir path
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Converted to:"),
        "stdout should contain 'Converted to:'; got: {stdout}"
    );

    // page.xml written to output dir
    let xml_path = out_dir.path().join("page.xml");
    assert!(
        xml_path.exists(),
        "page.xml should exist in output dir; files: {:?}",
        fs::read_dir(out_dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<Vec<_>>()
    );

    // page.xml contains valid storage XML content (not empty)
    let xml_content = fs::read_to_string(&xml_path).expect("read page.xml");
    assert!(
        !xml_content.is_empty(),
        "page.xml should not be empty"
    );

    // Tracing output must NOT appear on stdout (D-07)
    assert!(
        !stdout.contains("DEBUG") && !stdout.contains("INFO") && !stdout.contains("TRACE"),
        "tracing output must not appear on stdout; stdout: {stdout}"
    );

    // Keep temp dirs alive until end of test
    drop(md_dir);
    drop(out_dir);
}

/// convert command exits 1 when the markdown file does not exist (error path).
#[test]
fn test_convert_command_missing_file() {
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("convert")
        .arg("/nonexistent/path/doc.md")
        .arg(out_dir.path());

    let output = cmd.output().expect("run command");

    assert!(
        !output.status.success(),
        "convert with missing file should exit non-zero"
    );
}

// ---------------------------------------------------------------------------
// CLI-05: --output json emits valid JSON on stdout
// ---------------------------------------------------------------------------

/// convert --output json emits valid JSON with expected schema fields (CLI-05).
#[test]
fn test_json_output_mode() {
    let (md_dir, md_path) = temp_markdown("# JSON Test\n\nContent here.\n");
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("--output")
        .arg("json")
        .arg("convert")
        .arg(&md_path)
        .arg(out_dir.path())
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN");

    let output = cmd.output().expect("run command");

    assert!(
        output.status.success(),
        "convert --output json should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // stdout must be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect(&format!(
            "stdout must be valid JSON; got: {stdout}"
        ));

    // D-02 schema: success: true, output_dir, files
    assert_eq!(
        parsed["success"], serde_json::Value::Bool(true),
        "JSON must have success: true; got: {parsed}"
    );
    assert!(
        parsed["output_dir"].is_string(),
        "JSON must have output_dir string; got: {parsed}"
    );
    assert!(
        parsed["files"].is_array(),
        "JSON must have files array; got: {parsed}"
    );

    // files array must include at least one entry (page.xml)
    let files = parsed["files"].as_array().unwrap();
    assert!(
        !files.is_empty(),
        "files array must be non-empty; got: {parsed}"
    );
    let has_xml = files
        .iter()
        .any(|f| f.as_str().map(|s| s.ends_with("page.xml")).unwrap_or(false));
    assert!(has_xml, "files array must contain page.xml entry; got: {files:?}");

    // Tracing output must not appear on stdout (D-07)
    assert!(
        !stdout.contains("DEBUG") && !stdout.contains("INFO") && !stdout.contains("TRACE"),
        "tracing output must not appear on stdout in JSON mode; stdout: {stdout}"
    );

    drop(md_dir);
    drop(out_dir);
}

/// convert --output json emits JSON with success: false on error.
#[test]
fn test_json_output_mode_error() {
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("--output")
        .arg("json")
        .arg("convert")
        .arg("/nonexistent/doc.md")
        .arg(out_dir.path());

    let output = cmd.output().expect("run command");

    assert!(
        !output.status.success(),
        "convert --output json with missing file should exit 1"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect(&format!(
            "error output must still be valid JSON; got: {stdout}"
        ));

    assert_eq!(
        parsed["success"], serde_json::Value::Bool(false),
        "JSON error must have success: false; got: {parsed}"
    );
    assert!(
        parsed["error"].is_string(),
        "JSON error must have error string; got: {parsed}"
    );

    drop(out_dir);
}

// ---------------------------------------------------------------------------
// CLI-01: update command — error path (missing credentials) tested without
// live Confluence. Full happy-path requires wiremock with Confluence API mocks
// plus an Anthropic API mock (LLM merge step).
// ---------------------------------------------------------------------------

/// update command exits 1 and emits useful error when ANTHROPIC_API_KEY is
/// missing — verifies the credential-validation path in the Update arm (CLI-01).
#[test]
fn test_update_command_missing_api_key() {
    let (md_dir, md_path) = temp_markdown("# Update Test\n\nContent.\n");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("--confluence-url")
        .arg("https://localhost:19999")
        .arg("--confluence-username")
        .arg("user@example.com")
        .arg("--confluence-token")
        .arg("fake-token")
        .arg("update")
        .arg(&md_path)
        .arg("https://localhost:19999/wiki/spaces/TEST/pages/12345/Title")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN");

    let output = cmd.output().expect("run command");

    assert!(
        !output.status.success(),
        "update without ANTHROPIC_API_KEY should exit non-zero"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ANTHROPIC_API_KEY"),
        "stderr should mention the missing ANTHROPIC_API_KEY; got: {stderr}"
    );

    drop(md_dir);
}

/// upload command exits 1 and emits useful error when Confluence URL is
/// missing — verifies the credential-validation path in the Upload arm (CLI-02).
#[test]
fn test_upload_command_missing_credentials() {
    let (md_dir, md_path) = temp_markdown("# Upload Test\n\nContent.\n");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("upload")
        .arg(&md_path)
        .arg("http://localhost:19999/wiki/spaces/TEST/pages/12345/Title")
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN");

    let output = cmd.output().expect("run command");

    assert!(
        !output.status.success(),
        "upload without credentials should exit non-zero"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error") || stderr.contains("CONFLUENCE"),
        "stderr should mention missing config; got: {stderr}"
    );

    drop(md_dir);
}

// ---------------------------------------------------------------------------
// CLI-02: upload command — security guard and credential-error paths.
// Happy-path requires an https:// Confluence server. Config enforces https://
// (threat T-01-04), so wiremock (http-only) cannot be used for the full flow.
// ---------------------------------------------------------------------------

/// upload command rejects an http:// Confluence URL with a config error (CLI-02,
/// T-01-04). The security guard must fire before any HTTP request is made.
#[test]
fn test_upload_command_rejects_http_url() {
    let (md_dir, md_path) = temp_markdown("# Upload Test\n\nContent.\n");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("--confluence-url")
        .arg("http://insecure.example.com")
        .arg("--confluence-username")
        .arg("user@example.com")
        .arg("--confluence-token")
        .arg("fake-token")
        .arg("upload")
        .arg(&md_path)
        .arg("http://insecure.example.com/wiki/spaces/TEST/pages/12345/Title")
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN")
        .env_remove("ANTHROPIC_API_KEY");

    let output = cmd.output().expect("run command");

    assert!(
        !output.status.success(),
        "upload with http:// URL should exit non-zero"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("https://") || stderr.contains("CONFLUENCE_URL"),
        "error should mention https:// requirement; got: {stderr}"
    );

    drop(md_dir);
}

/// upload command happy path: convert → fetch (current page for version) →
/// upload attachments → PUT (CLI-02). No LLM / no Anthropic mock — upload
/// bypasses the merge pipeline. Per D-01 (localhost exemption) + D-02.
#[tokio::test]
#[serial]
async fn test_upload_command_happy_path() {
    let confluence = MockServer::start().await;
    let page_id = "54321";

    Mock::given(method("GET"))
        .and(path(format!("/rest/api/content/{page_id}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(page_json_plain(page_id, 3)),
        )
        .mount(&confluence)
        .await;

    Mock::given(method("PUT"))
        .and(path(format!("/rest/api/content/{page_id}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(page_json_plain(page_id, 4)),
        )
        .mount(&confluence)
        .await;

    let (md_dir, md_path) = temp_markdown("# Upload Test\n\nContent.\n");
    let page_url = format!(
        "{}/wiki/spaces/TEST/pages/{page_id}/Title",
        confluence.uri()
    );

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("--confluence-url")
        .arg(confluence.uri())
        .arg("--confluence-username")
        .arg("user@example.com")
        .arg("--confluence-token")
        .arg("fake-token")
        .arg("upload")
        .arg(&md_path)
        .arg(&page_url)
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("ANTHROPIC_BASE_URL");

    let output = cmd.output().expect("run command");

    assert!(
        output.status.success(),
        "upload happy path should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Uploaded to:"),
        "stdout should contain 'Uploaded to:'; got: {stdout}"
    );
    assert!(
        !stdout.contains("DEBUG")
            && !stdout.contains("INFO")
            && !stdout.contains("TRACE"),
        "tracing must not appear on stdout (D-07); got: {stdout}"
    );

    drop(md_dir);
}

// ---------------------------------------------------------------------------
// SCAF-03: --plantuml-path and --mermaid-path flags wired through convert arm
// ---------------------------------------------------------------------------

/// convert command accepts --plantuml-path and --mermaid-path flags and
/// succeeds, writing page.xml to the output directory (SCAF-03, task 08-01-02).
///
/// The markdown contains no diagram blocks so the fake paths are never
/// invoked — the test proves only that the flags are accepted at the CLI
/// boundary and the DiagramConfig waterfall wiring reaches the convert arm.
#[test]
fn test_convert_with_diagram_path_flags() {
    let (md_dir, md_path) = temp_markdown("# Diagram Flag Test\n\nPlain content, no diagrams.\n");
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("--plantuml-path")
        .arg("/fake/plantuml")
        .arg("--mermaid-path")
        .arg("/fake/mmdc")
        .arg("convert")
        .arg(&md_path)
        .arg(out_dir.path())
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN")
        // Override any env-var defaults so the test is deterministic
        .env_remove("PLANTUML_PATH")
        .env_remove("MERMAID_PATH");

    let output = cmd.output().expect("run command");

    // Flags must not produce "unexpected argument" — exit code must be 0
    assert!(
        output.status.success(),
        "convert with --plantuml-path and --mermaid-path should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // stdout contains the expected "Converted to:" line
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Converted to:"),
        "stdout should contain 'Converted to:'; got: {stdout}"
    );

    // page.xml must be written to the output directory
    let xml_path = out_dir.path().join("page.xml");
    assert!(
        xml_path.exists(),
        "page.xml should exist in output dir; files: {:?}",
        fs::read_dir(out_dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<Vec<_>>()
    );

    // page.xml must be non-empty
    let xml_content = fs::read_to_string(&xml_path).expect("read page.xml");
    assert!(!xml_content.is_empty(), "page.xml should not be empty");

    // Tracing output must not appear on stdout (D-07)
    assert!(
        !stdout.contains("DEBUG") && !stdout.contains("INFO") && !stdout.contains("TRACE"),
        "tracing output must not appear on stdout; stdout: {stdout}"
    );

    drop(md_dir);
    drop(out_dir);
}

// ---------------------------------------------------------------------------
// SCAF-03 env-var tier: PLANTUML_PATH / MERMAID_PATH env vars wired through
// clap-derive's env= attribute into cli.plantuml_path / cli.mermaid_path,
// then into DiagramConfig in the convert arm. Closes the gap where
// test_convert_with_diagram_path_flags only covered the CLI-flag tier.
// Per 09-CONTEXT.md D-06.
// ---------------------------------------------------------------------------

/// convert command honors PLANTUML_PATH and MERMAID_PATH env vars when no
/// CLI flag is provided (SCAF-03 env-var tier, D-06).
///
/// Clap-derive's `#[arg(long, env = "PLANTUML_PATH")]` resolves the env var
/// onto `cli.plantuml_path` at `Cli::parse()` time; the convert arm reads
/// that already-resolved value. This test proves end-to-end that setting
/// the env var (without passing the flag) reaches the DiagramConfig.
#[test]
#[serial]
fn test_convert_with_env_var_diagram_paths() {
    let (md_dir, md_path) = temp_markdown("# Env Var Test\n\nPlain content, no diagrams.\n");
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("convert")
        .arg(&md_path)
        .arg(out_dir.path())
        // Ensure Confluence env vars don't leak in from the shell
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN")
        // Set the env-var tier — NOT CLI flags (no --plantuml-path / --mermaid-path args)
        .env("PLANTUML_PATH", "/fake/plantuml-via-env")
        .env("MERMAID_PATH", "/fake/mmdc-via-env");

    let output = cmd.output().expect("run command");

    // The markdown has no diagrams, so /fake paths are never invoked — the test
    // only proves the env-var tier is accepted at the CLI boundary and wired
    // into the convert arm without error.
    assert!(
        output.status.success(),
        "convert with PLANTUML_PATH / MERMAID_PATH env vars should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // stdout must contain the expected "Converted to:" line
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Converted to:"),
        "stdout should contain 'Converted to:'; got: {stdout}"
    );

    // page.xml must be written to the output directory
    let xml_path = out_dir.path().join("page.xml");
    assert!(
        xml_path.exists(),
        "page.xml should exist in output dir; files: {:?}",
        fs::read_dir(out_dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<Vec<_>>()
    );

    // page.xml must be non-empty
    let xml_content = fs::read_to_string(&xml_path).expect("read page.xml");
    assert!(!xml_content.is_empty(), "page.xml should not be empty");

    drop(md_dir);
    drop(out_dir);
}

// ---------------------------------------------------------------------------
// CLI-01: update command happy path — full pipeline with wiremock for both
// Confluence and Anthropic. Per D-01 (localhost exemption), D-02 (wiremock for
// both), D-03 (ANTHROPIC_BASE_URL env-var override on AnthropicClient::new).
// ---------------------------------------------------------------------------

/// update command happy path: convert → fetch → merge (with LLM) → upload (CLI-01).
#[tokio::test]
#[serial]
async fn test_update_command_happy_path() {
    // --- Arrange Confluence wiremock ---
    let confluence = MockServer::start().await;
    let page_id = "12345";

    Mock::given(method("GET"))
        .and(path(format!("/rest/api/content/{page_id}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(page_json_with_comment(page_id, 7)),
        )
        .mount(&confluence)
        .await;

    Mock::given(method("PUT"))
        .and(path(format!("/rest/api/content/{page_id}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(page_json_with_comment(page_id, 8)),
        )
        .mount(&confluence)
        .await;

    // --- Arrange Anthropic wiremock (endpoint is the full URL, so match on "/") ---
    let anthropic = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_tool_use_keep_response()),
        )
        .mount(&anthropic)
        .await;

    // --- Arrange test inputs ---
    // Markdown intentionally echoes the `## Context` heading from the mocked
    // page body so the classifier finds a matching section with differing
    // content — this is the ambiguous path that fans out to the LLM.
    let (md_dir, md_path) =
        temp_markdown("# Happy Path\n\n## Context\n\nA completely different body here.\n");
    let page_url = format!(
        "{}/wiki/spaces/TEST/pages/{page_id}/Title",
        confluence.uri()
    );

    // --- Act: spawn the binary ---
    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("--confluence-url")
        .arg(confluence.uri())
        .arg("--confluence-username")
        .arg("user@example.com")
        .arg("--confluence-token")
        .arg("fake-token")
        .arg("--anthropic-api-key")
        .arg("fake-anthropic-key")
        .arg("update")
        .arg(&md_path)
        .arg(&page_url)
        .env("ANTHROPIC_BASE_URL", anthropic.uri())
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN");

    let output = cmd.output().expect("run command");

    // --- Assert ---
    assert!(
        output.status.success(),
        "update happy path should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Updated page:"),
        "stdout should contain 'Updated page:'; got: {stdout}"
    );
    assert!(
        !stdout.contains("DEBUG")
            && !stdout.contains("INFO")
            && !stdout.contains("TRACE"),
        "tracing must not appear on stdout (D-07); got: {stdout}"
    );

    // Verify the LLM was actually called (proves ANTHROPIC_BASE_URL wiring + D-03).
    let llm_requests = anthropic
        .received_requests()
        .await
        .expect("wiremock records requests");
    assert!(
        !llm_requests.is_empty(),
        "LLM should have been called for the inline comment marker"
    );

    drop(md_dir);
}
