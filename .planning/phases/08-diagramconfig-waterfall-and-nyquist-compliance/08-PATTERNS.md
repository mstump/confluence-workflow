---
phase: 8
slug: diagramconfig-waterfall-and-nyquist-compliance
mapped: 2026-04-14
files_analyzed: 5
analogs_found: 5
---

# Phase 8: DiagramConfig Waterfall and Nyquist Compliance — Pattern Map

**Mapped:** 2026-04-14
**Files analyzed:** 5 (3 Rust source modifications + 3 VALIDATION.md edits)
**Analogs found:** 5 / 5

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/cli.rs` | config | request-response | `src/cli.rs` lines 29–31 (existing `anthropic_api_key` field) | exact |
| `src/config.rs` | config | request-response | `src/config.rs` lines 47–52 (`CliOverrides`), lines 115–119 (`resolve_optional`) | exact |
| `src/lib.rs` | utility / dispatch | request-response | `src/lib.rs` lines 84–89 (`CliOverrides` construction in Update arm) | exact |
| `.planning/phases/01-.../01-VALIDATION.md` | planning artifact | — | `.planning/phases/06-.../06-VALIDATION.md` lines 1–9 (frontmatter block) | role-match |
| `.planning/phases/02-.../02-VALIDATION.md` | planning artifact | — | `.planning/phases/06-.../06-VALIDATION.md` lines 1–9 (frontmatter block) | role-match |
| `.planning/phases/03-.../03-VALIDATION.md` | planning artifact | — | `.planning/phases/06-.../06-VALIDATION.md` lines 1–9 (frontmatter block) | role-match |

---

## Pattern Assignments

### `src/cli.rs` — add `plantuml_path` and `mermaid_path` fields

**Analog:** `src/cli.rs` — existing `anthropic_api_key` field declaration

**Existing flag pattern** (lines 29–31):
```rust
/// Anthropic API key (for update command's LLM merge)
#[arg(long, env = "ANTHROPIC_API_KEY")]
pub anthropic_api_key: Option<String>,
```

**New fields to insert immediately after `anthropic_api_key` and before `verbose`:**
```rust
/// Path to PlantUML executable or JAR
#[arg(long, env = "PLANTUML_PATH")]
pub plantuml_path: Option<String>,

/// Path to mermaid-cli executable (mmdc)
#[arg(long, env = "MERMAID_PATH")]
pub mermaid_path: Option<String>,
```

**Placement rule:** All global flags live on the `Cli` struct above `#[command(subcommand)]` (line 41). Insert the two new fields between `anthropic_api_key` (line 31) and `verbose` (line 34). Clap reads the long flag first; if absent it reads the `env` var automatically — no extra code required.

---

### `src/config.rs` — extend `CliOverrides`, `Config`, and `load_with_home`

**Analog:** `src/config.rs` — all existing patterns are the direct template.

**CliOverrides extension** (current struct at lines 47–52; add two fields):
```rust
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

**Config struct extension** (current struct at lines 56–63; add one field):
```rust
pub struct Config {
    pub confluence_url: String,
    pub confluence_username: String,
    pub confluence_api_token: String,
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: String,
    pub anthropic_concurrency: usize,
    pub diagram_config: DiagramConfig,  // Phase 8
}
```

**Waterfall resolution pattern** (copy from `anthropic_api_key` pattern at lines 115–119; apply twice for diagram paths, with `.unwrap_or_else` default):
```rust
// Existing pattern for optional field (lines 115–119):
let anthropic_api_key = Self::resolve_optional(
    overrides.anthropic_api_key.as_deref(),
    "ANTHROPIC_API_KEY",
    home,
);

// New fields — same pattern with default fallback:
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

// Remaining DiagramConfig fields are env-only (no CLI flags in scope):
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

**`DiagramConfig` import:** `DiagramConfig` is already defined in `config.rs` (lines 6–37) — no new import needed. The struct construction above replaces what `DiagramConfig::from_env()` currently does, so the resolution logic is identical but now CLI flags feed in at the top of the waterfall.

**Updated `Ok(Config { ... })` return** (lines 134–142; add `diagram_config` field):
```rust
Ok(Config {
    confluence_url,
    confluence_username,
    confluence_api_token,
    anthropic_api_key,
    anthropic_model,
    anthropic_concurrency,
    diagram_config,   // Phase 8
})
```

**Test pattern** (copy from `test_anthropic_api_key_optional` at lines 402–425 — same `#[serial]`, env-save/restore, `CliOverrides` with `..Default::default()` spread):
```rust
#[test]
#[serial]
fn test_plantuml_path_cli_override() {
    let overrides = CliOverrides {
        confluence_url: Some("https://example.atlassian.net".to_string()),
        confluence_username: Some("user@example.com".to_string()),
        confluence_api_token: Some("token".to_string()),
        plantuml_path: Some("/custom/plantuml".to_string()),
        ..Default::default()
    };
    let saved = std::env::var("PLANTUML_PATH").ok();
    std::env::remove_var("PLANTUML_PATH");

    let config = Config::load_with_home(&overrides, Some(&no_home()))
        .expect("should load with plantuml_path override");

    if let Some(v) = saved { std::env::set_var("PLANTUML_PATH", v); }

    assert_eq!(config.diagram_config.plantuml_path, "/custom/plantuml");
}
```

---

### `src/lib.rs` — update all three `CliOverrides` construction sites and `MarkdownConverter` construction

**Analog:** `src/lib.rs` — the existing Update arm pattern at lines 84–101.

**Current `CliOverrides` construction (Update arm, lines 84–89):**
```rust
let overrides = CliOverrides {
    confluence_url: cli.confluence_url,
    confluence_username: cli.confluence_username,
    confluence_api_token: cli.confluence_token,
    anthropic_api_key: cli.anthropic_api_key.clone(),
};
```

**Updated construction for Update and Upload arms** (add two forwarded fields; `.clone()` on `anthropic_api_key` already present in Update arm):
```rust
let overrides = CliOverrides {
    confluence_url: cli.confluence_url,
    confluence_username: cli.confluence_username,
    confluence_api_token: cli.confluence_token,
    anthropic_api_key: cli.anthropic_api_key.clone(),
    plantuml_path: cli.plantuml_path.clone(),   // Phase 8
    mermaid_path: cli.mermaid_path.clone(),     // Phase 8
};
```

**Current `MarkdownConverter` construction (all three arms use the same line):**
```rust
let converter = MarkdownConverter::default();
```

**Replacement for Update and Upload arms** (after `Config::load()` resolves `diagram_config`):
```rust
let converter = MarkdownConverter::new(config.diagram_config.clone());
```

**Convert arm — no `Config::load()` present** (lines 203–237). Two approaches per RESEARCH.md; recommended approach is a standalone `DiagramConfig::load(overrides)` that mirrors `Config::load()`. The simpler inline alternative constructs `DiagramConfig` directly from CLI fields. Whichever the planner chooses, the convert arm pattern is:

Option A — `DiagramConfig::load()` helper (add to `config.rs`):
```rust
// In convert arm of lib.rs:
let diagram_overrides = DiagramConfig::load(&DiagramCliOverrides {
    plantuml_path: cli.plantuml_path,
    mermaid_path: cli.mermaid_path,
});
let converter = MarkdownConverter::new(diagram_overrides);
```

Option B — inline construction in convert arm (simpler, no new type):
```rust
// In convert arm of lib.rs:
dotenvy::dotenv().ok();
let diagram_config = DiagramConfig {
    plantuml_path: cli.plantuml_path
        .or_else(|| std::env::var("PLANTUML_PATH").ok())
        .unwrap_or_else(|| "plantuml".to_string()),
    mermaid_path: cli.mermaid_path
        .or_else(|| std::env::var("MERMAID_PATH").ok())
        .unwrap_or_else(|| "mmdc".to_string()),
    mermaid_puppeteer_config: std::env::var("MERMAID_PUPPETEER_CONFIG").ok(),
    timeout_secs: std::env::var("DIAGRAM_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30),
};
let converter = MarkdownConverter::new(diagram_config);
```

Note: `cli.plantuml_path` and `cli.mermaid_path` are `Option<String>` on `Cli`. In Update/Upload arms, use `.clone()` since `cli` is partially moved into the `CliOverrides`. In the Convert arm, fields can be moved directly since no `CliOverrides` is built.

---

### `.planning/phases/01-.../01-VALIDATION.md` — add frontmatter from scratch

**Analog:** `.planning/phases/06-credential-waterfall-fix/06-VALIDATION.md` lines 1–9

**Phase 06 reference frontmatter (canonical compliant form):**
```yaml
---
phase: 6
slug: credential-waterfall-fix
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-13
audited: 2026-04-13
---
```

**New frontmatter to prepend to `01-VALIDATION.md`** (file currently opens with bare `# Phase 01 Validation` heading — no `---` delimiters exist):
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

Place this block as the very first lines of the file, before the existing `# Phase 01 Validation` heading.

---

### `.planning/phases/02-.../02-VALIDATION.md` — update frontmatter values

**Analog:** `.planning/phases/06-credential-waterfall-fix/06-VALIDATION.md` lines 1–9

**Current frontmatter (lines 1–8 of `02-VALIDATION.md`):**
```yaml
---
phase: 2
slug: markdown-to-confluence-storage-format-converter
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-10
---
```

**Target frontmatter (fields to change; add `audited` key):**
```yaml
---
phase: 2
slug: markdown-to-confluence-storage-format-converter
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-10
audited: 2026-04-14
---
```

Pre-condition: executor must verify Phase 02 Wave 0 test items pass (`cargo test --lib converter::tests`) before setting `wave_0_complete: true`.

---

### `.planning/phases/03-.../03-VALIDATION.md` — update frontmatter values

**Analog:** `.planning/phases/06-credential-waterfall-fix/06-VALIDATION.md` lines 1–9

**Current frontmatter (lines 1–8 of `03-VALIDATION.md`):**
```yaml
---
phase: 3
slug: llm-client-and-comment-preserving-merge
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-10
---
```

**Target frontmatter:**
```yaml
---
phase: 3
slug: llm-client-and-comment-preserving-merge
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-10
audited: 2026-04-14
---
```

Pre-condition: executor must verify Phase 03 Wave 0 test items pass (`cargo test --lib merge` and `cargo test --lib llm`) before setting `wave_0_complete: true`.

---

## Shared Patterns

### `#[serial]` for env-var-mutating tests
**Source:** `src/config.rs` lines 237, 267, 294, 315, 346, 375, 402, 432
**Apply to:** All new `config::tests` functions that call `std::env::set_var` or `std::env::remove_var`

```rust
use serial_test::serial;

#[test]
#[serial]
fn test_new_waterfall_field() {
    // save current value
    let saved = std::env::var("PLANTUML_PATH").ok();
    std::env::set_var("PLANTUML_PATH", "test-value");

    // ... test body ...

    // restore
    match saved {
        Some(v) => std::env::set_var("PLANTUML_PATH", v),
        None => std::env::remove_var("PLANTUML_PATH"),
    }
}
```

### `no_home()` test helper
**Source:** `src/config.rs` lines 241–243
**Apply to:** All new `config::tests` that call `Config::load_with_home`

```rust
fn no_home() -> PathBuf {
    PathBuf::from("/nonexistent-test-home-dir-that-cannot-exist")
}
```

Use `Some(&no_home())` as the `home` argument to prevent any real `~/.claude/settings.json` from being read during tests.

### `..Default::default()` spread on `CliOverrides`
**Source:** `src/config.rs` lines 318–321, 349–352
**Apply to:** All test `CliOverrides` construction sites after new fields are added

```rust
let overrides = CliOverrides {
    confluence_url: Some("https://example.atlassian.net".to_string()),
    confluence_username: Some("user@example.com".to_string()),
    confluence_api_token: Some("token".to_string()),
    plantuml_path: Some("/custom/path".to_string()),
    ..Default::default()   // fills mermaid_path and anthropic_api_key with None
};
```

### `markdownlint` after VALIDATION.md edits
**Source:** CLAUDE.md
**Apply to:** All three VALIDATION.md file edits

Run `markdownlint --fix .` after editing any VALIDATION.md. The frontmatter block (`---` delimiters) is valid markdown; markdownlint will not alter it but may catch other formatting issues introduced during editing.

---

## No Analog Found

All files in this phase have close analogs in the codebase. No entries.

---

## Metadata

**Analog search scope:** `src/cli.rs`, `src/config.rs`, `src/lib.rs`, `src/converter/mod.rs`, `.planning/phases/06-*/06-VALIDATION.md`
**Files scanned:** 6 source files + 4 VALIDATION.md files
**Pattern extraction date:** 2026-04-14
