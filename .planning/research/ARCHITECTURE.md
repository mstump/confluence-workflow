# Architecture Patterns

**Domain:** Rust CLI tool for Markdown-to-Confluence conversion with LLM merge
**Researched:** 2026-04-10

## Recommended Architecture

### High-Level Component Diagram

```text
+------------------------------------------------------------------+
|  CLI Layer (clap)                                                |
|  Commands: update | upload | convert                             |
+------------------------------------------------------------------+
         |                    |                   |
         v                    v                   v
+------------------+  +---------------+  +------------------+
| Orchestrator     |  | Converter     |  | Config / Creds   |
| (merge pipeline) |  | (md -> XML)   |  | (~/.claude/, env)|
+------------------+  +---------------+  +------------------+
         |                    |                   |
         v                    v                   v
+------------------+  +---------------+  +------------------+
| LLM Client       |  | Diagram       |  | Confluence       |
| (Anthropic HTTP) |  | Renderer      |  | REST Client      |
|                  |  | (PlantUML)    |  | (reqwest)        |
+------------------+  +---------------+  +------------------+
```

### Crate / Module Layout

```
confluence-agent/
  Cargo.toml
  src/
    main.rs              # Entry point, clap CLI dispatch
    lib.rs               # Re-exports for library use and testing
    cli/
      mod.rs             # Command definitions and argument parsing
      update.rs          # `update` command handler
      upload.rs          # `upload` command handler
      convert.rs         # `convert` command handler
    config/
      mod.rs             # Config loading orchestration
      credentials.rs     # Claude Code credential file reader
      settings.rs        # Merged config (file + env + CLI args)
    confluence/
      mod.rs             # ConfluenceClient trait + impl
      api.rs             # REST API request/response types
      types.rs           # Page, Attachment, Version structs
    converter/
      mod.rs             # Markdown-to-storage pipeline
      markdown.rs        # Markdown parsing and preprocessing
      storage_xml.rs     # Confluence storage XML construction
      diagrams.rs        # PlantUML/Mermaid rendering
    llm/
      mod.rs             # LLM client trait + Anthropic impl
      anthropic.rs       # Anthropic Messages API client
      prompts.rs         # Prompt templates
      merge.rs           # Per-comment parallel merge logic
    xml/
      mod.rs             # XML manipulation utilities
      comments.rs        # Inline comment extraction/preservation
      transform.rs       # Storage format transformations
    error.rs             # Unified error types (thiserror)
```

### Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `cli` | Parse args, dispatch to orchestrator, format output | `config`, `orchestrator` (via lib.rs functions) |
| `config` | Load credentials from `~/.claude/`, env vars, CLI overrides | Filesystem, environment |
| `confluence` | HTTP client for Confluence REST API | Confluence Cloud API |
| `converter` | Markdown to Confluence storage XML, diagram rendering | `xml`, subprocess (PlantUML) |
| `llm` | Call Anthropic Messages API, parse structured responses | Anthropic API |
| `xml` | Parse, query, and modify Confluence storage XML | Internal only |
| `error` | Unified error enum for all modules | All modules |

### Data Flow

**`update` command (full LLM merge pipeline):**

```text
1. CLI parses args (markdown_path, page_url, options)
2. Config loads credentials: ~/.claude/credentials.json -> env ANTHROPIC_API_KEY -> error
3. Config loads Confluence creds: env vars (CONFLUENCE_URL, CONFLUENCE_USERNAME, CONFLUENCE_API_TOKEN)
4. Converter: read markdown file -> strip frontmatter -> render diagrams -> produce storage XML
5. Confluence client: GET existing page content + version + title
6. XML module: extract inline comment markers from existing page
7. Decision branch:
   a. No existing content OR no inline comments -> use new content directly
   b. Has inline comments -> run per-comment parallel LLM evaluation:
      - For each comment marker, create focused context (surrounding paragraphs)
      - Call Anthropic API in parallel (tokio::spawn or futures::join_all)
      - Each call returns: KEEP(updated_position) or DROP
      - Assemble final XML with surviving comments placed correctly
8. Confluence client: POST attachments (diagrams)
9. Confluence client: PUT updated page content with incremented version
```

**`upload` command (direct overwrite):**

```text
1-4. Same as update
5. Confluence client: GET page version + title (for version increment)
6. Confluence client: POST attachments
7. Confluence client: PUT page content (no merge, no LLM)
```

**`convert` command (local only):**

```text
1-4. Same as update (except no Confluence creds needed)
5. Write storage XML to output directory
6. Write diagram SVGs to output directory
```

## Claude Code Credential Files

**Confidence: MEDIUM** -- Based on training data about Claude Code's configuration. The exact file format should be verified against a real installation before implementation.

Claude Code stores configuration in `~/.claude/`. The relevant files for credential reading:

### Known File Locations

| File | Purpose | Format |
|------|---------|--------|
| `~/.claude/credentials.json` | OAuth tokens and API keys | JSON |
| `~/.claude/settings.json` | User preferences and feature flags | JSON |
| `~/.claude/config.json` | General configuration | JSON |

### Credential File Structure (Expected)

The credentials file likely contains OAuth session data from Claude Code's authentication flow. The structure is approximately:

```json
{
  "oauth": {
    "accessToken": "sk-ant-...",
    "refreshToken": "...",
    "expiresAt": "2026-..."
  }
}
```

Alternatively, Claude Code may store the API key directly or use a different key name. Possible key paths to check:

- `$.oauth.accessToken`
- `$.apiKey`
- `$.anthropic.apiKey`
- `$.claudeApiKey`

### Credential Loading Strategy

Implement a waterfall with clear precedence:

```rust
// Priority order (highest to lowest):
// 1. CLI flag: --api-key <key>
// 2. Environment variable: ANTHROPIC_API_KEY
// 3. Claude Code credentials file: ~/.claude/credentials.json
// 4. Error with helpful message pointing to Claude Code docs

fn load_anthropic_api_key(cli_key: Option<&str>) -> Result<String, ConfigError> {
    // 1. CLI override
    if let Some(key) = cli_key {
        return Ok(key.to_string());
    }
    // 2. Environment variable
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        return Ok(key);
    }
    // 3. Claude Code credentials file
    let cred_path = dirs::home_dir()
        .ok_or(ConfigError::NoHomeDir)?
        .join(".claude")
        .join("credentials.json");
    if cred_path.exists() {
        let content = std::fs::read_to_string(&cred_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        // Try known key paths
        for path in &["oauth.accessToken", "apiKey", "claudeApiKey"] {
            if let Some(key) = json_path_lookup(&json, path) {
                return Ok(key);
            }
        }
    }
    Err(ConfigError::NoApiKey {
        hint: "Set ANTHROPIC_API_KEY or authenticate with Claude Code".into(),
    })
}
```

**IMPORTANT: The exact credential file format needs verification.** Before implementing, either:

1. Inspect an actual `~/.claude/` directory on a machine with Claude Code installed
2. Check Claude Code's open-source components or documentation for the credential schema

This is flagged as a phase-specific research item.

## Rust XML Parsing and Manipulation

**Confidence: HIGH** -- Well-established Rust ecosystem with clear winners.

### Recommendation: `quick-xml` for All XML Operations

Use `quick-xml` because it handles both reading and writing, supports serde integration for structured parsing, and is the most actively maintained Rust XML crate. It is the de facto standard for XML in Rust.

| Crate | Read | Write | Serde | Use Case | Verdict |
|-------|------|-------|-------|----------|---------|
| `quick-xml` | Yes (SAX + serde) | Yes | Yes | General purpose, fast | **USE THIS** |
| `roxmltree` | Yes (DOM) | **No** | No | Read-only analysis | Not suitable (need write) |
| `minidom` | Yes | Yes | No | XMPP-focused DOM | Niche, less maintained |
| `xmltree` | Yes | Yes | No | Simple DOM | Less maintained |
| `lol_html` | Yes (streaming) | Yes (rewriting) | No | HTML rewriting | Overkill, HTML not XML |

### Why `quick-xml`

1. **Read + Write**: Confluence storage XML must be parsed, modified (comment insertion/removal), and serialized back. `roxmltree` cannot write.
2. **Serde integration**: Confluence API responses (JSON) and storage format structures can use the same serde derive patterns. `quick-xml`'s `serde` feature lets you deserialize XML fragments into typed structs.
3. **Performance**: SAX-style (pull parser) by default. No full DOM tree unless you need it. For large Confluence pages, this matters.
4. **Namespace support**: Confluence storage XML uses `ac:` and `ri:` namespaces extensively. `quick-xml` handles custom namespace prefixes.
5. **Active maintenance**: Regular releases, well-documented.

### XML Strategy for Confluence Storage Format

Confluence storage format is XHTML-like with custom namespaced macros:

```xml
<p>Some text <ac:inline-comment-marker ac:ref="abc123">commented text</ac:inline-comment-marker> more text</p>
<ac:structured-macro ac:name="code">
  <ac:parameter ac:name="language">python</ac:parameter>
  <ac:plain-text-body><![CDATA[def hello(): pass]]></ac:plain-text-body>
</ac:structured-macro>
```

**Approach**: Use `quick-xml`'s event-based reader for streaming through large pages. For inline comment extraction and manipulation, use targeted parsing:

```rust
use quick_xml::events::{Event, BytesStart};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

/// Extract all inline comment markers from storage XML
fn extract_inline_comments(storage_xml: &str) -> Vec<InlineComment> {
    let mut reader = Reader::from_str(storage_xml);
    let mut comments = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"ac:inline-comment-marker" => {
                // Extract ref attribute, capture inner text
                let ref_id = e.try_get_attribute("ac:ref")
                    .ok().flatten()
                    .map(|a| String::from_utf8_lossy(a.value.as_ref()).to_string());
                // ... collect the comment
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    comments
}
```

For the **write path** (reassembling XML with comments), use `quick-xml::Writer` to stream events, injecting or removing comment markers as needed. This avoids holding the entire DOM in memory and is more robust than regex-based manipulation.

### Serde for Structured Fragments

For Confluence API JSON responses and structured XML elements, define typed structs:

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ConfluencePage {
    id: String,
    title: String,
    body: PageBody,
    version: PageVersion,
}

#[derive(Deserialize)]
struct PageBody {
    storage: StorageRepresentation,
}

#[derive(Deserialize)]
struct StorageRepresentation {
    value: String,
    representation: String,
}

#[derive(Deserialize)]
struct PageVersion {
    number: u32,
}
```

## Confluence REST API Endpoints

**Confidence: HIGH** -- Well-documented, stable API.

The Confluence Cloud REST API uses two versions. For page operations, **v1 is more mature and better documented**. v2 is newer but still incomplete for some operations. Recommendation: use v1 endpoints for all current needs.

### Required Endpoints

#### 1. Get Page Content

```
GET /wiki/rest/api/content/{pageId}?expand=body.storage,version
```

**Response (relevant fields):**

```json
{
  "id": "12345",
  "title": "Page Title",
  "version": {
    "number": 42
  },
  "body": {
    "storage": {
      "value": "<p>Confluence storage XML here...</p>",
      "representation": "storage"
    }
  }
}
```

**Authentication:** Basic Auth with email + API token, or OAuth 2.0 bearer token.

```
Authorization: Basic base64(email:api_token)
```

#### 2. Update Page Content

```
PUT /wiki/rest/api/content/{pageId}
Content-Type: application/json
```

**Request body:**

```json
{
  "version": {
    "number": 43,
    "minorEdit": true
  },
  "title": "Page Title",
  "type": "page",
  "body": {
    "storage": {
      "value": "<p>Updated content...</p>",
      "representation": "storage"
    }
  }
}
```

**Notes:**

- Version number must be exactly `current_version + 1`
- `minorEdit: true` avoids notifying watchers (matches current Python behavior)
- Title is required even if unchanged

#### 3. Upload Attachment

```
POST /wiki/rest/api/content/{pageId}/child/attachment
Content-Type: multipart/form-data
X-Atlassian-Token: nocheck
```

**Form fields:**

- `file`: The file content (binary)
- `minorEdit`: `true` (optional, avoids notifications)

**For updating an existing attachment** (same filename):

```
POST /wiki/rest/api/content/{pageId}/child/attachment
```

The API automatically replaces existing attachments with the same filename.

#### 4. Extract Page ID from URL

Same regex-based extraction as the Python version. Confluence URLs have these patterns:

- `/pages/{pageId}/...`
- `/pages/edit-v2/{pageId}/...`
- `?pageId={pageId}`

This is pure string parsing -- no API call needed.

### Reqwest Client Structure

```rust
use reqwest::Client;
use base64::Engine;

pub struct ConfluenceClient {
    client: Client,
    base_url: String,  // e.g., "https://domain.atlassian.net/wiki"
    auth_header: String,
}

impl ConfluenceClient {
    pub fn new(base_url: &str, username: &str, api_token: &str) -> Self {
        let credentials = format!("{}:{}", username, api_token);
        let auth_header = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials)
        );
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header,
        }
    }

    pub async fn get_page(&self, page_id: &str) -> Result<ConfluencePage, Error> {
        let url = format!(
            "{}/rest/api/content/{}?expand=body.storage,version",
            self.base_url, page_id
        );
        let resp = self.client.get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn update_page(
        &self,
        page_id: &str,
        title: &str,
        content: &str,
        new_version: u32,
    ) -> Result<(), Error> {
        let url = format!("{}/rest/api/content/{}", self.base_url, page_id);
        let body = serde_json::json!({
            "version": { "number": new_version, "minorEdit": true },
            "title": title,
            "type": "page",
            "body": {
                "storage": {
                    "value": content,
                    "representation": "storage"
                }
            }
        });
        self.client.put(&url)
            .header("Authorization", &self.auth_header)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn upload_attachment(
        &self,
        page_id: &str,
        filename: &str,
        content: Vec<u8>,
    ) -> Result<(), Error> {
        let url = format!(
            "{}/rest/api/content/{}/child/attachment",
            self.base_url, page_id
        );
        let part = reqwest::multipart::Part::bytes(content)
            .file_name(filename.to_string())
            .mime_str("image/svg+xml")?;
        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("minorEdit", "true");
        self.client.post(&url)
            .header("Authorization", &self.auth_header)
            .header("X-Atlassian-Token", "nocheck")
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
```

## Testable Rust CLI Architecture

**Confidence: HIGH** -- Well-established patterns.

### Separation of Concerns: The Key Principle

**The binary (`main.rs`) should be a thin shell.** All logic lives in `lib.rs` and submodules. This allows:

- Unit tests that import library functions directly
- Integration tests in `tests/` that exercise the public API
- The binary to be swapped out (e.g., Claude Code skill invokes library functions differently)

```rust
// main.rs -- thin shell
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(confluence_agent::run(cli))
}
```

### Trait-Based Dependency Injection

Define traits for external services so tests can substitute mocks:

```rust
// Trait for Confluence operations
#[async_trait]
pub trait ConfluenceApi: Send + Sync {
    async fn get_page(&self, page_id: &str) -> Result<ConfluencePage>;
    async fn update_page(&self, page_id: &str, title: &str, content: &str, version: u32) -> Result<()>;
    async fn upload_attachment(&self, page_id: &str, filename: &str, content: Vec<u8>) -> Result<()>;
}

// Trait for LLM calls
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn evaluate_comment(
        &self,
        comment: &InlineComment,
        old_context: &str,
        new_context: &str,
    ) -> Result<CommentDecision>;
}

// Trait for diagram rendering
pub trait DiagramRenderer: Send + Sync {
    fn render_plantuml(&self, source: &str) -> Result<Vec<u8>>;
    fn render_mermaid(&self, source: &str) -> Result<Vec<u8>>;
}
```

### Test Strategy

| Layer | Test Type | Approach |
|-------|-----------|----------|
| XML parsing | Unit tests | Hardcoded XML strings, verify extracted comments |
| Markdown conversion | Unit tests | Markdown input -> expected storage XML output |
| Confluence client | Integration tests | Mock HTTP server (wiremock) or record/replay |
| LLM evaluation | Unit tests with mocks | Mock LlmClient trait, verify prompt construction |
| CLI dispatch | Integration tests | Assert on exit codes and stdout for known inputs |
| Config loading | Unit tests | Temp files with known content, verify loaded values |
| End-to-end | Integration tests | Mock Confluence + mock LLM, full pipeline |

### Error Handling Pattern

Use `thiserror` for typed errors, `anyhow` at the CLI boundary:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Confluence API error: {0}")]
    Confluence(#[from] ConfluenceError),

    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    #[error("XML parsing error: {0}")]
    Xml(#[from] XmlError),

    #[error("Conversion error: {0}")]
    Conversion(#[from] ConversionError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

Library functions return `Result<T, AppError>`. The CLI `main.rs` converts to `anyhow::Result` for user-facing error display.

## Binary Size and Startup Time

**Confidence: HIGH** -- Well-understood Rust characteristics.

### Expected Binary Size

| Component | Approximate Size Impact |
|-----------|------------------------|
| Base Rust binary (no deps) | ~300 KB |
| reqwest (with rustls) | +2-4 MB |
| tokio (full) | +1-2 MB |
| serde + serde_json | +300 KB |
| quick-xml | +100 KB |
| clap (derive) | +500 KB |
| Total (release, stripped) | **5-8 MB** |

### Optimization Strategies

```toml
# Cargo.toml [profile.release]
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization, slower compile
strip = true         # Strip debug symbols
panic = "abort"      # Smaller binary (no unwinding)
```

With these settings, expect **4-6 MB** stripped release binary. This is excellent for a CLI tool.

### Startup Time

Rust binaries start in **< 5 ms** typically. The main latency sources will be:

- Config file I/O: ~1 ms (reading `~/.claude/credentials.json`)
- TLS initialization (first reqwest call): ~10-50 ms
- DNS resolution: network-dependent

**Total cold start to first useful work: < 100 ms.** This is dramatically better than the Python version which requires interpreter startup (~200-500 ms) plus dependency loading.

### Dependency Choices for Size

| Choice | Recommendation | Rationale |
|--------|---------------|-----------|
| TLS backend | `rustls` (not `native-tls`) | No OpenSSL dependency, smaller, cross-platform |
| HTTP client | `reqwest` with minimal features | Only enable `json`, `multipart`, `rustls-tls` features |
| Async runtime | `tokio` with `rt-multi-thread`, `macros` | Needed for parallel LLM calls; do not enable `full` |
| Argument parser | `clap` with `derive` | Standard, good error messages |
| Markdown parser | `pulldown-cmark` | Fast, well-maintained, handles GFM |

```toml
[dependencies]
reqwest = { version = "0.12", default-features = false, features = ["json", "multipart", "rustls-tls"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
quick-xml = { version = "0.36", features = ["serialize"] }
pulldown-cmark = "0.12"
thiserror = "2"
anyhow = "1"
dirs = "6"
base64 = "0.22"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## Claude Code Skill Integration

**Confidence: MEDIUM** -- Based on Claude Code skills documentation from training data.

### How It Works

Claude Code skills are slash commands defined in `.claude/commands/` that can invoke external tools. The binary ships as a standalone CLI and skills wrap it:

```bash
# .claude/commands/confluence-update.md
Update a markdown file to a Confluence page, preserving inline comments.

Usage: confluence-update <markdown_path> <page_url>

This runs the confluence-agent binary which:
1. Converts markdown to Confluence storage format
2. Fetches the existing page
3. Uses Claude to evaluate which inline comments to preserve
4. Updates the page with merged content

```

The key architectural implication: the binary must work identically whether invoked from a terminal or from Claude Code. This means:

- **stdout/stderr for output** (not a TUI framework)
- **Exit codes for success/failure** (0 = success, 1 = error)
- **JSON output mode** (for machine-readable results when called from skills)
- **No interactive prompts** (Claude Code skills cannot provide interactive input)

### Output Format

Support both human-readable and JSON output:

```rust
#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Human,
    Json,
}

// CLI flag: --output-format json
```

## Patterns to Follow

### Pattern 1: Builder Pattern for Pipeline Configuration

```rust
pub struct UpdatePipeline<C: ConfluenceApi, L: LlmClient, D: DiagramRenderer> {
    confluence: C,
    llm: L,
    renderer: D,
    config: Config,
}

impl<C: ConfluenceApi, L: LlmClient, D: DiagramRenderer> UpdatePipeline<C, L, D> {
    pub async fn execute(&self, markdown: &str, page_url: &str) -> Result<UpdateResult> {
        let page_id = extract_page_id(page_url)?;
        let (storage_xml, attachments) = self.convert(markdown)?;
        let existing = self.confluence.get_page(&page_id).await?;
        let final_content = self.merge_with_comments(&existing, &storage_xml).await?;
        self.confluence.upload_attachments(&page_id, &attachments).await?;
        self.confluence.update_page(&page_id, &existing.title, &final_content, existing.version + 1).await?;
        Ok(UpdateResult { title: existing.title, version: existing.version + 1 })
    }
}
```

### Pattern 2: Parallel Comment Evaluation with Bounded Concurrency

```rust
use futures::stream::{self, StreamExt};

async fn evaluate_comments_parallel(
    llm: &dyn LlmClient,
    comments: Vec<InlineComment>,
    old_content: &str,
    new_content: &str,
    max_concurrent: usize,  // e.g., 5-10
) -> Result<Vec<(InlineComment, CommentDecision)>> {
    let results: Vec<_> = stream::iter(comments)
        .map(|comment| async move {
            let context = extract_surrounding_context(old_content, &comment, 500); // chars
            let new_context = find_matching_section(new_content, &comment);
            let decision = llm.evaluate_comment(&comment, &context, &new_context).await?;
            Ok::<_, AppError>((comment, decision))
        })
        .buffer_unordered(max_concurrent)
        .collect()
        .await;

    results.into_iter().collect()
}
```

### Pattern 3: Structured Logging with `tracing`

Use `tracing` instead of `log` for structured, span-based logging:

```rust
use tracing::{info, warn, instrument};

#[instrument(skip(llm, content), fields(page_id = %page_id))]
async fn update_page(
    llm: &dyn LlmClient,
    confluence: &dyn ConfluenceApi,
    page_id: &str,
    content: &str,
) -> Result<()> {
    info!("Starting page update");
    // ... operations with automatic span context
}
```

Controlled via `RUST_LOG` environment variable (e.g., `RUST_LOG=confluence_agent=debug`).

## Anti-Patterns to Avoid

### Anti-Pattern 1: Regex-Based XML Manipulation

**What:** Using regex to find and replace XML elements in Confluence storage format.
**Why bad:** Confluence storage XML has nested elements, CDATA sections, and namespace prefixes. Regex breaks on edge cases (nested comments, multi-line attributes, CDATA containing `<`). The existing Python code uses regex for some XML operations and it is fragile.
**Instead:** Use `quick-xml` event-based parsing for all XML manipulation. Parse -> transform -> serialize.

### Anti-Pattern 2: God Module

**What:** Putting all orchestration logic in a single file (like the Python `agent.py` which handles config, LLM setup, token counting, merge, reflect, critic, and page update).
**Instead:** Each concern in its own module with a clear trait boundary. The orchestrator calls into typed interfaces, not concrete implementations.

### Anti-Pattern 3: Stringly-Typed Errors

**What:** Returning `String` error messages (the Python version returns `f"Error: {e}"`).
**Instead:** Use `thiserror` for typed error variants. Each module defines its own error type. The CLI layer formats errors for human consumption.

### Anti-Pattern 4: Blocking in Async Context

**What:** Calling synchronous I/O (file reads, subprocess for PlantUML) inside an async function without `spawn_blocking`.
**Instead:** Use `tokio::task::spawn_blocking` for file I/O and subprocess calls, or use `tokio::fs` for async file operations.

## Scalability Considerations

| Concern | Small Pages (< 5 comments) | Large Pages (50+ comments) |
|---------|----------------------------|----------------------------|
| LLM calls | Serial is fine (~3 calls) | Must be parallel with bounded concurrency |
| XML parsing | In-memory DOM OK | Streaming parser preferred |
| Memory | Negligible | Cap context windows per comment evaluation |
| Timeout | 30s total is fine | Need per-call timeout + overall deadline |

## Sources

- Confluence REST API v1 documentation: <https://developer.atlassian.com/cloud/confluence/rest/v1/>
- quick-xml crate documentation: <https://docs.rs/quick-xml/>
- roxmltree crate documentation: <https://docs.rs/roxmltree/>
- Claude Code documentation: <https://docs.anthropic.com/en/docs/claude-code>
- Existing Python implementation in this repository (analyzed directly)

**Confidence Notes:**

- Confluence REST API endpoints: HIGH (stable, well-documented API)
- Rust XML crates comparison: HIGH (well-established ecosystem)
- CLI architecture patterns: HIGH (standard Rust practices)
- Claude Code credential file format: MEDIUM (needs verification against actual installation)
- Binary size estimates: HIGH (well-understood Rust compilation characteristics)
