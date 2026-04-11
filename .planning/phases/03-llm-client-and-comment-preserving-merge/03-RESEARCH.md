# Phase 3: LLM Client and Comment-Preserving Merge - Research

**Researched:** 2026-04-10
**Domain:** Anthropic Messages API client (Rust/reqwest), XML comment extraction, per-comment parallel merge
**Confidence:** HIGH

## Summary

Phase 3 delivers two tightly coupled components: an `LlmClient` (hand-rolled Anthropic Messages API client over reqwest) and a `MergeEngine` (per-comment parallel KEEP/DROP evaluator). The Anthropic Messages API is stable and well-documented -- the `v1/messages` endpoint with `tool_use` for structured output is production-ready and the request/response formats are straightforward to model in Rust with serde. All required dependencies except `tokio::sync` are already in `Cargo.toml`; the only addition is enabling the `sync` feature on `tokio`.

The merge engine extracts `<ac:inline-comment-marker>` elements via regex (matching the Python reference pattern), splits content into heading-scoped sections, applies deterministic short-circuits for trivial cases, and fans out ambiguous comments to bounded-concurrent LLM calls via `tokio::sync::Semaphore`. Surviving markers are re-injected into new content XML by exact anchor text match.

**Primary recommendation:** Build the Anthropic client as a minimal HTTP wrapper around `POST /v1/messages` with tool_use, hand-rolling exponential backoff with jitter rather than pulling in a backoff crate. Use `tokio::sync::Semaphore` for concurrency control. Model all API types with serde structs.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

1. **Section Context Extraction**: Heading-scoped section blocks (`<h1>`-`<h6>` to next heading or end-of-document) as context for LLM calls. Deterministic short-circuits: KEEP if section content identical, DROP if section heading deleted.

2. **LLM Model**: `claude-haiku-4-5-20251001` default, configurable via `ANTHROPIC_MODEL` env var. Config priority: env var -> config field -> default.

3. **Tool_use Schema**: Single `evaluate_comment` tool with `decision` (enum KEEP/DROP) and optional `reason` field. Malformed response (no tool_use block) defaults to KEEP + warning.

4. **Comment Re-injection**: Exact anchor text match; fallback to section-start injection for LLM KEEP; drop if no matching section. RELOCATE and fuzzy matching deferred to v2.

5. **Concurrency/Retry**: Semaphore bound 5 (configurable `ANTHROPIC_CONCURRENCY`). Retry on 429, 500, 502, 503, 529. Read `retry-after` header. Backoff: 1s start, 2x multiply, +/-25% jitter, 32s max, 5 retries max. Exhaustion -> KEEP + warning.

6. **Module Layout**: `src/llm/mod.rs` + `src/llm/types.rs`; `src/merge/mod.rs` + `src/merge/extractor.rs` + `src/merge/matcher.rs` + `src/merge/injector.rs`.

7. **LlmClient Trait**: `evaluate_comment(&self, old_section, new_section: Option, marker: &CommentMarker) -> Result<CommentDecision, LlmError>`.

8. **Empty Page / No Comments Short-Circuit**: Skip entire merge engine when new content empty or existing page has no comment markers.

### Claude's Discretion

No discretion areas specified -- all decisions are locked.

### Deferred Ideas (OUT OF SCOPE)

- Batch comment evaluation (group 5-10 per LLM call) -- v2 (MERGE2-04)
- Fuzzy anchor text matching for re-injection -- v2 (MERGE2-02)
- User-visible report of dropped comments -- v2 (MERGE2-03)
- OpenAI / Gemini providers -- v2

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LLM-01 | Hand-rolled Anthropic Messages API client over reqwest | Anthropic API endpoint, headers, request/response JSON structures documented below |
| LLM-02 | Structured output via Claude tool_use for KEEP/DROP classifications | Tool definition schema and tool_use response parsing documented |
| LLM-03 | Retry with exponential backoff on rate limit (429) and transient errors | Retry-after header behavior, retryable status codes (429, 500, 502, 503, 529) documented |
| LLM-04 | LLM client is trait-based (LlmClient trait) for testability | Follows established `ConfluenceApi` / `Converter` async_trait pattern |
| MERGE-01 | Extract all ac:inline-comment-marker elements with surrounding section context | Regex pattern from Python reference, section extraction by heading tags |
| MERGE-02 | Deterministic short-circuits: unchanged section -> KEEP, deleted section -> DROP | Section comparison logic, heading matching approach documented |
| MERGE-03 | Per-comment LLM evaluation for ambiguous cases | tool_use schema, prompt structure, fail-safe KEEP on error |
| MERGE-04 | Bounded concurrency via tokio semaphore | `tokio::sync::Semaphore` pattern with `Arc` documented |
| MERGE-05 | Surviving comment markers injected back into new content XML | Exact anchor text match strategy, section-start fallback |
| MERGE-06 | Empty page / no comments -> skip merge, use new content | Early-return check before XML parsing |

</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Run `uv run black .` after Python changes (not applicable to this phase -- Rust only)
- Run `uv run mypy .` after Python changes (not applicable)
- Pin dependency versions in pyproject.toml (not applicable -- Cargo.toml uses semver ranges)
- Run `markdownlint --fix .` after markdown changes
- MyPy strict mode enforced for Python (not applicable)
- Test with `uv run pytest` for Python; this phase uses `cargo test`

## Standard Stack

### Core (already in Cargo.toml)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| reqwest | 0.13.2 | HTTP client for Anthropic API | Already in deps; async, rustls-tls [VERIFIED: Cargo.toml] |
| tokio | 1.51 | Async runtime, semaphore, sleep | Already in deps; needs `sync` feature added [VERIFIED: Cargo.toml] |
| serde / serde_json | 1.x | JSON serialization for API types | Already in deps [VERIFIED: Cargo.toml] |
| async-trait | 0.1.89 | Trait with async methods | Already in deps [VERIFIED: Cargo.toml] |
| regex | 1.12.3 | Comment marker extraction | Already in deps [VERIFIED: Cargo.toml] |
| thiserror | 2.0.18 | Error types (LlmError, MergeError) | Already in deps [VERIFIED: Cargo.toml] |
| tracing | 0.1.44 | Structured logging | Already in deps [VERIFIED: Cargo.toml] |

### Additions Required

| Library | Version | Purpose | Why Needed |
|---------|---------|---------|------------|
| tokio (feature `sync`) | 1.51 | `tokio::sync::Semaphore` for bounded concurrency | Not currently enabled [VERIFIED: Cargo.toml has rt-multi-thread, macros, process, time but not sync] |
| rand | latest | Jitter for exponential backoff | Not currently in Cargo.toml; needed for +/-25% randomization |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled backoff | `backoff` crate | Extra dependency; our retry logic is simple and specific to Anthropic status codes -- hand-rolling is ~30 lines |
| Hand-rolled Anthropic client | `anthropic-rs` / `misanthropic` | No official Rust SDK; third-party crates may lag API changes; hand-rolling gives full control over tool_use parsing |
| regex for XML extraction | `quick-xml` parser | Confluence storage XML uses `ac:` prefixes without namespace declarations -- not well-formed XML; regex is pragmatic (per CONTEXT.md decision) |

**Cargo.toml changes:**
```toml
# Modify existing tokio line to add "sync":
tokio = { version = "1.51", features = ["rt-multi-thread", "macros", "process", "time", "sync"] }

# Add rand for jitter:
rand = "0.9"
```

## Architecture Patterns

### Recommended Module Structure

```
src/
  llm/
    mod.rs        # LlmClient trait + AnthropicClient struct
    types.rs      # Request/response serde types (MessageRequest, MessageResponse, ToolUse, etc.)
  merge/
    mod.rs        # MergeEngine struct, merge() entry point
    extractor.rs  # ac:inline-comment-marker extraction (regex)
    matcher.rs    # Section extraction, heading matching, short-circuit logic
    injector.rs   # Re-injection of surviving markers into new content XML
```

### Pattern 1: Anthropic Messages API Request

**What:** POST to `https://api.anthropic.com/v1/messages` with tool definitions
**When to use:** Every LLM evaluation call

```rust
// Source: https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview
// [VERIFIED: Anthropic official docs]

#[derive(Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    tools: Vec<ToolDefinition>,
    tool_choice: ToolChoice,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct ToolDefinition {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Serialize)]
struct ToolChoice {
    #[serde(rename = "type")]
    choice_type: String,  // "tool"
    name: String,         // "evaluate_comment"
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}
```

**Required HTTP headers:**
```
x-api-key: <ANTHROPIC_API_KEY>
anthropic-version: 2023-06-01
content-type: application/json
```
[VERIFIED: Anthropic official docs -- https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview]

### Pattern 2: Parsing tool_use Response

**What:** Extract the `tool_use` content block from the API response
**When to use:** After every successful API call

```rust
// Source: https://platform.claude.com/docs/en/agents-and-tools/tool-use/handle-tool-calls
// [VERIFIED: Anthropic official docs]

#[derive(Deserialize)]
struct MessageResponse {
    id: String,
    model: String,
    stop_reason: String,
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

// Parse decision from tool_use input:
#[derive(Deserialize)]
struct EvaluateCommentInput {
    decision: String, // "KEEP" or "DROP"
    reason: Option<String>,
}
```

**Response detection:** Check `stop_reason == "tool_use"`, then find the `ContentBlock::ToolUse` variant. If no `tool_use` block found, default to KEEP + warning (MERGE-03 fail-safe).

### Pattern 3: Bounded Concurrency with Semaphore

**What:** Limit concurrent LLM calls using `tokio::sync::Semaphore`
**When to use:** When fan-out evaluating multiple comments in parallel

```rust
// Source: https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html
// [VERIFIED: tokio docs]

use std::sync::Arc;
use tokio::sync::Semaphore;

let semaphore = Arc::new(Semaphore::new(concurrency_limit)); // default 5
let mut handles = Vec::new();

for marker in ambiguous_markers {
    let sem = semaphore.clone();
    let client = llm_client.clone(); // or Arc<dyn LlmClient>
    let handle = tokio::spawn(async move {
        let _permit = sem.acquire().await.unwrap();
        client.evaluate_comment(&old_section, new_section.as_deref(), &marker).await
    });
    handles.push(handle);
}

// Collect results
for handle in handles {
    match handle.await {
        Ok(Ok(decision)) => { /* apply decision */ }
        Ok(Err(e)) => { /* LLM error: default to KEEP + warn */ }
        Err(e) => { /* JoinError: default to KEEP + warn */ }
    }
}
```

### Pattern 4: Exponential Backoff with Jitter

**What:** Retry transient HTTP errors with increasing delays
**When to use:** Inside `AnthropicClient` for each API call

```rust
// [ASSUMED] -- standard backoff pattern, not library-specific

use rand::Rng;
use std::time::Duration;

async fn request_with_retry(&self, request_body: &MessageRequest) -> Result<MessageResponse, LlmError> {
    let mut attempt = 0;
    let mut delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(32);
    let max_retries = 5;

    loop {
        let response = self.client.post(&self.endpoint)
            .headers(self.headers.clone())
            .json(request_body)
            .send()
            .await
            .map_err(LlmError::Http)?;

        let status = response.status().as_u16();

        if status == 200 {
            return response.json::<MessageResponse>()
                .await
                .map_err(LlmError::Deserialize);
        }

        if ![429, 500, 502, 503, 529].contains(&status) {
            // Non-retryable error
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError { status, body });
        }

        attempt += 1;
        if attempt > max_retries {
            return Err(LlmError::RateLimitExhausted);
        }

        // Check retry-after header (seconds)
        let retry_after = response.headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<f64>().ok())
            .map(|secs| Duration::from_secs_f64(secs));

        let wait = retry_after.unwrap_or(delay);

        // Apply +/-25% jitter
        let jitter_factor = rand::rng().random_range(0.75..=1.25);
        let jittered = wait.mul_f64(jitter_factor);

        tracing::warn!(
            status, attempt, wait_ms = jittered.as_millis() as u64,
            "Retrying Anthropic API request"
        );
        tokio::time::sleep(jittered).await;

        delay = (delay * 2).min(max_delay);
    }
}
```

### Pattern 5: Comment Marker Extraction (Regex)

**What:** Extract `<ac:inline-comment-marker>` elements from Confluence storage XML
**When to use:** First step of merge engine

```rust
// Source: src/confluence_agent/agent.py lines 91-115
// [VERIFIED: Python reference in this codebase]

use regex::Regex;

pub struct CommentMarker {
    pub full_match: String,     // The entire XML element
    pub ac_ref: String,         // The ac:ref="uuid" value
    pub anchor_text: String,    // Text wrapped by the marker (empty for self-closing)
    pub position: usize,        // Byte offset in original content
}

fn extract_markers(content: &str) -> Vec<CommentMarker> {
    // Combined regex matching both self-closing and paired forms
    let pattern = Regex::new(
        r#"(?s)<ac:inline-comment-marker\b[^>]*?/>|<ac:inline-comment-marker\b[^>]*?>.*?</ac:inline-comment-marker>"#
    ).unwrap();

    let ref_pattern = Regex::new(r#"ac:ref="([^"]+)""#).unwrap();

    // For paired tags, extract anchor text between open and close tags
    let paired_pattern = Regex::new(
        r#"(?s)<ac:inline-comment-marker\b[^>]*?>(.*?)</ac:inline-comment-marker>"#
    ).unwrap();

    // ... iterate matches, extract ac:ref and anchor text
}
```

**Key detail from Python reference:** The regex uses `(?s)` (DOTALL) flag so `.` matches newlines in paired tags where anchor text may span lines.

### Anti-Patterns to Avoid

- **Full XML parser for comment extraction:** Confluence storage XML uses `ac:` prefixes without namespace declarations. `quick-xml` will reject these unless you wrap fragments in a synthetic root with namespace declarations. Use regex instead (per CONTEXT.md locked decision).
- **Unbounded tokio::spawn fan-out:** Without a semaphore, spawning N tasks for N comments can overwhelm the Anthropic rate limit. Always acquire a permit before making the API call.
- **Retrying non-transient errors:** Only retry 429, 500, 502, 503, 529. Status 400 (bad request), 401 (auth), 403 (permission) should fail immediately.
- **Blocking on response body before checking status:** Read `status()` and headers (including `retry-after`) before consuming the response body.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async runtime | Custom event loop | `tokio` (already in deps) | Battle-tested, required by reqwest |
| HTTP client | Raw TCP/TLS | `reqwest` (already in deps) | Connection pooling, TLS, async |
| JSON serialization | Manual string building | `serde_json` (already in deps) | Type-safe, zero-copy deserialization |
| Regex engine | Custom parser | `regex` crate (already in deps) | Handles DOTALL, Unicode, no backtracking DoS |

**Key insight:** The exponential backoff retry IS worth hand-rolling (only ~30 lines) because it needs Anthropic-specific status code handling and `retry-after` header parsing. A generic backoff crate would add unnecessary abstraction.

## Common Pitfalls

### Pitfall 1: Missing `sync` Feature on Tokio

**What goes wrong:** Compilation fails with "cannot find Semaphore in tokio::sync"
**Why it happens:** `tokio::sync` module requires the `sync` Cargo feature, not enabled by default
**How to avoid:** Add `"sync"` to tokio features in Cargo.toml
**Warning signs:** Compile error mentioning `tokio::sync`

### Pitfall 2: Serde Tag Mismatch on Content Blocks

**What goes wrong:** Deserialization fails for API responses containing mixed `text` and `tool_use` blocks
**Why it happens:** The Anthropic response `content` array contains heterogeneous block types discriminated by `"type"` field
**How to avoid:** Use `#[serde(tag = "type")]` on the `ContentBlock` enum with `#[serde(rename = "text")]` and `#[serde(rename = "tool_use")]` variants
**Warning signs:** "unknown variant" or "missing field type" serde errors in tests

### Pitfall 3: Regex DOTALL for Paired Comment Markers

**What goes wrong:** Paired comment markers with multi-line anchor text are not extracted
**Why it happens:** By default, `.` in regex does not match `\n`
**How to avoid:** Use `(?s)` flag in regex pattern (Rust regex crate supports inline flags)
**Warning signs:** Markers extracted from single-line content but not from multi-line content

### Pitfall 4: Consuming Response Body Before Reading Headers

**What goes wrong:** Cannot read `retry-after` header after calling `response.json()` or `response.text()`
**Why it happens:** reqwest consumes the response body; headers are still accessible from the response object but the body is gone
**How to avoid:** Read status code and headers first, then decide whether to parse body or retry
**Warning signs:** `retry-after` header always appears as None

### Pitfall 5: Race Between Permit Acquisition and Task Spawning

**What goes wrong:** Semaphore permit acquired in outer scope, then moved into spawned task -- permit is held across the spawn boundary
**Why it happens:** The permit must be acquired inside the spawned task, not before spawning
**How to avoid:** Clone the `Arc<Semaphore>` and call `acquire().await` inside the `tokio::spawn` closure
**Warning signs:** All tasks seem to run serially, or semaphore permits are never released

### Pitfall 6: Anthropic API Version Header Drift

**What goes wrong:** API returns 400 or unexpected response format
**Why it happens:** Using wrong `anthropic-version` header value
**How to avoid:** Use `2023-06-01` which is the current stable version [VERIFIED: Anthropic docs show this in all examples as of April 2026]
**Warning signs:** 400 errors from Anthropic API

## Code Examples

### Complete Anthropic API Request Body

```json
{
  "model": "claude-haiku-4-5-20251001",
  "max_tokens": 256,
  "tool_choice": {"type": "tool", "name": "evaluate_comment"},
  "tools": [{
    "name": "evaluate_comment",
    "description": "Evaluate whether an inline comment should be kept or dropped after a content update",
    "input_schema": {
      "type": "object",
      "properties": {
        "decision": {
          "type": "string",
          "enum": ["KEEP", "DROP"],
          "description": "Whether to keep or drop the inline comment"
        },
        "reason": {
          "type": "string",
          "description": "Brief explanation for the decision (used for debugging)"
        }
      },
      "required": ["decision"]
    }
  }],
  "messages": [{
    "role": "user",
    "content": "You are evaluating whether an inline comment on a Confluence page should survive a content update.\n\n## Old Section\n<old section XML here>\n\n## New Section\n<new section XML here>\n\n## Comment\nThe comment marker wraps the text: \"anchor text here\"\n\nShould this comment be KEPT (still relevant to the updated content) or DROPPED (no longer applicable)?"
  }]
}
```
[VERIFIED: Anthropic docs -- tool_choice type "tool" forces tool use]

### Expected Successful Response

```json
{
  "id": "msg_...",
  "model": "claude-haiku-4-5-20251001",
  "stop_reason": "tool_use",
  "content": [
    {
      "type": "tool_use",
      "id": "toolu_...",
      "name": "evaluate_comment",
      "input": {"decision": "KEEP", "reason": "The anchor text is still present in the new section"}
    }
  ]
}
```
[VERIFIED: Anthropic docs -- response format with stop_reason and content blocks]

### Error Types to Add

```rust
// Extends src/error.rs
// [ASSUMED] -- follows existing error pattern in codebase

#[derive(Debug, Error)]
pub enum LlmError {
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("Failed to deserialize Anthropic API response: {0}")]
    Deserialize(reqwest::Error),

    #[error("Anthropic API key not configured. Set ANTHROPIC_API_KEY or add to ~/.claude/settings.json")]
    MissingApiKey,

    #[error("Anthropic API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },

    #[error("Rate limit exhausted after {max_retries} retries")]
    RateLimitExhausted { max_retries: u32 },

    #[error("Malformed tool_use response: {0}")]
    MalformedResponse(String),
}

#[derive(Debug, Error)]
pub enum MergeError {
    #[error(transparent)]
    Llm(#[from] LlmError),

    #[error("Comment extraction failed: {0}")]
    ExtractionError(String),

    #[error("Comment injection failed: {0}")]
    InjectionError(String),
}
```

### Section Extraction Pattern

```rust
// [ASSUMED] -- standard approach for heading-to-heading section splitting

pub struct Section {
    pub heading: String,       // e.g., "Introduction"
    pub heading_level: u8,     // 1-6
    pub content: String,       // Full HTML from heading tag to next heading
    pub start_offset: usize,   // Byte offset in original content
    pub end_offset: usize,
}

fn extract_sections(html: &str) -> Vec<Section> {
    let heading_pattern = Regex::new(r"<h([1-6])\b[^>]*>(.*?)</h\1>").unwrap();
    let mut sections = Vec::new();
    let mut last_end = 0;

    let matches: Vec<_> = heading_pattern.find_iter(html).collect();
    for (i, m) in matches.iter().enumerate() {
        let end = matches.get(i + 1).map_or(html.len(), |next| next.start());
        let heading_caps = heading_pattern.captures(&html[m.start()..m.end()]).unwrap();
        sections.push(Section {
            heading: heading_caps[2].to_string(),
            heading_level: heading_caps[1].parse().unwrap(),
            content: html[m.start()..end].to_string(),
            start_offset: m.start(),
            end_offset: end,
        });
    }
    sections
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Anthropic SDK (Python only) | Hand-rolled reqwest client | Ongoing (no official Rust SDK) | Must model API types manually with serde |
| `anthropic-version: 2023-06-01` | Same header still current | 2023 | Stable; no breaking version changes |
| tool_use beta header required | tool_use is GA (no beta header) | 2024 | Simpler request headers |

**Deprecated/outdated:**
- `anthropic-beta: tools-2024-04-04` header: No longer needed; tool_use is GA [VERIFIED: Anthropic docs -- no beta header in current examples]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `rand` crate version 0.9 is current | Standard Stack | Low -- version can be corrected at install time |
| A2 | `rand::rng().random_range()` API for jitter | Code Examples | Low -- API may differ slightly; adjust at implementation |
| A3 | Heading regex `<h([1-6])\b[^>]*>(.*?)</h\1>` captures all Confluence headings | Code Examples | Medium -- Confluence might use attributes or nested elements in headings; test against real page XML |
| A4 | 256 max_tokens sufficient for tool_use KEEP/DROP response | Code Examples | Low -- Haiku's tool_use response is very short; 256 is generous |

## Open Questions

1. **Prompt engineering for KEEP/DROP evaluation**
   - What we know: Need to pass old section, new section, and comment marker to the LLM
   - What's unclear: Exact system prompt / user prompt wording for optimal classification accuracy
   - Recommendation: Start with a simple prompt, iterate based on test results. Not a research blocker -- the planner can specify initial prompt text and refine during implementation.

2. **`tool_choice: {"type": "tool", "name": "evaluate_comment"}` token cost**
   - What we know: Forcing tool use adds ~313 system prompt tokens (Haiku) [VERIFIED: Anthropic docs pricing table]
   - What's unclear: Whether per-call overhead is acceptable for pages with many comments
   - Recommendation: Acceptable -- 313 tokens overhead is trivial compared to section context (~500-1000 tokens). Short-circuits eliminate most LLM calls anyway.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All compilation | Assumed available | 1.80+ (rust-version in Cargo.toml) | -- |
| tokio (sync feature) | Semaphore | Needs feature addition | 1.51 | -- |
| rand crate | Jitter | Needs addition to Cargo.toml | -- | Could use simple modular arithmetic instead |

**Missing dependencies with no fallback:** None -- all are addable via Cargo.toml.

**Missing dependencies with fallback:**
- `rand` crate: If avoiding a new dependency is preferred, jitter can be approximated using `std::time::SystemTime::now()` nanoseconds modulo. However, `rand` is a standard Rust crate and the cleaner approach.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) + wiremock 0.6.5 + insta 1.47.2 |
| Config file | Cargo.toml [dev-dependencies] |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LLM-01 | Anthropic client sends correct request format | integration (wiremock) | `cargo test --test llm_integration` | Wave 0 |
| LLM-02 | tool_use response parsed to KEEP/DROP | unit | `cargo test llm::tests` | Wave 0 |
| LLM-03 | Retry on 429/5xx with backoff | integration (wiremock) | `cargo test --test llm_integration` | Wave 0 |
| LLM-04 | LlmClient trait mockable | unit | `cargo test llm::tests::test_mock_client` | Wave 0 |
| MERGE-01 | Extract comment markers from XML | unit | `cargo test merge::extractor::tests` | Wave 0 |
| MERGE-02 | Short-circuit KEEP/DROP | unit | `cargo test merge::matcher::tests` | Wave 0 |
| MERGE-03 | LLM evaluation for ambiguous comments | unit (mock LlmClient) | `cargo test merge::tests` | Wave 0 |
| MERGE-04 | Bounded concurrency | integration | `cargo test merge::tests::test_bounded_concurrency` | Wave 0 |
| MERGE-05 | Comment re-injection | unit | `cargo test merge::injector::tests` | Wave 0 |
| MERGE-06 | Empty page skip | unit | `cargo test merge::tests::test_empty_skip` | Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `src/llm/mod.rs` -- LlmClient trait + AnthropicClient + tests
- [ ] `src/llm/types.rs` -- serde types for API request/response
- [ ] `src/merge/mod.rs` -- MergeEngine entry point + tests
- [ ] `src/merge/extractor.rs` -- Comment extraction + tests
- [ ] `src/merge/matcher.rs` -- Section extraction + matching + tests
- [ ] `src/merge/injector.rs` -- Comment re-injection + tests
- [ ] `tests/llm_integration.rs` -- wiremock-based integration tests for Anthropic client

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | API key passed via `x-api-key` header; never logged; loaded from Config |
| V3 Session Management | no | Stateless API calls |
| V4 Access Control | no | Single-user CLI tool |
| V5 Input Validation | yes | Validate API response JSON structure; reject malformed responses gracefully |
| V6 Cryptography | no | TLS handled by reqwest/rustls; no custom crypto |

### Known Threat Patterns for Anthropic API Client

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| API key exposure in logs | Information Disclosure | Never log the `x-api-key` header value; use `tracing` with redacted fields |
| Prompt injection via comment content | Tampering | Comment anchor text is passed to LLM; use tool_use structured output to constrain response to KEEP/DROP enum |
| Rate limit exhaustion as DoS | Denial of Service | Semaphore bound + exponential backoff + max retries |

## Sources

### Primary (HIGH confidence)
- [Anthropic Tool Use Overview](https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview) -- API endpoint, headers, request format
- [Anthropic Define Tools](https://platform.claude.com/docs/en/agents-and-tools/tool-use/define-tools) -- Tool definition schema, tool_choice parameter
- [Anthropic Handle Tool Calls](https://platform.claude.com/docs/en/agents-and-tools/tool-use/handle-tool-calls) -- Response parsing, tool_use content blocks
- [Anthropic Errors](https://platform.claude.com/docs/en/api/errors) -- HTTP status codes (429, 529, etc.)
- [Anthropic Rate Limits](https://docs.anthropic.com/en/api/rate-limits) -- retry-after header behavior
- [Tokio Semaphore docs](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) -- Semaphore API, sync feature requirement
- `src/confluence_agent/agent.py` lines 91-115 -- Python comment extraction regex patterns

### Secondary (MEDIUM confidence)
- [Rust Concurrency Patterns (OneSignal)](https://onesignal.com/blog/rust-concurrency-patterns/) -- Semaphore patterns
- [backoff crate](https://github.com/ihrwein/backoff) -- Alternative retry approach (decided against)

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in Cargo.toml except tokio sync feature and rand
- Architecture: HIGH -- module layout locked in CONTEXT.md, trait patterns established in Phases 1-2
- Pitfalls: HIGH -- verified against official docs and codebase patterns
- API format: HIGH -- verified against current Anthropic official documentation

**Research date:** 2026-04-10
**Valid until:** 2026-05-10 (Anthropic API is stable; 30-day validity)
