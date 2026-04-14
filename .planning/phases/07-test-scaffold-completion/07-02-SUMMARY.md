---
phase: 07-test-scaffold-completion
plan: 02
subsystem: config-tests
tags: [testing, serial-test, dependency-cleanup]
dependency_graph:
  requires: []
  provides: [reliable-parallel-tests, clean-dependencies]
  affects: [Cargo.toml, src/config.rs]
tech_stack:
  added: [serial_test-3.4.0]
  patterns: [serial-test-annotation]
key_files:
  created: []
  modified: [Cargo.toml, src/config.rs]
decisions:
  - Annotated 7 tests with #[serial] instead of 2 (all env-var-mutating tests need serialization)
metrics:
  duration: 7m
  completed: 2026-04-14
---

# Phase 07 Plan 02: Config Test Race Condition Fix Summary

Fixed config test race condition with serial_test crate and removed unused anyhow dependency; 7 env-var-mutating tests annotated with #[serial] for reliable parallel execution.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add serial_test to dev-dependencies and remove anyhow | f95e2bd | Cargo.toml |
| 2 | Annotate config tests with #[serial] and verify | 4ce2b4c | src/config.rs |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Additional tests needed #[serial] annotation**
- **Found during:** Task 2
- **Issue:** Plan specified only 2 tests (test_fallthrough_to_env_vars, test_env_vars_used_when_cli_absent) for #[serial] annotation, but 5 additional tests also call std::env::set_var/remove_var on CONFLUENCE_* and ANTHROPIC_* environment variables. Without #[serial], test_missing_confluence_api_token_error failed because it observed leaked env vars from a concurrent test.
- **Fix:** Added #[serial] to all 7 tests that mutate environment variables: tests 2, 3, 4, 5, 6, 7, and 8. Tests 1, 9, and 10 do not mutate env vars and remain parallel.
- **Files modified:** src/config.rs
- **Commit:** 4ce2b4c

## Verification Results

- cargo build succeeds (anyhow removal clean)
- cargo test passes with default parallelism (no --test-threads=1)
- grep -c "anyhow" Cargo.toml returns 0
- grep -c "serial_test" Cargo.toml returns 1
- grep -c "#[serial]" src/config.rs returns 7
- Config tests pass reliably across 5 consecutive runs (10/10 each run)

## Self-Check: PASSED
