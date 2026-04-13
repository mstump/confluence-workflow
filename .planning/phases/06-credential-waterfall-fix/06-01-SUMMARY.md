---
phase: 06-credential-waterfall-fix
plan: 01
subsystem: cli-credentials
tags: [cli, credentials, waterfall, clap]
dependency_graph:
  requires: []
  provides: [anthropic-api-key-cli-flag, credential-waterfall-wiring]
  affects: [src/cli.rs, src/lib.rs, tests/cli_integration.rs]
tech_stack:
  added: []
  patterns: [clap-env-attribute, cli-override-wiring]
key_files:
  created: []
  modified:
    - src/cli.rs
    - src/lib.rs
    - tests/cli_integration.rs
decisions:
  - "Upload arm uses move instead of clone for anthropic_api_key since it is the last use"
metrics:
  duration: 144s
  completed: 2026-04-13T17:44:36Z
  tasks_completed: 2
  tasks_total: 2
  files_modified: 3
---

# Phase 06 Plan 01: Credential Waterfall Fix Summary

Wire --anthropic-api-key CLI flag end-to-end through CliOverrides so the credential waterfall (CLI > env > .env > ~/.claude/) is functional for the Anthropic API key; fix integration test to validate the correct error path.

## Completed Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add --anthropic-api-key CLI flag and wire through CliOverrides | 0937ca1 | src/cli.rs, src/lib.rs |
| 2 | Fix test_update_command_missing_api_key integration test | 22f5945 | tests/cli_integration.rs |

## Deviations from Plan

None -- plan executed exactly as written.

## Verification Results

1. `cargo build` -- zero warnings, exit 0
2. `cargo test --test cli_integration test_update_command_missing_api_key` -- passes
3. `cargo test --test cli_integration` -- all 7 tests pass (1 ignored: happy-path requires TLS server)
4. `grep -c "anthropic_api_key: None" src/lib.rs` -- returns 0 (no hardwired None remaining)
5. `grep "cli.anthropic_api_key" src/lib.rs` -- shows 2 matches (Update arm with .clone(), Upload arm with move)

## Notes

Two pre-existing unit tests (`config::tests::test_env_vars_used_when_cli_absent` and `config::tests::test_fallthrough_to_env_vars`) fail intermittently when run as part of the full `cargo test` suite due to env var pollution between test threads. They pass individually. This is a pre-existing test isolation issue unrelated to this plan's changes -- not fixed per scope boundary rules.

## Known Stubs

None.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The API key flows through the existing CliOverrides -> Config::load -> resolve_optional waterfall without any new logging or serialization points. T-06-02 (API key not logged via tracing) verified -- no tracing statements touch the key value.

## Self-Check: PASSED

- All 4 files verified present on disk
- Commit 0937ca1 verified in git log
- Commit 22f5945 verified in git log
