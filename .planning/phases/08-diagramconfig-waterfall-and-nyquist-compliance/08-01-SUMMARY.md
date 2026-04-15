---
plan: 08-01
phase: 08-diagramconfig-waterfall-and-nyquist-compliance
status: complete
completed: 2026-04-15
self_check: PASSED
---

# Plan 08-01: DiagramConfig CLI Flags and Waterfall Wiring

## What Was Built

Added `--plantuml-path` and `--mermaid-path` CLI flags to the `Cli` struct and wired `DiagramConfig` through the full `Config` waterfall (CLI override → env var → `~/.claude/` fallback → default). All three command arms in `lib.rs` now use `MarkdownConverter::new(diagram_config)` instead of `MarkdownConverter::default()`.

## Tasks Completed

| Task | Status | Commit |
|------|--------|--------|
| Task 1: Add CLI flags and extend CliOverrides/Config with DiagramConfig waterfall | ✓ | 4d12992 |
| Task 2: Wire DiagramConfig through lib.rs command arms | ✓ | 4d12992 |

## Key Files Changed

- `src/cli.rs` — Added `plantuml_path` and `mermaid_path` `Option<String>` fields with `#[arg(long, env)]`
- `src/config.rs` — Extended `CliOverrides` and `Config`; implemented waterfall resolution in `load_with_home()`; fixed test 7 struct literal; added 3 new waterfall tests (11–13)
- `src/lib.rs` — All three command arms (`update`, `upload`, `convert`) now forward CLI flags through `CliOverrides` and use `MarkdownConverter::new(diagram_config)`

## Verification

- `cargo build` — passed (zero errors)
- `cargo test --lib config::tests` — 13/13 passed (10 existing + 3 new)
- `cargo test` (full suite) — all tests passed
- `grep "MarkdownConverter::default()" src/lib.rs` — 0 matches ✓

## Deviations

None. Implementation followed the plan exactly.

## Self-Check: PASSED
