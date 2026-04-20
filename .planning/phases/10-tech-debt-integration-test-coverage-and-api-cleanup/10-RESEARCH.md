# Phase 10: Tech Debt — Integration Test Coverage and API Cleanup — Research

**Researched:** 2026-04-20
**Domain:** Rust integration testing (CLI binary + wiremock) and dead-code removal
**Confidence:** HIGH

## Summary

This is a closed-scope tech-debt phase with all research questions resolved by the existing codebase. The two goals — (1) add happy-path integration tests for `update` and `upload` against wiremock, and (2) delete `DiagramConfig::from_env()` plus `impl Default for DiagramConfig` — both have direct implementation precedents inside this repository:

- **Wiremock-based LLM happy-path test** already exists in `tests/llm_integration.rs` (uses `AnthropicClient::with_endpoint` — the pre-built test seam).
- **Wiremock-based Confluence happy-path test** already exists as unit tests in `src/confluence/client.rs` mod tests (uses `ConfluenceClient::new(&mock_server.uri(), ...)`).
- **Binary invocation + env-var-based test pattern** already exists in `tests/cli_integration.rs` (uses `assert_cmd::Command::cargo_bin("confluence-agent")` plus `serial_test::serial`).

The phase work is therefore **combination research, not discovery research**: the individual techniques are proven in-repo; Phase 10 simply composes them into two new end-to-end tests. The one genuinely new piece of plumbing is threading `ANTHROPIC_BASE_URL` from env into `AnthropicClient::new()` — since `with_endpoint()` already exists, this is a ~5-line constructor change.

**Primary recommendation:** Follow the existing `tests/cli_integration.rs` + `tests/llm_integration.rs` patterns exactly. Add both new integration tests to `tests/cli_integration.rs` (same file, not a new crate) to preserve visibility and keep the "happy-path" story in one place. Do the `DiagramConfig` cleanup in a single commit after test changes land so the planner can keep test failures and cleanup failures separable.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CLI argument parsing / env-var resolution | CLI (`src/cli.rs` via clap-derive) | — | Clap owns the Tier-1 CLI flag → env var resolution. |
| Config loading, https guard, localhost exemption | Config (`src/config.rs::Config::load_with_home`) | — | Single change point for D-01 localhost exemption. |
| Confluence HTTP calls under test | Test infra (wiremock `MockServer`) | Prod client (`ConfluenceClient`) | Client already accepts `&mock_server.uri()` as base URL. |
| Anthropic HTTP calls under test | Test infra (wiremock `MockServer`) | Prod client (`AnthropicClient::with_endpoint`) | Existing test seam; D-03 exposes it via env var. |
| Binary invocation + assertions | Test infra (`assert_cmd::Command::cargo_bin`) | — | Owns process-level I/O (stdout, stderr, exit code). |
| Markdown → storage XML | Converter (`src/converter/mod.rs`) | — | Exercised transparently by the binary during `update`/`upload`. |
| Merge pipeline (short-circuit on 0 comments) | Merge (`src/merge/mod.rs`) | — | For update happy-path, a page body with no `<ac:inline-comment-marker>` triggers the MERGE-06 short-circuit and skips LLM calls — relevant for test design choice (see Pattern 2 below). |

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01 (https localhost exemption):** The `Config::load_with_home` https guard (`src/config.rs` ~line 88) is relaxed to accept `http://localhost` and `http://127.0.0.1` in addition to `https://`. Change is ~3 lines. The `test_confluence_url_must_be_https` unit test (config.rs line 476) must be amended or complemented with a localhost-allowed assertion. Security invariant — rejecting arbitrary http URLs — is preserved.
- **D-02 (wiremock for both Confluence and Anthropic):** Happy-path integration tests spin up two `MockServer` instances and invoke the binary via `assert_cmd::Command::cargo_bin("confluence-agent")` with `--confluence-url http://localhost:{port}` and `ANTHROPIC_BASE_URL` env var pointing at the second server.
- **D-03 (`ANTHROPIC_BASE_URL` env var):** `AnthropicClient::new()` reads `ANTHROPIC_BASE_URL` (falling back to `https://api.anthropic.com/v1/messages`). Test-infrastructure concern, not CLI-exposed.
- **D-04 (DiagramConfig cleanup):** Delete `DiagramConfig::from_env()`, `impl Default for DiagramConfig`, and `impl Default for MarkdownConverter`. Rewrite all callers in `src/converter/tests.rs` (lines 87, 99, 107, 124, 142, 165, 198) and `src/converter/diagrams.rs` (lines 195, 203, 209, 215) to use explicit struct construction. `cargo build` must be warning-free.

### Claude's Discretion

- Exact wiremock stub shape for the Anthropic response (follow `tests/llm_integration.rs::tool_use_response` helper verbatim).
- Whether to un-ignore `test_upload_command_happy_path` in place or write a new test with a clearer name.
- Whether to add `#[serial]` on new tests that mutate `ANTHROPIC_BASE_URL`.
- Exact error message wording in the https-guard unit test for the localhost exemption.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLI-01 | `update <markdown_path> <page_url>` — test coverage (happy path) | REQUIREMENTS.md traceability row ("Phase 10 (test coverage)"). Satisfied by adding `test_update_command_happy_path` using wiremock for both Confluence + Anthropic. Pattern documented in "Code Examples" section below. |
| CLI-02 | `upload <markdown_path> <page_url>` — test coverage (happy path) | REQUIREMENTS.md traceability row ("Phase 10 (test coverage)"). Satisfied by un-ignoring `test_upload_command_happy_path` (currently `#[ignore]` at `tests/cli_integration.rs:332`) and rewriting it to use wiremock + localhost URL — no Anthropic mock required (upload skips LLM). |

## Project Constraints (from CLAUDE.md)

The repository-root `CLAUDE.md` is **stale** — it describes the project as a Python/uv/typer/mcp-agent codebase, but the current codebase is Rust (see `Cargo.toml`). The Rust project has replaced the Python one per ROADMAP Phase 1 ("Project Scaffolding"). The agent should treat the Rust repository itself — Cargo, clippy, tests — as the authoritative constraint source. Specifically:

- `cargo build` must compile warning-free after changes [VERIFIED: CONTEXT.md D-04 explicit; Phase 09 review IN-01].
- `cargo test --all-targets` must pass (all Rust tests, including unit tests in `src/`) [VERIFIED: existing test infrastructure in repo].
- Pin dev-dependency versions in `Cargo.toml` using the established style (`wiremock = "0.6"`, `assert_cmd = "2"`, `serial_test = "3.4.0"` — the latter exact-pinned) [VERIFIED: `Cargo.toml` as of 2026-04-20].
- Do NOT edit `CLAUDE.md` unless explicitly instructed — it remains stale Python documentation; changing it is out of phase scope.

**Note for planner:** When `CLAUDE.md` directives conflict with actual Rust codebase patterns (e.g., `uv run pytest` vs `cargo test`), follow the Rust patterns. The Python instructions in `CLAUDE.md` are vestigial from the pre-rewrite project. Add a follow-up item to cleanup `CLAUDE.md` in a future phase; do not slip it into Phase 10.

## Standard Stack

### Core (already in `Cargo.toml` dev-dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `wiremock` | 0.6 (locks to 0.6.5) | HTTP mock server for integration tests | Already used in `src/confluence/client.rs` and `tests/llm_integration.rs`; canonical async HTTP mocking crate for Rust. [VERIFIED: `Cargo.toml` line 35; `cargo search wiremock` → 0.6.5 latest as of 2026-04-20] |
| `assert_cmd` | 2 (locks to 2.2.1) | Spawn the built binary and assert on stdout/stderr/exit code | Already used in `tests/cli_integration.rs` for every existing CLI test. [VERIFIED: `Cargo.toml` line 37; `cargo search assert_cmd` → 2.2.1] |
| `serial_test` | 3.4.0 (exact) | Serialize env-var-mutating tests | Already used in `tests/cli_integration.rs` for `test_convert_with_env_var_diagram_paths`; required pattern for any test that touches process env. [VERIFIED: `Cargo.toml` line 38] |
| `tempfile` | 3 | `TempDir` for markdown + output fixtures | Already used in every `tests/cli_integration.rs` test. [VERIFIED: `Cargo.toml` line 32] |
| `serde_json` | 1 | Build wiremock response body JSON | Already a regular dependency. [VERIFIED: `Cargo.toml` line 24] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio::time::sleep` (already imported) | — | Not needed for happy path — only relevant if testing backoff | Skip for Phase 10. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| wiremock | `mockito`, `httpmock`, `hyper` handcrafted | Would introduce a second mocking stack; inconsistent with the two existing wiremock test files. Reject. |
| `assert_cmd` | `std::process::Command` | Would lose the built-in `Command::cargo_bin("confluence-agent")` binary-path resolution and require manual `target/debug/` path construction. Reject. |
| TLS wiremock (for keeping https guard strict) | `wiremock-tls` fork, `warp` with rustls | Would avoid touching `Config::load_with_home`, but introduces a TLS trust-anchor setup (self-signed cert loading in the binary under test). Rejected in CONTEXT.md — the localhost exemption is simpler and documented (D-01). |

**Installation:** All dependencies already present in `Cargo.toml`. No `cargo add` step required.

**Version verification (2026-04-20):**
```
wiremock 0.6.5     — latest on crates.io
assert_cmd 2.2.1   — latest on crates.io
serial_test 3.4.0  — latest on crates.io
```
[VERIFIED: `cargo search` output 2026-04-20]

## Architecture Patterns

### System Architecture Diagram (Happy-Path Test)

```
assert_cmd::Command::cargo_bin("confluence-agent")
        │  sets ANTHROPIC_BASE_URL=http://localhost:{anthropic_port}
        │  sets env-removes to avoid leaking real creds
        │  passes --confluence-url http://localhost:{confluence_port}
        ▼
    confluence-agent binary (main.rs)
        │  dotenvy::dotenv().ok()
        │  Cli::parse()  ← clap-derive resolves env → cli fields
        ▼
    run(cli)  (lib.rs)
        │
        ├──► Config::load(&cli)
        │       └─ https guard: ALLOW http://localhost (D-01)
        │
        ├──► MarkdownConverter::new(diagram_config).convert(&md)  (pure, no HTTP)
        │
        ├──► AnthropicClient::new(api_key, model)
        │       └─ reads ANTHROPIC_BASE_URL env (D-03) → wiremock #2
        │       ▲
        │       └── wiremock #2: POST /v1/messages returns tool_use KEEP
        │           (only hit when page has inline comments — see Pattern 2)
        │
        ├──► ConfluenceClient::new(&config.confluence_url, user, token)
        │       └─ base_url = http://localhost:{confluence_port} → wiremock #1
        │       ▲
        │       └── wiremock #1:
        │             GET /rest/api/content/{page_id}   → page JSON (version N)
        │             PUT /rest/api/content/{page_id}   → 200 OK (version N+1)
        │             POST /rest/api/content/{id}/child/attachment → 200 (if attachments)
        │
        ▼
    CommandResult::Update { ... }  |  CommandResult::Upload { ... }
        │
        ▼
    stdout: "Updated page: ..."  |  "Uploaded to: ..."    exit 0
```

### Component Responsibilities (files the tests touch)

| File | Responsibility | Change for Phase 10 |
|------|----------------|---------------------|
| `src/config.rs` | Config::load_with_home https guard | D-01: add localhost / 127.0.0.1 exemption (~3 lines) |
| `src/config.rs` | `DiagramConfig::from_env()`, `impl Default for DiagramConfig` | D-04: delete (lines 22-44) |
| `src/config.rs` | test `test_confluence_url_must_be_https` (line 476) | Amend to verify localhost is ACCEPTED alongside existing http rejection |
| `src/converter/mod.rs` | `impl Default for MarkdownConverter` (lines 47-51) | D-04: delete |
| `src/converter/mod.rs::tests` (`src/converter/tests.rs`) | 7 call sites of `MarkdownConverter::default()` / `DiagramConfig::default()` | Replace with explicit `DiagramConfig { ... }` construction (see Pattern 3) |
| `src/converter/diagrams.rs` (mod tests) | 4 call sites of `DiagramConfig::from_env()` | Replace with explicit construction; the env-var behavior being tested must be kept — re-implement inline using `std::env::var(...)` OR delete the `test_diagram_config_from_env` test entirely since the function is being removed |
| `src/llm/mod.rs` | `AnthropicClient::new` | D-03: read `ANTHROPIC_BASE_URL` env; fall back to hardcoded URL |
| `tests/cli_integration.rs` | Happy-path tests | Add/repurpose: `test_update_command_happy_path` (new); `test_upload_command_happy_path` (un-ignore, rewrite) |

### Recommended Project Structure

No structural changes. New tests go in the existing `tests/cli_integration.rs` file, alongside the existing `#[test] test_convert_command`, `test_update_command_missing_api_key`, and `test_upload_command_rejects_http_url` tests. Reuse the `temp_markdown()` helper.

### Pattern 1: wiremock `MockServer` + localhost `MockServer::uri()`

**What:** Start a wiremock server bound to a random localhost port, mount responses, and pass `server.uri()` (returns `http://127.0.0.1:{port}`) to the code under test.

**When to use:** Every happy-path test for `update` and `upload`.

**Example:**
```rust
// Source: src/confluence/client.rs mod tests (test_get_page_200, test_update_page_with_retry_succeeds_on_second_attempt)
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use serde_json::json;

fn page_json(id: &str, version: u32) -> serde_json::Value {
    json!({
        "id": id,
        "title": "Test Page",
        "body": {
            "storage": {
                "value": "<p>old content</p>",
                "representation": "storage"
            }
        },
        "version": { "number": version }
    })
}

#[tokio::test]
async fn example_happy_path() {
    let confluence = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/rest/api/content/12345"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_json("12345", 7)))
        .mount(&confluence)
        .await;

    Mock::given(method("PUT"))
        .and(path("/rest/api/content/12345"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_json("12345", 8)))
        .mount(&confluence)
        .await;

    // confluence.uri() == "http://127.0.0.1:{random_port}"
}
```

**Key wiremock semantics:** Once `MockServer` is dropped, the port is released. `.mount(&server)` is cumulative (multiple mounts on the same server compose). If a request doesn't match any mount, wiremock returns 404 by default — use `.expect(1)` to assert a mock was hit exactly once when you need stricter verification.

### Pattern 2: Short-circuit the merge when you don't want to mock the LLM

**What:** If the `GET /rest/api/content/{id}` response body contains zero `<ac:inline-comment-marker>` elements, `merge::merge()` short-circuits at `src/merge/mod.rs:85-92` and returns `new_content` verbatim with `kept=0, dropped=0, llm_evaluated=0` — the LLM is never invoked.

**When to use:** If the planner wants a simpler `update` happy-path test that covers the full pipeline (convert → fetch → merge → upload) but does not need to exercise LLM evaluation, use a mock Confluence response with a plain `<p>old content</p>` body. The LLM wiremock can then be set up but will register zero expected calls.

**Tradeoff:** Skipping the LLM mock reduces test value — a "happy path" that never hits the LLM doesn't prove the LLM-integration wiring. **Recommended:** Include at least one inline-comment marker in the old page body so the LLM is called, and assert the `wiremock::MockServer::received_requests()` count to prove the LLM was hit. Two complementary tests may be appropriate: `test_update_command_happy_path_no_comments` (fast, verifies plumbing) and `test_update_command_happy_path_with_comments` (slower, verifies LLM path).

**Example (old page body with one comment marker):**
```rust
fn page_json_with_comment(id: &str, version: u32) -> serde_json::Value {
    json!({
        "id": id,
        "title": "Test Page",
        "body": {
            "storage": {
                "value": "<p>Before <ac:inline-comment-marker ac:ref=\"abc-123\">important</ac:inline-comment-marker> after.</p>",
                "representation": "storage"
            }
        },
        "version": { "number": version }
    })
}
```

### Pattern 3: Explicit DiagramConfig construction (replacement for ::default())

**What:** Every deleted `DiagramConfig::default()` / `MarkdownConverter::default()` call is replaced with a minimal inline literal.

**Example (replacement for `src/converter/tests.rs:87` etc.):**
```rust
// Source: CONTEXT.md D-04 explicit replacement spec
let config = crate::config::DiagramConfig {
    plantuml_path: "plantuml".to_string(),
    mermaid_path: "mmdc".to_string(),
    mermaid_puppeteer_config: None,
    timeout_secs: 30,
};
let converter = crate::converter::MarkdownConverter::new(config);
```

**Note on DRY:** The planner may wish to factor the replacement literal into a private `fn test_diagram_config() -> DiagramConfig` helper inside the `#[cfg(test)] mod tests` block — this is already the pattern used in `src/converter/diagrams.rs` where a helper `config_with_defaults()` already exists (visible at `src/converter/diagrams.rs:228-230`). Reuse or extend that helper instead of scattering seven copies of the literal.

### Pattern 4: ANTHROPIC_BASE_URL env-var read in `AnthropicClient::new`

**What:** Small constructor change so that `AnthropicClient::new(api_key, model)` consults `ANTHROPIC_BASE_URL` before falling back to the hardcoded production URL.

**Example:**
```rust
// Replacement for src/llm/mod.rs:50-57
pub fn new(api_key: String, model: String) -> Self {
    let endpoint = std::env::var("ANTHROPIC_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());
    Self::with_endpoint(api_key, model, endpoint)
}
```

**Integration-test call site:** Setting `.env("ANTHROPIC_BASE_URL", anthropic_mock.uri())` on the `assert_cmd::Command` is now sufficient — no code changes in `lib.rs::run` needed. `with_endpoint()` remains public for existing unit tests in `tests/llm_integration.rs` to use directly.

### Pattern 5: `#[serial]` for env-var-mutating tests

**What:** Any test that sets or removes a process env var must run serially to avoid races with other tests. `serial_test` already pinned in dev-deps.

**Example:**
```rust
// Source: tests/cli_integration.rs:425-477 (test_convert_with_env_var_diagram_paths)
use serial_test::serial;

#[test]
#[serial]
fn test_update_command_happy_path() { /* sets ANTHROPIC_BASE_URL via .env() */ }
```

**Note:** `assert_cmd::Command::env()` sets env vars on the *child* process, not the parent. However, `serial_test::serial` is still recommended because (a) it serializes against other `#[serial]` tests that might read the same var, and (b) the existing convention in this codebase is to use it defensively. [VERIFIED: precedent in `tests/cli_integration.rs:425`]

### Anti-Patterns to Avoid

- **Hand-rolling an HTTP listener with tokio + hyper.** Wiremock is already in the stack; re-implementing would be pointless complexity and diverge from the established `src/confluence/client.rs` mod-test pattern.
- **Building a new integration-test file (e.g., `tests/update_happy_path.rs`).** Each additional Rust integration test file is a separate test binary that re-compiles — cumulative build time. Add to `tests/cli_integration.rs`.
- **Using `std::env::set_var` at test top-level.** Mutating parent-process env leaks to sibling tests. Always use `assert_cmd::Command::env(key, value)` to scope the mutation to the child.
- **Mocking the LLM by path-matching the production URL.** Mount on `path("/")` or don't filter the path — wiremock sees whatever path `endpoint` implies. The existing `tests/llm_integration.rs::test_sends_correct_headers` uses `path("/")` because the full URL is passed as the endpoint. Follow that convention.
- **Un-ignoring the test without fixing the body.** The current `test_upload_command_happy_path` (`tests/cli_integration.rs:333-336`) has an empty body — un-ignoring alone would make it a no-op pass. Replace the body entirely.
- **Leaving a stale comment.** `test_upload_command_happy_path`'s `#[ignore]` reason string references the old wiremock/TLS constraint (`"happy-path requires https:// server; wiremock is http-only"`). Delete it when un-ignoring.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP mock server | Custom tokio listener | `wiremock::MockServer` | Already in dev-deps; existing tests use it. |
| Binary path resolution | Hardcoded `target/debug/confluence-agent` | `assert_cmd::Command::cargo_bin("confluence-agent")` | Handles debug/release, cross-platform, cargo target dir overrides. |
| Env-var test isolation | Manual save/restore of `std::env::var` | `#[serial]` + `Command::env` | Avoids leaking to sibling tests; Command::env scopes to child process only. |
| Temp file / dir | `std::fs::write("/tmp/foo")` | `tempfile::TempDir` / `temp_markdown()` helper | Auto-cleanup on drop; avoids collisions. |
| Test-scope `DiagramConfig` | Seven literal copies | Single `fn test_diagram_config() -> DiagramConfig` helper (or reuse existing `config_with_defaults`) | DRY; see `src/converter/diagrams.rs:228`. |

**Key insight:** Every piece of this phase already has a precedent somewhere in the repo. The research task is to *locate* the precedent, not to *design* a solution.

## Runtime State Inventory

> This phase is a refactor (remove dead code) + test addition. State inventory is included for the `DiagramConfig` removal to catch anything the grep audit misses.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — `DiagramConfig::from_env()` is a constructor function, not a persistent key. No databases, caches, or serialized state reference it. | None. Verified by absence of `DiagramConfig` in any `serde`-derived struct used for on-disk formats. |
| Live service config | None — no external services reference `DiagramConfig::from_env()` or `::default()`. It is pure in-process construction. | None. |
| OS-registered state | None — no systemd/launchd/Task Scheduler entries reference these symbols. | None. |
| Secrets / env vars | `PLANTUML_PATH`, `MERMAID_PATH`, `MERMAID_PUPPETEER_CONFIG`, `DIAGRAM_TIMEOUT` are read by `DiagramConfig::from_env()`. After removal, these env vars are still read by (a) clap-derive via `#[arg(long, env = "PLANTUML_PATH")]` on `Cli::plantuml_path` / `Cli::mermaid_path`, and (b) `Config::load_with_home` (`src/config.rs:138-142`) for the puppeteer / timeout fields, and (c) the Convert arm inline construction in `src/lib.rs:220-224`. **No env-var contract is broken.** [VERIFIED: grep of `PLANTUML_PATH`, `MERMAID_PATH`, `DIAGRAM_TIMEOUT`, `MERMAID_PUPPETEER_CONFIG` in src/ — all have live readers after `from_env` removal] | The `test_diagram_config_from_env` test in `src/converter/diagrams.rs:182-224` directly tests `DiagramConfig::from_env()`. **Decision required by planner:** Either (a) delete the test entirely (the function it tests is being removed), or (b) replace with a test that sets env vars then invokes `Config::load_with_home` to verify end-to-end env-var coverage. Option (a) is simpler and the env-var tier is already covered by `tests/cli_integration.rs::test_convert_with_env_var_diagram_paths`. **Recommend option (a).** |
| Build artifacts / installed packages | `target/package/confluence-agent-0.1.0/src/config.rs` and `target/package/confluence-agent-0.1.0/src/converter/mod.rs` contain stale copies of the old code. These are regenerated by `cargo package` / `cargo publish`; they are not part of the source tree. | None — cargo regenerates on next `package`/`publish`. Ignore. |

**Canonical question — answered:** After every file is updated, does any runtime system still depend on `DiagramConfig::from_env()` or `impl Default for DiagramConfig`? **No.** The four PLANTUML/MERMAID/DIAGRAM_* env vars remain read by other code paths (clap, Config::load_with_home, Commands::Convert arm) — the env-var *contract* survives; only the helper function that also consumed them is removed.

## Common Pitfalls

### Pitfall 1: `extract_page_id` regex matches localhost page URLs

**What goes wrong:** The test passes `http://localhost:19999/wiki/spaces/TEST/pages/12345/Title` as the page URL and expects `extract_page_id` to return `"12345"`.

**Why it's fine:** `extract_page_id` at `src/confluence/url.rs:27-38` uses regex `"/pages/(\d+)"` — it matches any URL containing `/pages/{number}/`, regardless of scheme or host. [VERIFIED: `src/confluence/url.rs:44-49` test_extract_page_id_slash_pages uses a relative path with no scheme and passes.]

**Action:** No fix needed; test URL `http://localhost:{port}/wiki/spaces/TEST/pages/12345/Title` works as-is.

### Pitfall 2: The Confluence URL in CLI `--confluence-url` vs the `page_url` positional

These are separate arguments. `--confluence-url` goes into `Config::confluence_url` and becomes `ConfluenceClient::base_url`. `page_url` is the positional argument only parsed by `extract_page_id` for the ID. Both point to the same host in practice, but they are decoupled in the code. The test must pass **both** to the binary.

**Warning signs:** A test where `--confluence-url=http://localhost:{port}` is set but `page_url` points to a different host would still extract the page ID correctly but the API calls would go to localhost. This is fine — it's what wiremock expects. Just don't confuse the two flags.

### Pitfall 3: Wiremock ignores path in request if endpoint URL ends without `/v1/messages`

**What goes wrong:** `AnthropicClient` POSTs to `self.endpoint` literally (no path appending). If `ANTHROPIC_BASE_URL=http://localhost:{port}` (no `/v1/messages` suffix), wiremock receives the request at `/`, not `/v1/messages`. If your `Mock::given(...).and(path("/v1/messages"))` expects `/v1/messages`, the request won't match.

**Why it happens:** `AnthropicClient::with_endpoint` uses `self.endpoint` directly in `self.client.post(&self.endpoint)` at `src/llm/mod.rs:99`. The endpoint IS the full URL.

**How to avoid:** Either (a) set `ANTHROPIC_BASE_URL=http://localhost:{port}/v1/messages` (include the path) and match on `path("/v1/messages")`, or (b) set `ANTHROPIC_BASE_URL=http://localhost:{port}` (no path) and match on `path("/")` like `tests/llm_integration.rs:59` does. **Option (b) is the existing convention.** [VERIFIED: tests/llm_integration.rs:54-80]

### Pitfall 4: Verbose or debug output leaking to stdout breaks the "tracing must not appear on stdout" assertions

**What goes wrong:** Phase 04 established D-07 — tracing output must go to stderr, never stdout. Existing tests assert `!stdout.contains("DEBUG")`, `!stdout.contains("INFO")`, `!stdout.contains("TRACE")`. New happy-path tests should include the same assertion to guard against regression.

**How to avoid:** Copy the existing assertion block from `test_convert_command` (lines 74-78). Invoke the binary without `--verbose`.

### Pitfall 5: `assert_cmd` swallows environment from the parent shell

**What goes wrong:** `assert_cmd::Command::cargo_bin` inherits the parent's environment by default. If a developer has `ANTHROPIC_API_KEY=real-key` in their shell, a happy-path test that *intended* to use a fake key might silently leak and hit real Anthropic.

**How to avoid:** Use `.env_remove("ANTHROPIC_API_KEY")` to scrub the inherited value, then `.env("ANTHROPIC_API_KEY", "fake-for-test")` to set the intended test value. Do the same for all `CONFLUENCE_*` vars. This is already the pattern in `test_convert_command`. See lines 36-38, 264-266, and 432-437 for the established scrub list.

### Pitfall 6: `serial_test` conflict with parallel `cargo test`

**What goes wrong:** `#[serial]` serializes only tests sharing the same serial-key domain. Default `#[serial]` uses the same implicit key across the whole crate, so env-mutating tests will serialize. But unit tests in `src/config.rs` also use `#[serial]`. Since unit tests and integration tests run as separate binaries, they don't actually serialize against each other. Both binaries read `ANTHROPIC_BASE_URL` concurrently if both are running.

**Why it matters for this phase:** `tests/cli_integration.rs` happy-path tests set `ANTHROPIC_BASE_URL` via `Command::env` (child-scoped — no collision with parent-scoped tests). No direct collision expected. However: `src/config.rs` tests also use `#[serial]` but mutate `std::env::var` in the parent. If a cargo-test schedule interleaves, mutations to `ANTHROPIC_API_KEY` in `src/config.rs` tests don't affect the integration-test children.

**How to avoid:** Use `.env()` / `.env_remove()` on `assert_cmd::Command`, never `std::env::set_var` in integration tests. [VERIFIED: pattern at tests/cli_integration.rs:432-437]

### Pitfall 7: Removing `impl Default for MarkdownConverter` may break external consumers if the crate is published

**What goes wrong:** `MarkdownConverter` is `pub` (via `pub use` or `pub struct`). Removing `impl Default for MarkdownConverter` is a breaking change to the public API — but crate version is 0.1.0 (pre-1.0), so the convention allows breaking changes on minor version bumps.

**Why it's OK:** `Cargo.toml` line 3 is `version = "0.1.0"` — semver 0.x allows breaking changes. The crate has no downstream consumers yet (DIST-01 not satisfied — binary hasn't been published). [VERIFIED: `.planning/REQUIREMENTS.md` lines 128-131 — DIST-01..04 Pending]

**Action:** Proceed with removal. Note for Phase 5 (DIST): once the crate publishes to crates.io, public API removals will require version bumps.

## Code Examples

### Full `test_update_command_happy_path` skeleton

```rust
// Source: composition of src/confluence/client.rs mod tests +
//         tests/llm_integration.rs + tests/cli_integration.rs existing patterns.
//         See CONTEXT.md D-02 and D-03 for test spec.

use assert_cmd::Command;
use serde_json::json;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn page_json_with_comment(id: &str, version: u32) -> serde_json::Value {
    json!({
        "id": id,
        "title": "Happy Path Test Page",
        "body": {
            "storage": {
                "value": "<p>Before <ac:inline-comment-marker ac:ref=\"abc-123\">important</ac:inline-comment-marker> after.</p>",
                "representation": "storage"
            }
        },
        "version": { "number": version }
    })
}

fn anthropic_tool_use_keep_response() -> serde_json::Value {
    // Source: tests/llm_integration.rs::tool_use_response — copy verbatim
    json!({
        "id": "msg_test",
        "model": "claude-haiku-4-5-20251001",
        "stop_reason": "tool_use",
        "content": [
            {
                "type": "tool_use",
                "id": "toolu_test",
                "name": "evaluate_comment",
                "input": { "decision": "KEEP" }
            }
        ]
    })
}

fn temp_markdown(content: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let md_path = dir.path().join("doc.md");
    fs::write(&md_path, content).expect("write temp markdown");
    (dir, md_path)
}

#[tokio::test]
#[serial]
async fn test_update_command_happy_path() {
    // --- Arrange Confluence wiremock ---
    let confluence = MockServer::start().await;
    let page_id = "12345";

    Mock::given(method("GET"))
        .and(path(format!("/rest/api/content/{page_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_json_with_comment(page_id, 7)))
        .mount(&confluence)
        .await;

    Mock::given(method("PUT"))
        .and(path(format!("/rest/api/content/{page_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_json_with_comment(page_id, 8)))
        .mount(&confluence)
        .await;

    // --- Arrange Anthropic wiremock ---
    let anthropic = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(anthropic_tool_use_keep_response()))
        .mount(&anthropic)
        .await;

    // --- Arrange test inputs ---
    let (md_dir, md_path) = temp_markdown("# New Content\n\nHello, world.\n");
    let page_url = format!("{}/wiki/spaces/TEST/pages/{page_id}/Title", confluence.uri());

    // --- Act: spawn the binary ---
    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    cmd.arg("--confluence-url")
        .arg(confluence.uri())
        .arg("--confluence-username")
        .arg("user@example.com")
        .arg("--confluence-token")
        .arg("fake-token")
        .arg("--anthropic-api-key")
        .arg("fake-anthropic-key")
        .arg("update")
        .arg(&md_path)
        .arg(&page_url)
        .env("ANTHROPIC_BASE_URL", anthropic.uri())
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN");

    let output = cmd.output().expect("run command");

    // --- Assert ---
    assert!(
        output.status.success(),
        "update happy path should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated page:"), "stdout should contain 'Updated page:'; got: {stdout}");
    assert!(
        !stdout.contains("DEBUG") && !stdout.contains("INFO") && !stdout.contains("TRACE"),
        "tracing must not appear on stdout; got: {stdout}"
    );

    // Verify LLM was hit at least once
    let llm_requests = anthropic.received_requests().await.unwrap();
    assert!(!llm_requests.is_empty(), "LLM should have been called for the inline comment");

    drop(md_dir);
}
```

### `test_upload_command_happy_path` skeleton (rewrite of existing `#[ignore]`)

```rust
// Source: un-ignores and fills the body of tests/cli_integration.rs:332-336.
// No Anthropic mock — upload bypasses the LLM merge pipeline.

#[tokio::test]
#[serial]
async fn test_upload_command_happy_path() {
    let confluence = MockServer::start().await;
    let page_id = "54321";

    Mock::given(method("GET"))
        .and(path(format!("/rest/api/content/{page_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_json("Upload Test", page_id, 3)))
        .mount(&confluence)
        .await;

    Mock::given(method("PUT"))
        .and(path(format!("/rest/api/content/{page_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_json("Upload Test", page_id, 4)))
        .mount(&confluence)
        .await;

    let (md_dir, md_path) = temp_markdown("# Upload Test\n\nContent.\n");
    let page_url = format!("{}/wiki/spaces/TEST/pages/{page_id}/Title", confluence.uri());

    let mut cmd = Command::cargo_bin("confluence-agent").expect("binary exists");
    cmd.arg("--confluence-url")
        .arg(confluence.uri())
        .arg("--confluence-username")
        .arg("user@example.com")
        .arg("--confluence-token")
        .arg("fake-token")
        .arg("upload")
        .arg(&md_path)
        .arg(&page_url)
        .env_remove("CONFLUENCE_URL")
        .env_remove("CONFLUENCE_USERNAME")
        .env_remove("CONFLUENCE_API_TOKEN")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("ANTHROPIC_BASE_URL");

    let output = cmd.output().expect("run command");

    assert!(
        output.status.success(),
        "upload happy path should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Uploaded to:"), "stdout should contain 'Uploaded to:'; got: {stdout}");
    assert!(
        !stdout.contains("DEBUG") && !stdout.contains("INFO") && !stdout.contains("TRACE"),
        "tracing must not appear on stdout; got: {stdout}"
    );

    drop(md_dir);
}
```

### D-01 localhost exemption (`src/config.rs::load_with_home`)

```rust
// Replacement for src/config.rs:87-93
// Threat model T-01-04: validate scheme to prevent accidental HTTP use,
// but allow http://localhost and http://127.0.0.1 for integration testing (D-01).
let url_lower = confluence_url.to_ascii_lowercase();
if !url_lower.starts_with("https://")
    && !url_lower.starts_with("http://localhost")
    && !url_lower.starts_with("http://127.0.0.1")
{
    return Err(ConfigError::Invalid {
        name: "CONFLUENCE_URL",
        reason: "must start with https:// (or http://localhost for testing)",
    });
}
```

### D-01 test amendment (`src/config.rs::test_confluence_url_must_be_https`)

```rust
// Amend test at src/config.rs:476-496 to cover BOTH the rejection and the localhost exemption.
#[test]
fn test_confluence_url_must_be_https() {
    // Non-localhost http is still rejected
    let cli = Cli {
        confluence_url: Some("http://example.atlassian.net".to_string()),
        confluence_username: Some("user@example.com".to_string()),
        confluence_token: Some("token".to_string()),
        ..cli_blank()
    };
    let err = Config::load_with_home(&cli, Some(&no_home())).expect_err("should reject non-localhost http URL");
    assert!(matches!(err, ConfigError::Invalid { name: "CONFLUENCE_URL", .. }));
}

#[test]
fn test_confluence_url_localhost_exemption() {
    // http://localhost and http://127.0.0.1 are accepted for integration testing (D-01)
    for url in ["http://localhost:8080", "http://127.0.0.1:9999", "HTTP://LOCALHOST:1234"] {
        let cli = Cli {
            confluence_url: Some(url.to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token".to_string()),
            ..cli_blank()
        };
        let cfg = Config::load_with_home(&cli, Some(&no_home()))
            .unwrap_or_else(|e| panic!("{url} should be allowed, got: {e:?}"));
        assert_eq!(cfg.confluence_url.to_ascii_lowercase(), url.to_ascii_lowercase());
    }
}
```

### D-03 `ANTHROPIC_BASE_URL` read (`src/llm/mod.rs::new`)

```rust
// Replacement for src/llm/mod.rs:50-57
pub fn new(api_key: String, model: String) -> Self {
    let endpoint = std::env::var("ANTHROPIC_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());
    Self::with_endpoint(api_key, model, endpoint)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single `impl Default` funnel + `from_env()` helper for `DiagramConfig` | Explicit struct construction at each call site | Phase 10 (this phase) | Six lines longer per test but eliminates dead-code warnings; removes public API surface that had no production callers. |
| `test_upload_command_happy_path` `#[ignore]` with TLS justification | Localhost-exempted Config + wiremock | Phase 10 (this phase) | CLI-02 fully covered; test count +1 non-ignored. |
| No integration coverage for `update` command | Dual-wiremock `test_update_command_happy_path` | Phase 10 (this phase) | CLI-01 fully covered; proves end-to-end pipeline convert → fetch → merge (with LLM) → attachment upload → PUT. |
| `AnthropicClient` endpoint pinned to prod URL in `new()` | `ANTHROPIC_BASE_URL` env override | Phase 10 (this phase) | Tests no longer need to call `with_endpoint` manually; `AnthropicClient::new()` respects test config. |

**Deprecated/outdated:**
- `DiagramConfig::from_env()` — removed in Phase 10.
- `impl Default for DiagramConfig` — removed in Phase 10.
- `impl Default for MarkdownConverter` — removed in Phase 10.
- `src/converter/diagrams.rs::tests::test_diagram_config_from_env` — to be deleted (tests a removed function; env-var coverage exists in `tests/cli_integration.rs::test_convert_with_env_var_diagram_paths`).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Removing `test_diagram_config_from_env` (src/converter/diagrams.rs:182-224) is acceptable because its coverage is replaced by `test_convert_with_env_var_diagram_paths` at tests/cli_integration.rs:425. | Runtime State Inventory (Secrets/env vars row) | If reviewer disagrees, the test must be rewritten to exercise the env-var tier via `Config::load_with_home` instead. Discuss-phase can clarify. |
| A2 | The planner should use a single helper function like `fn test_diagram_config() -> DiagramConfig` rather than copy-paste the literal 7 times. | Pattern 3 | Low risk — aesthetic. If planner prefers copies, it still works. |
| A3 | Both new happy-path tests should include the D-07 stdout-tracing-free assertion. | Code Examples, Common Pitfalls 4 | Low risk — if skipped, a future tracing misconfiguration regression wouldn't be caught by this test, but other tests cover it. |
| A4 | The `update` happy-path test should include an inline comment marker in the old page body to actually exercise the LLM path, not short-circuit. | Pattern 2 | If planner chooses the no-LLM variant, the `ANTHROPIC_BASE_URL` wiring is still validated by constructor code but not by an end-to-end request. Medium risk — reduces test value. |

## Open Questions

1. **Should `test_upload_command_happy_path` be un-ignored in place, or renamed?**
   - What we know: CONTEXT.md D-02 leaves this to Claude's discretion. The current name is accurate and matches CLI-02 traceability.
   - What's unclear: Whether renaming (e.g., to `test_upload_command_happy_path_wiremock`) adds value.
   - Recommendation: Keep the existing name. Simpler diff; matches the REQUIREMENTS table row verbatim.

2. **Two `update` tests (with/without LLM) or one (with LLM)?**
   - What we know: The merge pipeline short-circuits when the old page body has no `<ac:inline-comment-marker>`. A no-comment test would not hit wiremock #2.
   - What's unclear: Whether both variants are worth the test runtime.
   - Recommendation: One test with LLM path exercised. If planner wants defense-in-depth, add a second test — but only as a follow-up task, not as phase blocking.

3. **Should the phase also fix the `ANTHROPIC_MODEL` env-var bug (WR-01 from Phase 09 review)?**
   - What we know: Phase 09 review flagged that `ANTHROPIC_MODEL` env var is silently ignored (`src/config.rs:118-123` — `resolve_optional(None, "ANTHROPIC_MODEL", home)` never consults `std::env::var`).
   - What's unclear: CONTEXT.md does not mention this.
   - Recommendation: **Out of scope for Phase 10.** Track for a future phase. Surfacing it now risks scope creep. If the planner flags it, either (a) defer or (b) add explicit CONTEXT.md addendum.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | Build and test | ✓ | (rustc 1.80+ per Cargo.toml) | — |
| `wiremock` crate | Integration tests | ✓ | 0.6.5 via Cargo.toml dev-deps | — |
| `assert_cmd` crate | Integration tests | ✓ | 2.2.1 via Cargo.toml dev-deps | — |
| `serial_test` crate | Integration tests | ✓ | 3.4.0 via Cargo.toml dev-deps | — |
| `tempfile` crate | Test fixtures | ✓ | 3.x via Cargo.toml dependencies | — |
| Internet access | — | N/A | — | Tests use localhost wiremock; no outbound required. |
| `plantuml` binary | `src/converter/tests.rs::test_plantuml_rendering_integration` | Unknown | — | Existing test auto-skips when binary absent. Not a Phase 10 blocker. |
| `mmdc` binary | `src/converter/tests.rs::test_mermaid_rendering_integration` | Unknown | — | Existing test auto-skips when binary absent. Not a Phase 10 blocker. |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** Diagram binaries (plantuml, mmdc) — existing tests handle absence gracefully; Phase 10 does not introduce new diagram-rendering test requirements.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `#[tokio::test]` (via `tokio = "1.51"` dev features) |
| Config file | None (cargo-managed) |
| Quick run command | `cargo test --test cli_integration` — runs just the integration test binary (~5 s after warm build) |
| Full suite command | `cargo test --all-targets` — runs unit tests, integration tests, doc tests |
| Linter (parallel) | `cargo build` (must emit zero warnings per D-04); `cargo clippy --all-targets --all-features -- -D warnings` (recommended not required) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| CLI-01 | `update` command happy path (convert → fetch → merge → upload, with LLM) | integration | `cargo test --test cli_integration test_update_command_happy_path` | ❌ Wave 0 (new) |
| CLI-02 | `upload` command happy path (convert → fetch → upload, no LLM) | integration | `cargo test --test cli_integration test_upload_command_happy_path` | ❌ Wave 0 (rewrite existing `#[ignore]` body) |
| D-01 | Localhost exemption accepted; arbitrary http rejected | unit | `cargo test --lib test_confluence_url_localhost_exemption test_confluence_url_must_be_https` | ❌ Wave 0 (new + amend) |
| D-03 | `AnthropicClient::new` respects `ANTHROPIC_BASE_URL` | unit / integration | Covered transitively by `test_update_command_happy_path` (the binary wouldn't hit wiremock without it) | ❌ Wave 0 (implicit coverage) |
| D-04 | `DiagramConfig::from_env` / `Default` / `MarkdownConverter::default` removed | build / compile check | `cargo build --release 2>&1 | grep -E "warning|error"` — must be empty | ✅ (enforced by compile) |
| D-04 | All prior callers rewritten to compile | unit | `cargo test --lib` (converter unit tests) | ✅ (existing tests must still pass) |

### Sampling Rate

- **Per task commit:** `cargo test --test cli_integration` (fastest meaningful quick-run — builds binary once, then runs integration tests)
- **Per wave merge:** `cargo test --all-targets` + `cargo build --release` (must produce zero warnings)
- **Phase gate:** Full suite green; no `#[ignore]` added by this phase; `cargo test` reports all CLI-01 / CLI-02 tests as passing.

### Wave 0 Gaps

- [ ] Test helpers — may need a shared `page_json(title, id, version)` helper in `tests/cli_integration.rs` to avoid duplication between update/upload tests (currently only `src/confluence/client.rs` mod tests have one, and those aren't visible to integration tests).
- [ ] Test helpers — `anthropic_tool_use_keep_response()` helper should be local to `tests/cli_integration.rs` or, better, factored into a shared `tests/common/mod.rs` module if reused across `tests/llm_integration.rs` and the new happy-path test.
- [ ] New test `test_confluence_url_localhost_exemption` added to `src/config.rs::tests`.
- [ ] Amended test `test_confluence_url_must_be_https` updated to explicitly assert non-localhost rejection (remove ambiguity).

*(Decision note:* a shared `tests/common/mod.rs` would require `mod common;` declarations in both integration test files and a `#![allow(dead_code)]` on the module. For two test files this is marginal; the planner may skip this refactor and accept a small amount of duplication.)

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (Confluence Basic Auth) | Already implemented in `ConfluenceClient::new` via base64-encoded user:token. No new auth code in Phase 10. |
| V3 Session Management | no | Stateless HTTP; no sessions. |
| V4 Access Control | no | CLI tool runs in user context; no access-control tier. |
| V5 Input Validation | yes (URL validation) | Existing https-guard; D-01 adds a narrow localhost exemption. See Threat Patterns table. |
| V6 Cryptography | yes (TLS via rustls) | `reqwest = { features = ["rustls"] }` — unchanged. |
| V7 Error Handling | yes | `thiserror`-based error types; D-04 does not change error paths. |
| V8 Data Protection | yes | API keys logged nowhere (verified in `tests/llm_integration.rs::test_api_key_not_in_debug_output`). Phase 10 adds no new logging. |

### Known Threat Patterns for this Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| T-01-04: Accidental http:// leak of API tokens | Information Disclosure | Config-level https guard (`src/config.rs::load_with_home`) rejects non-https URLs. D-01 narrowly exempts `http://localhost` and `http://127.0.0.1` for local-test use only; exemption is string-prefix-matched against lowercased URL and cannot match any external host. Threat invariant preserved — see `src/config.rs` lines 86-93 after edit. [VERIFIED: regex/prefix semantics; localhost resolves to the loopback interface in all mainstream OS configurations] |
| T-10-01 (new for this phase): Overly broad localhost exemption | Elevation of Privilege | Exemption MUST match ONLY `http://localhost` and `http://127.0.0.1` (with optional port). Do NOT use a looser regex like `http://*.localhost` or allow `http://0.0.0.0`. Verified by the amended test `test_confluence_url_must_be_https` which retains the rejection of arbitrary http URLs. |
| T-10-02 (new for this phase): ANTHROPIC_BASE_URL SSRF | Information Disclosure | An attacker with ability to set `ANTHROPIC_BASE_URL` on the process environment can redirect API calls (and thus the API key) to a malicious endpoint. **Acceptable risk:** the env var is a test-infrastructure affordance documented in CONTEXT.md D-03; an attacker who can set arbitrary env vars on the process already has sufficient control to exfiltrate the key by other means (e.g., reading `~/.claude/settings.json`). No additional mitigation required; do NOT log the resolved endpoint at WARN or higher level (only DEBUG is acceptable, and current code uses tracing::debug in the HTTP path). |
| T-10-03: Test credentials leak to prod hosts via test-env pollution | Information Disclosure | Integration tests use `Command::env_remove` to scrub inherited `CONFLUENCE_*` / `ANTHROPIC_*` env vars before setting test values; test creds are `"fake-token"` / `"fake-key"` literals, never real tokens. [VERIFIED: existing pattern at tests/cli_integration.rs:232-236] |

## Sources

### Primary (HIGH confidence)

- `src/config.rs` lines 22-44 — DiagramConfig::from_env() and Default impl targeted for removal [file read 2026-04-20]
- `src/config.rs` lines 85-93 — https guard location (single change point for D-01) [file read 2026-04-20]
- `src/config.rs` lines 475-496 — `test_confluence_url_must_be_https` to amend [file read 2026-04-20]
- `src/converter/mod.rs` lines 47-51 — `impl Default for MarkdownConverter` to remove [file read 2026-04-20]
- `src/converter/tests.rs` lines 87, 99, 107, 124, 142, 165, 198 — 7 call sites needing rewrite [file read 2026-04-20]
- `src/converter/diagrams.rs` lines 182-224 (`test_diagram_config_from_env`) — candidate for deletion [file read 2026-04-20]
- `src/llm/mod.rs` lines 50-57 (`AnthropicClient::new`) — D-03 change point; `with_endpoint` already exists at lines 60-83 [file read 2026-04-20]
- `src/lib.rs` lines 115-120, 160-200 — production call sites of `AnthropicClient::new` and `ConfluenceClient::new` used during `update`/`upload` [file read 2026-04-20]
- `src/confluence/client.rs` mod tests lines 198-394 — wiremock Confluence pattern (the template to follow) [file read 2026-04-20]
- `src/confluence/url.rs` — page-ID extraction works for any URL containing `/pages/{n}/` regardless of host [file read 2026-04-20]
- `tests/cli_integration.rs` lines 1-477 — existing CLI integration test patterns, including the `#[ignore]` upload test at line 332 to rewrite [file read 2026-04-20]
- `tests/llm_integration.rs` lines 1-436 — wiremock Anthropic pattern, including `tool_use_response` helper to copy [file read 2026-04-20]
- `src/merge/mod.rs` lines 52-92 — MERGE-06 short-circuit behaviour (determines whether the LLM mock needs to fire) [file read 2026-04-20]
- `Cargo.toml` — dev-dependency versions confirming wiremock 0.6, assert_cmd 2, serial_test 3.4.0, tempfile 3 [file read 2026-04-20]
- `.planning/phases/10-tech-debt-integration-test-coverage-and-api-cleanup/10-CONTEXT.md` — authoritative decisions D-01 through D-04 [file read 2026-04-20]
- `.planning/phases/09-convert-waterfall-fix-and-phase-08-verification/09-REVIEW.md` — IN-01 background for DiagramConfig dead-code [file read 2026-04-20]
- `.planning/REQUIREMENTS.md` — CLI-01 and CLI-02 traceability rows (lines 123-124) [file read 2026-04-20]

### Secondary (MEDIUM confidence)

- `cargo search` output 2026-04-20 — wiremock 0.6.5, assert_cmd 2.2.1, serial_test 3.4.0 latest versions on crates.io.

### Tertiary (LOW confidence)

- None — all claims are verified against the codebase or against CONTEXT.md decisions.

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — all dev-deps already in `Cargo.toml`; no version research needed.
- Architecture patterns: HIGH — every pattern has an in-repo precedent (wiremock Confluence in `src/confluence/client.rs`; wiremock Anthropic in `tests/llm_integration.rs`; binary invocation in `tests/cli_integration.rs`).
- Pitfalls: HIGH — each pitfall is either a direct read of the current code or a documented Phase-09/CONTEXT.md note.
- Security: HIGH — threat model T-01-04 already documented in code; T-10-01 and T-10-02 are first-order consequences of D-01 and D-03 with bounded impact.
- Runtime State Inventory: HIGH — verified by grep that all env-var contracts survive `DiagramConfig::from_env` removal.

**Research date:** 2026-04-20
**Valid until:** 2026-05-20 (30 days — stable stack, closed-scope phase)

---

*Phase: 10-tech-debt-integration-test-coverage-and-api-cleanup*
*Researched: 2026-04-20*
