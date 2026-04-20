# Phase 9: Convert Waterfall Fix and Phase 08 Verification — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-20
**Phase:** 09-convert-waterfall-fix-and-phase-08-verification
**Areas discussed:** Config waterfall approach, Test scope, Phase 08 VERIFICATION.md

---

## Config Waterfall Approach

| Option | Description | Selected |
|--------|-------------|----------|
| DiagramConfig::load_waterfall() | New method for diagram-only 3-tier waterfall, no Confluence creds | |
| Call Config::load(), ignore error | Literal success criteria wording; hides real config errors | |
| Make Confluence creds optional | Bigger refactor; affects update/upload arms | |
| clap-derive + Config from Cli | Remove CliOverrides, build Config from Cli fields directly | ✓ |

**User's choice:** "We should be using clap-derive for handling the config parsing. And there is no reason why DiagramConfig would need access to ~/.claude."

**Notes:** User clarified that:
1. The CLI is already on clap-derive and already handles CLI→env resolution via `#[arg(env = "...")]`
2. `DiagramConfig` does not need a `~/.claude/` tier — that's credentials-only
3. The structural fix is to remove `CliOverrides` and build `Config` from `Cli` fields directly, eliminating the duplicate env-var reading
4. "Config struct too" was selected for migration scope — both CLI and Config struct

---

## Test Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Env-var resolution via clap | Test PLANTUML_PATH/MERMAID_PATH env vars when no CLI flag provided | ✓ |
| Full waterfall for convert | Test CLI flag > env var > default in sequence | |
| Claude's discretion | Leave test design to planner | |

**User's choice:** Env-var resolution via clap (Recommended)

**Notes:** Existing `test_convert_with_diagram_path_flags` already covers the CLI flag tier. New test complements it by covering env var tier.

---

## Phase 08 VERIFICATION.md

| Option | Description | Selected |
|--------|-------------|----------|
| Automated evidence only | Goal-backward analysis; human items flagged as pending | ✓ |
| Include human verification | Requires manual testing against real Confluence first | |
| Claude's discretion | Planner determines scope | |

**User's choice:** Automated evidence only (Recommended)

**Notes:** Human-verification items (real Confluence upload, visual rendering) are flagged as `human_needed` but do not block verification.

---

## Claude's Discretion

- Whether to rename `Config::load()` to something more descriptive (e.g., `Config::from_cli()`)
- Exact refactoring boundary if `CliOverrides` removal requires touching tests
- VERIFICATION.md structure and scoring detail for Phase 08

## Deferred Ideas

None.
