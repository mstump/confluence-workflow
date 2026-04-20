# Phase 9: Convert Waterfall Fix and Phase 08 Verification — Context

**Gathered:** 2026-04-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Two targeted fixes:

1. Close the SCAF-03 integration gap: fix the `convert` arm's `DiagramConfig` construction by leveraging clap-derive's existing env-var resolution and refactoring `Config` to be built directly from `Cli` fields, removing the `CliOverrides` indirection layer.
2. Produce a Phase 08 `VERIFICATION.md` via goal-backward analysis of Phase 08's success criteria against the current codebase.

This phase does NOT add new capabilities.

</domain>

<decisions>
## Implementation Decisions

### Config Refactor (SCAF-03 fix)

- **D-01:** **Use clap-derive for CLI + Config.** The `Cli` struct already uses clap-derive with `#[arg(long, env = "...")]`, meaning clap already handles CLI → env resolution for all fields including `plantuml_path` and `mermaid_path`. The fix is to build `Config` directly from `Cli` fields, not from a separate `CliOverrides` struct that re-reads env vars manually.
- **D-02:** **Remove `CliOverrides`.** The `CliOverrides` struct is an unnecessary indirection layer. Clap's `env` attribute owns the two-tier (CLI flag → env var) resolution; `CliOverrides` duplicates that work. Downstream: `Config::load()` signature changes to accept `&Cli` (or relevant fields) directly.
- **D-03:** **`DiagramConfig` does NOT need the `~/.claude/` tier.** That tier is only for credentials (Anthropic API key, Confluence credentials). Diagram path resolution is CLI flag → env var → default, fully handled by clap.
- **D-04:** **Convert arm fix:** Use `cli.plantuml_path.clone().unwrap_or_else(|| "plantuml".to_string())` (and similar for mermaid). Clap already resolved the env var, so no manual `std::env::var()` call is needed. The manual `DiagramConfig` construction with redundant `std::env::var()` calls is the bug.
- **D-05:** **`~/.claude/` tier stays for credentials only.** `Config::load()` (or its replacement) should still read `~/.claude/settings.json` for `ANTHROPIC_API_KEY` and Confluence credentials when env vars and CLI flags are absent.

### Test for Env-Var Tier (Plan 09-01)

- **D-06:** New test verifies that `PLANTUML_PATH` and `MERMAID_PATH` env vars set the diagram paths when no CLI flag is provided. Clap handles this resolution, but a test confirms the wiring is correct end-to-end. Existing `test_convert_with_diagram_path_flags` covers the CLI flag tier; the new test covers the env var tier.

### Phase 08 VERIFICATION.md (Plan 09-02)

- **D-07:** Cover automated evidence only — goal-backward analysis of each Phase 08 success criterion against the current codebase and test suite. Human-verification items (real Confluence upload, visual rendering) are flagged as `human_needed` but do not block the verification from passing. Format follows the VERIFICATION.md convention already used in Phases 01–05.

### Claude's Discretion

- Whether to keep `Config::load()` as the method name or rename it (e.g., `Config::from_cli()`) — follow what makes the API clearest
- Exact refactoring boundary if `CliOverrides` removal requires touching tests — update tests as needed
- VERIFICATION.md structure and scoring detail for Phase 08

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Source Files (Phase 9 scope)

- `src/cli.rs` — existing `Cli` struct with clap-derive (already uses `#[arg(long, env = "...")]`)
- `src/config.rs` — `Config`, `DiagramConfig`, `CliOverrides`, `Config::load()`, and `load_from_claude_config()` — primary refactor target
- `src/lib.rs` — `run()` function, all three command arms; convert arm (~line 211) is the SCAF-03 gap location

### Phase 08 Reference (for VERIFICATION.md)

- `.planning/phases/08-diagramconfig-waterfall-and-nyquist-compliance/08-01-PLAN.md` — Phase 08 must_haves and success criteria
- `.planning/phases/08-diagramconfig-waterfall-and-nyquist-compliance/08-02-PLAN.md` — Phase 08 plan 2 criteria
- `.planning/phases/08-diagramconfig-waterfall-and-nyquist-compliance/08-VALIDATION.md` — existing validation evidence
- `.planning/v1.0-MILESTONE-AUDIT.md` — SCAF-03 finding details and Phase 08 verification status table

### Requirements

- `.planning/REQUIREMENTS.md` — SCAF-03 definition (credential + config waterfall)

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets

- `Cli` struct (`src/cli.rs`): already has `plantuml_path: Option<String>` and `mermaid_path: Option<String>` with `#[arg(long, env = "PLANTUML_PATH")]` / `#[arg(long, env = "MERMAID_PATH")]` — clap resolves CLI flag and env var automatically
- `load_from_claude_config()` (`src/config.rs`): internal function for reading `~/.claude/settings.json` — keep for credential fields

### Established Patterns

- All tests use `Config::load_with_home()` with a fake home path to avoid reading real credentials — this pattern must be preserved after refactor
- Existing `test_convert_with_diagram_path_flags` covers CLI flag tier for diagram paths — complement with env-var tier test, don't replace

### Integration Points

- `CliOverrides` is constructed in `src/lib.rs` `run()` from `Cli` fields and passed to `Config::load()` — this indirection disappears after refactor
- `DiagramConfig` is consumed only by `MarkdownConverter::new()` — interface unchanged

### Known Issue (convert arm, ~line 211–226 src/lib.rs)

- The `dotenvy::dotenv().ok()` call in the convert arm is redundant after refactor — `Config::load()` (for update/upload) and clap's env resolution handle this
- Manual `std::env::var("PLANTUML_PATH").ok()` in convert arm is what causes the SCAF-03 gap (bypasses clap's already-resolved value on `cli.plantuml_path`)

</code_context>

<specifics>
## Specific Ideas

- User explicitly stated: **"There is no reason why DiagramConfig would need access to ~/.claude"** — do not add `~/.claude/` reading to `DiagramConfig` or diagram path resolution
- User explicitly stated: **"We should be using clap-derive for handling the config parsing"** — the fix is structural (use what clap already provides), not a new waterfall layer
- The `CliOverrides` struct is the core structural problem — it duplicates resolution that clap-derive already performs

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 09-convert-waterfall-fix-and-phase-08-verification*
*Context gathered: 2026-04-20*
