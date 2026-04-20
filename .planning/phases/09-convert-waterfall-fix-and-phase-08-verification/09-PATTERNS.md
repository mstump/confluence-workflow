# Phase 9: Convert Waterfall Fix and Phase 08 Verification - Pattern Map

**Mapped:** 2026-04-20
**Files analyzed:** 6 (4 modified Rust sources + 1 new integration test + 1 new VERIFICATION.md)
**Analogs found:** 6 / 6

## File Classification

| Modified/New File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/cli.rs` | config / CLI-parse | transform (env → Option<String>) | `src/cli.rs` (self, unchanged — the pattern is to preserve the existing clap-derive pattern) | exact (already-idiomatic) |
| `src/config.rs` | config / waterfall loader | transform (Cli + file → Config) | `src/config.rs` (self, current `load_with_home` body) | exact (refactor in-place) |
| `src/lib.rs` | controller / dispatch | request-response (CLI match → side effects) | `src/lib.rs` (self, existing update/upload arms as template for convert arm) | exact (refactor in-place) |
| `src/main.rs` | entrypoint | batch (parse once, run once) | `src/main.rs` (self, currently calls `Cli::parse()` without `dotenvy`) | exact (small addition) |
| `tests/cli_integration.rs` (NEW test `test_convert_with_env_var_diagram_paths`) | test / integration | request-response (spawn binary + assert) | `tests/cli_integration.rs` lines 347-388 `test_convert_with_diagram_path_flags` | exact (same role, same data flow) |
| `.planning/phases/08-.../08-VERIFICATION.md` (NEW) | planning artifact | documentation | `.planning/phases/07-test-scaffold-completion/07-VERIFICATION.md` | exact (same phase type: post-execution verification with overrides support) |

## Pattern Assignments

### `src/cli.rs` (config / CLI-parse, transform)

**Analog:** `src/cli.rs` (self — field already correct, no change)

**Field pattern** (lines 33-39) — **preserve unchanged**:

```rust
/// Path to PlantUML executable or JAR
#[arg(long, env = "PLANTUML_PATH")]
pub plantuml_path: Option<String>,

/// Path to mermaid-cli executable (mmdc)
#[arg(long, env = "MERMAID_PATH")]
pub mermaid_path: Option<String>,
```

Clap has already resolved CLI flag → env var when `Cli::parse()` returns. **No code change required in this file for Phase 9.**

---

### `src/config.rs` (config / waterfall loader, transform)

**Analog:** `src/config.rs` current `load_with_home` body and the credential waterfall (lines 82-171). Follow the exact same control-flow shape, just drop the env-var tier for fields clap already resolved.

**Current `resolve_required` three-tier pattern** (lines 175-202) — keep the **CLI tier** and **`~/.claude/` tier**; drop the middle env-var tier (clap owns it):

```rust
// CURRENT — to be simplified
fn resolve_required(
    cli_override: Option<&str>,
    env_key: &'static str,
    home: Option<&Path>,
) -> Result<String, ConfigError> {
    // 1. CLI override
    if let Some(val) = cli_override {
        if !val.is_empty() {
            return Ok(val.to_string());
        }
    }
    // 2. Environment variable (already includes .env via dotenvy)
    if let Ok(val) = std::env::var(env_key) {
        if !val.is_empty() {
            return Ok(val);
        }
    }
    // 3. ~/.claude/ fallback (best-effort stub)
    if let Some(val) = load_from_claude_config(env_key, home) {
        if !val.is_empty() {
            return Ok(val);
        }
    }
    Err(ConfigError::Missing { name: env_key })
}
```

**Target pattern** — delete step 2 (clap already resolved env vars onto `cli_override`):

```rust
fn resolve_required(
    cli_value: Option<&str>,         // clap-resolved (CLI flag OR env var)
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
```

**Signature change pattern** — `Config::load` currently takes `&CliOverrides` (lines 74, 82); change to take `&Cli`:

```rust
// CURRENT (src/config.rs:74-85)
pub fn load(overrides: &CliOverrides) -> Result<Self, ConfigError> {
    dotenvy::dotenv().ok();
    Self::load_with_home(overrides, dirs::home_dir().as_deref())
}

pub(crate) fn load_with_home(
    overrides: &CliOverrides,
    home: Option<&Path>,
) -> Result<Self, ConfigError> {
    let confluence_url = Self::resolve_required(
        overrides.confluence_url.as_deref(),
        "CONFLUENCE_URL",
        home,
    )?;
    // ...
```

```rust
// TARGET
pub fn load(cli: &Cli) -> Result<Self, ConfigError> {
    Self::load_with_home(cli, dirs::home_dir().as_deref())
}

pub(crate) fn load_with_home(
    cli: &Cli,
    home: Option<&Path>,
) -> Result<Self, ConfigError> {
    let confluence_url = Self::resolve_required(
        cli.confluence_url.as_deref(),
        "CONFLUENCE_URL",
        home,
    )?;
    // ...
```

**Field-rename pattern** (CAUTION — `cli.confluence_token`, not `confluence_api_token`) — preserve the existing rename that currently happens in `src/lib.rs:87`:

```rust
// In load_with_home (new) — note cli.confluence_token reads from Cli field,
// but env_key stays "CONFLUENCE_API_TOKEN" for ~/.claude/ lookup
let confluence_api_token = Self::resolve_required(
    cli.confluence_token.as_deref(),
    "CONFLUENCE_API_TOKEN",
    home,
)?;
```

**URL normalization and HTTPS check pattern** (lines 93-102) — **preserve unchanged**:

```rust
let confluence_url = confluence_url.trim_end_matches('/').trim().to_string();
if !confluence_url.to_ascii_lowercase().starts_with("https://") {
    return Err(ConfigError::Invalid {
        name: "CONFLUENCE_URL",
        reason: "must start with https://",
    });
}
```

This satisfies the T-01-04 threat constraint (verified in existing `test_confluence_url_must_be_https`, lines 513-534).

**DiagramConfig construction pattern** (lines 137-160) — simplify: the `Self::resolve_optional(..., "PLANTUML_PATH", home)` call becomes a direct `cli.plantuml_path.clone().unwrap_or_else(...)` because clap owns that resolution and D-03 says no `~/.claude/` tier for diagram paths:

```rust
// CURRENT (lines 137-147) — clap-resolved value goes through resolve_optional again
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

```rust
// TARGET — diagram paths bypass ~/.claude/ tier (D-03/D-05)
let plantuml_path = cli.plantuml_path.clone()
    .unwrap_or_else(|| "plantuml".to_string());
let mermaid_path = cli.mermaid_path.clone()
    .unwrap_or_else(|| "mmdc".to_string());
```

**`CliOverrides` struct deletion pattern** — remove lines 45-54 entirely:

```rust
// DELETE
/// CLI override values — mirrors the optional CLI flags from `Cli`.
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub confluence_url: Option<String>,
    pub confluence_username: Option<String>,
    pub confluence_api_token: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub plantuml_path: Option<String>,
    pub mermaid_path: Option<String>,
}
```

**Import pattern** — add `use crate::cli::Cli;` at the top of `src/config.rs` (currently only `use crate::error::ConfigError` at line 1).

**Test update pattern** (13 tests in `mod tests` at lines 263-616) — mechanical conversion: every `CliOverrides { ... }` literal becomes `Cli { ... command: Commands::Convert { ... } }` (or similar dummy `command`). Example conversion for `test_load_from_cli_overrides` (lines 275-293):

```rust
// CURRENT
let overrides = CliOverrides {
    confluence_url: Some("https://example.atlassian.net".to_string()),
    confluence_username: Some("user@example.com".to_string()),
    confluence_api_token: Some("token123".to_string()),
    anthropic_api_key: Some("ant-key".to_string()),
    ..Default::default()
};
let config = Config::load_with_home(&overrides, Some(&no_home()))
    .expect("should load from CLI overrides");
```

```rust
// TARGET
use crate::cli::{Cli, Commands, OutputFormat};
use std::path::PathBuf;

let cli = Cli {
    confluence_url: Some("https://example.atlassian.net".to_string()),
    confluence_username: Some("user@example.com".to_string()),
    confluence_token: Some("token123".to_string()),   // NOTE: Cli field is confluence_token
    anthropic_api_key: Some("ant-key".to_string()),
    plantuml_path: None,
    mermaid_path: None,
    verbose: false,
    output: OutputFormat::Human,
    command: Commands::Convert {
        markdown_path: PathBuf::new(),
        output_dir: PathBuf::new(),
    },
};
let config = Config::load_with_home(&cli, Some(&no_home()))
    .expect("should load from CLI");
```

The 7 `#[serial]`-annotated tests (established in Phase 07 per `07-VERIFICATION.md` line 70) stay serialized — they mutate env vars via `std::env::set_var`. The `#[serial]` annotation comes from `use serial_test::serial;` at line 266.

**Error handling pattern** (lines 118-122, `Self::resolve_optional` for `anthropic_api_key`) — preserve; `anthropic_api_key` is Optional, not Required, and the missing case is handled in `src/lib.rs:95-99`:

```rust
let api_key = config.anthropic_api_key.clone().ok_or_else(|| {
    AppError::Config(ConfigError::Missing {
        name: "ANTHROPIC_API_KEY",
    })
})?;
```

---

### `src/lib.rs` (controller / dispatch, request-response)

**Analog:** `src/lib.rs` update arm (lines 80-159) and upload arm (lines 160-206) serve as templates for the refactored Config-consuming arms; the convert arm (lines 207-256) is the target of the SCAF-03 fix.

**Imports pattern** (line 12) — simplify after `CliOverrides` deletion:

```rust
// CURRENT (line 12)
use config::{CliOverrides, Config, DiagramConfig};
```

```rust
// TARGET
use config::{Config, DiagramConfig};
```

**Update arm pattern (Config construction)** — replace lines 84-92:

```rust
// CURRENT (lines 84-92)
let overrides = CliOverrides {
    confluence_url: cli.confluence_url,
    confluence_username: cli.confluence_username,
    confluence_api_token: cli.confluence_token,
    anthropic_api_key: cli.anthropic_api_key.clone(),
    plantuml_path: cli.plantuml_path.clone(),
    mermaid_path: cli.mermaid_path.clone(),
};
let config = Config::load(&overrides)?;
```

```rust
// TARGET — per Pitfall 2: borrow &cli BEFORE cli.command is moved by the match
// (in practice, the match already destructures cli.command, so we either
//  (a) match on &cli.command and take references to Paths/Strings, or
//  (b) pass &cli and ensure Config::load only reads non-`command` fields — which it does)
let config = Config::load(&cli)?;
```

**Crucially:** the current code does `match cli.command { Commands::Update { markdown_path, page_url } => { ... }` which moves `cli.command`. To then call `Config::load(&cli)` inside the arm, the compiler allows it **only if** `Config::load` reads fields other than `command` (it does). Alternatively, restructure to `match &cli.command` with pattern bindings. Choose whichever keeps the code clearest.

**Upload arm pattern** — same treatment as update arm. Replace lines 164-172 with `let config = Config::load(&cli)?;`.

**Convert arm pattern (THE SCAF-03 FIX)** — replace lines 207-227:

```rust
// CURRENT (lines 207-227) — the SCAF-03 bug
Commands::Convert {
    markdown_path,
    output_dir,
} => {
    let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
    dotenvy::dotenv().ok();                                          // REMOVE
    let diagram_config = DiagramConfig {
        plantuml_path: cli.plantuml_path
            .or_else(|| std::env::var("PLANTUML_PATH").ok())         // REMOVE — clap already did this
            .unwrap_or_else(|| "plantuml".to_string()),
        mermaid_path: cli.mermaid_path
            .or_else(|| std::env::var("MERMAID_PATH").ok())          // REMOVE — clap already did this
            .unwrap_or_else(|| "mmdc".to_string()),
        mermaid_puppeteer_config: std::env::var("MERMAID_PUPPETEER_CONFIG").ok(),  // KEEP — no CLI flag
        timeout_secs: std::env::var("DIAGRAM_TIMEOUT")                             // KEEP — no CLI flag
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
    };
    let converter = MarkdownConverter::new(diagram_config);
```

```rust
// TARGET — convert arm after SCAF-03 fix
Commands::Convert {
    markdown_path,
    output_dir,
} => {
    let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
    // dotenvy::dotenv() hoisted to main.rs per Pitfall 1 (see src/main.rs pattern)
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
    // ... rest unchanged: convert_result, create_dir_all, write page.xml, write attachments
```

**Error handling pattern** — unchanged: `AppError::Io` wrapping (lines 102, 175, 212, 230, 234, 240), `AppError::Config` via `?` conversion (line 92 today → retained after refactor).

**CommandResult pattern** (lines 21-66) — unchanged; this refactor does not touch the success-reporting enum.

---

### `src/main.rs` (entrypoint, batch)

**Analog:** `src/main.rs` itself — small addition to hoist `dotenvy::dotenv().ok()` before `Cli::parse()` per Pitfall 1 (Research section).

**Current pattern** (lines 19-27):

```rust
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let output_format = cli.output.clone();
    let verbose = cli.verbose;
    init_tracing(verbose);
    let result = confluence_agent::run(cli).await;
    // ...
```

**Target pattern** (add `dotenvy::dotenv().ok();` as the first line inside `main`):

```rust
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();       // ← hoisted from Config::load() so clap's env= sees .env
    let cli = Cli::parse();
    let output_format = cli.output.clone();
    let verbose = cli.verbose;
    init_tracing(verbose);
    let result = confluence_agent::run(cli).await;
    // ...
```

The `dotenvy::dotenv().ok()` call in `Config::load` (src/config.rs:76) can then be removed — it's redundant after the hoist.

---

### `tests/cli_integration.rs` (test / integration, request-response)

**Analog:** `tests/cli_integration.rs::test_convert_with_diagram_path_flags` (lines 347-388). Same role (CLI integration test), same data flow (spawn binary → env/args → assert exit status + file existence).

**Imports pattern** (lines 1-5 of the file) — reuse the existing imports; no new imports except the `#[serial_test::serial]` attribute which is already a dev-dependency:

```rust
//! Integration tests for CLI command wiring (CLI-01, CLI-02, CLI-03, CLI-05).

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;
```

**Helper pattern** (lines 11-18) — reuse `temp_markdown` as-is:

```rust
fn temp_markdown(content: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let md_path = dir.path().join("doc.md");
    fs::write(&md_path, content).expect("write temp markdown");
    (dir, md_path)
}
```

**Core test pattern (CLI flag tier)** — `test_convert_with_diagram_path_flags` (lines 347-388) — **to be mirrored for env-var tier**:

```rust
#[test]
fn test_convert_with_diagram_path_flags() {
    let (md_dir, md_path) = temp_markdown("# Diagram Flag Test\n\nPlain content, no diagrams.\n");
    let out_dir = TempDir::new().expect("create output dir");

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    cmd.arg("--plantuml-path")
        .arg("/fake/plantuml")
        .arg("--mermaid-path")
        .arg("/fake/mmdc")
        .arg("convert")
        .arg(&md_path)
        .arg(out_dir.path())
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN")
        .env_remove("PLANTUML_PATH")                // isolate CLI tier
        .env_remove("MERMAID_PATH");                // isolate CLI tier

    let output = cmd.output().expect("run command");

    assert!(
        output.status.success(),
        "convert with --plantuml-path and --mermaid-path should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Converted to:"), "stdout should contain 'Converted to:'; got: {stdout}");
    let xml_path = out_dir.path().join("page.xml");
    assert!(xml_path.exists(), "page.xml should exist in output dir");
    drop(md_dir);
}
```

**New test pattern (env-var tier, D-06)** — insert directly after `test_convert_with_diagram_path_flags` in `tests/cli_integration.rs`. Mirror the flag-tier test but invert: NO `--plantuml-path` / `--mermaid-path` args, and SET `PLANTUML_PATH` / `MERMAID_PATH` env vars via `cmd.env()`:

```rust
/// convert command honors PLANTUML_PATH and MERMAID_PATH env vars when no
/// CLI flag is provided (SCAF-03 env-var tier, D-06). Clap-derive's
/// `#[arg(long, env = "...")]` resolves env var → cli.plantuml_path /
/// cli.mermaid_path; the convert arm reads those already-resolved values.
#[test]
#[serial_test::serial]   // env-var mutation must not race other env-mutating tests
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
        // env-var tier — NOT CLI flags
        .env("PLANTUML_PATH", "/fake/plantuml-via-env")
        .env("MERMAID_PATH", "/fake/mmdc-via-env");

    let output = cmd.output().expect("run command");

    assert!(
        output.status.success(),
        "convert with PLANTUML_PATH / MERMAID_PATH env vars should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let xml_path = out_dir.path().join("page.xml");
    assert!(xml_path.exists(), "page.xml should exist in output dir");

    drop(md_dir);
}
```

**Serialization pattern** — `#[serial_test::serial]` is the idiom already established in Phase 07 for env-var-mutating tests (see `07-VERIFICATION.md` line 55: `use serial_test::serial` at src/config.rs:235; `serial_test = "3.4.0"` in dev-dependencies). Use the fully-qualified `#[serial_test::serial]` since `tests/cli_integration.rs` does not currently import the macro at file scope (adding `use serial_test::serial;` is also acceptable; either is fine).

---

### `.planning/phases/08-.../08-VERIFICATION.md` (planning artifact, documentation)

**Analog:** `.planning/phases/07-test-scaffold-completion/07-VERIFICATION.md` — exact structural match (multi-plan phase with override-applied example) and `.planning/phases/06-credential-waterfall-fix/06-VERIFICATION.md` (single-plan phase, no overrides; simpler baseline).

**Frontmatter pattern** (from 07-VERIFICATION.md lines 1-12):

```yaml
---
phase: 08-diagramconfig-waterfall-and-nyquist-compliance
verified: 2026-04-20T00:00:00Z
status: passed                  # or human_needed if any SC requires manual
score: 9/9 must-haves verified  # 5 from 08-01-PLAN.md truths + 4 from 08-02-PLAN.md truths
overrides_applied: 0
---
```

If any override is needed (e.g., Phase 08 success criteria worded differently than the code actually achieves), follow the 07-VERIFICATION.md override block (lines 7-11):

```yaml
overrides_applied: 1
overrides:
  - must_have: "<exact must-have text>"
    reason: "<structural reason why this is acceptable>"
    accepted_by: "gsd-verifier"
    accepted_at: "2026-04-20T00:00:00Z"
```

**Body structure pattern** — copy 07-VERIFICATION.md's section order verbatim:

1. `# Phase 8: ... Verification Report` (H1)
2. `**Phase Goal:**`, `**Verified:**`, `**Status:**`, `**Re-verification:**` preamble lines (lines 16-19 of 07-VERIFICATION.md)
3. `## Goal Achievement` → `### Observable Truths` with 4-column evidence table (`#`, `Truth`, `Status`, `Evidence`) citing file paths and line numbers
4. `### Deferred Items` (None for Phase 08 per D-07)
5. `### Required Artifacts` with 4-column table
6. `### Key Link Verification` with 5-column table (`From`, `To`, `Via`, `Status`, `Details`)
7. `### Data-Flow Trace (Level 4)` — state "Not applicable" if no dynamic rendering (Phase 08 is config wiring, so "Not applicable")
8. `### Behavioral Spot-Checks` with 4-column table (`Behavior`, `Command`, `Result`, `Status`)
9. `### Requirements Coverage` — table mapping each requirement ID (SCAF-03) to plan source + evidence
10. `### Anti-Patterns Found` — table (or "No blockers found")
11. `### Human Verification Required` — "None" per CONTEXT D-07 + Research A5
12. `### Gaps Summary` — narrative summary of each SC satisfaction
13. Footer: `_Verified: 2026-04-20_` / `_Verifier: Claude (gsd-verifier)_`

**Evidence row pattern** (from 06-VERIFICATION.md line 22):

```
| 1 | Running `--anthropic-api-key sk-xxx` passes the key through to Config without requiring ANTHROPIC_API_KEY env var | VERIFIED | `src/cli.rs` line 30-31: `#[arg(long, env = "ANTHROPIC_API_KEY")] pub anthropic_api_key: Option<String>`; `src/lib.rs` line 88: `anthropic_api_key: cli.anthropic_api_key.clone()` (Update arm) |
```

Every Evidence cell MUST cite file path + line number + relevant code excerpt. Do not write "verified" without structural evidence.

**Requirements Coverage row pattern** (from 06-VERIFICATION.md line 61):

```
| SCAF-03 | 06-01-PLAN.md | Credentials loaded via waterfall: CLI flag → env var → ~/.claude/ | SATISFIED | Four-tier waterfall confirmed: (1) CLI flag via clap `env` attr, (2) env var via same attr, (3) .env via `dotenvy::dotenv()` in `Config::load`, (4) `~/.claude/` via `resolve_optional` home-dir fallback in config.rs |
```

**Scope pattern (CRITICAL — Pitfall 5)** — VERIFICATION.md for Phase 08 must describe Phase 08's state *at close* (2026-04-15). If Plan 09-02 runs AFTER Plan 09-01's refactor, VERIFICATION.md cites stale line numbers. **Recommendation: Plan 09-02 runs Wave 1 BEFORE Plan 09-01's Wave 2.** Or 09-02 freezes evidence against commit SHA `c4997ef` (last Phase 08 commit, per 08-02-SUMMARY.md).

---

## Shared Patterns

### Pattern: Clap-Derive env Resolution (apply to ALL Cli Option<String> fields)

**Source:** `src/cli.rs` lines 18-39 (all 4 credential fields + 2 diagram path fields)
**Apply to:** Every config-read in Phase 9 — trust `cli.<field>` as the already-resolved value

```rust
#[arg(long, env = "CONFLUENCE_URL")]
pub confluence_url: Option<String>,
```

At `Cli::parse()` return, `cli.confluence_url` is `Some(...)` if EITHER `--confluence-url https://x` was passed OR `CONFLUENCE_URL=https://x` was set. No downstream `std::env::var("CONFLUENCE_URL")` call is ever needed. This is D-01/D-02 in action.

**Anti-pattern:** `cli.plantuml_path.or_else(|| std::env::var("PLANTUML_PATH").ok())` — the SCAF-03 bug. Clap already did this.

### Pattern: Error Handling — `AppError` via `?` and Explicit Mapping

**Source:** `src/lib.rs` lines 102, 175, 212 (`.map_err(AppError::Io)`) and lines 95-99 (`AppError::Config`)
**Apply to:** All arms in `src/lib.rs::run()`; all return paths in `Config::load_with_home`

```rust
let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
```

```rust
let api_key = config.anthropic_api_key.clone().ok_or_else(|| {
    AppError::Config(ConfigError::Missing { name: "ANTHROPIC_API_KEY" })
})?;
```

The `From<ConfigError> for AppError` impl (confirmed via `Config::load(&cli)?` on the update/upload arms) means `ConfigError` promotes to `AppError` automatically via `?`. No manual map required for config errors.

### Pattern: Test Isolation for Env-Var-Mutating Tests

**Source:** `src/config.rs` line 266 (`use serial_test::serial;`) and 7 `#[serial]` annotations (per 07-VERIFICATION.md line 70)
**Apply to:** The new `test_convert_with_env_var_diagram_paths` test; any test file using `std::env::set_var` or `cmd.env()` on `PLANTUML_PATH` / `MERMAID_PATH` / credentials

```rust
#[test]
#[serial_test::serial]
fn test_foo_that_mutates_env_vars() {
    // ...
}
```

This prevents races with other env-var tests when `cargo test` runs with default parallelism. Established as a hard requirement in Phase 07.

### Pattern: `no_home()` Sentinel for Credential Test Isolation

**Source:** `src/config.rs` lines 270-272

```rust
/// A non-existent home path used in tests to prevent reading real ~/.claude/ credentials.
fn no_home() -> PathBuf {
    PathBuf::from("/nonexistent-test-home-dir-that-cannot-exist")
}
```

**Apply to:** Every updated test in `src/config.rs::mod tests` after the `CliOverrides` → `Cli` conversion — pass `Some(&no_home())` as the `home` argument to `Config::load_with_home(&cli, Some(&no_home()))` so `load_from_claude_config` never reads the real developer's `~/.claude/settings.json`.

### Pattern: `env_remove` for CLI-Flag Isolation in Integration Tests

**Source:** `tests/cli_integration.rs` lines 360-365 (`test_convert_with_diagram_path_flags`)

```rust
cmd.env_remove("CONFLUENCE_URL")
   .env_remove("CONFLUENCE_USERNAME")
   .env_remove("CONFLUENCE_API_TOKEN")
   .env_remove("PLANTUML_PATH")
   .env_remove("MERMAID_PATH");
```

**Apply to:** `test_convert_with_env_var_diagram_paths` **should NOT** use `env_remove` on `PLANTUML_PATH` / `MERMAID_PATH` (those are the tier under test); it SHOULD use `env_remove` on the Confluence credential vars so the test doesn't accidentally succeed via leaked shell state.

### Pattern: VERIFICATION.md Frontmatter Schema

**Source:** `.planning/phases/{06,07}-*/VERIFICATION.md` lines 1-12
**Apply to:** The new 08-VERIFICATION.md

Required keys: `phase`, `verified`, `status`, `score`, `overrides_applied`. Optional: `overrides:` list (when `overrides_applied > 0`), `human_verification:` list (when `status: human_needed`).

Status values observed: `passed` (06, 07), `human_needed` (01). For Phase 08 per Research A5, use `passed`.

---

## No Analog Found

None. Every file touched in Phase 9 has either a same-file precedent (refactor in-place: cli.rs, config.rs, lib.rs, main.rs), an exact sibling-test analog (cli_integration.rs), or an exact prior-phase artifact analog (VERIFICATION.md). This is characteristic of a refactor + verification phase — all patterns exist in-tree.

---

## Metadata

**Analog search scope:**

- `src/` (all 4 files modified)
- `tests/cli_integration.rs` (integration-test analog)
- `.planning/phases/01-*/01-VERIFICATION.md` (human_needed reference)
- `.planning/phases/06-*/06-VERIFICATION.md` (single-plan passed reference)
- `.planning/phases/07-*/07-VERIFICATION.md` (multi-plan passed + overrides reference)
- `.planning/phases/08-*/{08-01-PLAN.md, 08-02-PLAN.md, 08-VALIDATION.md}` (source of Phase 08 must-haves)

**Files scanned:** 13

**Pattern extraction date:** 2026-04-20

**Project-specific notes (from CLAUDE.md + Research):**

- CLAUDE.md describes the Python predecessor; for Rust files use `cargo build` / `cargo test` instead of `black` / `mypy` / `pytest`.
- `markdownlint --fix` DOES apply to the new 08-VERIFICATION.md (Plan 09-02).
- Version-pinning rule applies if any new deps were added — none are needed for Phase 9.
