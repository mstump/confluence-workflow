---
phase: 01-project-scaffolding-and-confluence-api-client
verified: 2026-04-10T12:00:00Z
status: human_needed
score: 4/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run `confluence-agent upload <real-markdown-file> <real-confluence-url>` against an actual Confluence instance"
    expected: "Command loads credentials, contacts Confluence REST API, increments page version, and returns success message on stdout"
    why_human: "Cannot test against live Confluence without real credentials and a test page; wiremock tests verify the HTTP logic but not the end-to-end credential flow with a real server"
---

# Phase 1: Project Scaffolding and Confluence API Client Verification Report

**Phase Goal:** A buildable Rust workspace with credential loading, a trait-based Confluence client, and a working direct-upload path (no LLM) against a real Confluence instance
**Verified:** 2026-04-10T12:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cargo build` succeeds with zero warnings on a clean checkout | VERIFIED | `cargo build` completes with `Finished dev profile` and no warnings; `.cargo/config.toml` enforces `-D warnings` |
| 2 | Running the binary with `upload` subcommand against a real Confluence page overwrites that page's content and returns success | ? NEEDS HUMAN | Upload command is fully wired in `src/lib.rs` (config load, client build, page ID extract, retry upload) but real-instance test requires live credentials and a test page |
| 3 | Credentials are loaded from `ANTHROPIC_API_KEY` env var or `~/.claude/` config file without requiring both | VERIFIED | `Config::load()` calls `dotenvy::dotenv()` then `load_with_home()` which checks env var `ANTHROPIC_API_KEY` and falls through to `load_from_claude_config()` reading `~/.claude/settings.json`; field is `Option<String>` so absence is not an error |
| 4 | Confluence API errors (auth failure, 404, 409 version conflict) produce clear, actionable error messages — not raw HTTP status codes | VERIFIED | `ConfluenceError` enum in `src/error.rs` has human-readable messages for `Unauthorized`, `PageNotFound`, `VersionConflict`, `InvalidPageUrl`, `AttachmentUpload`; wiremock tests confirm 401/404/409 map to correct variants |
| 5 | A mock `ConfluenceApi` trait implementation can be substituted in tests without touching production code | VERIFIED | `MockConfluenceClient` in `src/confluence/mod.rs` implements `ConfluenceApi` via `async_trait`; `update_page_with_retry` accepts `&dyn ConfluenceApi` enabling mock injection; test passes |

**Score:** 4/5 roadmap truths verified (1 requires human)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | Workspace manifest with pinned deps | VERIFIED | All 13 production deps and 3 dev deps present with exact versions; `[profile.release]` with lto/strip/panic=abort |
| `.cargo/config.toml` | `-D warnings` build flag | VERIFIED | `rustflags = ["-D", "warnings"]` confirmed |
| `src/main.rs` | Thin shell calling `Cli::parse()` + `confluence_agent::run()` | VERIFIED | 8 lines; `#[tokio::main]`, `Cli::parse()`, delegates to `confluence_agent::run(cli).await` |
| `src/lib.rs` | Module re-exports, `run()` function | VERIFIED | Exports `cli`, `config`, `confluence`, `error` modules; `run()` matches all three `Commands` variants with upload wired end-to-end |
| `src/cli.rs` | `Cli` struct and `Commands` enum with clap derive | VERIFIED | `Cli` has `confluence_url`, `confluence_username`, `confluence_token`, `verbose` with `env = "..."` fallback; `Commands` has `Update`, `Upload`, `Convert` with correct args |
| `src/error.rs` | `AppError`, `ConfigError`, `ConfluenceError` enums | VERIFIED | Three-level hierarchy with `thiserror`; all variants have user-facing `#[error("...")]` messages; `#[from]` conversions on `AppError` |
| `src/config.rs` | `Config` struct and waterfall loader | VERIFIED | `CliOverrides`, `Config`, `Config::load()`, `Config::load_with_home()`, `load_from_claude_config()` all present; 10 unit tests |
| `src/confluence/mod.rs` | `ConfluenceApi` trait and re-exports | VERIFIED | Trait defined with `get_page`, `update_page`, `upload_attachment`; re-exports `ConfluenceClient`, `Page`, `extract_page_id` |
| `src/confluence/client.rs` | `ConfluenceClient` implementing `ConfluenceApi` | VERIFIED | 394 lines; full implementation with Basic Auth, 30s timeout, retry logic; 8 wiremock tests |
| `src/confluence/types.rs` | `Page`, `PageBody`, `StorageRepresentation`, `PageVersion` | VERIFIED | All four structs with `Deserialize, Debug, Clone`; deserialization test passes |
| `src/confluence/url.rs` | `extract_page_id` with three URL patterns | VERIFIED | Three `OnceLock<Regex>` patterns (edit-v2 first, then /pages/, then pageId=); 5 unit tests cover all patterns |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `src/cli.rs` | `Cli::parse()` | WIRED | Line 6: `let cli = Cli::parse()` |
| `src/main.rs` | `src/lib.rs` | `confluence_agent::run()` | WIRED | Line 7: `confluence_agent::run(cli).await` |
| `src/config.rs` | `src/error.rs` | returns `ConfigError` on failure | WIRED | `Config::load_with_home()` returns `Result<Self, ConfigError>`; `ConfigError::Missing` used for all required-field errors |
| `src/config.rs` | `dotenvy` | `dotenvy::dotenv()` call | WIRED | Line 30 in `Config::load()`: `dotenvy::dotenv().ok()` |
| `src/confluence/client.rs` | `reqwest::Client` | shared HTTP client | WIRED | `reqwest::Client::builder().timeout(30s).build()` in `ConfluenceClient::new()` |
| `src/confluence/client.rs` | `src/error.rs` | returns `ConfluenceError` variants | WIRED | All HTTP status matches return `ConfluenceError` variants; trait impl signature enforces this |
| `src/confluence/mod.rs` | `src/confluence/client.rs` | `impl ConfluenceApi for ConfluenceClient` | WIRED | `#[async_trait] impl ConfluenceApi for ConfluenceClient` in `client.rs` lines 39-154 |
| `src/lib.rs` | `src/config.rs` | `Config::load()` in upload command | WIRED | Line 27: `let config = Config::load(&overrides)?` |
| `src/lib.rs` | `src/confluence` | `ConfluenceClient::new()` + `extract_page_id()` | WIRED | Lines 28-33 in `run()` upload branch |
| `src/lib.rs` | `update_page_with_retry` | retry wrapper for upload | WIRED | Line 38: `update_page_with_retry(&client, &page_id, &markdown, 3).await?` |

### Data-Flow Trace (Level 4)

Not applicable for this phase — all artifacts are CLI infrastructure and API client code, not data-rendering components. The upload command flow is end-to-end (credentials → client → API call) and verified by wiremock tests.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `--help` shows all three subcommands | `cargo run -- --help` | Shows `update`, `upload`, `convert` with descriptions | PASS |
| Unknown subcommand produces clear error | `cargo run -- unknown-subcommand` | `error: unrecognized subcommand 'unknown-subcommand'` + help hint | PASS |
| `upload --help` shows correct args | `cargo run -- upload --help` | Shows `MARKDOWN_PATH` and `PAGE_URL` | PASS |
| `update --help` shows correct args | `cargo run -- update --help` | Shows `MARKDOWN_PATH` and `PAGE_URL` | PASS |
| `convert --help` shows correct args | `cargo run -- convert --help` | Shows `MARKDOWN_PATH` and `OUTPUT_DIR` | PASS |
| All unit tests pass (sequential) | `cargo test -- --test-threads=1` | 25 passed; 0 failed | PASS |
| All unit tests pass (parallel) | `cargo test` | 23 passed; 2 failed (env var race) | FAIL (see Anti-Patterns) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| SCAF-01 | 01-01 | Cargo workspace builds cleanly | SATISFIED | `cargo build` succeeds; `-D warnings` in `.cargo/config.toml` |
| SCAF-02 | 01-01 | CLI accepts update/upload/convert subcommands | SATISFIED | clap derive with `Commands` enum; all three subcommands confirmed in `--help` |
| SCAF-03 | 01-02 | Credential waterfall: CLI > env > .env > ~/.claude/ stub | SATISFIED | `Config::load()` implements full waterfall; 10 unit tests |
| SCAF-04 | 01-02 | Config supports all required fields | SATISFIED | `Config` has `confluence_url`, `confluence_username`, `confluence_api_token`, `anthropic_api_key` |
| SCAF-05 | 01-01 | Structured error types with actionable messages | SATISFIED | Three-level `AppError > ConfigError/ConfluenceError` hierarchy with `thiserror` |
| CONF-01 | 01-03 | Fetch page content (storage XML + version) | SATISFIED | `get_page()` GETs `?expand=body.storage,version`; wiremock test verifies 200 response |
| CONF-02 | 01-03 | Update with version increment + retry-on-409 | SATISFIED | `update_page_with_retry()` re-fetches on 409; wiremock tests cover retry success and exhaustion |
| CONF-03 | 01-03 | Upload SVG attachment with nocheck header | SATISFIED | `upload_attachment()` sets `X-Atlassian-Token: nocheck`; wiremock test verifies header |
| CONF-04 | 01-03 | Extract page ID from all URL patterns | SATISFIED | Three regex patterns with `OnceLock`; 5 unit tests cover all patterns |
| CONF-05 | 01-03 | Mock ConfluenceApi substitutable in tests | SATISFIED | `MockConfluenceClient` implements trait; `update_page_with_retry` takes `&dyn ConfluenceApi` |

### Anti-Patterns Found

| File | Lines | Pattern | Severity | Impact |
|------|-------|---------|----------|--------|
| `src/config.rs` | 209-224, 231-247 | `std::env::set_var` in parallel test contexts causes race condition | Warning | 2 config tests fail under default `cargo test` (parallel); all 25 pass with `--test-threads=1` |
| `src/lib.rs` | 15 | `println!("update command: not yet implemented")` | Info | Intentional per plan; Phase 3 will implement |
| `src/lib.rs` | 47 | `println!("convert command: not yet implemented")` | Info | Intentional per plan; Phase 2 will implement |
| `src/lib.rs` | 37 | Upload command sends raw markdown (not storage XML) | Info | Intentional per plan; Phase 2 converter will produce proper storage XML |

**Note on test race condition:** The parallel test failure is documented in `deferred-items.md` and was known before Phase 1 closed. The fix (using `serial_test` crate or restructuring tests to avoid global env mutation) is deferred as low-priority. It does not affect production code behavior.

### Human Verification Required

#### 1. Upload command against real Confluence instance

**Test:** Create a test markdown file (e.g., `test.md` with content `# Test`). Run:
```
confluence-agent upload test.md https://<your-domain>.atlassian.net/wiki/spaces/<SPACE>/pages/<ID>/Test+Page
```
With credentials set via env vars `CONFLUENCE_URL`, `CONFLUENCE_USERNAME`, `CONFLUENCE_API_TOKEN`.

**Expected:** Command outputs `Uploaded test.md to <url>` and the Confluence page content is replaced with the markdown text. No error is returned.

**Why human:** Cannot test against a live Confluence instance programmatically without real credentials and a writable test page. The HTTP logic is verified by 8 wiremock tests, but end-to-end flow with a real server requires human validation.

### Gaps Summary

No blocking gaps found. All five success criteria are implemented. One success criterion (SC-2: real Confluence upload) requires human verification because it requires a live Confluence instance.

The test parallel-execution race is a known, documented issue that does not affect production code. It is classified as a warning-level anti-pattern, not a gap.

The `update` and `convert` command stubs are intentional per the plan — these commands integrate with Phase 2 (converter) and Phase 3 (LLM merge) respectively.

---

_Verified: 2026-04-10T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
