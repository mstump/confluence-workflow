# Technology Stack

**Project:** Confluence Agent (Rust rewrite)
**Researched:** 2026-04-10
**Note:** WebSearch and WebFetch were unavailable during research. Findings are based on training data (cutoff ~mid-2025) and a confirmed 404 on `github.com/anthropics/anthropic-sdk-rust`. All recommendations should be verified against crates.io before pinning versions.

## Recommended Stack

### Anthropic Claude API Client

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **reqwest + serde (hand-rolled client)** | reqwest ~0.12, serde ~1.0 | Anthropic Messages API | No official Rust SDK exists (confirmed: `anthropic-sdk-rust` returns 404 on GitHub). Community crates (`anthropic-rs`, `misanthropic`) exist but are maintained by individuals, lag behind API changes, and add a dependency you can own yourself. The Anthropic Messages API is a single POST endpoint with well-documented JSON schemas -- building a thin typed wrapper is ~200 lines of Rust and gives full control over streaming, retries, and new features. |

**Confidence: HIGH** (404 confirmed no official SDK; Anthropic's official SDK list as of training data: Python, TypeScript, Java, Go -- no Rust)

**Community crate assessment (LOW confidence -- could not verify current state):**

| Crate | Notes | Recommendation |
|-------|-------|----------------|
| `anthropic-rs` (abdelhamidbakhta) | Was last meaningfully updated ~2024. Wraps older API versions. | Do not use -- likely stale |
| `misanthropic` (mdegans) | More recent, supports Messages API, streaming. Small maintainer bus-factor. | Evaluate if still maintained, but prefer hand-rolled |
| `clust` | Another community option. | Same bus-factor concerns |

**Recommendation: Hand-roll a thin async client.** The Anthropic Messages API is stable and simple:

```rust
// Core types needed (~150 lines)
#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    system: Option<String>,
}

#[derive(Deserialize)]
struct MessagesResponse {
    id: String,
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: Usage,
}
```

### Core Framework

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| tokio | ~1.37+ | Async runtime | The Rust async runtime. Used by reqwest, virtually all async crates. No real alternative for production use. |
| reqwest | ~0.12 | HTTP client | Mature, async, built on tokio+hyper. Handles TLS, connection pooling, timeouts. Used for both Anthropic API and Confluence REST API. |
| serde + serde_json | ~1.0 | Serialization | The Rust serialization standard. Zero real alternatives for JSON work. |

**Confidence: HIGH** (these are the undisputed Rust ecosystem standards)

### CLI Framework

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **clap** (with derive) | ~4.5+ | CLI argument parsing | Use clap. It is the dominant Rust CLI framework by an enormous margin. Derive macros make it ergonomic. Supports subcommands, env var fallbacks, shell completions, help generation. |

**Confidence: HIGH**

**Alternatives considered:**

| Crate | Why Not |
|-------|---------|
| `argh` (Google) | Minimalist, no color output, no env var support, fewer features. Good for embedded/tiny tools, wrong choice for a feature-rich CLI. |
| `bpaf` | Interesting combinator approach but much smaller community. |
| `pico-args` | Zero-dependency but manual -- no derive, no help generation. |

**Verdict:** clap with derive macros. The `update`, `upload`, `convert` subcommand pattern maps perfectly to clap's subcommand derive.

### Configuration and Credentials

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| serde_json | ~1.0 | Parse ~/.claude/ config JSON | Claude Code stores credentials as JSON files. serde_json reads them natively. |
| dirs | ~5.0 | Resolve home directory cross-platform | `dirs::home_dir()` handles ~ expansion on macOS/Linux/Windows. |
| dotenvy | ~0.15 | .env file loading | For local development environment variables (API keys, Confluence credentials). |

**Claude Code credential format** (LOW confidence -- format may have changed):
Claude Code stores API keys in `~/.claude/` as JSON. The exact file structure needs verification at implementation time. Likely `~/.claude/credentials.json` or similar. Plan to support both file-based config and environment variables (`ANTHROPIC_API_KEY`).

### Markdown to HTML/XML Conversion

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **pulldown-cmark** | ~0.11+ | Markdown parsing | The standard Rust markdown parser. CommonMark compliant, fast, streaming, well-maintained (raphlinus/pulldown-cmark). |
| **pulldown-cmark-to-cmark** | ~15.0+ | Markdown round-tripping | If you need to modify and re-emit markdown. |

**Confidence: MEDIUM** (pulldown-cmark is definitely the standard; version numbers from training data may be slightly off)

**Note on Confluence storage format:** pulldown-cmark outputs HTML. Confluence uses "storage format" which is XHTML with custom `ac:*` and `ri:*` namespaced elements. You will need a post-processing step to:

1. Convert pulldown-cmark HTML output to XHTML
2. Inject Confluence-specific macros (code blocks, info panels, etc.)
3. Handle `ac:image` and `ac:structured-macro` elements

For XHTML cleanup, consider:

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **quick-xml** | ~0.36+ | XML parsing/writing | Fast, low-allocation XML handling. Good for manipulating Confluence storage format XML. Preferred over xml-rs for performance. |

### Async Parallel LLM Calls

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **tokio** (JoinSet) | ~1.37+ | Parallel task spawning | `tokio::task::JoinSet` is the modern way to spawn and collect parallel async tasks. Better than raw `tokio::spawn` + manual join. |
| **futures** | ~0.3 | Stream utilities | `futures::stream::FuturesUnordered` for bounded concurrency over many LLM calls. |
| **tokio::sync::Semaphore** | (part of tokio) | Rate limiting | Bound concurrent API calls to avoid Anthropic rate limits. Essential for parallel comment evaluation. |

**Confidence: HIGH** (well-established patterns)

**Best practice for parallel LLM calls:**

```rust
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

// Limit concurrent API calls (Anthropic rate limits)
let semaphore = Arc::new(Semaphore::new(5)); // 5 concurrent calls
let mut join_set = JoinSet::new();

for comment in comments {
    let sem = semaphore.clone();
    let client = client.clone();
    join_set.spawn(async move {
        let _permit = sem.acquire().await.unwrap();
        client.evaluate_comment(comment).await
    });
}

// Collect results
let mut results = Vec::new();
while let Some(result) = join_set.join_next().await {
    results.push(result??);
}
```

**Alternative pattern** using `futures::stream`:

```rust
use futures::stream::{self, StreamExt};

let results: Vec<Result<_, _>> = stream::iter(comments)
    .map(|comment| {
        let client = client.clone();
        async move { client.evaluate_comment(comment).await }
    })
    .buffer_unordered(5) // max 5 concurrent
    .collect()
    .await;
```

Both patterns work. `JoinSet` is better when tasks may be heterogeneous or you want to cancel all on first error. `buffer_unordered` is more ergonomic for homogeneous map-style parallelism.

### PlantUML Rendering

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **std::process::Command** | (stdlib) | Call PlantUML jar | Simple, no dependency. Matches the existing Python approach (`subprocess`). |
| **tokio::process::Command** | (part of tokio) | Async jar invocation | Non-blocking process spawning. Use this since the rest of the app is async. Avoids blocking the tokio runtime. |

**Confidence: HIGH**

**Two approaches:**

1. **Local JAR** (recommended for CLI): `tokio::process::Command::new("java").args(["-jar", "plantuml.jar", "-tsvg", "-pipe"])` with stdin/stdout piping.
2. **HTTP render server**: Hit a PlantUML server via reqwest. Better for server deployments but adds an external dependency.

Recommend supporting both via a config flag, defaulting to local JAR.

### Error Handling

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **thiserror** | ~2.0 | Error type definitions | Derive macro for clean error enums. The standard for library-style error types. |
| **anyhow** | ~1.0 | Application error handling | For the CLI binary top-level. Wraps any error with context. Use in `main()` and CLI handlers. |
| **color-eyre** | ~0.6 | Pretty error reports | Alternative to anyhow with colored backtraces. Nice for CLI UX. Choose one of anyhow or color-eyre, not both. |

**Recommendation:** `thiserror` for internal error types, `anyhow` for the CLI binary. Skip `color-eyre` unless you want fancy error output.

**Confidence: HIGH**

### Logging and Output

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **tracing** | ~0.1 | Structured logging | The modern Rust logging framework. Async-aware, structured, span-based. Superior to `log` crate for async applications. |
| **tracing-subscriber** | ~0.3 | Log output formatting | Configurable formatting, filtering (RUST_LOG env var), JSON output option. |
| **indicatif** | ~0.17 | Progress bars | For showing LLM call progress, upload progress. Good CLI UX. |

**Confidence: HIGH**

### Testing

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **tokio::test** | (part of tokio) | Async test runtime | `#[tokio::test]` attribute for async test functions. |
| **wiremock** | ~0.6 | HTTP mocking | Mock Anthropic and Confluence API responses. Spins up a real HTTP server. |
| **insta** | ~1.39 | Snapshot testing | Excellent for testing LLM prompt construction and XML/HTML output. |
| **assert_cmd** | ~2.0 | CLI integration tests | Test the compiled binary end-to-end. |

**Confidence: MEDIUM** (versions approximate)

## Full Dependency Summary

### Production Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive", "env"] }
pulldown-cmark = "0.11"
quick-xml = "0.36"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dirs = "5"
dotenvy = "0.15"
indicatif = "0.17"
futures = "0.3"
```

### Dev Dependencies

```toml
[dev-dependencies]
wiremock = "0.6"
insta = { version = "1", features = ["json"] }
assert_cmd = "2"
tokio-test = "0.4"
```

**IMPORTANT:** Pin exact versions in Cargo.lock (automatic) but use semver ranges in Cargo.toml. Verify all version numbers against crates.io before starting -- training data versions may be slightly off.

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Anthropic client | Hand-rolled reqwest | `misanthropic`, `anthropic-rs` | Bus-factor risk, may lag API updates, unnecessary abstraction over simple REST API |
| HTTP client | reqwest | surf, ureq, hyper (raw) | reqwest is the ecosystem standard; ureq is sync-only; surf is less maintained; raw hyper is too low-level |
| Async runtime | tokio | async-std, smol | tokio has overwhelming ecosystem dominance; reqwest requires tokio |
| CLI | clap (derive) | argh, bpaf, structopt | clap is feature-complete and dominant; structopt merged into clap 3+; argh too minimal |
| Markdown | pulldown-cmark | comrak, markdown-rs | pulldown-cmark is faster and more widely used; comrak supports GFM but heavier; either works |
| XML | quick-xml | xml-rs, roxmltree | quick-xml is fastest for read/write; roxmltree is read-only |
| Errors | thiserror + anyhow | eyre, snafu | thiserror+anyhow is the most common pattern; snafu is heavier |
| Logging | tracing | log + env_logger | tracing is superior for async code; log is simpler but lacks spans |

## Architecture Notes

### Credential Loading Priority

1. `ANTHROPIC_API_KEY` environment variable (highest priority)
2. `.env` file in project directory
3. `~/.claude/` config files (parse JSON, extract API key)

The `~/.claude/` format should be reverse-engineered at implementation time. Plan for it to be a JSON file containing an `api_key` or `apiKey` field, but verify.

### reqwest Client Reuse

Create a single `reqwest::Client` instance and clone it (cheap Arc clone) across all async tasks. Do NOT create a new client per request -- this wastes TLS handshakes and connection pools.

```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(120)) // LLM calls can be slow
    .build()?;
```

### Streaming Responses

The Anthropic API supports Server-Sent Events (SSE) streaming. For long LLM responses, streaming improves UX (show partial output) and avoids timeouts. Use `reqwest`'s `bytes_stream()` and parse SSE events manually or use the `eventsource-stream` crate (~0.2).

Consider adding `eventsource-stream` if you want streaming:

```toml
eventsource-stream = "0.2"
```

### Structured Output

The Anthropic API supports tool_use for structured output. Define Rust types with serde, serialize the JSON schema, and pass as a tool definition. Parse the tool_use response back into your Rust type. This replaces the Pydantic models (`ConfluenceContent`, `CriticResponse`) from the Python version.

## Sources

- GitHub 404 confirmed: No official `anthropics/anthropic-sdk-rust` repository
- Training data knowledge (cutoff ~mid-2025) for crate ecosystem -- marked as MEDIUM/LOW confidence where noted
- Existing project `pyproject.toml` for understanding current Python architecture being ported

## Verification TODOs

Before starting implementation, verify on crates.io:

- [ ] `pulldown-cmark` latest version and API stability
- [ ] `quick-xml` latest version
- [ ] `clap` 4.x latest version
- [ ] `reqwest` 0.12.x latest version
- [ ] `tracing` and `tracing-subscriber` latest versions
- [ ] Check if any official Anthropic Rust SDK has appeared since mid-2025
- [ ] Verify `~/.claude/` credential file format on your local machine
- [ ] Check `misanthropic` crate status -- if actively maintained and up-to-date, it could save time vs hand-rolling
