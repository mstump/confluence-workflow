---
phase: 04-cli-command-wiring-and-integration
plan: 02
subsystem: cli-output-and-tracing
tags: [cli, tracing, json-output, exit-codes]
dependency_graph:
  requires: [04-01]
  provides: [json-output-formatting, tracing-init, exit-code-handling]
  affects: [src/main.rs, src/lib.rs]
tech_stack:
  added: []
  patterns: [tracing-subscriber-stderr, json-output-mode, exit-code-dispatch]
key_files:
  created: []
  modified: [src/lib.rs, src/main.rs]
decisions:
  - JSON errors emitted to stdout (not stderr) per D-03 for machine parsing
  - main() uses explicit process::exit(1) instead of Result return to ensure JSON output always emitted before exit
  - Verbose detail (comment counts, file list) routed to stderr via eprintln
metrics:
  duration: 210s
  completed: "2026-04-13T16:21:41Z"
  tasks_completed: 2
  tasks_total: 2
---

# Phase 04 Plan 02: Output Formatting, Tracing, and Exit Codes Summary

JSON and human output formatting with tracing-subscriber on stderr and 0/1 exit codes

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add JSON output formatting functions to lib.rs | 9fbf2cb | src/lib.rs |
| 2 | Implement tracing, output dispatch, and exit codes in main.rs | 2bed648 | src/main.rs |

## What Was Built

### JSON Output Formatting (lib.rs)

- `result_to_json()`: Converts each `CommandResult` variant to a JSON Value with `success: true` and variant-specific fields (page_url, comments_kept, comments_dropped, output_dir, files)
- `error_to_json()`: Converts `AppError` to `{ success: false, error: "..." }` using Display impl (no credential leakage)

### Tracing Subscriber (main.rs)

- `init_tracing(verbose)`: Initializes tracing-subscriber with stderr-only output
- Default level: `warn`; with `--verbose`: `debug`
- Uses `EnvFilter` for level control, `fmt()` for human-readable format

### Output Dispatch (main.rs)

- JSON mode: Both success and error output to stdout as single JSON object; silent during execution
- Human mode: Success prints one line to stdout; failure prints error to stderr
- Verbose mode: Additional detail (comment counts, file lists) to stderr via `eprintln!()`

### Exit Codes (main.rs)

- Exit 0 on success (implicit)
- Exit 1 on failure via `std::process::exit(1)` after output is emitted

## Deviations from Plan

None - plan executed exactly as written.

## Verification Results

- `cargo check` passes with zero errors
- `cargo test --lib` passes (113/115; 2 pre-existing flaky config tests due to env var races, documented in STATE.md)
- All acceptance criteria verified

## Self-Check: PASSED

- src/lib.rs: FOUND, contains result_to_json (1), error_to_json (1)
- src/main.rs: FOUND, contains init_tracing (2), OutputFormat::Json (1), OutputFormat::Human (1), process::exit(1) (2)
- Commit 9fbf2cb: FOUND
- Commit 2bed648: FOUND
