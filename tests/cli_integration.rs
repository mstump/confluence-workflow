//! Integration tests for CLI command wiring (CLI-01, CLI-02, CLI-03, CLI-05).

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

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

// ---------------------------------------------------------------------------
// CLI-03: convert command — fully automated, no Confluence needed
// ---------------------------------------------------------------------------

/// convert command writes page.xml to the output directory (CLI-03).
#[test]
fn test_convert_command() {
    let (md_dir, md_path) = temp_markdown("# Hello World\n\nSome content.\n");
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
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

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
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

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
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

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
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

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    cmd.arg("--confluence-url")
        .arg("http://localhost:19999")
        .arg("--confluence-username")
        .arg("user@example.com")
        .arg("--confluence-token")
        .arg("fake-token")
        .arg("update")
        .arg(&md_path)
        .arg("http://localhost:19999/wiki/spaces/TEST/pages/12345/Title")
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
        stderr.contains("ANTHROPIC_API_KEY") || stderr.contains("Error"),
        "stderr should mention the missing key; got: {stderr}"
    );

    drop(md_dir);
}

/// upload command exits 1 and emits useful error when Confluence URL is
/// missing — verifies the credential-validation path in the Upload arm (CLI-02).
#[test]
fn test_upload_command_missing_credentials() {
    let (md_dir, md_path) = temp_markdown("# Upload Test\n\nContent.\n");

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
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

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
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

/// upload command happy-path against a live Confluence instance.
/// Blocked at binary level: Config enforces https:// but wiremock is http-only.
/// TLS-capable mock server would be required to automate this path end-to-end.
/// The ConfluenceClient layer is already tested via unit tests in src/confluence/client.rs.
#[test]
#[ignore = "happy-path requires https:// server; wiremock is http-only (T-01-04 constraint)"]
fn test_upload_command_happy_path() {
    // Would need: a TLS-capable mock Confluence server OR a real Confluence instance.
    // The unit tests in src/confluence/client.rs cover the HTTP layer via wiremock.
}
