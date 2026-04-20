---
phase: 09
plan: 02
subsystem: config
tags: [scaf-03, config, refactor, clap-derive, waterfall, dotenvy]
depends_on:
  requires:
    - phase: 09
      plan: 01
      provides: phase-08-verification-artifact (line numbers captured before this refactor)
    - phase: 08
      plan: 01
      provides: diagram-config-waterfall (the waterfall this refactor trusts)
  provides:
    - convert-arm-waterfall-fix
    - config-takes-cli-directly
    - env-var-tier-integration-coverage
  affects:
    - v1.0-milestone-audit
    - phase-10-tech-debt-cleanup
tech_stack:
  added: []
  patterns:
    - "Trust clap-derive: env-var tier lives in #[arg(env = \"...\")] attributes, not in downstream code"
    - "Config::load takes &Cli directly; no intermediate DTO struct"
    - "dotenvy hoisted to main.rs so .env is visible at Cli::parse() time"
    - "match &cli.command instead of match cli.command to keep cli unmoved for Config::load(&cli)"
key_files:
  created: []
  modified:
    - src/main.rs
    - src/config.rs
    - src/lib.rs
    - tests/cli_integration.rs
decisions:
  - "Used match &cli.command (not match cli.command) so cli remains unmoved and can be passed as &cli to Config::load — avoids Pitfall 2 from 09-RESEARCH.md (partial-move borrow error)"
  - "Removed two obsolete env-var-tier unit tests (test_fallthrough_to_env_vars, test_env_vars_used_when_cli_absent) — the code path they tested (Config::resolve_required reading std::env::var) no longer exists; coverage migrated to tests/cli_integration.rs::test_convert_with_env_var_diagram_paths which exercises the end-to-end clap-derive env= path"
  - "Retained DiagramConfig::from_env() and impl Default for DiagramConfig in src/config.rs because they have out-of-scope test-only callers in src/converter/; removing them is tracked as tech debt for Phase 10 (per 09-01-VERIFICATION.md Info-severity note and ROADMAP.md Phase 10 goals)"
  - "Dropped the ~/.claude/ fallback tier for diagram paths per D-03/D-05 — credentials only get that tier; diagram paths resolve CLI-tier-or-default only"
requirements_completed:
  - SCAF-03
metrics:
  duration: ~10 min
  completed: 2026-04-20
  tasks_completed: 2
  tasks_total: 2
  files_created: 0
  files_modified: 4
---

# Phase 9 Plan 02: Convert Waterfall Fix Summary

Refactored `Config::load` to take `&Cli` directly, deleted the `CliOverrides` DTO, hoisted `dotenvy::dotenv()` to `main.rs`, rewrote all three `lib.rs` command arms to trust clap-derive's already-resolved env-var values, and added an end-to-end env-var-tier integration test — closing the SCAF-03 structural gap identified in the v1.0 milestone audit.

## Performance

- **Duration:** ~10 min
- **Started:** 2026-04-20T20:27:03Z (base commit 8bbbbd1)
- **Completed:** 2026-04-20T20:36:28Z (final commit 9a75fb2)
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- `CliOverrides` struct deleted from `src/config.rs`; `Config::load` and `Config::load_with_home` now take `&Cli` directly.
- `dotenvy::dotenv().ok()` hoisted to the first line of `main()` in `src/main.rs`, ensuring `.env` values are visible to clap-derive's `#[arg(env = "...")]` attribute at `Cli::parse()` time.
- All three `lib.rs` command arms (Update, Upload, Convert) now consume `Config::load(&cli)?` or read `cli.plantuml_path.clone()` / `cli.mermaid_path.clone()` directly — no manual `or_else(|| std::env::var(...).ok())` waterfall remains.
- `resolve_required` and `resolve_optional` in `src/config.rs` dropped their env-var tier (clap owns it); they now have a 2-tier waterfall: Cli → `~/.claude/settings.json`.
- New `test_convert_with_env_var_diagram_paths` integration test in `tests/cli_integration.rs` exercises the env-var tier end-to-end (PLANTUML_PATH / MERMAID_PATH set but no `--plantuml-path` / `--mermaid-path` flags).
- Full `cargo test` suite passes (116 lib tests + 9 cli_integration + 12 llm_integration + 2 output_format = 139 passing, 1 ignored for TLS mock constraint).
- Zero compiler warnings, zero compiler errors.
- Phase 08 invariants preserved: `MarkdownConverter::default()` remains absent from `src/lib.rs`; `test_confluence_url_must_be_https` still passes (T-01-04 threat mitigation).

## Task Commits

Each task was committed atomically:

1. **Task 1: Hoist dotenvy to main.rs and refactor Config to take &Cli** — `00352a8` (refactor)
2. **Task 2: Rewrite all three lib.rs command arms and add env-var tier integration test** — `9a75fb2` (refactor)

## Files Created/Modified

- `src/main.rs` — Added `dotenvy::dotenv().ok();` as the first statement in `main()` (before `Cli::parse()`).
- `src/config.rs` — Deleted `CliOverrides` struct; added `use crate::cli::Cli` import; changed `Config::load` and `load_with_home` signatures to accept `&Cli`; removed env-var tier from `resolve_required` / `resolve_optional`; inlined diagram-path resolution as `cli.plantuml_path.clone().unwrap_or_else(...)`; replaced all 13 test-site `CliOverrides { ... }` literals with `Cli { ..cli_blank() }` via new `cli_blank()` helper; removed 2 obsolete env-var-tier unit tests with a migration comment.
- `src/lib.rs` — Changed `use config::{CliOverrides, Config, DiagramConfig}` to `use config::{Config, DiagramConfig}`; restructured outer match to `match &cli.command` so `cli` stays unmoved; Update and Upload arms replaced CliOverrides wrapper construction with a single `Config::load(&cli)?` call; Convert arm reads `cli.plantuml_path.clone()` / `cli.mermaid_path.clone()` directly and no longer calls `dotenvy::dotenv()` or `std::env::var("PLANTUML_PATH")` / `std::env::var("MERMAID_PATH")`.
- `tests/cli_integration.rs` — Added `use serial_test::serial` import; appended `test_convert_with_env_var_diagram_paths` (D-06, env-var-tier end-to-end coverage); existing `test_convert_with_diagram_path_flags` (CLI-flag-tier) left unchanged.

## Verification Results

### Automated acceptance criteria (all met)

Task 1:

- `grep -c "pub struct CliOverrides" src/config.rs` → 0 (struct deleted)
- `grep -c "CliOverrides" src/config.rs` → 0 (no references remain)
- `grep -c "pub fn load(cli: &Cli)" src/config.rs` → 1 (signature changed)
- `grep -c "dotenvy::dotenv" src/main.rs` → 1 (hoisted)
- `grep -c "cli.plantuml_path.clone" src/config.rs` → 1 (direct diagram-path read)
- `grep -c "cli.mermaid_path.clone" src/config.rs` → 1
- `grep -c "cli.confluence_token.as_deref" src/config.rs` → 1 (Pitfall 4 — field name is `confluence_token` on Cli)
- `grep -c "fn cli_blank" src/config.rs` → 1 (test helper)
- `cargo test --lib config::tests -- --test-threads=1` → 11 passed (13 original minus 2 removed env-var-tier tests, per decision)

Task 2:

- `grep -c "CliOverrides" src/lib.rs` → 0 (all references removed)
- `grep -c "Config::load(&cli)" src/lib.rs` → 2 (Update arm + Upload arm)
- `grep -c "dotenvy::dotenv" src/lib.rs` → 2 (only in comments — real call is in main.rs)
- `grep -c 'std::env::var("PLANTUML_PATH")' src/lib.rs` → 0 (clap-derive owns this tier)
- `grep -c 'std::env::var("MERMAID_PATH")' src/lib.rs` → 0
- `grep -c "std::env::var" src/lib.rs` → 2 (only MERMAID_PUPPETEER_CONFIG and DIAGRAM_TIMEOUT — they have no CLI flag, so must remain)
- `grep -c "cli.plantuml_path.clone" src/lib.rs` → 1 (Convert arm direct read)
- `grep -c "cli.mermaid_path.clone" src/lib.rs` → 1
- `grep -c "MarkdownConverter::default()" src/lib.rs` → 0 (Phase 08 invariant preserved)
- `grep -c "fn test_convert_with_env_var_diagram_paths" tests/cli_integration.rs` → 1
- `grep -c "fn test_convert_with_diagram_path_flags" tests/cli_integration.rs` → 1 (unchanged, still present)

### Test suite

- `cargo build` → 0 errors, 0 warnings
- `cargo test` → 116 lib + 9 cli_integration (1 ignored) + 12 llm_integration + 2 output_format = 139 passing, 1 ignored
- `cargo test --lib config::tests::test_confluence_url_must_be_https -- --exact` → 1 passed (T-01-04 preserved)
- `cargo test --lib config::tests::test_plantuml_path_cli_override -- --exact` → 1 passed
- `cargo test --lib config::tests::test_mermaid_path_cli_override -- --exact` → 1 passed
- `cargo test --lib config::tests::test_diagram_config_defaults_when_no_override -- --exact` → 1 passed
- `cargo test --test cli_integration test_convert_with_diagram_path_flags` → 1 passed (CLI-flag tier regression)
- `cargo test --test cli_integration test_convert_with_env_var_diagram_paths` → 1 passed (new env-var-tier coverage)

## Decisions Made

1. **`match &cli.command` instead of `match cli.command`** — Borrowing `cli.command` (instead of moving it) keeps `cli` unmoved so `Config::load(&cli)` compiles inside each arm. Arm-bound fields (`markdown_path`, `page_url`, `output_dir`) are cloned at the top of each arm. This avoids Pitfall 2 from 09-RESEARCH.md (partially-moved value) without restructuring at a higher cost.
2. **Removed two env-var-tier unit tests (`test_fallthrough_to_env_vars`, `test_env_vars_used_when_cli_absent`)** — these tested the `Config::resolve_required` env-var tier, which has been deleted because clap-derive's `#[arg(env = "...")]` attribute now owns that resolution. The tests could not be ported meaningfully: they construct `Cli` directly (bypassing `Cli::parse()`) so clap's env= never fires, and the old env-var tier inside `resolve_required` no longer exists. End-to-end env-var-tier coverage is provided by the new integration test `test_convert_with_env_var_diagram_paths` which exercises the full `Cli::parse()` → `cli.plantuml_path` → DiagramConfig flow.
3. **Dropped `~/.claude/` fallback tier for diagram paths (D-03/D-05)** — Only credentials (Confluence URL/username/token, Anthropic API key) get the `~/.claude/settings.json` fallback. Diagram paths resolve Cli-tier-or-default (`"plantuml"` / `"mmdc"`) only.
4. **Retained `DiagramConfig::from_env()` and `impl Default for DiagramConfig`** — These are out-of-scope test-fixture helpers with no production callers in `src/lib.rs`. Removing them is tracked as tech debt for Phase 10 per `08-VERIFICATION.md` Info-severity note and `.planning/ROADMAP.md` (Phase 10 goal: "remove the dead `DiagramConfig::from_env()` public API").

## Deviations from Plan

None — plan executed exactly as written. The two optional adjustments the plan authorized (match-by-ref vs. by-value; removing obsolete tests) were the recommended path per Pitfall 2 and the plan's own B.7 guidance, so they are not deviations.

## Issues Encountered

- None. The plan's pre-flight research (09-RESEARCH.md Pitfalls 2 and 3) accurately predicted the two design decisions required (match-by-ref, test-site cascade), so execution was mechanical.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- SCAF-03 integration gap closed. The convert arm no longer reconstructs the env-var waterfall manually; it trusts clap-derive's already-resolved `Cli` fields end-to-end.
- Phase 09-01 VERIFICATION.md line numbers remain accurate because this plan ran in Wave 2 (after 09-01 authored the verification artifact).
- Phase 10 can now proceed to remove the orphaned `DiagramConfig::from_env()` and `impl Default for DiagramConfig` public API surface (tech debt documented in 08-VERIFICATION.md Anti-Patterns section; referenced in ROADMAP.md Phase 10).
- No blockers.

## Self-Check

- `src/main.rs` (modified) — FOUND
- `src/config.rs` (modified) — FOUND
- `src/lib.rs` (modified) — FOUND
- `tests/cli_integration.rs` (modified) — FOUND
- Commit `00352a8` (Task 1) — FOUND
- Commit `9a75fb2` (Task 2) — FOUND

## Self-Check: PASSED

---
*Phase: 09-convert-waterfall-fix-and-phase-08-verification*
*Plan: 02*
*Completed: 2026-04-20*
