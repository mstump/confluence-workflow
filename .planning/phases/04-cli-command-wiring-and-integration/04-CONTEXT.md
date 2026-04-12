# Phase 4: CLI Command Wiring and Integration — Context

**Gathered:** 2026-04-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire all three CLI commands (`update`, `upload`, `convert`) through the full pipeline assembled from Phases 1–3. This phase does NOT add new capabilities — it connects the existing `ConfluenceClient`, `Converter`, `LlmClient`, and `MergeEngine` into working end-to-end command implementations, adds output formatting, and sets up structured logging.

</domain>

<decisions>
## Implementation Decisions

### Output Flag Design

- **D-01:** `--output` is a **global flag** on the top-level `Cli` struct (same level as `--verbose`). Applies to all subcommands consistently.
- **D-02:** JSON output schema per command:
  - `update`: `{ success: bool, page_url: String, comments_kept: u32, comments_dropped: u32, error?: String }`
  - `upload`: `{ success: bool, page_url: String, error?: String }`
  - `convert`: `{ success: bool, output_dir: String, files: [String], error?: String }` — lists all files written (storage XML + SVG attachments)
- **D-03:** In JSON mode, all output goes to stdout as a single JSON object. Errors also go to stdout as `{ success: false, error: "..." }` rather than stderr. This keeps stdout machine-parseable for Claude Code skills.

### Progress During LLM Calls

- **D-04:** Default (non-verbose) mode is **silent until done** — no output during LLM evaluation. On success: one line (e.g., `Updated page: <url>` or page URL). On failure: error message to stderr.
- **D-05:** `--output json` also stays silent during execution; emits the JSON object on completion.

### Tracing / Logging Format

- **D-06:** `--verbose` enables `tracing-subscriber` with **human-readable pretty format** (default `tracing_subscriber::fmt()` with timestamps, level, and span names).
- **D-07:** All tracing/logging output goes to **stderr**. Stdout is reserved for normal output (page URL line) or JSON. This keeps piping clean when `--output json` is active.
- **D-08:** Default log level without `--verbose`: warnings and errors only. With `--verbose`: debug level (shows per-comment evaluation spans, HTTP request details, etc.).

### Exit Codes

- **D-09:** Standard **0/1 only** — 0 for success, 1 for any failure. Error details communicated via stderr (human mode) or the `error` field in JSON output. No granular codes.

### Claude's Discretion

- Tracing span design (which events/fields to instrument within the pipeline steps) — follow existing `tracing::info!` / `tracing::warn!` patterns from Phases 1–3
- Whether to add `tracing::instrument` attributes or manual spans — Claude decides based on what's useful for debugging
- Exact human-readable success message format (e.g., `Updated page: <url>` vs `Successfully updated <url>`)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### CLI and Entry Point
- `src/cli.rs` — existing Cli struct and Commands enum (--verbose already declared; --output needs to be added)
- `src/lib.rs` — existing `run()` function with stubs to be replaced with real implementations
- `src/main.rs` — entry point; tracing subscriber init goes here

### Phase Component Interfaces (to wire together)
- `src/confluence/mod.rs` — ConfluenceApi trait and ConfluenceClient
- `src/converter/mod.rs` — Converter trait
- `src/llm/mod.rs` — LlmClient trait and AnthropicClient
- `src/merge/mod.rs` — MergeEngine / merge() function
- `src/config.rs` — Config struct (has all credential fields)
- `src/error.rs` — AppError and error hierarchy

### Requirements
- `.planning/REQUIREMENTS.md` §CLI Commands — CLI-01 through CLI-05 are the requirements for this phase

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Cli` struct in `src/cli.rs` — already has `--verbose` bool flag; just needs `--output` added as `Option<OutputFormat>` enum
- `Config::load()` in `src/config.rs` — already handles credential waterfall; called in upload stub
- `ConfluenceClient::new()` + `extract_page_id()` — already used in upload stub; reuse pattern for update
- `tracing` crate — already in Cargo.toml and used in lib.rs

### Established Patterns
- Error handling: `AppError` enum with `#[from]` conversions; `anyhow::Result` at boundaries
- All I/O components behind traits — `ConfluenceApi`, `Converter`, `LlmClient` — so `run()` can accept mocks for integration tests
- `update_page_with_retry()` in `confluence::client` — already exists for retry-on-409

### Integration Points
- `lib.rs run()` — all wiring happens here; this is the main file being changed in 04-01
- `main.rs` — tracing subscriber init (currently missing) goes here before `run()`
- The upload stub shows the pattern: load config → build client → call API → print result

</code_context>

<specifics>
## Specific Ideas

- No specific UI/UX references — the requirements (CLI-01 to CLI-05) and success criteria in ROADMAP.md are the spec
- For JSON output, the Claude Code skill integration is the primary consumer — keep the schema minimal and stable

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

*Phase: 04-cli-command-wiring-and-integration*
*Context gathered: 2026-04-12*
