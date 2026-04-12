# Phase 4: CLI Command Wiring and Integration - Research

**Researched:** 2026-04-12
**Domain:** Rust CLI wiring, tracing-subscriber, clap output flags, serde_json
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** `--output` is a global flag on the top-level `Cli` struct (same level as `--verbose`). Applies to all subcommands consistently.

**D-02:** JSON output schema per command:
- `update`: `{ success: bool, page_url: String, comments_kept: u32, comments_dropped: u32, error?: String }`
- `upload`: `{ success: bool, page_url: String, error?: String }`
- `convert`: `{ success: bool, output_dir: String, files: [String], error?: String }` — lists all files written (storage XML + SVG attachments)

**D-03:** In JSON mode, all output goes to stdout as a single JSON object. Errors also go to stdout as `{ success: false, error: "..." }` rather than stderr. This keeps stdout machine-parseable for Claude Code skills.

**D-04:** Default (non-verbose) mode is silent until done — no output during LLM evaluation. On success: one line (e.g., `Updated page: <url>` or page URL). On failure: error message to stderr.

**D-05:** `--output json` also stays silent during execution; emits the JSON object on completion.

**D-06:** `--verbose` enables `tracing-subscriber` with human-readable pretty format (default `tracing_subscriber::fmt()` with timestamps, level, and span names).

**D-07:** All tracing/logging output goes to stderr. Stdout is reserved for normal output (page URL line) or JSON. This keeps piping clean when `--output json` is active.

**D-08:** Default log level without `--verbose`: warnings and errors only. With `--verbose`: debug level (shows per-comment evaluation spans, HTTP request details, etc.).

**D-09:** Standard 0/1 only — 0 for success, 1 for any failure. Error details communicated via stderr (human mode) or the `error` field in JSON output. No granular codes.

### Claude's Discretion

- Tracing span design (which events/fields to instrument within the pipeline steps) — follow existing `tracing::info!` / `tracing::warn!` patterns from Phases 1–3
- Whether to add `tracing::instrument` attributes or manual spans — Claude decides based on what's useful for debugging
- Exact human-readable success message format (e.g., `Updated page: <url>` vs `Successfully updated <url>`)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLI-01 | `update <markdown_path> <page_url>` — full merge pipeline (convert → fetch → merge → upload) | Wiring pattern from upload stub in lib.rs; all phase components have verified public APIs |
| CLI-02 | `upload <markdown_path> <page_url>` — direct overwrite without LLM merge | Upload stub already exists; needs converter wiring and output formatting |
| CLI-03 | `convert <markdown_path> <output_dir>` — local conversion only, no Confluence upload | MarkdownConverter::convert() returns ConvertResult with storage_xml + attachments; write to fs |
| CLI-04 | `--verbose` flag for debug output; structured logs via `tracing` | tracing-subscriber 0.3.23 already in Cargo.toml; subscriber init missing from main.rs |
| CLI-05 | JSON output mode (`--output json`) for machine-readable results | clap `ValueEnum` derive for OutputFormat; serde_json::to_string for emission |
</phase_requirements>

## Summary

Phase 4 is a pure wiring phase: all the component pieces (ConfluenceClient, MarkdownConverter, AnthropicClient, merge()) are implemented and tested. The work is connecting them inside `src/lib.rs`'s `run()` function, adding the `--output` flag to `src/cli.rs`, and initializing the tracing subscriber in `src/main.rs`.

The codebase is in excellent shape for this phase. The upload command already shows the exact wiring pattern (load config → build client → call API). The update command follows the same pattern extended with converter → fetch → merge → upload steps. The convert command is the simplest: no credentials required, just convert and write files to disk.

Tracing-subscriber 0.3.23 (already in Cargo.toml) provides everything needed for D-06 through D-08 with zero new dependencies. The `tracing_subscriber::fmt()` builder can route to stderr, filter by env-filter or runtime condition, and format as compact or pretty. The JSON output mode (D-02/D-03) is just `serde_json::json!()` emitted to stdout at the end of each command path.

**Primary recommendation:** Wire lib.rs first (update command is the core), then add OutputFormat enum to cli.rs and implement the output layer as a thin wrapper around the existing error-propagation chain.

## Standard Stack

### Core (already in Cargo.toml — no new dependencies needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tracing | 0.1 | Span/event instrumentation | Already used throughout lib; `#[instrument]` attributes |
| tracing-subscriber | 0.3.23 | Subscriber init, stderr routing, level filtering | Already in Cargo.toml; env-filter feature present |
| serde_json | 1.0.149 | JSON output serialization | Already in Cargo.toml; `serde_json::json!` macro sufficient |
| clap | 4.6.0 | `ValueEnum` derive for OutputFormat enum | Already in Cargo.toml with derive feature |
| anyhow | 1 | Error propagation in run() | Already used; `anyhow::Result<()>` boundary |

[VERIFIED: cargo metadata output above]

### No New Dependencies Required

All libraries needed for Phase 4 are already declared in Cargo.toml. The only code changes are:
- `src/cli.rs` — add `OutputFormat` enum and `--output` field to `Cli`
- `src/lib.rs` — wire all three commands, add output formatting logic
- `src/main.rs` — add tracing subscriber init before `run()`

## Architecture Patterns

### Recommended File Structure (changes only)

```
src/
├── cli.rs          # Add OutputFormat enum + --output field to Cli struct
├── lib.rs          # Wire all three commands; add output_result() helper
├── main.rs         # Add tracing subscriber init before run()
```

No new modules needed. Everything stays in existing files.

### Pattern 1: OutputFormat Enum (clap ValueEnum)

**What:** A clap-derivable enum for the `--output` flag.
**When to use:** D-01 requires `--output` to be a global flag on `Cli`.

```rust
// Source: clap 4.x docs / [ASSUMED] — pattern is well-established
use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}
```

Add to `Cli`:
```rust
/// Output format
#[arg(long, value_enum, default_value_t = OutputFormat::Human)]
pub output: OutputFormat,
```

### Pattern 2: Tracing Subscriber Init in main.rs

**What:** Initialize tracing-subscriber once at startup, routing to stderr, with level controlled by `--verbose`.
**When to use:** D-06, D-07, D-08.

```rust
// Source: tracing-subscriber 0.3 docs
// [VERIFIED: tracing-subscriber 0.3.23 in resolved Cargo.lock]
use tracing_subscriber::{EnvFilter, fmt};

fn init_tracing(verbose: bool) {
    let level = if verbose { "debug" } else { "warn" };
    fmt()
        .with_env_filter(EnvFilter::new(level))
        .with_writer(std::io::stderr)
        .init();
}
```

Call `init_tracing(cli.verbose)` in `main()` before `confluence_agent::run(cli).await`.

Key API notes [VERIFIED: tracing-subscriber 0.3 docs]:
- `.with_writer(std::io::stderr)` — routes all log output to stderr (D-07)
- `EnvFilter::new("warn")` vs `EnvFilter::new("debug")` — controls verbosity (D-08)
- Default `fmt()` format is compact; `fmt().pretty()` adds multi-line formatting — either works for `--verbose` (Claude's discretion)
- `env-filter` feature is already enabled in Cargo.toml

### Pattern 3: Update Command Pipeline

**What:** Sequential async steps — convert → fetch → merge → upload.
**When to use:** CLI-01.

```rust
// All public APIs verified by reading source files above
Commands::Update { markdown_path, page_url } => {
    // 1. Build config and clients
    let config = Config::load(&overrides)?;
    let confluence = ConfluenceClient::new(...);
    let converter = MarkdownConverter::default();
    let llm = AnthropicClient::new(api_key, config.anthropic_model.clone());

    // 2. Convert markdown
    let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
    let convert_result = converter.convert(&markdown).await?;

    // 3. Fetch existing page
    let page_id = extract_page_id(&page_url)?;
    let page = confluence.get_page(&page_id).await?;
    let old_content = &page.body.storage.value;

    // 4. Merge (or skip if empty)
    let merge_result = merge::merge(
        old_content,
        &convert_result.storage_xml,
        Arc::new(llm),
        config.anthropic_concurrency,
    ).await?;

    // 5. Upload attachments
    for att in &convert_result.attachments {
        confluence.upload_attachment(&page_id, &att.filename, att.content.clone(), &att.content_type).await?;
    }

    // 6. Update page
    let next_version = page.version.number + 1;
    confluence.update_page(&page_id, &page.title, &merge_result.content, next_version).await?;

    // 7. Output
    output_result(output_format, UpdateOutput {
        success: true,
        page_url: page_url.clone(),
        comments_kept: merge_result.kept as u32,
        comments_dropped: merge_result.dropped as u32,
        error: None,
    });
}
```

Note: `AnthropicClient::new()` requires `anthropic_api_key` to be present — must validate and surface a clear error if missing (use `config.anthropic_api_key.ok_or(AppError::Config(ConfigError::Missing { name: "ANTHROPIC_API_KEY" }))?`).

### Pattern 4: Convert Command (no credentials)

**What:** Convert markdown and write files to output directory without Confluence.
**When to use:** CLI-03.

```rust
Commands::Convert { markdown_path, output_dir } => {
    let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
    let converter = MarkdownConverter::default();
    let result = converter.convert(&markdown).await?;

    std::fs::create_dir_all(&output_dir).map_err(AppError::Io)?;

    // Write storage XML
    let xml_path = output_dir.join("page.xml");
    std::fs::write(&xml_path, &result.storage_xml).map_err(AppError::Io)?;
    let mut files = vec![xml_path.to_string_lossy().to_string()];

    // Write SVG attachments
    for att in &result.attachments {
        let att_path = output_dir.join(&att.filename);
        std::fs::write(&att_path, &att.content).map_err(AppError::Io)?;
        files.push(att_path.to_string_lossy().to_string());
    }

    output_result(output_format, ConvertOutput { success: true, output_dir: ..., files, error: None });
}
```

Important: Convert command does NOT require Confluence credentials. Must NOT call `Config::load()` for the convert path (or at minimum not require the Confluence fields). [VERIFIED: `src/config.rs` — `Config::load()` requires all three Confluence fields and will fail if absent]

### Pattern 5: JSON Output via serde_json

**What:** Emit a single JSON object to stdout on completion.
**When to use:** D-02, D-03, D-05.

```rust
// [VERIFIED: serde_json 1.0 in Cargo.toml]
use serde_json::json;

fn emit_json_update(success: bool, page_url: &str, kept: u32, dropped: u32, error: Option<&str>) {
    let obj = if success {
        json!({
            "success": true,
            "page_url": page_url,
            "comments_kept": kept,
            "comments_dropped": dropped
        })
    } else {
        json!({
            "success": false,
            "page_url": page_url,
            "comments_kept": kept,
            "comments_dropped": dropped,
            "error": error.unwrap_or("unknown error")
        })
    };
    println!("{}", obj);
}
```

Per D-03: errors in JSON mode go to stdout (not stderr). The `run()` function must catch errors and convert to the JSON error shape rather than propagating them through `?`.

### Pattern 6: Error Handling in JSON Mode

**What:** `run()` must not propagate errors via `?` when in JSON mode — it must catch them and emit the error JSON to stdout.
**When to use:** D-03.

The cleanest approach: wrap each command's inner logic in a closure or helper that returns `Result<_, AppError>`, then at the top level of each match arm, check the output format:

```rust
let result = run_update_inner(&config, ...).await;
match (output_format, result) {
    (OutputFormat::Json, Ok(out)) => println!("{}", serde_json::to_string(&out).unwrap()),
    (OutputFormat::Json, Err(e)) => println!("{}", json!({"success": false, "error": e.to_string()})),
    (OutputFormat::Human, Ok(out)) => println!("Updated page: {}", out.page_url),
    (OutputFormat::Human, Err(e)) => { eprintln!("Error: {e}"); std::process::exit(1); }
}
```

### Pattern 7: Exit Code Handling

**What:** Return exit code 1 on failure.
**When to use:** D-09.

In `main.rs`, `main()` returns `anyhow::Result<()>` which sets exit code 1 on `Err`. However, with JSON mode, errors are captured and emitted as JSON — `run()` must return `Ok(())` even on command failure in JSON mode, and call `std::process::exit(1)` manually, OR restructure so that `run()` returns a typed result the caller converts to an exit code.

Simplest approach that satisfies both modes:

```rust
// main.rs
async fn main() {
    let cli = Cli::parse();
    init_tracing(cli.verbose);
    if let Err(e) = confluence_agent::run(cli).await {
        // Only reached in human mode (JSON mode handles internally)
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
```

`run()` returns `Ok(())` in JSON mode always (after emitting `{"success":false,...}` to stdout). Returns `Err(...)` in human mode on failure.

### Anti-Patterns to Avoid

- **Calling Config::load() for the convert command path:** Convert does not need Confluence credentials. If `Config::load()` is called unconditionally before the match, it will fail when credentials are absent. Route the `Commands::Convert` arm before credential loading, or use `Config::load()` only inside the credential-requiring arms.
- **Writing tracing/log output to stdout:** D-07 requires stderr for all tracing. Tracing-subscriber's default writer is stdout — must explicitly set `.with_writer(std::io::stderr)`.
- **Printing progress messages in non-verbose mode:** D-04/D-05 require silence during execution. Do not use `println!()` for intermediate steps.
- **Emitting errors to stderr in JSON mode:** D-03 requires all output including errors to stdout in JSON mode.
- **Using `tracing_subscriber::fmt::init()` shorthand:** This writes to stdout. Must use the builder form to set stderr writer.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Log level filtering | Custom log level check | `EnvFilter::new(level)` | Already in Cargo.toml; handles env overrides |
| JSON serialization | Manual string building | `serde_json::json!()` | Already in Cargo.toml; handles escaping |
| Enum → string for clap | Manual From/Into impls | `#[derive(ValueEnum)]` | clap 4 derive feature already enabled |
| Retry-on-409 for update_page | Custom retry loop | `update_page_with_retry()` | Already exists in `confluence::client` |

## Common Pitfalls

### Pitfall 1: Config::load() Requires Confluence Credentials
**What goes wrong:** `Commands::Convert` is called without CONFLUENCE_* env vars set. If `Config::load()` runs unconditionally at the top of `run()`, it errors before reaching the convert arm.
**Why it happens:** Config treats Confluence URL/username/token as required fields.
**How to avoid:** Call `Config::load()` inside the match arms that need it (`Update`, `Upload`), not at the top level. `Convert` arm can run without config entirely.
**Warning signs:** Integration test for convert fails with "Missing required configuration: CONFLUENCE_URL" when no Confluence env vars are set.

### Pitfall 2: tracing-subscriber init() Panics on Double Init
**What goes wrong:** If `init_tracing()` is called more than once (e.g., in tests), it panics with "global default already set".
**Why it happens:** `tracing_subscriber::fmt().init()` sets a global subscriber. Tests that call `run()` directly may each try to init.
**How to avoid:** Use `try_init()` instead of `init()` in test contexts, or initialize in `main()` only (not in `run()`). Keep subscriber init strictly in `main.rs`.
**Warning signs:** Test panics with "SetGlobalDefaultError" or "cannot set global default twice".

### Pitfall 3: AnthropicClient Requires API Key for Update Command
**What goes wrong:** `update` is called without `ANTHROPIC_API_KEY` set. `config.anthropic_api_key` is `Option<String>` — accessing it without checking yields None and AnthropicClient panics or produces a confusing error later.
**Why it happens:** The config intentionally makes `anthropic_api_key` optional (it's not needed for `upload` or `convert`).
**How to avoid:** In the `update` arm, explicitly unwrap with a useful error: `config.anthropic_api_key.clone().ok_or_else(|| AppError::Config(ConfigError::Missing { name: "ANTHROPIC_API_KEY" }))?`.
**Warning signs:** LLM call fails with a reqwest auth error instead of a clear "API key not configured" message.

### Pitfall 4: Page Title Required for update_page()
**What goes wrong:** `confluence.update_page()` requires a `title` parameter. The update and upload commands receive the page URL, not the title. Title must be fetched from the existing page.
**Why it happens:** Confluence REST API v1 PUT requires title even when unchanged.
**How to avoid:** Always use `page.title` from the `get_page()` response when calling `update_page()`. Already handled in `update_page_with_retry()` — check its signature.
**Warning signs:** 400 error from Confluence API mentioning missing title field.

### Pitfall 5: update_page_with_retry vs update_page
**What goes wrong:** `update_page_with_retry` in `confluence::client` already handles version-conflict retry. Calling `confluence.update_page()` directly in the update/upload commands skips this retry logic.
**Why it happens:** Two update paths exist: the trait method and the free function with retry.
**How to avoid:** Use `update_page_with_retry(&confluence, &page_id, &content, N)` for update and upload commands, not `confluence.update_page()` directly.
**Warning signs:** Version conflict errors (409) surface to the user on busy pages.

Note: Check `update_page_with_retry` signature — it may need `title` parameter too. Read `src/confluence/client.rs` fully before implementing.

### Pitfall 6: stdout vs stderr Contamination in JSON Mode
**What goes wrong:** Debug `println!()` statements or `tracing::info!()` output appears on stdout, breaking JSON parsing by the calling skill.
**Why it happens:** Default tracing-subscriber writes to stdout; `println!()` always goes to stdout.
**How to avoid:** All tracing output on stderr (`.with_writer(std::io::stderr)`); all intermediate status on stderr or suppressed in non-verbose mode; only the final result on stdout.
**Warning signs:** JSON parse error in the calling Claude Code skill; `jq` errors on the output.

## Code Examples

### tracing-subscriber Init with stderr (verified API)

```rust
// Source: tracing-subscriber 0.3 docs; version 0.3.23 confirmed in lock file
use tracing_subscriber::{fmt, EnvFilter};

fn init_tracing(verbose: bool) {
    let level = if verbose { "debug" } else { "warn" };
    fmt()
        .with_env_filter(EnvFilter::new(level))
        .with_writer(std::io::stderr)
        .init();
}
```

[VERIFIED: tracing-subscriber 0.3.23 in resolved deps; env-filter feature in Cargo.toml]

### clap ValueEnum for OutputFormat

```rust
// Source: clap 4.x derive docs; version 4.6.0 confirmed
use clap::ValueEnum;

#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}
```

Clap renders `--output human` and `--output json` as valid values. Default can be set with `#[arg(default_value_t = OutputFormat::Human)]`.

[VERIFIED: clap 4.6.0 in deps; derive + env features enabled in Cargo.toml]

### update_page_with_retry Call Pattern (from existing upload stub)

```rust
// Source: src/lib.rs upload stub (existing verified code)
update_page_with_retry(&client, &page_id, &content, 3).await?;
```

Note: The existing stub passes raw markdown content. Phase 4 replaces this with `merge_result.content` (for update) or `convert_result.storage_xml` (for upload).

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `tracing_subscriber::fmt::init()` | `fmt().with_writer(stderr).init()` | stderr routing required by D-07 |
| `println!()` for all output | Conditional on OutputFormat | JSON mode requires structured stdout only |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `update_page_with_retry` signature accepts `(client, page_id, content, retries)` without requiring `title` | Architecture Patterns §5 | Compile error; need to read full client.rs |
| A2 | `fmt().with_writer(std::io::stderr)` compiles in tracing-subscriber 0.3.23 | Code Examples | Compile error; use `MakeWriter` alternative |

**Only 2 assumptions — both are low-risk implementation details easily resolved by reading client.rs during planning.**

## Open Questions

1. **update_page_with_retry signature for title**
   - What we know: The function exists in `confluence::client` and is used in the upload stub
   - What's unclear: Whether it takes a `title` parameter (needed for update_page API call)
   - Recommendation: Read the rest of `src/confluence/client.rs` before writing the update command plan

2. **JSON mode exit code**
   - What we know: D-09 requires exit code 1 on failure; D-03 requires error JSON on stdout
   - What's unclear: Exact mechanism — does `run()` return Ok(()) after emitting error JSON, then main calls exit(1)?
   - Recommendation: `run()` returns `Ok(())` always in JSON mode; emit JSON first, then `std::process::exit(1)` for failures

## Environment Availability

Step 2.6: SKIPPED — Phase 4 is pure code wiring with no new external dependencies. All libraries are already in Cargo.toml and the Rust toolchain is already in use.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) + wiremock 0.6 + assert_cmd 2 |
| Config file | none (cargo.toml workspace) |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLI-01 | update command wires convert→fetch→merge→upload | integration | `cargo test --test '*' -- update` | ❌ Wave 0 |
| CLI-02 | upload command converts and overwrites page | integration | `cargo test --test '*' -- upload` | ❌ Wave 0 |
| CLI-03 | convert writes storage XML + SVGs to output dir | integration | `cargo test --test '*' -- convert` | ❌ Wave 0 |
| CLI-04 | --verbose sends tracing to stderr, not stdout | unit | `cargo test --lib -- tracing` | ❌ Wave 0 |
| CLI-05 | --output json emits valid JSON on stdout | unit/integration | `cargo test --lib -- json_output` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `tests/cli_integration.rs` — covers CLI-01 (update), CLI-02 (upload), CLI-03 (convert) using wiremock + tempdir
- [ ] `tests/cli_integration.rs` — covers CLI-05 (JSON output) by parsing stdout
- [ ] Unit tests in `src/lib.rs` or a new `tests/output_format.rs` for CLI-04 (stderr routing)

Note: `assert_cmd 2` is already in `[dev-dependencies]` — use it for CLI-level integration tests that spawn the binary and capture stdout/stderr.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Credentials already handled in Config + ConfluenceClient |
| V3 Session Management | no | Stateless CLI tool |
| V4 Access Control | no | Single-user CLI |
| V5 Input Validation | yes | File path validation (markdown_path, output_dir must exist/be writable) |
| V6 Cryptography | no | TLS handled by reqwest/rustls; no hand-rolled crypto |

### Known Threat Patterns for CLI Wiring Phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| API key leaked to stdout in verbose mode | Information Disclosure | D-07: all tracing to stderr; existing test `test_api_key_not_in_debug_output` in llm_integration.rs verifies this pattern |
| Credentials in JSON output | Information Disclosure | JSON schemas (D-02) do not include credential fields — only page URLs, counts, and errors |
| Path traversal via output_dir | Tampering | `std::fs::create_dir_all` + write only to paths derived from `output_dir` join |

## Sources

### Primary (HIGH confidence)
- `src/cli.rs` — existing Cli struct, Commands enum, --verbose field [VERIFIED: read above]
- `src/lib.rs` — existing run() function with upload stub showing wiring pattern [VERIFIED: read above]
- `src/main.rs` — entry point; tracing init placement [VERIFIED: read above]
- `src/error.rs` — AppError, ConfigError, all error variants [VERIFIED: read above]
- `src/config.rs` — Config::load(), CliOverrides, DiagramConfig [VERIFIED: read above]
- `src/confluence/mod.rs` — ConfluenceApi trait surface [VERIFIED: read above]
- `src/converter/mod.rs` — Converter trait, ConvertResult, MarkdownConverter [VERIFIED: read above]
- `src/merge/mod.rs` — merge() function signature and MergeResult [VERIFIED: read above]
- `src/llm/mod.rs` — LlmClient trait, AnthropicClient::new() [VERIFIED: read above]
- `Cargo.toml` — dependency versions and features [VERIFIED: read above]
- `cargo metadata` — resolved versions: tracing-subscriber 0.3.23, clap 4.6.0, serde_json 1.0.149 [VERIFIED: bash output above]

### Secondary (MEDIUM confidence)
- tracing-subscriber 0.3 builder API (`.with_writer()`, `EnvFilter::new()`) — [ASSUMED based on tracing-subscriber 0.3 documentation patterns; confirmed version 0.3.23 is in lock file]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies verified in Cargo.toml and cargo metadata
- Architecture: HIGH — all component APIs verified by reading source; wiring pattern is direct
- Pitfalls: HIGH — identified from reading actual Config implementation and existing code patterns
- Test plan: MEDIUM — assert_cmd is present; specific test file names are proposals

**Research date:** 2026-04-12
**Valid until:** 2026-05-12 (stable Rust codebase; no fast-moving dependencies)
