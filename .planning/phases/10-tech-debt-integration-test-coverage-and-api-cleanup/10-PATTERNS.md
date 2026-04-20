# Phase 10: Tech Debt — Integration Test Coverage and API Cleanup — Pattern Map

**Mapped:** 2026-04-20
**Files analyzed:** 7 (modifications; zero net-new files)
**Analogs found:** 7 / 7 (100%)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `tests/cli_integration.rs` (add `test_update_command_happy_path`, rewrite `test_upload_command_happy_path`) | integration-test | request-response (binary + HTTP mocks) | `tests/cli_integration.rs::test_convert_with_env_var_diagram_paths` (serial/env-var binary test) + `src/confluence/client.rs` mod tests (wiremock Confluence) + `tests/llm_integration.rs` (wiremock Anthropic) | exact (composition of three in-repo patterns) |
| `src/config.rs` (D-01: relax https guard, amend `test_confluence_url_must_be_https`, add `test_confluence_url_localhost_exemption`) | config / validation | transform | Existing guard at `src/config.rs:86-93` and existing test at `src/config.rs:475-496` | exact (in-place edit of the same pattern) |
| `src/config.rs` (D-04: delete `DiagramConfig::from_env` + `impl Default for DiagramConfig`) | config / constructor (dead code removal) | — (deletion) | `src/config.rs:22-44` itself (the code being deleted) | exact (surgical removal) |
| `src/llm/mod.rs` (D-03: `AnthropicClient::new` consults `ANTHROPIC_BASE_URL`) | http-client constructor | transform (env-var → endpoint string) | `src/llm/mod.rs::AnthropicClient::with_endpoint` (lines 60-83) — the existing test seam; and the env-var-first pattern at `src/config.rs::DiagramConfig::from_env` (lines 25-37) | exact (re-composes existing pieces) |
| `src/converter/mod.rs` (D-04: delete `impl Default for MarkdownConverter`) | converter / constructor (dead code removal) | — (deletion) | `src/converter/mod.rs:47-51` itself | exact (surgical removal) |
| `src/converter/tests.rs` (D-04: rewrite 7 call sites from `::default()` to explicit struct) | unit-test | — (refactor) | `src/converter/diagrams.rs::config_with_defaults` helper (lines 170-177) — established DRY helper for test-scope `DiagramConfig` | exact (reuse/extend existing helper pattern) |
| `src/converter/diagrams.rs` (D-04: delete `test_diagram_config_from_env`; keep `config_with_defaults` helper) | unit-test | — (deletion) | `src/converter/diagrams.rs:182-224` itself; existing callers at lines 229, 245 already use `config_with_defaults` | exact (surgical removal; other callers are already on the explicit-struct pattern) |

## Pattern Assignments

### `tests/cli_integration.rs::test_update_command_happy_path` (new integration test)

**Role:** integration-test  **Data Flow:** request-response (spawn binary; both Confluence and Anthropic mocked via wiremock)

**Analogs (composition of three):**
- **Binary invocation + env scrubbing pattern** — `tests/cli_integration.rs::test_convert_command` (lines 27-83) and `tests/cli_integration.rs::test_convert_with_env_var_diagram_paths` (lines 424-477)
- **Confluence wiremock stub pattern** — `src/confluence/client.rs` mod tests (lines 198-394)
- **Anthropic wiremock stub pattern** — `tests/llm_integration.rs` (lines 1-80)

**Imports pattern** — copy from `tests/cli_integration.rs:1-6` and extend with wiremock/tokio/serde_json/serial_test:
```rust
// Source: tests/cli_integration.rs:1-6 (existing) + additions from tests/llm_integration.rs:1-4
use assert_cmd::Command;
use serde_json::json;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
```

**Confluence mock pattern** — copy the `page_json` helper shape from `src/confluence/client.rs:204-216`:
```rust
// Source: src/confluence/client.rs:204-216 (adapted — title field + optional inline-comment-marker)
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
```

**Confluence Mock-mounting pattern** — copy from `src/confluence/client.rs:218-232` (GET 200) and `src/confluence/client.rs:263-279` (PUT 200):
```rust
// Source: src/confluence/client.rs:218-232 (GET 200) + src/confluence/client.rs:263-279 (PUT 200)
let confluence = MockServer::start().await;

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
```

**Anthropic mock-response pattern** — copy `tool_use_response` verbatim from `tests/llm_integration.rs:19-37`:
```rust
// Source: tests/llm_integration.rs:19-37 — KEEP variant with no reason
fn anthropic_tool_use_keep_response() -> serde_json::Value {
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
```

**Anthropic Mock-mounting pattern** — copy from `tests/llm_integration.rs:54-68` (match on `path("/")`, NOT `/v1/messages` — see Pitfall 3 in RESEARCH.md):
```rust
// Source: tests/llm_integration.rs:54-68 — endpoint is the full URL, so match on "/"
let anthropic = MockServer::start().await;
Mock::given(method("POST"))
    .and(path("/"))
    .respond_with(ResponseTemplate::new(200).set_body_json(anthropic_tool_use_keep_response()))
    .mount(&anthropic)
    .await;
```

**Binary spawn + env scrub pattern** — copy from `tests/cli_integration.rs:220-252` (`test_update_command_missing_api_key`, same flag set) and extend with `.env("ANTHROPIC_BASE_URL", anthropic.uri())`:
```rust
// Source: tests/cli_integration.rs:223-236 (arg-set) + tests/cli_integration.rs:264-266 (env-remove)
// + D-03 addition: .env("ANTHROPIC_BASE_URL", ...)
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
```

**Assertion pattern** — copy the success + stdout-substring + D-07 tracing-free block from `tests/cli_integration.rs:42-78`:
```rust
// Source: tests/cli_integration.rs:42-78
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
```

**Wiremock call-count verification pattern** — copy the `received_requests()` approach from `tests/llm_integration.rs:107-111`:
```rust
// Source: tests/llm_integration.rs:107-111 — proves the LLM endpoint was actually hit
let llm_requests = anthropic.received_requests().await.unwrap();
assert!(!llm_requests.is_empty(), "LLM should have been called for the inline comment");
```

**`#[serial]` attribute pattern** — copy from `tests/cli_integration.rs:425`:
```rust
// Source: tests/cli_integration.rs:425 — precedent for env-var-mutating test
#[tokio::test]
#[serial]
async fn test_update_command_happy_path() { /* ... */ }
```

---

### `tests/cli_integration.rs::test_upload_command_happy_path` (rewrite of `#[ignore]` stub)

**Role:** integration-test  **Data Flow:** request-response (binary + single wiremock for Confluence)

**Analog:** Same as `test_update_command_happy_path` above, minus the Anthropic mock (upload bypasses LLM merge). The existing stub at `tests/cli_integration.rs:331-336` (empty body, `#[ignore]`) is the site to edit in place.

**Pattern to replace** (`tests/cli_integration.rs:331-336`, current state):
```rust
#[test]
#[ignore = "happy-path requires https:// server; wiremock is http-only (T-01-04 constraint)"]
fn test_upload_command_happy_path() {
    // Would need: a TLS-capable mock Confluence server OR a real Confluence instance.
    // The unit tests in src/confluence/client.rs cover the HTTP layer via wiremock.
}
```

**Replacement pattern** — un-ignore, switch to `#[tokio::test] #[serial] async`, body reuses the same patterns as the update test (Confluence wiremock GET + PUT, binary spawn, assertions) but:
- No Anthropic wiremock server
- No `--anthropic-api-key` arg
- `.env_remove("ANTHROPIC_API_KEY")` and `.env_remove("ANTHROPIC_BASE_URL")` instead of `.env(...)`
- Assert `stdout.contains("Uploaded to:")` instead of `"Updated page:"`

Concrete pattern matches the skeleton already provided in `10-RESEARCH.md:509-561`. No new patterns introduced — pure composition of the update-test patterns above.

---

### `src/config.rs` D-01 — relax https guard for localhost

**Role:** config / validation  **Data Flow:** transform (string-prefix check)

**Analog (same file, in-place edit):** `src/config.rs:86-93` (the current strict guard)

**Existing guard to modify** (current state, lines 86-93):
```rust
// Source: src/config.rs:86-93 (current, strict)
// Threat model T-01-04: validate scheme to prevent accidental HTTP use.
// Use to_ascii_lowercase() so mixed-case inputs like "HTTPS://" are accepted.
if !confluence_url.to_ascii_lowercase().starts_with("https://") {
    return Err(ConfigError::Invalid {
        name: "CONFLUENCE_URL",
        reason: "must start with https://",
    });
}
```

**Replacement pattern** (per D-01; matches the ascii-lowercase style already in use):
```rust
// Replacement: add narrow localhost exemption. Reuse the existing ascii-lowercase
// comparison style; T-01-04 invariant preserved for all non-localhost hosts.
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

**Test amendment pattern** — the existing test at `src/config.rs:475-496` stays shape-wise identical but gains a companion test. Copy the scaffold from the existing test and add a new `test_confluence_url_localhost_exemption`:

```rust
// Source: src/config.rs:475-496 — existing test structure (scaffold, Cli builder pattern)
#[test]
fn test_confluence_url_must_be_https() {
    let cli = Cli {
        confluence_url: Some("http://example.atlassian.net".to_string()),  // non-localhost http
        confluence_username: Some("user@example.com".to_string()),
        confluence_token: Some("token".to_string()),
        ..cli_blank()
    };
    let err = Config::load_with_home(&cli, Some(&no_home()))
        .expect_err("should reject non-localhost http URL");
    assert!(matches!(err, ConfigError::Invalid { name: "CONFLUENCE_URL", .. }));
}

// NEW — mirrors the structure of test_confluence_url_must_be_https
#[test]
fn test_confluence_url_localhost_exemption() {
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

**Key reuse:** `cli_blank()` helper at `src/config.rs:261-276` and `no_home()` helper at `src/config.rs:254-256` — both already exist; use struct-update syntax.

---

### `src/config.rs` D-04 — delete `DiagramConfig::from_env` and `impl Default for DiagramConfig`

**Role:** dead-code removal

**Analog:** The code itself at `src/config.rs:22-44` is the removal target. Other constructors in the codebase show the surviving style — explicit struct literals (e.g., `src/config.rs:144-149` builds a `DiagramConfig { ... }` struct literal inline in `Config::load_with_home`, which is the style callers should adopt).

**Existing "keep" pattern** — `src/config.rs:144-149` is the canonical explicit-struct construction that remains after D-04:
```rust
// Source: src/config.rs:144-149 — production call site, survives D-04
let diagram_config = DiagramConfig {
    plantuml_path,
    mermaid_path,
    mermaid_puppeteer_config,
    timeout_secs,
};
```

**Action:** delete lines 22-44 (the `impl DiagramConfig { from_env }` block and the `impl Default for DiagramConfig` block). No imports need to change.

---

### `src/llm/mod.rs` D-03 — `AnthropicClient::new` reads `ANTHROPIC_BASE_URL`

**Role:** http-client constructor  **Data Flow:** transform (env-var → endpoint string)

**Analog 1 (existing test seam):** `src/llm/mod.rs::with_endpoint` at lines 60-83 — already exists, unchanged; `new()` delegates to it.

**Analog 2 (env-var-first-then-default idiom):** `src/config.rs::DiagramConfig::from_env` at lines 25-37 (being removed by D-04 but the idiom itself is idiomatic Rust):
```rust
// Source pattern: env::var(...).ok().unwrap_or_else(...) — from src/config.rs:27-28
std::env::var("PLANTUML_PATH").unwrap_or_else(|_| "plantuml".to_string())
```

**Existing `new()` to replace** (`src/llm/mod.rs:49-57`):
```rust
// Source: src/llm/mod.rs:49-57 (current state — hardcoded URL)
/// Create a new client pointing at the production Anthropic API.
pub fn new(api_key: String, model: String) -> Self {
    Self::with_endpoint(
        api_key,
        model,
        "https://api.anthropic.com/v1/messages".to_string(),
    )
}
```

**Replacement pattern** (reads `ANTHROPIC_BASE_URL`, filters empty string, falls back to hardcoded prod URL, delegates to existing `with_endpoint`):
```rust
// Replacement for src/llm/mod.rs:49-57. Filters empty strings (defensive) because
// .env("ANTHROPIC_BASE_URL", "") semantics vary. with_endpoint() is unchanged.
pub fn new(api_key: String, model: String) -> Self {
    let endpoint = std::env::var("ANTHROPIC_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());
    Self::with_endpoint(api_key, model, endpoint)
}
```

**Integration point:** `src/lib.rs:117` already calls `AnthropicClient::new(api_key, model)`; no change to the call site — the env-var read is internal to `new()`.

---

### `src/converter/mod.rs` D-04 — delete `impl Default for MarkdownConverter`

**Role:** constructor / dead-code removal

**Analog:** The target is `src/converter/mod.rs:47-51`:
```rust
// Source: src/converter/mod.rs:47-51 (to delete)
impl Default for MarkdownConverter {
    fn default() -> Self {
        Self::new(DiagramConfig::default())
    }
}
```

**Surviving pattern** — callers should explicitly construct:
```rust
// Source: src/converter/mod.rs:41-45 (the explicit constructor, unchanged)
impl MarkdownConverter {
    pub fn new(diagram_config: DiagramConfig) -> Self {
        Self { diagram_config }
    }
}
```

After removal, callers invoke `MarkdownConverter::new(diagram_config)` with an explicit `DiagramConfig { ... }` literal.

---

### `src/converter/tests.rs` D-04 — rewrite 7 call sites

**Role:** unit-test refactor

**Analog (DRY helper pattern, same repo, same test tier):** `src/converter/diagrams.rs::config_with_defaults` at lines 170-177 — already the canonical test-scope helper:

```rust
// Source: src/converter/diagrams.rs:170-177 — the existing test-scope DRY helper to reuse/mirror
fn config_with_defaults() -> DiagramConfig {
    DiagramConfig {
        plantuml_path: "plantuml".to_string(),
        mermaid_path: "mmdc".to_string(),
        mermaid_puppeteer_config: None,
        timeout_secs: 30,
    }
}
```

**Replacement pattern for each of the 7 sites** — two options, ranked:

1. **Recommended:** add a local helper `fn test_diagram_config() -> DiagramConfig` (mirrors `config_with_defaults` naming and shape) at the top of `src/converter/tests.rs`, then call it at each of the 7 sites. Factoring matches the existing style.
2. **Acceptable fallback:** inline the literal at each of the 7 sites (CONTEXT.md D-04 explicit block).

**Call-site rewrites** — each of these must change:
- Line 87: `let converter = MarkdownConverter::default();` → `let converter = MarkdownConverter::new(test_diagram_config());`
- Line 99: `let converter = MarkdownConverter::default();` → `let converter = MarkdownConverter::new(test_diagram_config());`
- Line 107: `let converter = MarkdownConverter::default();` → `let converter = MarkdownConverter::new(test_diagram_config());`
- Line 124: `let config = crate::config::DiagramConfig::default();` → `let config = test_diagram_config();`
- Line 142: (same) → `let config = test_diagram_config();`
- Line 165: (same) → `let config = test_diagram_config();`
- Line 198: (same) → `let config = test_diagram_config();`

Canonical replacement literal (per CONTEXT.md D-04):
```rust
// Source: CONTEXT.md D-04 explicit spec (identical to src/converter/diagrams.rs::config_with_defaults)
fn test_diagram_config() -> crate::config::DiagramConfig {
    crate::config::DiagramConfig {
        plantuml_path: "plantuml".to_string(),
        mermaid_path: "mmdc".to_string(),
        mermaid_puppeteer_config: None,
        timeout_secs: 30,
    }
}
```

---

### `src/converter/diagrams.rs` D-04 — delete `test_diagram_config_from_env`

**Role:** dead-code-test removal

**Analog:** The target test at `src/converter/diagrams.rs:180-224`. Its only surviving peers — `test_render_plantuml_invalid_binary_returns_error` (line 226) and `test_render_mermaid_invalid_binary_returns_error` (line 242) — already use `config_with_defaults()` with struct-update syntax (`..config_with_defaults()`), so the rest of the file is untouched:

```rust
// Source: src/converter/diagrams.rs:228-231 — the surviving pattern
let config = DiagramConfig {
    plantuml_path: "nonexistent-plantuml-binary-xyz".to_string(),
    ..config_with_defaults()
};
```

**Rationale (RESEARCH.md Runtime State Inventory):** the env-var contract (`PLANTUML_PATH` / `MERMAID_PATH` / `DIAGRAM_TIMEOUT` / `MERMAID_PUPPETEER_CONFIG`) survives via (a) `#[arg(long, env = "...")]` on `Cli`, (b) inline reads in `Config::load_with_home` lines 138-142, and (c) `tests/cli_integration.rs::test_convert_with_env_var_diagram_paths` (the integration-tier coverage). No coverage is lost.

---

## Shared Patterns

### Wiremock `MockServer` lifecycle

**Source:** `src/confluence/client.rs:218-232` and `tests/llm_integration.rs:54-68`
**Apply to:** Both new happy-path tests in `tests/cli_integration.rs`

```rust
// Start + mount + pass server.uri() to the code under test. Drop-releases the port.
let server = MockServer::start().await;
Mock::given(method("GET"))
    .and(path("/..."))
    .respond_with(ResponseTemplate::new(200).set_body_json(...))
    .mount(&server)
    .await;
// ...code under test receives server.uri() == "http://127.0.0.1:{random_port}"
```

### `temp_markdown` helper

**Source:** `tests/cli_integration.rs:14-19`
**Apply to:** Both new happy-path tests (already the existing reuse pattern)

```rust
// Source: tests/cli_integration.rs:14-19
fn temp_markdown(content: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let md_path = dir.path().join("doc.md");
    fs::write(&md_path, content).expect("write temp markdown");
    (dir, md_path)
}
```

### Env-scrub before binary invocation

**Source:** `tests/cli_integration.rs:36-38`, `tests/cli_integration.rs:233-236`, `tests/cli_integration.rs:264-266`, `tests/cli_integration.rs:306-309`, `tests/cli_integration.rs:361-366`, `tests/cli_integration.rs:435-437`
**Apply to:** Both new happy-path tests — always `env_remove` `CONFLUENCE_URL`, `CONFLUENCE_USERNAME`, `CONFLUENCE_API_TOKEN`; for upload also remove `ANTHROPIC_API_KEY` and `ANTHROPIC_BASE_URL`; for update add `env("ANTHROPIC_BASE_URL", anthropic.uri())`.

```rust
// Source: tests/cli_integration.rs:233-236 — the canonical scrub list
.env_remove("ANTHROPIC_API_KEY")
.env_remove("CONFLUENCE_URL")
.env_remove("CONFLUENCE_USERNAME")
.env_remove("CONFLUENCE_API_TOKEN");
```

### D-07 tracing-free stdout assertion

**Source:** `tests/cli_integration.rs:74-78`
**Apply to:** Both new happy-path tests — defends against regressions where tracing macros accidentally emit to stdout.

```rust
// Source: tests/cli_integration.rs:74-78
assert!(
    !stdout.contains("DEBUG") && !stdout.contains("INFO") && !stdout.contains("TRACE"),
    "tracing output must not appear on stdout; stdout: {stdout}"
);
```

### `#[serial]` attribute for env-var-mutating tests

**Source:** `tests/cli_integration.rs:425` (`test_convert_with_env_var_diagram_paths`)
**Apply to:** Both new happy-path tests — `test_update_command_happy_path` sets `ANTHROPIC_BASE_URL` via `.env()`; `test_upload_command_happy_path` env-removes credentials. Defensive serialization per RESEARCH.md Pattern 5.

```rust
// Source: tests/cli_integration.rs:424-426
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_...() { /* ... */ }
```

### `cli_blank()` + struct-update syntax for `Cli` fixtures

**Source:** `src/config.rs:261-276`
**Apply to:** New `test_confluence_url_localhost_exemption` test in `src/config.rs`

```rust
// Source: src/config.rs:261-276 — build a blank Cli and override only the fields the test needs
fn cli_blank() -> Cli { /* all-None defaults */ }

// Usage: struct-update syntax
let cli = Cli {
    confluence_url: Some("http://localhost:8080".to_string()),
    confluence_username: Some("user@example.com".to_string()),
    confluence_token: Some("token".to_string()),
    ..cli_blank()
};
```

### Explicit `DiagramConfig` literal (replacement for `::default()`)

**Source:** `src/converter/diagrams.rs:170-177` (`config_with_defaults` helper)
**Apply to:** All call sites in `src/converter/tests.rs` after D-04 removes `Default` impls. Recommend a local `test_diagram_config()` helper mirroring the existing `config_with_defaults` naming.

```rust
// Source: src/converter/diagrams.rs:170-177
fn test_diagram_config() -> DiagramConfig {
    DiagramConfig {
        plantuml_path: "plantuml".to_string(),
        mermaid_path: "mmdc".to_string(),
        mermaid_puppeteer_config: None,
        timeout_secs: 30,
    }
}
```

### Env-var-first-then-default idiom

**Source:** `src/config.rs:27-28` (currently inside `DiagramConfig::from_env`, removed by D-04 but the idiom itself is reused)
**Apply to:** `AnthropicClient::new` (D-03)

```rust
// Source idiom: std::env::var(...).ok().filter(|s| !s.is_empty()).unwrap_or_else(...)
let endpoint = std::env::var("ANTHROPIC_BASE_URL")
    .ok()
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());
```

## No Analog Found

None — every file modification in this phase has a direct in-repo precedent. RESEARCH.md confidence: HIGH. The phase is pure composition of existing patterns; no new architectural tier is introduced.

## Metadata

**Analog search scope:**
- `src/config.rs` (https guard + DiagramConfig)
- `src/converter/mod.rs`, `src/converter/tests.rs`, `src/converter/diagrams.rs` (DiagramConfig callers)
- `src/confluence/client.rs` mod tests (wiremock Confluence template)
- `src/llm/mod.rs` (AnthropicClient + with_endpoint seam)
- `src/lib.rs` (production call sites for AnthropicClient::new / ConfluenceClient::new)
- `tests/cli_integration.rs` (binary-invocation + env-scrub + #[serial] template)
- `tests/llm_integration.rs` (wiremock Anthropic template, `tool_use_response` helper)

**Files scanned:** 8 source files + 2 integration test files + 2 upstream docs (CONTEXT.md, RESEARCH.md) = 12

**Pattern extraction date:** 2026-04-20

---

*Phase: 10-tech-debt-integration-test-coverage-and-api-cleanup*
*Patterns mapped: 2026-04-20*
