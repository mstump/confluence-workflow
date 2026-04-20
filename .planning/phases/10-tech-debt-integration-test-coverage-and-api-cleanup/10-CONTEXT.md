# Phase 10: Tech Debt — Integration Test Coverage and API Cleanup — Context

**Gathered:** 2026-04-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Two targeted tech-debt closures from the v1.0 audit:

1. **Integration test coverage** — Add passing happy-path integration tests for the `update` and `upload` commands (CLI-01, CLI-02). Currently `test_upload_command_happy_path` is `#[ignore]` due to the https constraint; no happy-path test exists for `update`. Tests exercise the full binary path (convert → fetch → merge → upload pipeline).

2. **API cleanup** — Remove the dead `DiagramConfig::from_env()` public function and `impl Default for DiagramConfig`. No production callers remain after Phase 09; both are tech debt noted in the Phase 09 code review (IN-01).

This phase does NOT add new features or change user-visible behavior.

</domain>

<decisions>
## Implementation Decisions

### https Constraint Resolution (D-01)

- **D-01:** **Localhost exemption in Config::load_with_home.** The https guard (`must start with https://`) is relaxed to allow `http://localhost` and `http://127.0.0.1` URLs. This enables wiremock-based integration tests (which bind to `http://localhost:PORT`) to pass through `Config::load` without triggering the security error.

  Change is ~2 lines in `src/config.rs load_with_home`:
  ```rust
  let url_lower = confluence_url.to_ascii_lowercase();
  if !url_lower.starts_with("https://")
     && !url_lower.starts_with("http://localhost")
     && !url_lower.starts_with("http://127.0.0.1") {
      return Err(ConfigError::Validation { ... });
  }
  ```

  The `test_confluence_url_must_be_https` unit test must be updated to reflect the exemption (or a new complementary test added verifying localhost is allowed). The security invariant — rejecting non-localhost http URLs — is fully preserved.

### Update + Upload Test Mock Approach (D-02)

- **D-02:** **Wiremock for both Confluence and Anthropic.** The happy-path integration tests spin up two wiremock `MockServer` instances:
  - One for Confluence API calls (`GET /rest/api/content/{id}`, `PUT /rest/api/content/{id}`, attachment upload)
  - One for Anthropic API calls (`POST /v1/messages` returning a tool_use KEEP response)

  The binary is invoked via `assert_cmd::Command::cargo_bin("confluence-agent")` with:
  - `--confluence-url http://localhost:{confluence_port}` (allowed by D-01 exemption)
  - `ANTHROPIC_API_KEY=fake-key` env var
  - `ANTHROPIC_BASE_URL=http://localhost:{anthropic_port}` env var (D-03)

### Anthropic Base URL Configurability (D-03)

- **D-03:** **`ANTHROPIC_BASE_URL` env var.** `AnthropicClient` reads `ANTHROPIC_BASE_URL` env var and uses it as the base URL when set, falling back to `https://api.anthropic.com/v1/messages`. The env var is read in `AnthropicClient::new()` (or a small constructor refactor) — no new CLI flag needed. Tests set this env var to point at a wiremock server.

  Note: `ANTHROPIC_BASE_URL` is a test-infrastructure concern, not a user-facing config. It does not need to be added to the CLI struct or `Config::load`.

### DiagramConfig Cleanup (D-04)

- **D-04:** **Remove `DiagramConfig::from_env()` and `impl Default for DiagramConfig` entirely.** Both are dead public API with no production callers. Removal cascade:
  - Delete `from_env()` function and `impl Default for DiagramConfig` from `src/config.rs`
  - Delete `impl Default for MarkdownConverter` from `src/converter/mod.rs` (it calls `DiagramConfig::default()`)
  - Update `src/converter/tests.rs` — replace `DiagramConfig::default()` and `MarkdownConverter::default()` with explicit construction:
    ```rust
    let config = DiagramConfig {
        plantuml_path: "plantuml".to_string(),
        mermaid_path: "mmdc".to_string(),
        mermaid_puppeteer_config: None,
        timeout_secs: 30,
    };
    ```
  - Update `src/converter/diagrams.rs` tests — replace `DiagramConfig::from_env()` with the same explicit construction
  - `cargo build` must compile clean with zero warnings after removal

### Claude's Discretion

- Exact wiremock stub shape for the Anthropic response (the tool_use KEEP format) — follow the existing `tests/llm_integration.rs` response fixtures
- Whether `test_upload_command_happy_path` is un-ignored and repurposed, or replaced by a new test with a clearer name
- Whether to add a `#[serial]` attribute on the new tests (env vars mutated: `ANTHROPIC_BASE_URL`)
- Exact error message text for the localhost-allowed exemption path in the existing https test

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Source Files (Phase 10 scope)
- `src/config.rs` — `Config::load_with_home`, the https guard (search: `starts_with("https://")`), and `DiagramConfig::from_env()`/`impl Default for DiagramConfig` (lines 22-43)
- `src/llm/mod.rs` — `AnthropicClient::new()` and the hardcoded `https://api.anthropic.com/v1/messages` URL (line ~55)
- `src/converter/mod.rs` — `impl Default for MarkdownConverter` (lines ~47-51)
- `src/converter/tests.rs` — all uses of `DiagramConfig::default()` and `MarkdownConverter::default()` (~lines 87, 99, 107, 124, 142, 165, 198)
- `src/converter/diagrams.rs` — all uses of `DiagramConfig::from_env()` in tests (~lines 195, 203, 209, 215)
- `tests/cli_integration.rs` — `test_upload_command_happy_path` (#[ignore], ~line 332); the wiremock pattern for adding new tests

### Reference Test Patterns
- `src/confluence/client.rs` (mod tests) — how wiremock MockServer is used for Confluence unit tests (the template for integration test mock setup)
- `tests/llm_integration.rs` — Anthropic response fixtures (tool_use format, KEEP/DROP shape); follow these for the Anthropic wiremock stub

### Requirements
- `.planning/REQUIREMENTS.md` — CLI-01 and CLI-02 traceability rows (gap closure)

### Prior Code Review Finding
- `.planning/phases/09-convert-waterfall-fix-and-phase-08-verification/09-REVIEW.md` — IN-01 (DiagramConfig dead code), WR-02 (DiagramConfig duplication) — these are the tech-debt items Phase 10 closes

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `MockConfluenceClient` in `src/confluence/mod.rs` — existing test mock, shows the ConfluenceApi trait shape
- `MockLlmClient` in `src/llm/mod.rs` — existing test mock for LLM responses
- `wiremock` crate (already in dev-dependencies) — `MockServer::start().await`, `Mock::given(method(...)).and(path(...)).respond_with(...).mount(...).await`
- `assert_cmd::Command::cargo_bin("confluence-agent")` — the binary invocation pattern used by all existing integration tests
- `serial_test::serial` attribute — already used in env-var-mutating tests; required for any test that sets `ANTHROPIC_BASE_URL`

### Established Patterns
- Integration tests in `tests/cli_integration.rs` use `temp_markdown()` helper + `TempDir` + `Command::cargo_bin(...)` + `.env(...)` / `.env_remove(...)`
- The Anthropic API response shape (tool_use KEEP) is documented in `tests/llm_integration.rs` fixtures — copy-paste that response JSON for the wiremock stub
- Config unit tests use `Config::load_with_home(cli, Some(&no_home()))` with a fake home path to avoid reading real credentials

### Integration Points
- `AnthropicClient::new(api_key, model)` — `api_key` and `model` come from `config.anthropic_api_key` and `config.anthropic_model` in `run()`; `ANTHROPIC_BASE_URL` env var will be read inside `AnthropicClient::new()` when set
- `Config::load_with_home` https guard (src/config.rs ~line 88) — single change point for D-01

</code_context>

<specifics>
## Specific Ideas

- The localhost exemption should specifically allow `http://localhost` and `http://127.0.0.1` — not any arbitrary http URL
- Wiremock-based integration tests should use `#[serial]` since they mutate `ANTHROPIC_BASE_URL` env var
- The `test_upload_command_happy_path` function can be repurposed (remove `#[ignore]`, add the wiremock setup) rather than creating a new function
- The existing `test_confluence_url_must_be_https` unit test needs a companion assertion that `http://localhost:8080` is now ACCEPTED (or the test is amended to verify the exemption)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 10-tech-debt-integration-test-coverage-and-api-cleanup*
*Context gathered: 2026-04-20*
