//! Integration tests for CLI command wiring (CLI-01, CLI-02, CLI-03, CLI-05).
//!
//! These are compile-only stubs created as Wave 0 scaffolding.
//! They will be expanded with real assertions after the command
//! implementations land in Plan 04-01 Task 1 and Task 2.

use assert_cmd::Command;

/// Stub: update command accepts required args and produces output.
/// Will test: convert -> fetch -> merge -> upload pipeline (CLI-01).
#[test]
#[ignore = "stub — requires implementation from 04-01 Task 2"]
fn test_update_command() {
    let _cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    // TODO: set up wiremock server, temp markdown file, invoke update, assert exit code + output
}

/// Stub: upload command accepts required args and produces output.
/// Will test: convert -> direct upload pipeline (CLI-02).
#[test]
#[ignore = "stub — requires implementation from 04-01 Task 2"]
fn test_upload_command() {
    let _cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    // TODO: set up wiremock server, temp markdown file, invoke upload, assert exit code + output
}

/// Stub: convert command writes storage XML and SVGs to output dir.
/// Will test: local conversion without Confluence credentials (CLI-03).
#[test]
#[ignore = "stub — requires implementation from 04-01 Task 2"]
fn test_convert_command() {
    let _cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    // TODO: create temp markdown, invoke convert with temp output dir, assert files written
}

/// Stub: --output json emits valid JSON on stdout.
/// Will test: JSON output mode (CLI-05).
#[test]
#[ignore = "stub — requires implementation from 04-02"]
fn test_json_output_mode() {
    let _cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    // TODO: invoke convert --output json, parse stdout as JSON, assert schema matches D-02
}
