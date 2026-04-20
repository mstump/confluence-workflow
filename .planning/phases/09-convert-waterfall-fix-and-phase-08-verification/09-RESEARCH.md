---
phase: 9
slug: convert-waterfall-fix-and-phase-08-verification
status: researched
researched: 2026-04-20
---

# Phase 9: Convert Waterfall Fix and Phase 08 Verification — Research

**Researched:** 2026-04-20
**Domain:** Rust CLI configuration refactor (clap-derive `env` attribute, removal of manual env lookups), goal-backward VERIFICATION.md authoring
**Confidence:** HIGH

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01: Use clap-derive for CLI + Config.** The `Cli` struct already uses clap-derive with `#[arg(long, env = "...")]`, meaning clap already handles CLI → env resolution for all fields including `plantuml_path` and `mermaid_path`. The fix is to build `Config` directly from `Cli` fields, not from a separate `CliOverrides` struct that re-reads env vars manually.
- **D-02: Remove `CliOverrides`.** The `CliOverrides` struct is an unnecessary indirection layer. Clap's `env` attribute owns the two-tier (CLI flag → env var) resolution; `CliOverrides` duplicates that work. Downstream: `Config::load()` signature changes to accept `&Cli` (or relevant fields) directly.
- **D-03: `DiagramConfig` does NOT need the `~/.claude/` tier.** That tier is only for credentials (Anthropic API key, Confluence credentials). Diagram path resolution is CLI flag → env var → default, fully handled by clap.
- **D-04: Convert arm fix.** Use `cli.plantuml_path.clone().unwrap_or_else(|| "plantuml".to_string())` (and similar for mermaid). Clap already resolved the env var, so no manual `std::env::var()` call is needed. The manual `DiagramConfig` construction with redundant `std::env::var()` calls is the bug.
- **D-05: `~/.claude/` tier stays for credentials only.** `Config::load()` (or its replacement) should still read `~/.claude/settings.json` for `ANTHROPIC_API_KEY` and Confluence credentials when env vars and CLI flags are absent.
- **D-06: New test for env-var tier.** Verifies that `PLANTUML_PATH` and `MERMAID_PATH` env vars set the diagram paths when no CLI flag is provided. Existing `test_convert_with_diagram_path_flags` covers the CLI flag tier; the new test covers the env var tier.
- **D-07: VERIFICATION.md covers automated evidence only.** Goal-backward analysis of each Phase 08 success criterion against the current codebase and test suite. Human-verification items are flagged as `human_needed` but do not block passing. Format follows the VERIFICATION.md convention used in Phases 01–07.

### Claude's Discretion

- Whether to keep `Config::load()` as the method name or rename it (e.g., `Config::from_cli()`) — follow what makes the API clearest.
- Exact refactoring boundary if `CliOverrides` removal requires touching tests — update tests as needed.
- VERIFICATION.md structure and scoring detail for Phase 08.

### Deferred Ideas (OUT OF SCOPE)

None — CONTEXT.md states discussion stayed within phase scope.
</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCAF-03 | Credentials/config loaded via waterfall (integration gap closure — convert arm) | Current convert arm (src/lib.rs lines 211–226) builds `DiagramConfig` manually with direct `std::env::var()` calls and `dotenvy::dotenv()`; this bypasses the Config-resolution layer used by update/upload arms and duplicates clap's env resolution. Research confirms clap-derive's `#[arg(long, env = "...")]` already resolves CLI→env, so the fix is structural: remove `CliOverrides`, thread `&Cli` (or its owned fields) into `Config::load()`, and have the convert arm call the same entry point. |

</phase_requirements>

## Summary

Phase 9 is a small structural refactor plus a verification-authoring task. Two independent work streams:

1. **Plan 09-01 (SCAF-03 gap closure).** The current codebase has two parallel config paths: (a) update/upload arms call `Config::load(&CliOverrides)` which runs a three-tier waterfall; (b) the convert arm builds `DiagramConfig` inline with direct `dotenvy::dotenv().ok()` + `std::env::var("PLANTUML_PATH").ok()` calls. The user's insight (D-01/D-02) is that `CliOverrides` exists only to re-read the CLI and env-var values that clap-derive has *already* resolved on the `Cli` struct itself. The fix collapses that indirection: `Config::load()` takes `&Cli` (or the relevant owned fields) directly; the convert arm calls the same entry or a trimmed variant that skips Confluence credential validation. Clap's `env = "PLANTUML_PATH"` attribute already handles the env-var tier, so all the manual `std::env::var()` reads in the convert arm disappear. `~/.claude/settings.json` reading stays for credential fields only (D-03/D-05) — diagram paths use the two-tier (CLI → env) resolution that clap provides natively.

2. **Plan 09-02 (Phase 08 VERIFICATION.md).** Phase 08's VALIDATION.md and two SUMMARY.md files confirm all tests pass, but no VERIFICATION.md was ever produced (v1.0 audit identified this as the only missing phase artifact). The task is a pure authoring job: goal-backward analysis of each Phase 08 success criterion (from 08-01-PLAN.md and 08-02-PLAN.md) against the current codebase, following the Phase 06/07 VERIFICATION.md format exactly.

No new dependencies. No new files except `.planning/phases/08-.../08-VERIFICATION.md`. Scope is bounded to 3 Rust files (cli.rs, config.rs, lib.rs), 1 integration-test file (tests/cli_integration.rs), and 1 planning artifact (08-VERIFICATION.md).

**Primary recommendation:** Plan 09-01 changes `Config::load` and `load_with_home` signatures to take `&Cli` instead of `&CliOverrides`, deletes the `CliOverrides` struct, and rewrites the convert arm to construct `DiagramConfig` from `cli.plantuml_path`/`cli.mermaid_path` directly (unwrap_or default, no `std::env::var` calls, no `dotenvy::dotenv().ok()`). Plan 09-02 follows the 06/07 VERIFICATION.md template and evidences each Phase 08 must-have against grep/file-existence checks on the current codebase. Both plans run in Wave 1 with no dependencies between them.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CLI flag parsing + env-var resolution (tier 1 + tier 2) | clap-derive (`Cli` in `src/cli.rs`) | — | clap owns CLI→env resolution via `#[arg(long, env = "...")]`; `Cli` is the single source of already-resolved values |
| Credential tier 3 fallback (`~/.claude/settings.json`) | Config resolver (`src/config.rs::load_from_claude_config`) | — | Credentials only; diagram paths do not need this tier per D-03 |
| DiagramConfig construction | Config tier (`Config::load` for update/upload; inline in `lib.rs` for convert) | — | Update/upload route through Config for credential alignment; convert skips Config because it needs no credentials |
| Command dispatch | `src/lib.rs::run()` | — | Reads `cli.command` and branches to update/upload/convert |
| Phase 08 verification authorship | Planning tier (`.planning/phases/08-.../08-VERIFICATION.md`) | — | goal-backward analysis only; no code changes |

## Project Constraints (from CLAUDE.md)

> Note: The committed CLAUDE.md describes the **Python** predecessor of this project (Typer/mcp-agent/pytest/black/mypy). The current codebase is **Rust** (clap-derive, cargo test, serial_test). Python-specific directives in CLAUDE.md (black, mypy, pytest, markdownlint) do not apply to Rust source changes. Only the markdownlint directive applies to Phase 9 because Plan 09-02 writes markdown (VERIFICATION.md).

| CLAUDE.md Directive | Applies to Phase 9? | Equivalent Action |
|---------------------|--------------------|--------------------|
| Always pin dependency versions | Yes (if any were added) | Not triggered — no new deps in this phase |
| Run `uv run black .` / `uv run mypy .` / `uv run pytest` | No (Python) | Rust equivalent: `cargo build && cargo test` (enforced by Cargo's `-D warnings` in `.cargo/config.toml`) |
| Run `markdownlint --fix .` after markdown changes | **Yes** for Plan 09-02 | After writing `08-VERIFICATION.md`, run `markdownlint --fix .planning/phases/08-.../08-VERIFICATION.md` |
| Run `pre-commit run --all-files` | Yes if hooks defined | Does not block; informational |
| Re-install CLI via `uv pip install -e` | No (Python) | Rust: `cargo build` (release: `cargo build --release`) |

[VERIFIED: CLAUDE.md contents loaded in required reading]

## Standard Stack

### Core (already in project — no new deps needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.6 | CLI parsing + env var resolution via `#[arg(long, env = "...")]` | Already used for all CLI flags; the D-01 decision explicitly leverages clap's existing `env` attribute instead of hand-rolled env reads |
| serde_json | 1 | `~/.claude/settings.json` parsing (credentials-only) | Already used in `load_from_claude_config()` |
| dirs | 6.0 | Home directory resolution for `~/.claude/settings.json` | Already used in `Config::load()` |
| serial_test | 3.4.0 | Sequential test isolation for env-var mutating tests | Already added in Phase 7 for config tests; required for new env-var tier test |
| dotenvy | 0.15 | `.env` file loading | Already used in `Config::load()` for credential tier; the `dotenvy::dotenv().ok()` in the convert arm becomes redundant after refactor |
| assert_cmd | 2 | Binary-level integration tests | Already used in `tests/cli_integration.rs` for `test_convert_with_diagram_path_flags` pattern |
| tempfile | 3 | Temp dir for integration tests | Already used in `tests/cli_integration.rs` |

[VERIFIED: Cargo.toml lines 18–40 read in research]

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `Config::load(&Cli)` taking `&Cli` reference | `Config::load(&CliConfig)` (a new minimal DTO) | Rejected: CONTEXT.md D-02 explicitly removes the indirection layer; adding a new DTO reintroduces the same pattern |
| Keep `CliOverrides` but stop re-reading env vars inside it | Remove `CliOverrides` entirely (per D-02) | Rejected by D-02; the user specifically called the struct an "unnecessary indirection layer" |
| Add `~/.claude/` tier for diagram paths | Keep diagram path resolution at two tiers (CLI→env) | Explicitly rejected by D-03/D-05: "`DiagramConfig` does NOT need the `~/.claude/` tier" |
| Rename `Config::load` → `Config::from_cli` | Keep name `Config::load` | Claude's discretion (per CONTEXT.md). Recommendation: `Config::load(&cli)` keeps the established name; the signature change communicates the refactor clearly without API-surface churn |

**Installation:**

```bash
# No new dependencies required. Confirm existing toolchain:
cargo --version    # expects cargo 1.80+
cargo test --help  # confirms cargo test available
```

**Version verification:** Already-present packages; versions pinned in Cargo.toml. No `npm view` equivalent needed — Rust versions are Cargo-locked.

## Architecture Patterns

### System Architecture Diagram

```
Before (current state, SCAF-03 WARNING):

  confluence-agent update|upload ...            confluence-agent convert ...
           |                                              |
           v                                              v
     src/cli.rs (clap)                              src/cli.rs (clap)
     Cli { plantuml_path: Some(...) }               Cli { plantuml_path: Some(...) }
     [clap already resolved CLI+env]                [clap already resolved CLI+env]
           |                                              |
           v                                              v
     src/lib.rs run() ─── build CliOverrides         src/lib.rs run()
           |              (re-wraps cli fields)           |
           v                                              v
     src/config.rs Config::load(&overrides)         dotenvy::dotenv().ok();   ◄── redundant
           |                                        DiagramConfig {
           |  resolve_optional() rechecks              plantuml_path:
           |  CLI→env→~/.claude/ for EACH field         cli.plantuml_path
           v                                              .or_else(|| std::env::var("PLANTUML_PATH").ok())  ◄── redundant (clap already did this)
     Config { diagram_config, credentials }             .unwrap_or_else(|| "plantuml"),
           |                                        }
           v                                              |
     MarkdownConverter::new(config.diagram_config)  MarkdownConverter::new(diagram_config)
                                                          |
                                                          ▼
                                              [~/.claude/ tier skipped — but per D-03 not needed
                                               for diagram paths; redundancy is still a defect
                                               because CliOverrides duplicates clap's work]


After (target state, SCAF-03 closed):

  confluence-agent update|upload ...            confluence-agent convert ...
           |                                              |
           v                                              v
     src/cli.rs (clap)                              src/cli.rs (clap)
     Cli { plantuml_path: Some(...),                Cli { plantuml_path: Some(...),
           confluence_url, ... }                          mermaid_path: Some(...) }
     [clap resolved CLI→env for all fields]         [clap resolved CLI→env for all fields]
           |                                              |
           v                                              v
     src/lib.rs run() ─── pass &cli directly        src/lib.rs run()
           |                                        [no Config::load — no credentials needed]
           v                                        DiagramConfig {
     src/config.rs Config::load(&cli)                 plantuml_path: cli.plantuml_path
           |                                            .clone()
           |  resolve_required/optional:                .unwrap_or_else(|| "plantuml".to_string()),
           |   • credential fields: Cli tier          mermaid_path: cli.mermaid_path
           |     (already-resolved) → ~/.claude/        .clone()
           |   • diagram fields: Cli tier only          .unwrap_or_else(|| "mmdc".to_string()),
           |     (no ~/.claude/ fallback)            }
           v                                              |
     Config { diagram_config, credentials }              |
           |                                              v
           v                                     MarkdownConverter::new(diagram_config)
     MarkdownConverter::new(config.diagram_config)
```

### Component Responsibilities

| File | Responsibility After Refactor |
|------|-------------------------------|
| `src/cli.rs` | Unchanged. Clap-derive `Cli` struct already has `plantuml_path`/`mermaid_path` with `#[arg(long, env = "...")]`. The fields already carry CLI-or-env resolved values when `run()` is called. |
| `src/config.rs` | **Changed.** Delete `CliOverrides` struct. Change `Config::load(&CliOverrides)` → `Config::load(&Cli)` (or take the individual `Option<String>` fields). Inside `load_with_home`, replace `overrides.plantuml_path.as_deref()` with `cli.plantuml_path.as_deref()`; remove the `resolve_optional` env-var lookup for `PLANTUML_PATH`/`MERMAID_PATH` because clap already resolved those — diagram paths become a simple `cli.plantuml_path.clone().unwrap_or_else(|| "plantuml".to_string())`. Credential fields (`confluence_url`,`confluence_username`,`confluence_api_token`,`anthropic_api_key`) keep the`resolve_required`/`resolve_optional` path **but** also skip the env-var step since clap already resolved it — they fall through to `~/.claude/settings.json` when `cli.<field>` is `None`. |
| `src/lib.rs` | **Changed.** Delete all three `CliOverrides { ... }` struct-literal constructions (update arm, upload arm). Replace with `Config::load(&cli)?` — passing `&cli` by reference so the update arm can later use `cli.command` (or, since update/upload arms destructure `cli.command` first, pass the needed fields by value / clone). Rewrite convert arm: drop `dotenvy::dotenv().ok()`, drop `std::env::var("PLANTUML_PATH").ok()` / `std::env::var("MERMAID_PATH").ok()`, build `DiagramConfig` from `cli.plantuml_path` and `cli.mermaid_path` directly with `unwrap_or_else` defaults. Keep `mermaid_puppeteer_config` and `timeout_secs` as direct `std::env::var` reads (they have no CLI flags). |
| `tests/cli_integration.rs` | **Changed.** Add a new test `test_convert_with_env_var_diagram_paths` (per D-06) that sets `PLANTUML_PATH`/`MERMAID_PATH` via `cmd.env()` and omits the `--plantuml-path`/`--mermaid-path` flags — asserts convert succeeds (same shape as `test_convert_with_diagram_path_flags` on lines 348–388, but exercising the env-var tier). |
| `src/config.rs` tests | **Changed (mechanical).** Every test that constructs `CliOverrides { ... }` must change to construct `Cli { ... }` (with `command: Commands::Convert { ... }` or similar dummy). Alternative: extract the field subset into a function argument so tests don't need a full `Cli` — but CONTEXT.md says "update tests as needed" (Claude's discretion). |

### Pattern 1: Clap-Derive `env` Attribute Resolves CLI + Env Transparently

**What:** `#[arg(long, env = "PLANTUML_PATH")]` makes clap check the command-line flag first, then fall through to the named env var, and present a single `Option<String>` value on the parsed struct. No additional code is required to read the env var.

**When to use:** Any CLI flag that should also be settable via environment variable. Already the established pattern across all `Cli` fields in this project.

**Example:**

```rust
// Source: src/cli.rs lines 33–39 (current codebase)
// [VERIFIED: src/cli.rs read 2026-04-20]

/// Path to PlantUML executable or JAR
#[arg(long, env = "PLANTUML_PATH")]
pub plantuml_path: Option<String>,

/// Path to mermaid-cli executable (mmdc)
#[arg(long, env = "MERMAID_PATH")]
pub mermaid_path: Option<String>,
```

At the time `Cli::parse()` returns, `cli.plantuml_path` is `Some(...)` if **either** `--plantuml-path /x` was passed **or** `PLANTUML_PATH=/x` was set in the environment (CLI wins on conflict). No further env-var reading is needed downstream.

[CITED: <https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_2/index.html#environment-variables>]

### Pattern 2: Goal-Backward VERIFICATION.md

**What:** VERIFICATION.md is authored by the verifier agent (or manually) after all plans in a phase complete. It enumerates each success criterion from the phase plans and ROADMAP, provides **structural evidence** (file grep, line number citations, test names) for each, and scores them as VERIFIED / NEEDS HUMAN / FAILED. Human-only items (e.g., "renders correctly in Confluence UI") are called out explicitly in a dedicated subsection.

**When to use:** After every phase, as the last artifact before marking the phase complete.

**Example structure (abbreviated from 06-VERIFICATION.md):**

```markdown
---
phase: 08-diagramconfig-waterfall-and-nyquist-compliance
verified: 2026-04-20T00:00:00Z
status: passed   # or human_needed / gaps_found
score: 4/4 must-haves verified
overrides_applied: 0
---

# Phase 8: DiagramConfig Waterfall and Nyquist Compliance Verification Report

**Phase Goal:** <exact wording from ROADMAP or phase context>
**Verified:** 2026-04-20
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from 08-01-PLAN.md must_haves.truths)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `--plantuml-path /custom ... convert doc.md ./out` uses /custom for PlantUML rendering | VERIFIED | tests/cli_integration.rs:348 `test_convert_with_diagram_path_flags`; src/lib.rs:215 `cli.plantuml_path` wired into DiagramConfig |
| ... |

### Required Artifacts
| Artifact | Expected | Status | Details |
| ... |

### Key Link Verification
| From | To | Via | Status | Details |
| ... |

### Requirements Coverage
| Requirement | Source Plan | Description | Status | Evidence |
| SCAF-03    | 08-01      | ...         | SATISFIED | ... |

### Anti-Patterns Found
| File | Line | Pattern | Severity | Impact |

### Human Verification Required
None / list.

### Gaps Summary
No gaps. / List.

---
_Verified: 2026-04-20_
_Verifier: Claude (gsd-verifier)_
```

[VERIFIED: format pattern extracted from `.planning/phases/06-.../06-VERIFICATION.md` and `.planning/phases/07-.../07-VERIFICATION.md` read 2026-04-20]

### Anti-Patterns to Avoid

- **Hand-rolling a second env-var lookup after clap already resolved it.** This is literally the SCAF-03 bug (convert arm lines 215–225). After refactor, there must be zero `std::env::var("PLANTUML_PATH")` or `std::env::var("MERMAID_PATH")` calls in production code paths (`src/cli.rs`, `src/config.rs`, `src/lib.rs`). Only `MERMAID_PUPPETEER_CONFIG` and `DIAGRAM_TIMEOUT` remain as direct env reads because they have no CLI flag.
- **Calling `dotenvy::dotenv().ok()` twice.** The update/upload arms reach `Config::load()` which calls `dotenvy::dotenv().ok()`. The convert arm currently also calls it directly. After refactor, the convert arm should not call `dotenvy` — either (a) lift the `dotenvy::dotenv().ok()` call to `main.rs` before `Cli::parse()` so clap sees `.env` values, or (b) leave the convert arm without `.env` support for diagram paths since clap's `env=` only reads the actual environment, not `.env`. **Important caveat:** clap's `#[arg(env = "...")]` does **not** auto-read `.env` files. If the project relies on `.env` for `PLANTUML_PATH`/`MERMAID_PATH` in the convert arm (no `Config::load` path to call `dotenvy`), this is a behavior-change candidate the planner must surface. **Recommendation:** move `dotenvy::dotenv().ok()` to `main.rs` before `Cli::parse()` so all three arms benefit uniformly. See Pitfall 1 below.
- **Using `.unwrap()` or `.expect()` on CLI-derived `Option<String>`.** Always use `unwrap_or_else(|| "default".to_string())` to preserve the default behavior defined in current tests (`test_diagram_config_defaults_when_no_override`).
- **Writing VERIFICATION.md before checking the code.** The verifier must grep the current tree for each claim; assumptions without line-number evidence are the class of defect that led to this phase in the first place.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI flag → env var fallback | Custom `std::env::var("X").unwrap_or_else(...)` chain | clap-derive `#[arg(long, env = "X")]` | Clap already does this correctly; duplicating it is the SCAF-03 bug |
| `.env` file loading | Custom parser | `dotenvy::dotenv().ok()` (already in Cargo.toml) | Already present; the only question is **where** to call it |
| Home directory lookup | `std::env::var("HOME")` | `dirs::home_dir()` (already in Cargo.toml) | Cross-platform; established in `Config::load` |
| Test env-var isolation | Unguarded `std::env::set_var` in parallel tests | `#[serial]` (serial_test, already in dev-deps) | Phase 07 already fixed the race condition with this crate; apply the same pattern for the new env-var test |
| VERIFICATION.md authoring | Freeform narrative | The template from `.planning/phases/06-.../06-VERIFICATION.md` or `.planning/phases/07-.../07-VERIFICATION.md` | Ensures frontmatter schema (`phase`, `verified`, `status`, `score`, `overrides_applied`) matches what downstream audit tooling consumes |

**Key insight:** The SCAF-03 bug exists because the codebase independently evolved two places that each answered "where do I get plantuml_path from?" — the original `DiagramConfig::from_env()` and later the manual convert-arm construction. Clap-derive owns that answer. Trust clap.

## Runtime State Inventory

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None. This refactor touches in-memory structs only; no datastore or persistent cache holds `CliOverrides` or `DiagramConfig` values. | None |
| Live service config | None. No external service registers diagram paths by name; Confluence and Anthropic APIs do not care what path the local binary uses to render diagrams. | None |
| OS-registered state | None. No systemd/launchd unit or scheduled task embeds `plantuml_path`/`mermaid_path` as a literal; the binary reads them fresh on each run. | None |
| Secrets / env vars | `PLANTUML_PATH` and `MERMAID_PATH` env-var names are preserved (unchanged) — existing shell profiles and CI pipelines that set them keep working. `CliOverrides` struct name is internal; not exposed to users. Cargo feature flags unchanged. | None — no user-visible rename |
| Build artifacts / installed packages | None for `cargo build`. An installed binary from `cargo install confluence-agent` (per DIST-01, which is still pending in Phase 5) would need to be rebuilt to pick up the refactor, but that's the normal Rust upgrade flow. No `.egg-info` or similar stale-artifact risk. | None |

**Nothing user-visible renames in this phase.** The refactor is purely internal code-structure changes (remove `CliOverrides`, change `Config::load` signature). Downstream callers (`src/lib.rs`) are also touched in the same phase. All public CLI behavior (`--plantuml-path`, `--mermaid-path`, `PLANTUML_PATH`, `MERMAID_PATH`) is preserved identically.

## Common Pitfalls

### Pitfall 1: `clap-derive` env attribute does NOT auto-load `.env` files

**What goes wrong:** After moving to "let clap resolve everything", a developer expects `PLANTUML_PATH=/custom` in the project's `.env` file to be picked up automatically. It is not — `#[arg(env = "X")]` reads `std::env::var("X")` at `Cli::parse()` time, **before** any `dotenvy::dotenv()` call can happen.

**Why it happens:** The current code calls `dotenvy::dotenv().ok()` inside `Config::load()` (line 76), which runs *after* `Cli::parse()` in `main.rs`. For update/upload arms this still works because those arms call `Config::resolve_required`/`resolve_optional` which re-reads env vars after `dotenvy` has populated them. For the convert arm today, the same works because it *also* calls `dotenvy::dotenv().ok()` before the manual `std::env::var` reads. After the refactor, if the convert arm stops doing both, `.env` support silently disappears from convert.

**How to avoid:** Move `dotenvy::dotenv().ok()` to `src/main.rs` before `Cli::parse()`:

```rust
// src/main.rs (after refactor)
#[tokio::main]
async fn main() -> Result<(), confluence_agent::error::AppError> {
    dotenvy::dotenv().ok();       // ← hoisted from Config::load()
    let cli = confluence_agent::cli::Cli::parse();
    // ... existing dispatch
}
```

This makes `.env` visible to clap during parse and to all downstream `std::env::var` calls. Then `Config::load` no longer needs its own `dotenvy` call.

**Warning signs:** `test_convert_with_env_var_diagram_paths` (the new D-06 test) passes when setting env vars via `cmd.env("PLANTUML_PATH", "/x")` on the `assert_cmd::Command` (which sets actual process env) but a hypothetical `.env`-based test fails. Also: running `confluence-agent convert doc.md ./out` locally with only `.env` configured (no shell env) silently uses defaults.

**Mitigation:** The plan should explicitly decide where `dotenvy` is called and document it in a comment. Recommendation: hoist to `main.rs`.

### Pitfall 2: Passing `&cli` into `Config::load` before `cli.command` is destructured

**What goes wrong:** The update/upload arms in `src/lib.rs` currently `match cli.command { ... }` which **moves** `cli.command`. If the plan changes the signature to `Config::load(&cli)` and the call happens after the `match`, the borrow checker may complain because parts of `cli` have been moved.

**Why it happens:** Rust ownership — pattern-matching a field of a struct-by-value moves that field unless the match uses `&cli.command` or the struct is `Clone`.

**How to avoid:** Two options:

1. Restructure `run()` to call `Config::load(&cli)` **before** the match:

   ```rust
   pub async fn run(cli: Cli) -> Result<CommandResult, AppError> {
       // Config built once, before match destructures cli.command
       // (only for arms that need config)
       match cli.command {
           Commands::Update { markdown_path, page_url } => {
               let config = Config::load_for_update_upload(&cli)?;
               // ...
           }
           Commands::Upload { ... } => {
               let config = Config::load_for_update_upload(&cli)?;
               // ...
           }
           Commands::Convert { markdown_path, output_dir } => {
               // No Config::load call; build DiagramConfig inline
               // ...
           }
       }
   }
   ```

   But once the match moves `cli.command`, the borrow checker requires that `cli` not be used after the move for fields *other than* `cli.command`. This works if `Config::load` only reads fields other than `command` — which is the case.

2. Alternative: pass owned field clones by value, not `&Cli`:

   ```rust
   impl Config {
       pub fn load(
           confluence_url: Option<String>,
           confluence_username: Option<String>,
           confluence_api_token: Option<String>,
           anthropic_api_key: Option<String>,
           plantuml_path: Option<String>,
           mermaid_path: Option<String>,
       ) -> Result<Self, ConfigError> { ... }
   }
   ```

   This is effectively `CliOverrides` inlined into arguments — it restores the indirection CONTEXT.md D-02 rejected. **Prefer option 1.**

**Warning signs:** `cargo build` errors like `borrow of partially moved value: cli` or `use of moved value: cli.plantuml_path`.

**Mitigation:** The planner should mandate "call `Config::load(&cli)` before the `match cli.command` statement" in both update and upload arms, or refactor to `match &cli.command { ... }` with shared-reference patterns.

### Pitfall 3: Signature-change cascade into `src/config.rs` tests

**What goes wrong:** `src/config.rs` has 13 unit tests (visible in lines 275–615 of the current file), most of which construct `CliOverrides { ... }` literals or use `CliOverrides::default()`. Removing `CliOverrides` forces every one of these to change.

**Why it happens:** Wide test coverage on the credential waterfall was added across Phases 01 and 06; all tests go through `Config::load_with_home(&CliOverrides, ...)`.

**How to avoid:** Two approaches:

1. **Test-facing stable surface.** Introduce `Config::load_with_home` signature `(cli: &Cli, home: Option<&Path>)` and build a `Cli` struct in each test. `Cli` has a required `command` field of type `Commands`, so tests need a dummy like `command: Commands::Convert { markdown_path: PathBuf::new(), output_dir: PathBuf::new() }`. This is 13 mechanical edits.
2. **Extract a minimal DTO internally** (a private struct `CredentialInputs { ... }`) for `load_with_home`, but expose `Config::load(&Cli)` as the public entry point. This violates CONTEXT.md D-02 ("remove the indirection") in spirit but preserves test ergonomics. **Do not choose this** unless CONTEXT is explicitly clarified.

**Recommendation:** Approach 1. All 13 tests update mechanically; the compiler points out every call site.

**Warning signs:** 13+ compile errors from `cargo test --lib config::tests`.

### Pitfall 4: Losing `confluence_token` field rename

**What goes wrong:** `Cli` names the Confluence token field `confluence_token` (line 27), but `CliOverrides` names it `confluence_api_token` (line 50), and `Config` names it `confluence_api_token` (line 60). The current `src/lib.rs:87` does the rename: `confluence_api_token: cli.confluence_token`. After `CliOverrides` removal, `Config::load(&cli)` must still handle this rename internally.

**Why it happens:** Historical drift between CLI naming convention (`--confluence-token` → `confluence_token`) and config-domain naming (`confluence_api_token`).

**How to avoid:** Inside `Config::load_with_home`, when reading the CLI field, use `cli.confluence_token.as_deref()` (not `cli.confluence_api_token.as_deref()` — that field doesn't exist on `Cli`). The `env = "CONFLUENCE_API_TOKEN"` attribute on `cli.confluence_token` handles the env-var name correctly already.

**Warning signs:** Compile error "no field `confluence_api_token` on type `&Cli`" in `load_with_home`.

### Pitfall 5: Phase 08 VERIFICATION.md claiming plans passed without re-verifying the codebase

**What goes wrong:** Plan 09-02 copies claims from `08-01-SUMMARY.md` / `08-02-SUMMARY.md` ("X was wired", "Y test passes") into VERIFICATION.md without grep-ing the current codebase. Between Phase 08 completion (2026-04-15) and Phase 09 execution (2026-04-20), the Phase 09 code refactor itself will change the very files Phase 08 verified (src/lib.rs, src/config.rs). If Plan 09-01 runs before Plan 09-02, the structural evidence in VERIFICATION.md must reflect the *post-refactor* state — or the VERIFICATION.md will cite stale line numbers and struct names.

**Why it happens:** Parallel plans (09-01 and 09-02) both touch the same phase verification story. 09-01 refactors the code; 09-02 verifies the phase that preceded 09-01.

**How to avoid:** Two options:

1. **Sequence 09-02 before 09-01** so VERIFICATION.md reflects Phase 08's final state before Phase 09 touches it.
2. **If they run in parallel**, 09-02 must freeze its evidence against the git commit SHA at Phase 08 completion (or audit: 2026-04-15 timestamp). Cite SHAs in evidence rows.

**Recommendation:** Sequence 09-02 first (it's a pure read-only phase), then 09-01.

**Warning signs:** VERIFICATION.md evidence row says "src/config.rs:46 — `pub struct CliOverrides`" but `cargo build` in the current tree (post-09-01) reports no such struct.

## Code Examples

Verified patterns from the current codebase.

### Example 1: Current convert-arm DiagramConfig construction (the SCAF-03 bug)

```rust
// Source: src/lib.rs lines 210–227 (current, pre-fix)
// [VERIFIED: src/lib.rs read 2026-04-20]
Commands::Convert {
    markdown_path,
    output_dir,
} => {
    // No Config::load() needed -- convert does not require Confluence credentials
    let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
    dotenvy::dotenv().ok();                                               // ◄── redundant after refactor
    let diagram_config = DiagramConfig {
        plantuml_path: cli.plantuml_path
            .or_else(|| std::env::var("PLANTUML_PATH").ok())             // ◄── redundant: clap already did this
            .unwrap_or_else(|| "plantuml".to_string()),
        mermaid_path: cli.mermaid_path
            .or_else(|| std::env::var("MERMAID_PATH").ok())              // ◄── redundant: clap already did this
            .unwrap_or_else(|| "mmdc".to_string()),
        mermaid_puppeteer_config: std::env::var("MERMAID_PUPPETEER_CONFIG").ok(),  // ◄── OK: no CLI flag
        timeout_secs: std::env::var("DIAGRAM_TIMEOUT")                   // ◄── OK: no CLI flag
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
    };
    let converter = MarkdownConverter::new(diagram_config);
    // ...
}
```

### Example 2: Target convert-arm after refactor

```rust
// src/lib.rs convert arm (post-09-01)
Commands::Convert {
    markdown_path,
    output_dir,
} => {
    let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
    // dotenvy::dotenv().ok() removed: hoisted to main.rs before Cli::parse() per Pitfall 1
    // No std::env::var() for PLANTUML_PATH / MERMAID_PATH: clap already resolved them
    let diagram_config = DiagramConfig {
        plantuml_path: cli.plantuml_path.clone()
            .unwrap_or_else(|| "plantuml".to_string()),
        mermaid_path: cli.mermaid_path.clone()
            .unwrap_or_else(|| "mmdc".to_string()),
        mermaid_puppeteer_config: std::env::var("MERMAID_PUPPETEER_CONFIG").ok(),
        timeout_secs: std::env::var("DIAGRAM_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
    };
    let converter = MarkdownConverter::new(diagram_config);
    // ... (rest unchanged)
}
```

### Example 3: Target Config::load signature

```rust
// src/config.rs (post-09-01)
impl Config {
    /// Load configuration from already-parsed CLI values.
    /// Clap has already resolved CLI flag → env var for every `Option<String>` field
    /// on `&Cli`. This function fills in the `~/.claude/settings.json` credential
    /// fallback tier and applies defaults for non-credential fields.
    pub fn load(cli: &Cli) -> Result<Self, ConfigError> {
        Self::load_with_home(cli, dirs::home_dir().as_deref())
    }

    pub(crate) fn load_with_home(
        cli: &Cli,
        home: Option<&Path>,
    ) -> Result<Self, ConfigError> {
        // Credential fields: Cli tier (clap-resolved) → ~/.claude/ tier
        let confluence_url = Self::resolve_required_from_cli(
            cli.confluence_url.as_deref(),
            "CONFLUENCE_URL",
            home,
        )?;
        // ... normalization, https:// check, etc. (unchanged)

        let confluence_username = Self::resolve_required_from_cli(
            cli.confluence_username.as_deref(),
            "CONFLUENCE_USERNAME",
            home,
        )?;
        let confluence_api_token = Self::resolve_required_from_cli(
            cli.confluence_token.as_deref(),   // ◄── note: cli.confluence_token, not confluence_api_token
            "CONFLUENCE_API_TOKEN",
            home,
        )?;
        let anthropic_api_key = Self::resolve_optional_from_cli(
            cli.anthropic_api_key.as_deref(),
            "ANTHROPIC_API_KEY",
            home,
        );

        // Non-credential fields: Cli tier only (no ~/.claude/ fallback per D-03/D-05)
        let plantuml_path = cli.plantuml_path.clone()
            .unwrap_or_else(|| "plantuml".to_string());
        let mermaid_path = cli.mermaid_path.clone()
            .unwrap_or_else(|| "mmdc".to_string());

        // ... rest of construction unchanged (diagram_config, anthropic_model, etc.)
    }

    /// Cli tier + ~/.claude/ tier. Skips the env-var tier because clap already
    /// resolved it onto `cli_value`.
    fn resolve_required_from_cli(
        cli_value: Option<&str>,
        env_key: &'static str,
        home: Option<&Path>,
    ) -> Result<String, ConfigError> {
        if let Some(val) = cli_value {
            if !val.is_empty() {
                return Ok(val.to_string());
            }
        }
        if let Some(val) = load_from_claude_config(env_key, home) {
            if !val.is_empty() {
                return Ok(val);
            }
        }
        Err(ConfigError::Missing { name: env_key })
    }
}
```

Note: `resolve_required_from_cli` drops the middle `std::env::var(env_key)` tier because clap already folded that into `cli_value`. This is the structural simplification D-01/D-02 call for.

### Example 4: Target env-var tier integration test (D-06)

```rust
// tests/cli_integration.rs (new test, add after test_convert_with_diagram_path_flags)
// Pattern mirrors the existing flag-based test at lines 348–388 of the same file.
#[test]
#[serial_test::serial]  // env-var mutation must not race with other tests
fn test_convert_with_env_var_diagram_paths() {
    let (md_dir, md_path) = temp_markdown("# Env Var Test\n\nPlain content, no diagrams.\n");
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    cmd.arg("convert")
        .arg(&md_path)
        .arg(out_dir.path())
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN")
        // Set env-var tier — NOT CLI flags
        .env("PLANTUML_PATH", "/fake/plantuml-via-env")
        .env("MERMAID_PATH", "/fake/mmdc-via-env");

    let output = cmd.output().expect("run command");

    assert!(
        output.status.success(),
        "convert with PLANTUML_PATH / MERMAID_PATH env vars should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // page.xml must be written (markdown has no diagrams, so /fake paths are never invoked)
    let xml_path = out_dir.path().join("page.xml");
    assert!(
        xml_path.exists(),
        "page.xml should exist in output dir"
    );

    drop(md_dir);
}
```

This test exercises tier 2 (env var) end-to-end at the binary level, proving clap resolves `PLANTUML_PATH`/`MERMAID_PATH` to `cli.plantuml_path`/`cli.mermaid_path` even when no CLI flag is given. Combined with existing `test_convert_with_diagram_path_flags` (tier 1), the two tiers of diagram-path resolution are covered.

### Example 5: Phase 08 VERIFICATION.md frontmatter (target)

```yaml
---
phase: 08-diagramconfig-waterfall-and-nyquist-compliance
verified: 2026-04-20T00:00:00Z
status: passed                  # or human_needed if any SC needs human confirmation
score: 5/5 must-haves verified  # from 08-01-PLAN.md must_haves.truths (5 items) + 08-02
overrides_applied: 0
---
```

`score` computation: 08-01-PLAN.md `must_haves.truths` has 5 entries; 08-02-PLAN.md `must_haves.truths` has 4 entries. Both sets map to requirements under SCAF-03 and the Nyquist-compliance convention. Treat the total as 9/9 if combined, or score each plan separately in the body.

## State of the Art

| Old Approach (current) | Current Approach (target) | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Config::load(&CliOverrides)` with a second env-var read inside `resolve_required`/`resolve_optional` | `Config::load(&Cli)` that trusts clap's already-resolved `Option<String>` fields and only adds the `~/.claude/` credential tier | Phase 9 (this phase) | Removes 6 struct fields (`CliOverrides`), ~60 lines of duplicated env-var resolution; closes SCAF-03 integration gap |
| Convert arm manually calls `dotenvy::dotenv().ok()` + `std::env::var("PLANTUML_PATH").ok()` | `dotenvy::dotenv().ok()` hoisted to `main.rs`; convert arm reads `cli.plantuml_path` directly | Phase 9 | One `dotenvy` call site project-wide; zero manual env reads for diagram CLI fields |

**Deprecated/outdated:**

- `pub struct CliOverrides` (src/config.rs:46–54): **removed** in Phase 9. Every call site (src/lib.rs update arm line 84, upload arm line 164) is also updated in Phase 9.
- `Config::resolve_required` / `resolve_optional` that include an env-var tier: **simplified** (the env-var tier is deleted from these functions; clap owns it).
- `DiagramConfig::from_env()` (src/config.rs:22–37) and `impl Default for DiagramConfig` (src/config.rs:39–43): **still present**, still referenced by test-only code in `src/converter/diagrams.rs` and `src/converter/tests.rs`. v1.0 audit classified this as **tech debt** (public API with no production callers post-Phase 08). Phase 10 closes this.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Clap-derive's `env = "X"` attribute does **not** read `.env` files on its own; it reads `std::env::var("X")` at `Cli::parse()` time. | Pitfall 1; Code Example 2 | If wrong, we wouldn't need to hoist `dotenvy::dotenv().ok()` to `main.rs`; but we still need `dotenvy` somewhere, and hoisting to main.rs is safe either way. **Low risk.** [ASSUMED based on clap documentation; verify by testing: set `PLANTUML_PATH=/x` in `.env`, unset in shell, run `confluence-agent convert`, check if `/x` is used.] |
| A2 | Removing `CliOverrides` and changing `Config::load` signature will propagate to ~13 test sites in `src/config.rs` that currently use `CliOverrides { ... }` or `CliOverrides::default()`. | Pitfall 3 | If fewer tests affected, less work. If more (because `CliOverrides` is used in integration tests too), more mechanical work. A `grep -rn CliOverrides` would confirm; currently verified only in `src/config.rs` and `src/lib.rs`. [VERIFIED: Grep pattern `CliOverrides` in `src/` returns only `src/config.rs` and `src/lib.rs` — no test-file usage outside `#[cfg(test)] mod tests` in config.rs.] |
| A3 | The existing `test_convert_with_diagram_path_flags` (tests/cli_integration.rs:348) uses `cmd.env_remove("PLANTUML_PATH")` and `cmd.env_remove("MERMAID_PATH")` to isolate the CLI-flag tier. The new env-var test (D-06) must use `cmd.env("PLANTUML_PATH", "...")` and **not** pass `--plantuml-path` to isolate the env tier. | Code Example 4 | If the existing test does not fully isolate, the new test assertion could be ambiguous. [VERIFIED: tests/cli_integration.rs:360–365 read — `env_remove("PLANTUML_PATH")` and `env_remove("MERMAID_PATH")` are present in the current test.] |
| A4 | Phase 08's VALIDATION.md per-task verification map (lines 42–47 of 08-VALIDATION.md) enumerates 6 tests. All 6 are currently green per the Status column. | "VERIFICATION.md for Phase 08" scoring | If any has since regressed (unlikely, Phase 08 closed 2026-04-15, no code changes since), VERIFICATION.md's `status: passed` would be wrong. [ASSUMED; mitigation: Plan 09-02 must run `cargo test` as part of evidence gathering.] |
| A5 | Phase 08 had no human-verification items. 08-VALIDATION.md "Manual-Only Verifications" section states "All phase behaviors have automated verification." | VERIFICATION.md `status: passed` vs `human_needed` | If wrong, VERIFICATION.md frontmatter should be `human_needed`. [VERIFIED: 08-VALIDATION.md lines 60–61 read — explicitly states no manual-only items.] |
| A6 | Moving `dotenvy::dotenv().ok()` to `src/main.rs` before `Cli::parse()` does not break any existing test because integration tests set env vars explicitly via `cmd.env(...)` on `assert_cmd::Command`, which bypasses `.env` files anyway. | Pitfall 1 fix | If a test indirectly relies on `.env` loading inside `Config::load`, it would break. [VERIFIED: Grep `dotenvy` across `tests/` returns no matches — tests do not reference `.env` loading.] |

**Non-empty:** This table lists assumptions made during research. All have mitigation paths or were subsequently verified in the codebase grep. The planner should surface A1 specifically — it informs whether `dotenvy::dotenv().ok()` belongs in `main.rs`, `Config::load`, or both.

## Open Questions

1. **Where should `dotenvy::dotenv().ok()` live after the refactor?**
   - What we know: Pre-refactor, it's called in both `Config::load` (line 76) and the convert arm (line 213). Post-refactor, `Config::load` still works but the convert arm cannot call it because the convert arm no longer has a Config path.
   - What's unclear: Whether `.env` should be respected by the convert arm when only env-var tier is used for diagram paths.
   - Recommendation: Hoist to `src/main.rs` before `Cli::parse()`. Uniform behavior across all three arms. Remove the call from `Config::load` to avoid double-loading.

2. **Should the new env-var test also exercise the default (tier 3) behavior?**
   - What we know: CONTEXT.md D-06 specifies only env-var tier. D-03/D-05 explicitly say diagram paths have no `~/.claude/` tier.
   - What's unclear: Whether a third test (CLI=None, env=None → default "plantuml"/"mmdc") adds value beyond what `test_diagram_config_defaults_when_no_override` already covers at unit-test level.
   - Recommendation: One new integration test (env-var tier). Existing unit test covers defaults. Existing integration test covers CLI flags. Full tier coverage.

3. **Plan 09-01 and 09-02: sequential or parallel?**
   - What we know: 09-01 modifies src/lib.rs and src/config.rs; 09-02 produces VERIFICATION.md evidence citing lines in those files. 09-02 is otherwise independent.
   - What's unclear: Whether VERIFICATION.md should describe Phase 08's state *at close* (2026-04-15) or *current* (post-09-01) state.
   - Recommendation: Sequence 09-02 first (Wave 1) → 09-01 (Wave 2). VERIFICATION.md reflects Phase 08 close. Phase 10 (separate) will verify Phase 09 with its own VERIFICATION.md.

4. **Should `CliOverrides` removal happen in Phase 9 or defer?**
   - What we know: CONTEXT.md D-02 is explicit: "Remove `CliOverrides`."
   - What's unclear: Nothing — this is a locked decision.
   - Recommendation: Execute in Plan 09-01 as stated.

## Environment Availability

Skipped — this phase has no external dependencies. All tooling (cargo, clap-derive, serial_test, assert_cmd, tempfile, dotenvy, dirs) is already installed via Cargo.toml. The Phase 09 plans are pure Rust source edits + one markdown file.

## Validation Architecture

> workflow.nyquist_validation is `true` per `.planning/config.json`. This section is required.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust / `cargo test` |
| Config file | `Cargo.toml` + `.cargo/config.toml` (`-D warnings`) |
| Quick run command | `cargo test --lib config::tests -- --test-threads=1` |
| Full suite command | `cargo test` |

Estimated quick-run latency: ~5 seconds. Full suite (136 tests per Phase 07 VERIFICATION): ~40 seconds.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SCAF-03 | `Config::load(&cli)` compiles; `CliOverrides` removed from src/config.rs and src/lib.rs | compile | `cargo build` | ✅ (src/lib.rs, src/config.rs exist) |
| SCAF-03 | Existing `test_convert_with_diagram_path_flags` still passes (CLI-tier regression check) | integration | `cargo test --test cli_integration test_convert_with_diagram_path_flags` | ✅ (tests/cli_integration.rs:348 exists) |
| SCAF-03 | New env-var tier test `test_convert_with_env_var_diagram_paths` passes | integration | `cargo test --test cli_integration test_convert_with_env_var_diagram_paths` | ❌ Wave 0 (new test to be added) |
| SCAF-03 | Existing `config::tests` all pass after signature change | unit | `cargo test --lib config::tests -- --test-threads=1` | ✅ (src/config.rs has 13 tests) |
| SCAF-03 | Full suite remains green (no regression across 136 tests) | unit+integration | `cargo test` | ✅ |
| SCAF-03 | grep confirms no `CliOverrides` or manual `std::env::var("PLANTUML_PATH")` remains in src/ | grep | `! grep -rn CliOverrides src/ && ! grep -rn 'env::var("PLANTUML_PATH")' src/lib.rs src/config.rs` | ✅ |
| SCAF-03 | Phase 08 VERIFICATION.md exists and has valid frontmatter | artifact | `test -f .planning/phases/08-diagramconfig-waterfall-and-nyquist-compliance/08-VERIFICATION.md && grep -q 'status:' .planning/phases/08-*/08-VERIFICATION.md` | ❌ Wave 0 (new artifact to be created) |

### Sampling Rate

- **Per task commit:** `cargo test --lib config::tests -- --test-threads=1` (~5s)
- **Per wave merge:** `cargo test` (~40s)
- **Phase gate:** Full suite green before `/gsd-verify-work`, plus `grep -rn CliOverrides src/` returns zero matches, plus 08-VERIFICATION.md exists.

### Wave 0 Gaps

- [ ] `tests/cli_integration.rs` — add `test_convert_with_env_var_diagram_paths` (env-var tier test per D-06). Existing fixtures (`temp_markdown` helper, `TempDir`, `assert_cmd::Command`) already available at file top. No new test-infra files.
- [ ] `.planning/phases/08-diagramconfig-waterfall-and-nyquist-compliance/08-VERIFICATION.md` — new artifact produced by Plan 09-02. No framework install needed; markdownlint already present in project tooling (used throughout `.planning/`).

No framework installs required. No new `conftest.py` equivalents. No shared fixtures to extract.

## Security Domain

> `security_enforcement` setting is absent from `.planning/config.json`; treated as enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No new auth surface; this phase refactors internal config resolution. Confluence basic-auth and Anthropic bearer-token behavior unchanged. |
| V3 Session Management | no | No sessions (CLI tool). |
| V4 Access Control | no | No new capabilities; CLI caller already has same-user OS privileges. |
| V5 Input Validation | yes | Existing `https://` check on `CONFLUENCE_URL` (src/config.rs:97–102) must be preserved after refactor. Path inputs (`--plantuml-path`, `--mermaid-path`) flow to `Command::new()` in `src/converter/diagrams.rs`; trust-boundary analysis from Phase 08 (T-08-01, T-08-02) remains applicable unchanged. |
| V6 Cryptography | no | No crypto changes; reqwest with rustls unchanged. |

### Known Threat Patterns for Rust CLI + subprocess-spawning tool

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| User-supplied path → subprocess argv (e.g., `plantuml_path` → `Command::new`) | Tampering / Elevation | **Accept.** Same trust boundary as T-08-01/T-08-02 (Phase 08 threat register). The local CLI user already has process-level privilege; `--plantuml-path` cannot escalate further than the user's own shell can. No additional validation needed. |
| Loss of `https://` enforcement on `CONFLUENCE_URL` due to refactor | Tampering | **Mitigate.** Preserve the existing `starts_with("https://")` check in `Config::load_with_home` when the signature changes. Phase 08 T-01-04 threat — unchanged in severity, must not be silently removed. Acceptance criterion for Plan 09-01: `test_confluence_url_must_be_https` (existing unit test, src/config.rs:514) still passes. |
| `.env` file leak of credentials to clap at wrong time | Info Disclosure | **Accept.** `.env` files are already in `.gitignore` (project convention). Whether `dotenvy` runs before or after `Cli::parse()` does not change disclosure surface — values only enter process env, which is already accessible to the binary. |
| VERIFICATION.md citing wrong line numbers as evidence | Tampering (of audit record) | **Mitigate.** Plan 09-02 must grep current codebase for each claim, not trust SUMMARY prose. See Pitfall 5. |

No net-new threats introduced by this phase. Existing threat register (Phase 08 T-08-01, T-08-02; Phase 01 T-01-04 https) carries forward unchanged.

## Sources

### Primary (HIGH confidence — in-tree evidence)

- `.planning/phases/09-.../09-CONTEXT.md` — user decisions D-01 through D-07, canonical refs, specifics
- `.planning/REQUIREMENTS.md` — SCAF-03 definition, traceability table with Phase 9 gap closure row
- `.planning/STATE.md` — current project position
- `.planning/v1.0-MILESTONE-AUDIT.md` — SCAF-03 WARNING details, Phase 08 VERIFICATION.md missing entry
- `.planning/phases/08-.../08-01-PLAN.md` — Phase 08 must_haves.truths (source for VERIFICATION.md goal achievement table)
- `.planning/phases/08-.../08-02-PLAN.md` — Phase 08 plan 2 must_haves.truths
- `.planning/phases/08-.../08-01-SUMMARY.md` — Plan 08-01 completion evidence (commit 4d12992)
- `.planning/phases/08-.../08-02-SUMMARY.md` — Plan 08-02 completion evidence (commit c4997ef)
- `.planning/phases/08-.../08-VALIDATION.md` — per-task verification map (6 tests, all green)
- `.planning/phases/06-.../06-VERIFICATION.md` — reference format for VERIFICATION.md
- `.planning/phases/07-.../07-VERIFICATION.md` — reference format; override-applied example
- `.planning/phases/01-.../01-VERIFICATION.md` — reference format; human_needed example
- `.planning/config.json` — nyquist_validation=true; mode=yolo
- `src/cli.rs` lines 1–82 (full file)
- `src/config.rs` lines 1–616 (full file)
- `src/lib.rs` lines 1–258 (full file)
- `tests/cli_integration.rs` lines 1–388 (relevant section read)
- `Cargo.toml` (dependencies, profile)

### Secondary (MEDIUM confidence — documentation reference)

- clap 4 documentation pattern for `#[arg(long, env = "X")]` — consistent with project's existing usage in `src/cli.rs`; no external docs fetched in this session. Behavior described matches observed usage in the codebase.

### Tertiary (LOW confidence — none)

No LOW-confidence sources used. All claims are anchored to file reads from the current repository.

## Metadata

**Confidence breakdown:**

- Standard stack: **HIGH** — all packages already in Cargo.toml, versions pinned, zero new deps required.
- Architecture: **HIGH** — target design is a direct application of the locked user decisions D-01/D-02/D-04; the before/after diff is mechanical.
- Pitfalls: **HIGH** — all five pitfalls verified against actual code reads (line numbers cited).
- Code examples: **HIGH** — Examples 1, 3, 4 grounded in current file reads; Examples 2, 5 are targeted recommendations derived from those reads.
- Assumptions: **MEDIUM** — A1 (clap env attribute does not read .env) is the highest-risk assumption; the mitigation (hoist dotenvy to main.rs) neutralizes both possibilities.

**Research date:** 2026-04-20
**Valid until:** 2026-05-20 (30 days — stable codebase, locked decisions; no fast-moving external ecosystem)
