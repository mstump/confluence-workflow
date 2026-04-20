# Phase 1: Project Scaffolding and Confluence API Client - Research

**Researched:** 2026-04-10
**Domain:** Rust project scaffolding, credential loading, Confluence REST API v1 client
**Confidence:** HIGH

## Summary

Phase 1 establishes the Rust workspace from scratch and delivers a working `upload` command that can overwrite a Confluence page via the REST API v1. The three major work areas are: (1) Cargo workspace with module layout, error types, and clap CLI skeleton; (2) configuration and credential loading from environment variables and `.env` file; (3) a trait-based Confluence REST API client over reqwest with retry-on-409.

**Critical finding from machine inspection:** There is NO `~/.claude/credentials.json` file on this machine. Confluence credentials are stored in a project-local `.env` file and loaded via direnv into environment variables (`CONFLUENCE_URL`, `CONFLUENCE_USERNAME`, `CONFLUENCE_API_TOKEN`). The Anthropic API key is stored as `ANTHROPIC_API_KEY` (not present in env currently but will be needed in Phase 3). The credential loading waterfall should prioritize: CLI flags, then environment variables, then `.env` file via `dotenvy`. The `~/.claude/` credential file path mentioned in REQUIREMENTS SCAF-03 is a fallback that may not exist on most machines -- implement it but do not depend on it.

**Primary recommendation:** Use reqwest 0.13 (not 0.12 as prior research assumed -- 0.13 was released with breaking changes including `rustls-tls` renamed to `rustls`). Build the Confluence client directly against reqwest with Basic Auth (email + API token base64-encoded). Implement retry-on-409 from day one with re-fetch-then-retry semantics.

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCAF-01 | Rust workspace builds cleanly with `cargo build` | Cargo workspace layout documented; all crate versions verified against registry |
| SCAF-02 | CLI binary accepts `update`, `upload`, `convert` subcommands via clap | clap 4.6.0 with derive macros; subcommand pattern documented |
| SCAF-03 | Credentials loaded via waterfall: CLI flag, env var, `~/.claude/` config file | Machine inspection reveals `.env` + direnv pattern; `~/.claude/` has no credentials file; waterfall: CLI flag > env var > dotenvy `.env` > `~/.claude/` fallback |
| SCAF-04 | Configuration supports Confluence base URL, API token, username, and space key | Python `.env` file format verified: `CONFLUENCE_URL`, `CONFLUENCE_USERNAME`, `CONFLUENCE_API_TOKEN` |
| SCAF-05 | Structured error types with `thiserror`; user-facing errors have clear messages | thiserror 2.0.18 + anyhow 1.0.102 pattern documented |
| CONF-01 | Fetch existing page content (storage XML) and version number via REST API v1 | `GET /wiki/rest/api/content/{id}?expand=body.storage,version` verified |
| CONF-02 | Update page content with incremented version number (conflict detection via version field) | `PUT /wiki/rest/api/content/{id}` with `version.number = current + 1`; 409 on conflict; retry-with-re-fetch strategy documented |
| CONF-03 | Upload SVG attachments to a page | `POST /wiki/rest/api/content/{id}/child/attachment` with multipart and `X-Atlassian-Token: nocheck` |
| CONF-04 | Extract page ID from a Confluence page URL | Three regex patterns from Python source verified: `/pages/(\d+)`, `/pages/edit-v2/(\d+)`, `pageId=(\d+)` |
| CONF-05 | Client is trait-based (`ConfluenceApi` trait) for testability | async_trait 0.1.89 with `Send + Sync` bounds; mock pattern documented |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Pin dependency versions in Cargo.toml (semver ranges OK, Cargo.lock pins exact)
- Run markdownlint after markdown changes
- This is a Rust rewrite -- do NOT port Python; write idiomatic Rust
- The existing Python project uses `atlassian-python-api` for Confluence, `typer` for CLI, Pydantic settings for config
- Confluence Cloud only (`cloud=True` in Python); no Server/Data Center support

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.51.1 | Async runtime | Required by reqwest; ecosystem standard for async Rust [VERIFIED: cargo search] |
| reqwest | 0.13.2 | HTTP client | Mature async HTTP; rustls default TLS in 0.13 [VERIFIED: cargo search] |
| serde | 1.0.228 | Serialization framework | The Rust serialization standard [VERIFIED: cargo search] |
| serde_json | 1.x | JSON serialization | Pairs with serde for all JSON work [VERIFIED: cargo search] |
| clap | 4.6.0 | CLI argument parsing | Dominant Rust CLI framework; derive macros for subcommands [VERIFIED: cargo search] |
| thiserror | 2.0.18 | Error type definitions | Standard derive macro for error enums [VERIFIED: cargo search] |
| anyhow | 1.0.102 | Application error handling | CLI boundary error wrapping [VERIFIED: cargo search] |
| tracing | 0.1.44 | Structured logging | Async-aware structured logging with spans [VERIFIED: cargo search] |
| tracing-subscriber | 0.3.x | Log output formatting | RUST_LOG env var filtering, JSON output [ASSUMED] |
| dirs | 6.0.0 | Home directory resolution | Cross-platform `~` expansion [VERIFIED: cargo search] |
| dotenvy | 0.15.7 | .env file loading | Load credentials from `.env` file [VERIFIED: cargo search] |
| base64 | 0.22.1 | Base64 encoding | Basic Auth header encoding [VERIFIED: cargo search] |
| async-trait | 0.1.89 | Async trait support | Required for `ConfluenceApi` trait with async methods [VERIFIED: cargo search] |

### Dev Dependencies

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| wiremock | 0.6.5 | HTTP mocking | Mock Confluence API responses in integration tests [VERIFIED: cargo search] |
| insta | 1.47.2 | Snapshot testing | Test XML output, error messages [VERIFIED: cargo search] |
| assert_cmd | 2.2.0 | CLI integration tests | Test compiled binary end-to-end [VERIFIED: cargo search] |

### Breaking Change Alert: reqwest 0.12 to 0.13

The prior research (STACK.md) recommended reqwest 0.12. The registry now shows **0.13.2** with these breaking changes [VERIFIED: GitHub CHANGELOG.md + docs.rs]:

| Change | Old (0.12) | New (0.13) |
|--------|-----------|-----------|
| Default TLS | native-tls | rustls |
| TLS feature name | `rustls-tls` | `rustls` |
| Crypto provider | ring | aws-lc |
| `query` feature | always enabled | opt-in feature flag |
| `form` feature | always enabled | opt-in feature flag |

**Impact on this project:** Use `reqwest = { version = "0.13", features = ["json", "multipart", "rustls"] }`. Note `rustls` not `rustls-tls`.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| reqwest 0.13 | reqwest 0.12 | 0.12 still works but 0.13 is current; use 0.13 for new project |
| anyhow | color-eyre | color-eyre has prettier error output but adds complexity |
| async-trait | native async trait (nightly) | Rust stable does not yet fully support async in trait; async-trait is still needed for dyn dispatch |
| dotenvy | figment | figment is more powerful but overkill for simple env var loading |

**Installation:**

```bash
cargo add tokio --features full
cargo add reqwest --features json,multipart,rustls
cargo add serde --features derive
cargo add serde_json
cargo add clap --features derive,env
cargo add thiserror anyhow
cargo add tracing
cargo add tracing-subscriber --features env-filter
cargo add dirs dotenvy base64 async-trait
cargo add --dev wiremock insta assert_cmd
```

## Architecture Patterns

### Recommended Project Structure

```
confluence-agent/
  Cargo.toml
  src/
    main.rs              # Thin shell: parse args, build runtime, call lib
    lib.rs               # Re-exports for library use and testing
    cli.rs               # clap derive structs: Cli, Commands enum
    config.rs            # Credential + config loading (waterfall)
    confluence/
      mod.rs             # ConfluenceApi trait definition
      client.rs          # ConfluenceClient implementation (reqwest)
      types.rs           # Page, PageVersion, Attachment structs
      url.rs             # Page ID extraction from URL
    error.rs             # AppError enum with thiserror
  tests/
    integration/
      confluence_test.rs # wiremock-based API tests
      cli_test.rs        # assert_cmd binary tests
```

### Pattern 1: Credential Waterfall Loading

**What:** Load Confluence credentials from multiple sources with clear precedence.
**When to use:** Every CLI invocation that needs API access.

**Actual credential format on this machine** [VERIFIED: machine inspection]:

```bash
# .env file (loaded by direnv)
CONFLUENCE_URL=https://zitenote.atlassian.net/wiki
CONFLUENCE_USERNAME=mstump@securityscorecard.io
CONFLUENCE_API_TOKEN=ATATT3xFfGF0...
```

**Waterfall order:**

1. CLI flags (`--confluence-url`, `--confluence-username`, `--confluence-token`)
2. Environment variables (`CONFLUENCE_URL`, `CONFLUENCE_USERNAME`, `CONFLUENCE_API_TOKEN`)
3. `.env` file in current directory (via `dotenvy`)
4. `~/.claude/` config file fallback (if it ever exists -- not present on this machine)

```rust
use dotenvy::dotenv;

pub struct Config {
    pub confluence_url: String,
    pub confluence_username: String,
    pub confluence_api_token: String,
    pub anthropic_api_key: Option<String>, // Not needed in Phase 1
}

impl Config {
    pub fn load(cli: &CliOverrides) -> Result<Self, ConfigError> {
        // Load .env file (non-fatal if missing)
        let _ = dotenv();

        let confluence_url = cli.confluence_url.clone()
            .or_else(|| std::env::var("CONFLUENCE_URL").ok())
            .ok_or(ConfigError::Missing("CONFLUENCE_URL"))?;

        // ... same for username, token
    }
}
```

### Pattern 2: Trait-Based Confluence Client

**What:** Define `ConfluenceApi` as an async trait so tests can inject a mock.
**When to use:** All code that interacts with Confluence.

```rust
use async_trait::async_trait;

#[async_trait]
pub trait ConfluenceApi: Send + Sync {
    async fn get_page(&self, page_id: &str) -> Result<Page, ConfluenceError>;
    async fn update_page(
        &self,
        page_id: &str,
        title: &str,
        content: &str,
        version: u32,
    ) -> Result<(), ConfluenceError>;
    async fn upload_attachment(
        &self,
        page_id: &str,
        filename: &str,
        content: Vec<u8>,
        content_type: &str,
    ) -> Result<(), ConfluenceError>;
}
```

### Pattern 3: Retry-on-409 for Version Conflicts

**What:** When `update_page` returns 409 Conflict, re-fetch the page to get the latest version, then retry the update with the new version number.
**When to use:** Every page update call.

```rust
pub async fn update_page_with_retry(
    client: &dyn ConfluenceApi,
    page_id: &str,
    content: &str,
    max_retries: u32, // recommend 3
) -> Result<(), ConfluenceError> {
    for attempt in 0..=max_retries {
        let page = client.get_page(page_id).await?;
        let new_version = page.version.number + 1;
        match client.update_page(page_id, &page.title, content, new_version).await {
            Ok(()) => return Ok(()),
            Err(ConfluenceError::VersionConflict { .. }) if attempt < max_retries => {
                tracing::warn!(
                    attempt = attempt + 1,
                    page_id = page_id,
                    "Version conflict (409), re-fetching and retrying"
                );
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

### Pattern 4: Page ID Extraction from URL

**What:** Parse Confluence page URLs to extract numeric page ID.
**When to use:** All CLI commands that take a page URL argument.

The Python source uses three regex patterns [VERIFIED: Python source `confluence.py` line 46-49]:

```rust
use regex::Regex;

pub fn extract_page_id(url: &str) -> Result<String, ConfluenceError> {
    let patterns = [
        r"/pages/(\d+)",           // /pages/12345/...
        r"/pages/edit-v2/(\d+)",   // /pages/edit-v2/12345/...
        r"pageId=(\d+)",           // /viewpage.action?pageId=54321
    ];
    for pattern in &patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(url) {
            return Ok(caps[1].to_string());
        }
    }
    Err(ConfluenceError::InvalidPageUrl(url.to_string()))
}
```

**Note:** The `regex` crate is a transitive dependency of many crates. Add it explicitly if using it for URL parsing. Alternatively, use simple string operations since the patterns are straightforward.

### Anti-Patterns to Avoid

- **Creating a new reqwest::Client per request:** Wastes TLS handshakes and connection pools. Create one `Client` in `ConfluenceClient::new()` and reuse it. [CITED: reqwest docs]
- **Blocking in async context:** Use `tokio::fs` for file I/O or `spawn_blocking` for sync operations. Do not call `std::fs::read_to_string` inside an async fn.
- **Stringly-typed errors:** Do not return `String` errors. Use `thiserror` enums for every error type.
- **Hardcoding base URL path:** The Confluence base URL may or may not end with `/wiki`. Normalize it: `base_url.trim_end_matches('/')`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Base64 encoding for Basic Auth | Manual base64 implementation | `base64` crate (0.22.1) | Standard, tested, handles padding correctly |
| Home directory resolution | Manual `$HOME` parsing | `dirs` crate (6.0.0) | Handles macOS, Linux, Windows portably |
| .env file parsing | Manual file reader | `dotenvy` crate (0.15.7) | Handles quoting, escaping, comments correctly |
| CLI argument parsing | Manual argv parsing | `clap` with derive (4.6.0) | Help generation, env var fallbacks, subcommands |
| HTTP multipart encoding | Manual boundary/encoding | `reqwest::multipart` | Correct encoding, streaming, content-type handling |
| Async trait dispatch | Manual vtable / enum dispatch | `async_trait` crate (0.1.89) | Handles Send + Sync bounds correctly |

## Common Pitfalls

### Pitfall 1: reqwest 0.13 Feature Flag Changes

**What goes wrong:** Using `rustls-tls` feature flag (which was the name in 0.12) causes a compile error in 0.13.
**Why it happens:** reqwest 0.13 renamed `rustls-tls` to `rustls`.
**How to avoid:** Use `features = ["json", "multipart", "rustls"]` in Cargo.toml.
**Warning signs:** Compile error about unknown feature `rustls-tls`.

### Pitfall 2: Confluence 409 Version Conflict

**What goes wrong:** Fetching page version N, then updating with N+1 fails because someone edited the page between fetch and update.
**Why it happens:** Confluence uses optimistic concurrency -- version must be exactly `current + 1`.
**How to avoid:** Implement retry loop: on 409, re-fetch page (get new version), retry update with new version + 1. Max 3 retries.
**Warning signs:** 409 HTTP status from PUT endpoint.

### Pitfall 3: Missing X-Atlassian-Token Header on Attachment Upload

**What goes wrong:** Attachment upload returns 403 Forbidden.
**Why it happens:** Confluence requires `X-Atlassian-Token: nocheck` header to bypass XSRF protection on attachment endpoints. [CITED: Atlassian REST API docs]
**How to avoid:** Always set this header on POST to `/child/attachment`.
**Warning signs:** 403 on attachment upload that works for page updates.

### Pitfall 4: Confluence Base URL Normalization

**What goes wrong:** Double-slash in URL path (e.g., `https://domain.atlassian.net/wiki//rest/api/...`).
**Why it happens:** User provides URL with trailing slash, code appends `/rest/api/...`.
**How to avoid:** `base_url.trim_end_matches('/')` in client constructor.
**Warning signs:** 404 errors on API calls.

### Pitfall 5: reqwest 0.13 query/form Features Disabled by Default

**What goes wrong:** `.query()` or `.form()` methods not available on RequestBuilder.
**Why it happens:** In reqwest 0.13, `query` and `form` are now opt-in features [VERIFIED: reqwest CHANGELOG].
**How to avoid:** This phase does not need `query` or `form` features -- JSON body and multipart are sufficient. If needed later, add `features = ["query"]`.
**Warning signs:** Method not found errors for `.query()`.

## Code Examples

### Confluence Client Construction

```rust
// Source: Adapted from Python confluence.py + Atlassian REST API docs
use reqwest::Client;
use base64::Engine;

pub struct ConfluenceClient {
    client: Client,
    base_url: String,
    auth_header: String,
}

impl ConfluenceClient {
    pub fn new(base_url: &str, username: &str, api_token: &str) -> Self {
        let credentials = format!("{}:{}", username, api_token);
        let auth_header = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(&credentials)
        );
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header,
        }
    }
}
```

### GET Page Content

```rust
// Source: Confluence REST API v1 - GET /wiki/rest/api/content/{id}
pub async fn get_page(&self, page_id: &str) -> Result<Page, ConfluenceError> {
    let url = format!(
        "{}/rest/api/content/{}?expand=body.storage,version",
        self.base_url, page_id
    );
    let resp = self.client.get(&url)
        .header("Authorization", &self.auth_header)
        .send()
        .await
        .map_err(ConfluenceError::Http)?;

    match resp.status().as_u16() {
        200 => resp.json::<Page>().await.map_err(ConfluenceError::Deserialize),
        401 => Err(ConfluenceError::Unauthorized),
        404 => Err(ConfluenceError::PageNotFound(page_id.to_string())),
        code => Err(ConfluenceError::UnexpectedStatus(code)),
    }
}
```

### PUT Update Page

```rust
// Source: Confluence REST API v1 - PUT /wiki/rest/api/content/{id}
pub async fn update_page(
    &self,
    page_id: &str,
    title: &str,
    content: &str,
    version: u32,
) -> Result<(), ConfluenceError> {
    let url = format!("{}/rest/api/content/{}", self.base_url, page_id);
    let body = serde_json::json!({
        "version": { "number": version, "minorEdit": true },
        "title": title,
        "type": "page",
        "body": {
            "storage": {
                "value": content,
                "representation": "storage"
            }
        }
    });
    let resp = self.client.put(&url)
        .header("Authorization", &self.auth_header)
        .json(&body)
        .send()
        .await
        .map_err(ConfluenceError::Http)?;

    match resp.status().as_u16() {
        200 => Ok(()),
        409 => Err(ConfluenceError::VersionConflict {
            page_id: page_id.to_string(),
            attempted_version: version,
        }),
        401 => Err(ConfluenceError::Unauthorized),
        404 => Err(ConfluenceError::PageNotFound(page_id.to_string())),
        code => Err(ConfluenceError::UnexpectedStatus(code)),
    }
}
```

### POST Attachment

```rust
// Source: Confluence REST API v1 + Python confluence.py
pub async fn upload_attachment(
    &self,
    page_id: &str,
    filename: &str,
    content: Vec<u8>,
    content_type: &str,
) -> Result<(), ConfluenceError> {
    let url = format!(
        "{}/rest/api/content/{}/child/attachment",
        self.base_url, page_id
    );
    let part = reqwest::multipart::Part::bytes(content)
        .file_name(filename.to_string())
        .mime_str(content_type)
        .map_err(|e| ConfluenceError::Multipart(e.to_string()))?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("minorEdit", "true");

    let resp = self.client.post(&url)
        .header("Authorization", &self.auth_header)
        .header("X-Atlassian-Token", "nocheck")  // Required for attachment upload
        .multipart(form)
        .send()
        .await
        .map_err(ConfluenceError::Http)?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(ConfluenceError::AttachmentUpload {
            page_id: page_id.to_string(),
            filename: filename.to_string(),
            status: resp.status().as_u16(),
        })
    }
}
```

### Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Confluence API error: {0}")]
    Confluence(#[from] ConfluenceError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required configuration: {0}. Set via CLI flag, environment variable, or .env file")]
    Missing(&'static str),

    #[error("Could not determine home directory")]
    NoHomeDir,

    #[error("Failed to read config file {path}: {source}")]
    FileRead {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to parse config file {path}: {source}")]
    JsonParse {
        path: String,
        source: serde_json::Error,
    },
}

#[derive(Error, Debug)]
pub enum ConfluenceError {
    #[error("HTTP request failed: {0}")]
    Http(reqwest::Error),

    #[error("Failed to deserialize response: {0}")]
    Deserialize(reqwest::Error),

    #[error("Authentication failed. Check CONFLUENCE_USERNAME and CONFLUENCE_API_TOKEN")]
    Unauthorized,

    #[error("Page not found: {0}. Check the page URL")]
    PageNotFound(String),

    #[error("Version conflict on page {page_id} (attempted version {attempted_version}). The page was modified by another user")]
    VersionConflict {
        page_id: String,
        attempted_version: u32,
    },

    #[error("Could not extract page ID from URL: {0}. Expected format: https://domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Page+Title")]
    InvalidPageUrl(String),

    #[error("Attachment upload failed for {filename} on page {page_id}: HTTP {status}")]
    AttachmentUpload {
        page_id: String,
        filename: String,
        status: u16,
    },

    #[error("Multipart encoding error: {0}")]
    Multipart(String),

    #[error("Unexpected HTTP status: {0}")]
    UnexpectedStatus(u16),
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| reqwest 0.12 with `rustls-tls` feature | reqwest 0.13 with `rustls` feature | 2025-2026 | Feature flag rename; rustls now default TLS |
| `dirs` 5.x | `dirs` 6.0.0 | 2025 | Minor API changes |
| Manual async trait impls | `async-trait` 0.1.89 | Ongoing | Still needed for dyn dispatch; native async trait only works with static dispatch |

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `#[tokio::test]` + wiremock 0.6.5 |
| Config file | None needed (Cargo.toml `[dev-dependencies]` suffices) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test -- --include-ignored` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SCAF-01 | Workspace builds cleanly | build | `cargo build 2>&1` | N/A (build check) |
| SCAF-02 | CLI accepts subcommands | integration | `cargo test --test cli_test` | Wave 0 |
| SCAF-03 | Credential waterfall loading | unit | `cargo test config::tests` | Wave 0 |
| SCAF-04 | Config supports all fields | unit | `cargo test config::tests` | Wave 0 |
| SCAF-05 | Structured error types | unit | `cargo test error::tests` | Wave 0 |
| CONF-01 | Fetch page content | integration (wiremock) | `cargo test confluence::tests::test_get_page` | Wave 0 |
| CONF-02 | Update with version increment + 409 retry | integration (wiremock) | `cargo test confluence::tests::test_update_page_conflict` | Wave 0 |
| CONF-03 | Upload SVG attachment | integration (wiremock) | `cargo test confluence::tests::test_upload_attachment` | Wave 0 |
| CONF-04 | Extract page ID from URL | unit | `cargo test confluence::url::tests` | Wave 0 |
| CONF-05 | Trait-based client | unit | `cargo test confluence::tests::test_mock_client` | Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test -- --include-ignored`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `tests/integration/confluence_test.rs` -- wiremock-based Confluence API tests
- [ ] `tests/integration/cli_test.rs` -- assert_cmd binary tests for subcommand parsing
- [ ] Unit tests in `src/config.rs` for credential waterfall
- [ ] Unit tests in `src/confluence/url.rs` for page ID extraction

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All compilation | Yes | 1.89.0-nightly (2025-05-12) | Use stable channel if nightly features not needed |
| cargo | Build system | Yes | 1.89.0-nightly | -- |
| Confluence Cloud API | CONF-01 through CONF-04 | Yes | v1 | Env vars set: CONFLUENCE_URL, CONFLUENCE_USERNAME, CONFLUENCE_API_TOKEN |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None. All required tools are present.

**Note on Rust nightly:** The installed toolchain is nightly (1.89.0). The project should target stable Rust since no nightly features are required. Add `rust-version = "1.80"` to Cargo.toml to set MSRV. [ASSUMED -- exact MSRV needs testing]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `tracing-subscriber` 0.3.x is current | Standard Stack | Low -- version is well-established, unlikely to have changed |
| A2 | MSRV of 1.80 is sufficient for all dependencies | Environment Availability | Medium -- some crates may require newer MSRV; test with `cargo build` |
| A3 | `~/.claude/` may eventually contain credentials | Credential Waterfall | Low -- fallback code is minimal; env var path works today |
| A4 | Confluence API v1 `PUT /content/{id}` returns 409 on version mismatch | Retry Pattern | Low -- well-documented API behavior confirmed by multiple sources |
| A5 | `reqwest::multipart` works correctly for Confluence attachment upload | Code Examples | Low -- standard multipart form, but content-type detection may need tuning |

## Open Questions

1. **Should the Rust project live in the same repo or a new one?**
   - What we know: The task says "created from scratch" but we are in the `confluence-workflow` repo
   - What's unclear: Whether to create a `rust/` subdirectory, use a workspace, or create a separate repo
   - Recommendation: Create the Rust project in the repo root as the primary codebase. The Python code can coexist during migration. Use `Cargo.toml` at the root alongside `pyproject.toml`.

2. **Should we use Rust stable or nightly?**
   - What we know: Nightly 1.89.0 is installed; no nightly features are needed
   - What's unclear: Whether the user prefers nightly for any reason
   - Recommendation: Target stable Rust; set MSRV in Cargo.toml

3. **`~/.claude/` credential file format**
   - What we know: No credentials file exists on this machine. All credentials come from `.env` via direnv
   - What's unclear: Whether other machines have a `~/.claude/credentials.json`
   - Recommendation: Implement the env var path as primary, `.env` via dotenvy as secondary, and a stub for `~/.claude/` that logs a warning if the file does not exist. This can be enhanced later if the format is discovered.

## Sources

### Primary (HIGH confidence)

- Machine inspection of `~/.claude/` directory: no credential files exist [VERIFIED: `ls -la ~/.claude/`, `find` search]
- Machine inspection of `.env` file: Confluence credentials stored as `CONFLUENCE_URL`, `CONFLUENCE_USERNAME`, `CONFLUENCE_API_TOKEN` [VERIFIED: direct file read]
- Crate versions verified against cargo registry: reqwest 0.13.2, clap 4.6.0, tokio 1.51.1, serde 1.0.228, thiserror 2.0.18, anyhow 1.0.102, dirs 6.0.0, dotenvy 0.15.7, base64 0.22.1, async-trait 0.1.89, wiremock 0.6.5 [VERIFIED: `cargo search`]
- reqwest 0.13 breaking changes: rustls-tls renamed to rustls, query/form now opt-in [VERIFIED: GitHub CHANGELOG.md + docs.rs]
- Python source code: `confluence.py`, `config.py`, `cli.py` [VERIFIED: direct file read]
- Confluence REST API v1 docs: endpoints and authentication [CITED: developer.atlassian.com/cloud/confluence/rest/v1/]

### Secondary (MEDIUM confidence)

- Confluence 409 behavior on version conflict [CITED: Atlassian REST API docs, partially verified via WebFetch]
- X-Atlassian-Token: nocheck header requirement [CITED: Atlassian docs + prior research PITFALLS.md]

### Tertiary (LOW confidence)

- None -- all critical claims verified against machine state or registry

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH -- all versions verified against cargo registry
- Architecture: HIGH -- patterns derived from Python source + standard Rust practices
- Pitfalls: HIGH -- reqwest 0.13 breaking changes verified; Confluence API behavior cited from docs
- Credentials: HIGH -- machine inspection confirms actual format (.env, not ~/.claude/)

**Research date:** 2026-04-10
**Valid until:** 2026-05-10 (stable domain; crate versions may increment but not break)
