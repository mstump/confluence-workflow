---
phase: 8
slug: diagramconfig-waterfall-and-nyquist-compliance
status: researched
researched: 2026-04-14
---

# Phase 8: DiagramConfig Waterfall and Nyquist Compliance — Research

**Researched:** 2026-04-14
**Domain:** Rust CLI configuration waterfall (clap 4), VALIDATION.md frontmatter compliance
**Confidence:** HIGH

## Summary

Phase 8 is a gap-closure phase with two distinct work streams. The first (Plan 08-01) extends the established CLI > env > config waterfall pattern from Phase 6 to cover `DiagramConfig`. Currently `DiagramConfig` is constructed independently via `DiagramConfig::from_env()` at the point of use (`MarkdownConverter::default()`), bypassing `Config::load()` entirely. The fix adds `--plantuml-path` and `--mermaid-path` flags to `cli.rs`, extends `CliOverrides` with those fields, adds diagram path resolution to `Config::load()`, embeds `DiagramConfig` directly in `Config`, and passes it explicitly to `MarkdownConverter::new()` in `lib.rs`.

The second work stream (Plan 08-02) achieves Nyquist compliance for Phases 01, 02, and 03 by updating their VALIDATION.md files with `nyquist_compliant: true` and `wave_0_complete: true` frontmatter. Phase 01's VALIDATION.md has no frontmatter at all; Phases 02 and 03 have the frontmatter keys but both are set to `false`. The Phase 6 VALIDATION.md is the reference implementation of a compliant file.

The overall scope is small and mechanically clear: two Rust files modified for the waterfall fix, three VALIDATION.md files updated for compliance. No new dependencies are required.

**Primary recommendation:** Follow the exact pattern established in Phase 6 (`--anthropic-api-key` → `CliOverrides.anthropic_api_key` → `Config::resolve_optional`) applied twice for `plantuml_path` and `mermaid_path`. The Nyquist compliance work is VALIDATION.md frontmatter edits only — no code changes.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CLI flag parsing | CLI (clap / cli.rs) | — | clap owns all user-facing argument declarations |
| Config resolution (waterfall) | Config tier (config.rs) | — | Config::load() is the single authority for resolved settings |
| DiagramConfig delivery to converter | API / lib.rs dispatch | — | lib.rs constructs MarkdownConverter after Config is resolved |
| VALIDATION.md frontmatter | Planning artifacts | — | YAML frontmatter in .planning/phases/*/VALIDATION.md |

## Project Constraints (from CLAUDE.md)

- Always pin dependency versions in pyproject.toml. This is a Python convention; the Rust equivalent is explicit version pinning in Cargo.toml. [VERIFIED: CLAUDE.md]
- Run `cargo build`, `uv run mypy .`, `uv run pytest` after Python changes. For this phase (Rust-only), the equivalent is: `cargo build` + `cargo test` after any Rust change. [VERIFIED: CLAUDE.md]
- Run `markdownlint --fix .` after markdown changes — applies to VALIDATION.md edits. [VERIFIED: CLAUDE.md]
- Re-install CLI via `uv pip uninstall confluence-agent && uv pip install -e '.[dev]'` only relevant to Python CLI. The Rust CLI is invoked via `cargo run`. [VERIFIED: CLAUDE.md]

## Standard Stack

### Core (already in project — no new deps needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | (pinned in Cargo.toml) | CLI argument parsing + env var reading | Already used for all other CLI flags; `#[arg(long, env = "...")]` pattern established |
| serde_json | (pinned) | JSON config fallback via `load_from_claude_config` | Already used in config.rs for ~/.claude/settings.json reads |
| serial_test | (pinned) | Sequential test isolation for env var mutation | Already added in Phase 7 for config test race condition |

[VERIFIED: grep of src/cli.rs, src/config.rs, Cargo.toml contents observed in research]

No new dependencies are required for Plan 08-01 or 08-02.

**Version verification:** No new packages to add. All dependencies already present.

## Architecture Patterns

### System Architecture Diagram

```
CLI invocation: confluence-agent --plantuml-path /usr/bin/plantuml convert doc.md ./out
                        |
                        v
                    src/cli.rs (clap)
                    Cli { plantuml_path: Some("/usr/bin/plantuml"), ... }
                        |
                        v
                    src/lib.rs run()
                    CliOverrides { plantuml_path: Some("/usr/bin/plantuml"), ... }
                        |
                        v
                    src/config.rs Config::load(&overrides)
                    resolve_optional("plantuml_path", "PLANTUML_PATH", home)
                    -> Config { diagram_config: DiagramConfig { plantuml_path: "/usr/bin/plantuml", ... } }
                        |
                        v
                    MarkdownConverter::new(config.diagram_config)
                    [previously: MarkdownConverter::default() -> DiagramConfig::from_env()]
                        |
                        v
                    src/converter/diagrams.rs
                    render_plantuml(..., &self.diagram_config)
```

### Recommended Project Structure

No new files. Modifications only:

```
src/
├── cli.rs           # Add plantuml_path, mermaid_path Option<String> fields to Cli
├── config.rs        # Extend CliOverrides + Config; add resolve_optional calls in load_with_home
└── lib.rs           # Pass config.diagram_config to MarkdownConverter::new() in all 3 arms

.planning/phases/
├── 01-.../01-VALIDATION.md   # Add YAML frontmatter block
├── 02-.../02-VALIDATION.md   # Change nyquist_compliant/wave_0_complete to true
└── 03-.../03-VALIDATION.md   # Change nyquist_compliant/wave_0_complete to true
```

### Pattern 1: Clap Flag + Env Fallback (established in Phase 6)

**What:** Declare an `Option<String>` field on `Cli` with `#[arg(long, env = "ENV_VAR")]`. Clap reads the flag first, falls through to the env var automatically.

**When to use:** All user-configurable paths or credentials that can come from CLI or environment.

**Example (Phase 6 reference):**
```rust
// Source: src/cli.rs (verified in codebase)
/// Anthropic API key (for update command's LLM merge)
#[arg(long, env = "ANTHROPIC_API_KEY")]
pub anthropic_api_key: Option<String>,
```

Apply identically for diagram paths:
```rust
/// Path to PlantUML executable or JAR
#[arg(long, env = "PLANTUML_PATH")]
pub plantuml_path: Option<String>,

/// Path to mermaid-cli executable (mmdc)
#[arg(long, env = "MERMAID_PATH")]
pub mermaid_path: Option<String>,
```

### Pattern 2: CliOverrides Extension (established in Phase 6)

**What:** `CliOverrides` is the typed bridge between `Cli` (clap) and `Config::load()`. Add a field for each new CLI flag.

**Example:**
```rust
// Source: src/config.rs (verified in codebase)
pub struct CliOverrides {
    pub confluence_url: Option<String>,
    pub confluence_username: Option<String>,
    pub confluence_api_token: Option<String>,
    pub anthropic_api_key: Option<String>,
    // Phase 8 additions:
    pub plantuml_path: Option<String>,
    pub mermaid_path: Option<String>,
}
```

### Pattern 3: Config::resolve_optional for Optional Paths

**What:** `DiagramConfig.plantuml_path` and `mermaid_path` are optional strings with sensible defaults ("plantuml", "mmdc"). Use `resolve_optional` and fall back to the defaults if no source provides a value.

**Example:**
```rust
// Source: src/config.rs Config::load_with_home (verified in codebase)
// Pattern for optional fields with defaults:
let plantuml_path = Self::resolve_optional(
    overrides.plantuml_path.as_deref(),
    "PLANTUML_PATH",
    home,
).unwrap_or_else(|| "plantuml".to_string());

let mermaid_path = Self::resolve_optional(
    overrides.mermaid_path.as_deref(),
    "MERMAID_PATH",
    home,
).unwrap_or_else(|| "mmdc".to_string());
```

Note: `mermaid_puppeteer_config` and `timeout_secs` remain env-only (no CLI flags needed per phase scope). They can be folded into `DiagramConfig` construction inside `Config::load_with_home()` using the existing `std::env::var` pattern.

### Pattern 4: DiagramConfig Embedded in Config

**What:** Move `DiagramConfig` from a standalone `from_env()` construction at call sites into `Config` as a field.

**Example:**
```rust
pub struct Config {
    pub confluence_url: String,
    pub confluence_username: String,
    pub confluence_api_token: String,
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: String,
    pub anthropic_concurrency: usize,
    pub diagram_config: DiagramConfig,   // Phase 8 addition
}
```

### Pattern 5: MarkdownConverter construction change in lib.rs

**What:** Replace `MarkdownConverter::default()` (which calls `DiagramConfig::from_env()` implicitly) with `MarkdownConverter::new(config.diagram_config)` in all three command arms where conversion occurs. The `convert` arm currently uses `MarkdownConverter::default()` without calling `Config::load()` at all — this needs special handling.

**The convert arm challenge:** `convert` intentionally does not require Confluence credentials, so `Config::load()` is not called. Two options:

1. **Partial config load:** Extract only `DiagramConfig` from env/CLI without requiring Confluence fields. This would require a separate `DiagramConfig::load(overrides)` function — effectively recreating the waterfall for diagram paths only.
2. **Pass diagram fields separately to convert arm:** The `convert` arm receives the `Cli` struct; read `cli.plantuml_path` / `cli.mermaid_path` directly and construct `DiagramConfig` from those plus env fallbacks.

Option 2 is simpler and consistent with the existing pattern where `convert` avoids `Config::load()`. The planner should choose: either a standalone `DiagramConfig::load()` helper (cleanest) or inline construction in the convert arm.

[ASSUMED] The preferred approach is a standalone `DiagramConfig::load(overrides)` that mirrors the waterfall pattern, keeping `Config::load()` for credential-bearing commands and `DiagramConfig::load()` for credential-free commands. This assumption should be confirmed by the planner.

### Anti-Patterns to Avoid

- **`DiagramConfig::from_env()` called at point of use:** This is the existing anti-pattern being fixed. It bypasses the CLI tier entirely.
- **Constructing `MarkdownConverter::default()` in any command arm:** After this phase, `default()` becomes dead code for production paths. Consider whether to keep it for test convenience or remove it.
- **Setting env vars in new parallel tests without `#[serial]`:** The existing `test_diagram_config_from_env` test in `converter/diagrams.rs` already uses save/restore but not `#[serial]`. New config tests that mutate env vars must use `#[serial]` (added in Phase 7 for config tests).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI flag + env fallback | Custom argument parsing | clap `#[arg(long, env = "...")]` | Already in use; handles priority automatically |
| Config waterfall | Bespoke resolution chain | `Config::resolve_optional` | Already implemented and tested in config.rs |
| YAML frontmatter | Custom parser | Standard YAML between `---` delimiters | Markdown frontmatter is a widely understood convention; gsd tooling reads it directly |

**Key insight:** Both work streams in this phase are about applying existing patterns to new fields, not building new infrastructure. The resolution logic, test patterns, and VALIDATION.md format are all established.

## Common Pitfalls

### Pitfall 1: Forgetting the `convert` arm has no Config::load()

**What goes wrong:** `update` and `upload` arms call `Config::load(&overrides)` and get `config.diagram_config`. The `convert` arm does not call `Config::load()` — adding `config.diagram_config` there without a separate mechanism leaves diagram CLI flags silently ignored for the convert command.

**Why it happens:** The convert arm was intentionally designed to not require credentials. Fixing the waterfall for update/upload is not enough.

**How to avoid:** Explicitly handle diagram config construction in the convert arm. Either call a `DiagramConfig::load(overrides)` helper, or construct `DiagramConfig` directly from CLI fields in that arm.

**Warning signs:** Test `confluence-agent --plantuml-path /custom/path convert doc.md ./out` and verify the custom path is used. If `from_env()` is still called, the test will pass only if `PLANTUML_PATH` env var is also set.

### Pitfall 2: `test_diagram_config_from_env` becomes incorrect after waterfall change

**What goes wrong:** The existing test in `converter/diagrams.rs::tests::test_diagram_config_from_env` calls `DiagramConfig::from_env()` directly. After Phase 8, `from_env()` may be removed or deprecated in favor of waterfall resolution. The test would still compile but would test a code path no longer used in production.

**Why it happens:** Tests written for the old construction path are not automatically updated.

**How to avoid:** If `DiagramConfig::from_env()` is kept as a helper (used internally by the waterfall), the test remains valid. If it is removed, add equivalent tests to `config::tests` that exercise the full waterfall for diagram paths.

### Pitfall 3: Phase 01 VALIDATION.md has no frontmatter block at all

**What goes wrong:** Phase 01's VALIDATION.md opens with a bare markdown heading — no `---` delimiters, no YAML. Simply editing `nyquist_compliant: false` to `true` is not possible because the key does not exist.

**Why it happens:** Phase 01 was created before the Nyquist frontmatter convention was established.

**How to avoid:** Add a complete frontmatter block at the top of the file. Reference the Phase 6 VALIDATION.md frontmatter as the canonical model:

```yaml
---
phase: 1
slug: project-scaffolding-and-confluence-api-client
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-10
audited: 2026-04-14
---
```

### Pitfall 4: CliOverrides struct initialization becomes non-exhaustive

**What goes wrong:** Adding fields to `CliOverrides` causes compile errors at every construction site that uses struct literal syntax (`CliOverrides { confluence_url: ..., ... }`). There are at least 3 construction sites in `lib.rs` (one per command arm).

**Why it happens:** Rust struct literals require all fields to be specified unless `..Default::default()` is used.

**How to avoid:** After adding `plantuml_path` and `mermaid_path` to `CliOverrides`, update all 3 construction sites in `lib.rs`. The `Convert` arm must provide the CLI values; `Update` and `Upload` arms must also forward them. Run `cargo build` immediately to surface any missed sites.

### Pitfall 5: Marking wave_0_complete: true when Wave 0 gaps still exist

**What goes wrong:** Phases 02 and 03 VALIDATION.md files list multiple Wave 0 items as unchecked (`- [ ]`). Setting `wave_0_complete: true` would be inaccurate if those items were never completed.

**Why it happens:** The Nyquist compliance task says "set the frontmatter" without auditing whether the underlying Wave 0 work was actually done.

**How to avoid:** Before setting `wave_0_complete: true`, verify the actual state of the Wave 0 items:
- Phase 02 Wave 0: `src/converter/tests.rs` — does it exist and have the required stubs?
- Phase 03 Wave 0: `src/llm/mod.rs`, `src/merge/mod.rs`, etc. — do they exist with tests?

If Wave 0 items were completed during phase execution but the frontmatter was never updated, setting `true` is correct. If items remain genuinely incomplete, the planner should either complete them or document the known delta.

[VERIFIED: src/converter/tests.rs exists; src/llm/mod.rs, src/merge/extractor.rs, src/merge/matcher.rs, src/merge/injector.rs all exist in codebase. Actual test content should be confirmed by the executor.]

## Code Examples

### Adding flags to Cli struct (src/cli.rs)

```rust
// Source: src/cli.rs (verified — anthropic_api_key field added in Phase 6 as template)
/// Path to PlantUML executable (or JAR path)
#[arg(long, env = "PLANTUML_PATH")]
pub plantuml_path: Option<String>,

/// Path to mermaid-cli executable (mmdc)
#[arg(long, env = "MERMAID_PATH")]
pub mermaid_path: Option<String>,
```

Place these after `anthropic_api_key` and before `verbose`.

### Extending CliOverrides (src/config.rs)

```rust
// Source: src/config.rs (verified — CliOverrides struct, lines 47-52)
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub confluence_url: Option<String>,
    pub confluence_username: Option<String>,
    pub confluence_api_token: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub plantuml_path: Option<String>,   // Phase 8
    pub mermaid_path: Option<String>,    // Phase 8
}
```

### Waterfall resolution inside Config::load_with_home

```rust
// Diagram paths — optional with defaults
let plantuml_path = Self::resolve_optional(
    overrides.plantuml_path.as_deref(),
    "PLANTUML_PATH",
    home,
).unwrap_or_else(|| "plantuml".to_string());

let mermaid_path = Self::resolve_optional(
    overrides.mermaid_path.as_deref(),
    "MERMAID_PATH",
    home,
).unwrap_or_else(|| "mmdc".to_string());

// Remaining DiagramConfig fields remain env-only (no CLI flag in scope)
let mermaid_puppeteer_config = std::env::var("MERMAID_PUPPETEER_CONFIG").ok();
let timeout_secs = std::env::var("DIAGRAM_TIMEOUT")
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(30);

let diagram_config = DiagramConfig {
    plantuml_path,
    mermaid_path,
    mermaid_puppeteer_config,
    timeout_secs,
};
```

### Updated Config struct

```rust
pub struct Config {
    pub confluence_url: String,
    pub confluence_username: String,
    pub confluence_api_token: String,
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: String,
    pub anthropic_concurrency: usize,
    pub diagram_config: DiagramConfig,  // Phase 8 addition
}
```

### Updated MarkdownConverter construction in lib.rs Update arm

```rust
// Replace: let converter = MarkdownConverter::default();
// With:
let converter = MarkdownConverter::new(config.diagram_config.clone());
```

(Clone is needed because `config` is consumed by later fields. Alternatively, extract `diagram_config` before consuming `config`.)

### VALIDATION.md frontmatter — Phase 01 (must be added from scratch)

```yaml
---
phase: 1
slug: project-scaffolding-and-confluence-api-client
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-10
audited: 2026-04-14
---
```

### VALIDATION.md frontmatter — Phases 02 and 03 (keys exist, values must change)

For Phase 02, change:
```yaml
nyquist_compliant: false  ->  nyquist_compliant: true
wave_0_complete: false    ->  wave_0_complete: true
```

Add `audited: 2026-04-14` field to record when compliance was achieved.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `DiagramConfig::from_env()` at call sites | `DiagramConfig` embedded in `Config::load()` waterfall | Phase 8 | CLI flags `--plantuml-path`/`--mermaid-path` become effective |
| `MarkdownConverter::default()` | `MarkdownConverter::new(config.diagram_config)` | Phase 8 | Converter respects CLI-supplied paths |
| Phase 01 VALIDATION.md: no frontmatter | YAML frontmatter with `nyquist_compliant: true` | Phase 8 | gsd tooling can read compliance status |
| Phases 02/03 VALIDATION.md: `nyquist_compliant: false` | `nyquist_compliant: true` | Phase 8 | Nyquist audit passes for these phases |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The `convert` arm should use a standalone `DiagramConfig::load(overrides)` helper rather than inline construction | Architecture Patterns (Pattern 5) | Planner may choose inline construction instead — both are valid; choose one and be consistent |
| A2 | Wave 0 items for Phases 02 and 03 were actually completed during execution (src/converter/tests.rs, src/llm/mod.rs etc. exist with content) | Common Pitfalls (Pitfall 5) | If Wave 0 items are stubs with no real tests, setting `wave_0_complete: true` is inaccurate — executor must verify |
| A3 | `DiagramConfig::from_env()` should be kept as a private helper used inside the waterfall (not removed) so existing tests remain valid | Architecture Patterns (Pattern 3) | If removed, test_diagram_config_from_env breaks — safe to either keep or migrate the test |

## Open Questions

1. **Does `convert` arm need `Config::load()` or a lighter DiagramConfig resolver?**
   - What we know: `convert` currently skips `Config::load()` to avoid requiring Confluence credentials
   - What's unclear: Whether to add a separate `DiagramConfig::load(overrides)` function or inline the waterfall resolution
   - Recommendation: Add `DiagramConfig::load(overrides)` — mirrors established Config pattern, avoids code duplication across arms

2. **Should `DiagramConfig::from_env()` be removed or kept?**
   - What we know: After Phase 8, no production code path should call `from_env()` directly
   - What's unclear: Whether to delete it (cleaner) or keep it (backward compatible for tests)
   - Recommendation: Keep it but mark it `#[doc(hidden)]` or add a comment noting it is only used in tests; alternatively migrate `test_diagram_config_from_env` to test the waterfall directly

3. **Are the Phase 02 and Phase 03 Wave 0 items actually complete?**
   - What we know: The files exist in the codebase (`src/converter/tests.rs`, `src/llm/mod.rs`, etc.)
   - What's unclear: Whether the test functions specified in the VALIDATION.md tables were implemented or left as stubs
   - Recommendation: The executor of Plan 08-02 must run the specific test commands listed in each VALIDATION.md and verify they pass before setting `wave_0_complete: true`

## Environment Availability

Step 2.6: SKIPPED — Phase 8 is code and config-only changes with no new external dependencies. All diagram rendering binaries (`plantuml`, `mmdc`) were already present when Phase 2 ran.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | Cargo.toml |
| Quick run command | `cargo test --lib config::tests` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SCAF-03 | `--plantuml-path` CLI flag overrides `PLANTUML_PATH` env var | unit | `cargo test --lib config::tests` | ✅ (new test needed in config::tests) |
| SCAF-03 | `--mermaid-path` CLI flag overrides `MERMAID_PATH` env var | unit | `cargo test --lib config::tests` | ✅ (new test needed in config::tests) |
| SCAF-03 | DiagramConfig embedded in Config, not loaded via from_env() at call site | unit | `cargo test --lib config::tests` | ✅ (new test needed in config::tests) |
| SCAF-03 | Env var fallback when no CLI flag provided | unit | `cargo test --lib config::tests` | ✅ (existing test_diagram_config_from_env covers env path) |

### Sampling Rate

- **Per task commit:** `cargo test --lib config::tests`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

None — existing test infrastructure covers all phase requirements. New tests for diagram waterfall will be added to the existing `src/config.rs` `#[cfg(test)]` block.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes (path values) | Paths are passed to subprocess `Command::new()`; validate that path strings are not empty before use |
| V6 Cryptography | no | — |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Arbitrary binary execution via `--plantuml-path` | Tampering / Elevation | Path is supplied by the authenticated local user (same trust level as the process); no mitigation required beyond existing behavior. The `convert` command already spawns `plantuml` and `mmdc` with user-supplied config. |
| Path traversal via `--plantuml-path` value | Tampering | Not applicable — the path is passed to `Command::new()` directly, not concatenated into a file path. The subprocess resolves the path via the OS. |

## Sources

### Primary (HIGH confidence)

- Codebase: `src/config.rs` — verified CliOverrides, Config, resolve_optional, load_with_home structure
- Codebase: `src/cli.rs` — verified Cli struct, existing flag declaration pattern
- Codebase: `src/lib.rs` — verified 3 command arms and MarkdownConverter construction sites
- Codebase: `src/converter/mod.rs` — verified MarkdownConverter::default() and ::new()
- Codebase: `src/converter/diagrams.rs` — verified DiagramConfig usage and existing tests
- Codebase: `.planning/v1.0-MILESTONE-AUDIT.md` — authoritative source for gap descriptions
- Codebase: `.planning/phases/06-*/06-01-PLAN.md` — Phase 6 waterfall pattern (template)
- Codebase: `.planning/phases/06-*/06-VALIDATION.md` — canonical compliant VALIDATION.md

### Secondary (MEDIUM confidence)

- Codebase: `.planning/phases/01-*/01-VALIDATION.md` — confirmed no frontmatter present
- Codebase: `.planning/phases/02-*/02-VALIDATION.md` — confirmed `nyquist_compliant: false`
- Codebase: `.planning/phases/03-*/03-VALIDATION.md` — confirmed `nyquist_compliant: false`

### Tertiary (LOW confidence)

None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all libraries already in use
- Architecture: HIGH — waterfall pattern is identical to Phase 6; only new field names differ
- Pitfalls: HIGH — gaps identified directly from audit report and codebase inspection

**Research date:** 2026-04-14
**Valid until:** 2026-05-14 (stable codebase, no external API changes)

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCAF-03 | Credentials loaded via waterfall: CLI flag → env var → ~/.claude/ config file; CLI flag functional for all credentials including Anthropic API key | Pattern established in Phase 6 applies directly: add `plantuml_path`/`mermaid_path` to `CliOverrides` + `Config`, resolve via `resolve_optional`. The `convert` arm requires separate handling since it skips `Config::load()`. |
</phase_requirements>
