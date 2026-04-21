//! Tests for output formatting and stderr routing (CLI-04).

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn temp_markdown(content: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let md_path = dir.path().join("doc.md");
    fs::write(&md_path, content).expect("write temp markdown");
    (dir, md_path)
}

/// --verbose sends tracing output to stderr, not stdout (CLI-04, D-07).
///
/// With --verbose the tracing level is "debug", so tracing output appears on
/// stderr. Stdout must only contain the one-line human result.
#[test]
fn test_stderr_routing() {
    let (md_dir, md_path) = temp_markdown("# Verbose Test\n\nSome content.\n");
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("--verbose")
        .arg("convert")
        .arg(&md_path)
        .arg(out_dir.path())
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN");

    let output = cmd.output().expect("run command");

    assert!(
        output.status.success(),
        "convert --verbose should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // stdout must contain the success line
    assert!(
        stdout.contains("Converted to:"),
        "stdout must contain 'Converted to:'; got: {stdout}"
    );

    // stdout must NOT contain tracing/debug output (D-07)
    assert!(
        !stdout.contains("DEBUG") && !stdout.contains("TRACE"),
        "tracing output must not appear on stdout; stdout: {stdout}"
    );

    // With --verbose, tracing is at debug level — stderr should have content.
    // The verbose file-list path also writes to stderr via eprintln!().
    // Either tracing spans or the verbose file list must appear on stderr.
    assert!(
        !stderr.is_empty(),
        "stderr should have content when --verbose is used; stderr was empty"
    );

    drop(md_dir);
    drop(out_dir);
}

/// Default (non-verbose) mode produces no tracing output on stderr (D-04, D-08).
///
/// Without --verbose the tracing level is "warn". A successful convert
/// produces no warnings, so stderr should be empty.
#[test]
fn test_default_silent_mode() {
    let (md_dir, md_path) = temp_markdown("# Silent Test\n\nContent.\n");
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-workflow").expect("binary exists");
    cmd.arg("convert")
        .arg(&md_path)
        .arg(out_dir.path())
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN");

    let output = cmd.output().expect("run command");

    assert!(
        output.status.success(),
        "convert should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // stdout has the result line
    assert!(
        stdout.contains("Converted to:"),
        "stdout must contain 'Converted to:'; got: {stdout}"
    );

    // stderr should be empty in default (non-verbose) mode
    assert!(
        stderr.is_empty(),
        "stderr should be empty in default mode; got: {stderr}"
    );

    drop(md_dir);
    drop(out_dir);
}
