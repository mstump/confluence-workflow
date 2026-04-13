//! Tests for output formatting and stderr routing (CLI-04).
//!
//! These are compile-only stubs created as Wave 0 scaffolding.
//! They will be expanded with real assertions after Plan 04-02.

use assert_cmd::Command;

/// Stub: --verbose sends tracing output to stderr, not stdout.
/// Will test: stderr routing for tracing subscriber (CLI-04, D-07).
#[test]
#[ignore = "stub — requires implementation from 04-02"]
fn test_stderr_routing() {
    let _cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    // TODO: invoke convert --verbose, assert stderr has tracing output, stdout has only result line
}

/// Stub: default (non-verbose) mode produces no tracing output.
/// Will test: silent execution until done (D-04).
#[test]
#[ignore = "stub — requires implementation from 04-02"]
fn test_default_silent_mode() {
    let _cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    // TODO: invoke convert (no --verbose), assert stderr is empty or warnings-only
}
