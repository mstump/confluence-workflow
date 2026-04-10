# Domain Pitfalls

**Domain:** Rust rewrite of Python Confluence+LLM CLI tool
**Researched:** 2026-04-10
**Overall confidence:** MEDIUM (based on training data; WebSearch/WebFetch unavailable for live verification)

---

## Critical Pitfalls

Mistakes that cause rewrites, data loss, or major schedule blowouts.

### Pitfall 1: Confluence Version Conflict on Page Update (TOCTOU Race)

**What goes wrong:** The current Python code fetches page version N, does LLM processing (which can take 30-90 seconds across three LLM calls), then updates with version N+1. If anyone edits the page during that window, the update fails with a 409 Conflict, or worse -- silently overwrites their changes if the API does not enforce optimistic locking correctly.

**Why it happens:** The Confluence REST API uses optimistic concurrency control via the `version.number` field. You must send `version.number = current + 1` in the PUT body. If someone else incremented the version in the meantime, the API returns 409. The current codebase (`agent.py` line 365) increments version but has no retry-with-re-fetch logic.

**Consequences:** Lost human edits on the Confluence page. Users lose trust in the tool. In Rust, this is equally dangerous because the async processing window may be even longer if you add per-comment parallel evaluation.

**Prevention:**

- Implement a retry loop: on 409, re-fetch page content and version, re-run merge (or at minimum re-attempt the update with fresh version).
- Consider a "compare-and-swap" approach: after LLM processing, re-fetch the page, diff against the original snapshot, and abort if content changed meaningfully.
- Add a `--force` flag for explicit override, defaulting to safe behavior.

**Detection:** Monitor for 409 responses. Log the time delta between fetch and update.

---

### Pitfall 2: Confluence Storage Format XML Is Not Real XML

**What goes wrong:** Confluence "storage format" uses custom XML-like namespaced elements (`ac:structured-macro`, `ac:inline-comment-marker`, `ri:attachment`) that are NOT valid standalone XML. Standard XML parsers choke on them because the namespace prefixes are not declared in the document. The current Python code already works around this with regex (`converter.py`, `agent.py`), but a Rust rewrite may tempt developers to use a proper XML parser.

**Why it happens:** Confluence storage format is a fragment embedded in a larger XHTML context. The `ac:` and `ri:` namespace prefixes are defined at the Confluence application level, not in the fragment itself. Libraries like `quick-xml` or `xml-rs` in Rust will reject these fragments unless you pre-wrap them in a root element with namespace declarations.

**Consequences:** Parse failures on valid Confluence content. Silent data corruption if you escape/unescape incorrectly. The inline comment markers (`ac:inline-comment-marker`) are the most fragile -- they must be preserved byte-for-byte.

**Prevention:**

- Use string-based manipulation (regex) for targeted operations, just like the current Python code does.
- If you must parse as XML, wrap fragments in a synthetic root: `<root xmlns:ac="..." xmlns:ri="...">CONTENT</root>`, parse, then unwrap.
- Write property-based tests: roundtrip real Confluence storage fragments through your parser and assert byte-level equality.

**Detection:** Test with real Confluence page exports, especially pages with inline comments, macros, and attachments.

---

### Pitfall 3: Async Python to Async Rust Is Not a 1:1 Translation

**What goes wrong:** Developers assume `asyncio` patterns map directly to `tokio`. They do not. Key differences:

- Python's `asyncio` is single-threaded by default; `tokio` is multi-threaded. Data shared across `.await` points needs `Send + Sync` bounds in Rust.
- Python freely mixes sync and async code (blocking in async is bad practice but works). Rust's tokio will deadlock if you call blocking code on the async runtime without `spawn_blocking`.
- Python's dynamic typing means you never fight the borrow checker. Rust async closures and callbacks are notoriously difficult with lifetimes.

**Why it happens:** Surface-level API similarity (both have `async/await`) masks fundamental runtime model differences.

**Consequences:** Weeks of fighting compiler errors. Overuse of `Arc<Mutex<T>>` leading to poor ergonomics. Or the opposite: abandoning async for sync, losing concurrency benefits needed for parallel LLM calls.

**Prevention:**

- Design data flow to minimize shared mutable state. Use message passing (channels) between async tasks rather than shared state.
- Accept `clone()` for small config/settings structs rather than fighting lifetime annotations.
- Use `reqwest` (built on tokio/hyper) for HTTP; it handles async natively and the API is similar to Python's `requests`/`httpx`.
- Start with synchronous Rust and add async only where needed (HTTP calls, parallel LLM evaluation).

**Detection:** If you find yourself writing `Arc<Mutex<Arc<...>>>`, step back and redesign.

---

### Pitfall 4: Anthropic API Streaming SSE Parsing Is Tricky

**What goes wrong:** The Anthropic Messages API uses Server-Sent Events (SSE) for streaming. Each event has a `type` field (`message_start`, `content_block_start`, `content_block_delta`, `content_block_stop`, `message_delta`, `message_stop`). Implementers commonly:

- Fail to handle `content_block_delta` events that split mid-UTF8 codepoint
- Miss that tool use responses come as separate content blocks with `type: "tool_use"` rather than `type: "text"`
- Forget to accumulate `usage` from both `message_start` (input tokens) and `message_delta` (output tokens)
- Mishandle the `overloaded` error event type, which can appear mid-stream

**Why it happens:** SSE looks simple but the Anthropic event schema has multiple event types with different payload structures. Most SSE libraries give you raw events; you must implement the state machine yourself.

**Consequences:** Corrupted responses, missing tool call results, incorrect token counting, silent data loss on overload events.

**Prevention:**

- Define an explicit state machine for SSE event processing: `Idle -> MessageStarted -> InContentBlock -> ContentBlockDone -> ... -> MessageComplete`.
- For non-streaming (simpler, recommended for this use case): just POST and parse the JSON response. The three-call LLM chain does not benefit from streaming since each call must complete before the next begins.
- Use `reqwest` + manual SSE parsing or the `eventsource-client` crate if streaming is needed.

**Detection:** Test with responses that include tool_use blocks, multi-block responses, and error events.

---

### Pitfall 5: Rust Ecosystem Lacks a `markdown-to-confluence` Equivalent

**What goes wrong:** The Python codebase uses `markdown-to-confluence` (md2conf) to convert Markdown to Confluence storage format. There is no equivalent Rust crate. `pulldown-cmark` handles Markdown-to-HTML, but Confluence storage format is NOT HTML -- it uses `ac:structured-macro`, `ac:rich-text-body`, `ri:attachment`, and other custom elements.

**Why it happens:** Confluence storage format is a niche output format. The Rust ecosystem has excellent Markdown parsing but no Confluence-specific rendering.

**Consequences:** You must write a custom Confluence storage format renderer on top of `pulldown-cmark` events. This is weeks of work to handle: code blocks (mapped to `ac:structured-macro ac:name="code"`), images (mapped to `ac:image` with `ri:attachment`), tables, links, and the various post-processing steps currently in `converter.py`.

**Prevention:**

- Option A: Keep the Python converter as a subprocess/sidecar and have Rust call it. This is the pragmatic choice for phase 1.
- Option B: Write a custom `pulldown-cmark` event visitor that emits Confluence storage XML. Budget 2-3 weeks for this.
- Option C: Use `comrak` (CommonMark in Rust) with a custom formatter plugin.

**Detection:** Early spike: attempt to convert the 5 most complex test Markdown files and compare output against the Python converter.

---

## Moderate Pitfalls

### Pitfall 6: Confluence Cloud vs Server API Differences

**What goes wrong:** The current code hardcodes `cloud=True` in the Confluence client. The Confluence REST API has significant differences between Cloud and Server/Data Center:

- Cloud uses `/wiki/rest/api/content`, Server may use `/rest/api/content`
- Cloud authentication is email + API token (Basic auth). Server supports Personal Access Tokens (Bearer) and may still use session cookies.
- Cloud has different rate limits (undocumented but aggressive, especially on free tiers).
- The V2 API (Cloud only) uses different endpoints and response shapes.

**Prevention:** If Server support is ever needed, abstract the API client behind a trait with Cloud and Server implementations. For now, document the Cloud-only assumption prominently.

---

### Pitfall 7: Confluence Attachment Upload Quirks

**What goes wrong:** The current Python code (`confluence.py` line 91-93) iterates attachments and uploads via `attach_file`. Several known issues:

- Uploading an attachment with the same filename replaces the existing one, but the old version remains (Confluence keeps attachment history). This can bloat space storage.
- The `attach_file` API uses `multipart/form-data` with a specific field name (`file`). Getting the multipart encoding wrong in Rust silently fails or returns 400.
- Attachment size limits differ between Cloud tiers (free: 250MB, standard: 250MB, premium: 250MB per file but different total limits).
- The `X-Atlassian-Token: nocheck` header is REQUIRED for attachment uploads to bypass XSRF protection. Missing it returns 403.

**Prevention:**

- Use `reqwest::multipart::Form` in Rust, and always set `X-Atlassian-Token: nocheck`.
- Add content-type detection (the current code passes `content_type="auto"`; in Rust, use `mime_guess` crate).
- Test with SVG files specifically -- Confluence sometimes rejects SVGs that contain script elements.

---

### Pitfall 8: Token Cost Explosion with Per-Comment Parallel Evaluation

**What goes wrong:** The new architecture fans out one LLM call per inline comment. Each call includes context (the page content, the comment, evaluation criteria). If a page has 50 inline comments and each evaluation prompt is 2K tokens input + 500 tokens output, that is 50 * 2,500 = 125K tokens per page update. At Anthropic's Claude pricing, this adds up fast.

**Why it happens:** The fan-out pattern trades latency for cost. Each call is independent, so parallelism is natural, but the total token consumption scales linearly with comment count.

**Consequences:**

- Unexpected API bills. A page with 100 comments costs 40-100x more than the current 3-call pipeline.
- Hitting rate limits (Anthropic enforces both requests-per-minute and tokens-per-minute limits).

**Prevention:**

- Batch comments into groups of 5-10 per LLM call. Each call evaluates multiple comments. This reduces the number of calls from N to N/5-10.
- Set a configurable comment count threshold. Above it, switch to batch mode or warn the user.
- Implement a cost estimator that calculates expected token usage before execution and prompts for confirmation.
- Cache evaluation results: if a comment was already evaluated against identical page content, skip re-evaluation.

---

### Pitfall 9: Rate Limit Handling for Parallel LLM Fan-Out

**What goes wrong:** Anthropic's API returns HTTP 429 with a `retry-after` header when rate limited. When fanning out N parallel requests, you can hit the rate limit on request #3, causing requests #4-N to also fail. Naive retry logic retries all failed requests simultaneously, causing a "thundering herd" that triggers more 429s.

**Why it happens:** Rate limits are per-API-key (not per-request). Parallel requests share the same budget.

**Consequences:** Cascade failures. Wasted tokens on partial completions that get cut off. Slow total execution due to backoff.

**Prevention:**

- Use a semaphore/token bucket to limit concurrency (e.g., max 5 concurrent LLM calls).
- Implement exponential backoff with jitter on 429 responses.
- In Rust, use `tokio::sync::Semaphore` to gate concurrent requests.
- Read the `retry-after` header and respect it (it tells you exactly how long to wait).
- Consider a priority queue: evaluate the most important/recent comments first.

**Detection:** Log rate limit hits. Alert if more than 10% of requests get 429.

---

### Pitfall 10: Partial Failure Handling in Fan-Out

**What goes wrong:** When evaluating 20 comments in parallel, 18 succeed and 2 fail (timeout, rate limit, malformed response). The question: what do you do with the page update?

**Why it happens:** Network errors, transient API issues, malformed LLM responses (fails structured output validation).

**Consequences:**

- If you require all-or-nothing: one flaky comment evaluation blocks the entire update.
- If you proceed with partial results: you might remove resolved comments that should have been kept, or keep stale comments.

**Prevention:**

- Default to "proceed with successful evaluations, keep failed-to-evaluate comments unchanged" (conservative approach).
- Report which comments could not be evaluated and why.
- Allow retry of just the failed evaluations with `--retry-failed`.
- Set a threshold: if more than 30% of evaluations fail, abort the entire operation.

---

### Pitfall 11: Rust Cross-Compilation and Binary Distribution

**What goes wrong:** The promise of Rust is "single binary, no runtime." But:

- Cross-compiling for Linux from macOS requires `cross` or Docker-based toolchains.
- OpenSSL linkage is the #1 cross-compilation headache. `reqwest` defaults to native TLS, which pulls in system OpenSSL.
- Binary sizes with Rust are 10-30MB for a CLI tool (compared to a Python wheel + interpreter).
- Apple Silicon (aarch64-darwin) vs Intel Mac (x86_64-darwin) vs Linux (x86_64-unknown-linux-gnu/musl) = 3+ targets minimum.

**Prevention:**

- Use `rustls` instead of native-tls for `reqwest`. This eliminates OpenSSL dependency entirely: `reqwest = { features = ["rustls-tls"], default-features = false }`.
- Use `cargo-zigbuild` or `cross` for cross-compilation CI.
- Strip binaries and use `lto = true` in release profile to reduce size.
- Distribute via GitHub Releases with platform-specific binaries + a shell installer script.

---

### Pitfall 12: Structured Output from Claude via Raw HTTP

**What goes wrong:** Getting reliable JSON from Claude without an SDK requires careful prompt engineering and response parsing. The Anthropic API supports tool use (function calling), which is the most reliable way to get structured output. But:

- Tool use responses come as `content` blocks with `type: "tool_use"`, not `type: "text"`. You must handle both.
- The `input` field in a tool_use block is a JSON object, but it may contain escaped strings that need double-parsing.
- If you use prompt-based JSON extraction (without tool use), Claude may wrap JSON in markdown code fences or add preamble text.

**Why it happens:** LLMs are not deterministic JSON generators. Tool use is the API-level mechanism to force structured output, but it adds complexity to request/response handling.

**Consequences:** Flaky parsing. The current Python code already has a `_generate_structured_with_retry` function that retries on ValidationError -- this pattern must be preserved in Rust.

**Prevention:**

- Use tool use (function calling) for all structured output. Define tools with JSON Schema matching your Pydantic models (`ConfluenceContent`, `CriticResponse`).
- Parse the `tool_use` content block's `input` field directly as your target struct (serde_json).
- Implement retry with backoff on deserialization failures (3 attempts, same as current Python code).
- Use `serde` with `#[serde(deny_unknown_fields)]` to catch schema drift early.

---

## Minor Pitfalls

### Pitfall 13: Rust String Handling for Confluence Content

**What goes wrong:** Confluence storage format contains HTML entities, CDATA sections, and mixed UTF-8 content. Rust's `String` is always valid UTF-8, which is good, but regex operations on large XML strings are slower than expected if you use the `regex` crate's Unicode mode (default).

**Prevention:** Use `regex::bytes` for performance-critical regex operations on large pages. For the inline comment marker extraction, the current regex patterns will work in Rust's `regex` crate with minimal modification.

---

### Pitfall 14: Missing Python Library Equivalents in Rust

**What goes wrong:** Key Python dependencies that need Rust replacements:

| Python Library | Purpose | Rust Equivalent | Gap |
|----------------|---------|-----------------|-----|
| `atlassian-python-api` | Confluence API client | None (write your own with `reqwest`) | Must implement API client from scratch |
| `markdown-to-confluence` (md2conf) | MD to storage format | None | Major gap; see Pitfall 5 |
| `tiktoken` | OpenAI tokenization | `tiktoken-rs` | Good parity |
| `typer` | CLI framework | `clap` (derive) | Excellent; arguably better |
| `pydantic` / `pydantic-settings` | Config + validation | `serde` + `config` crate | Different patterns but capable |
| `mcp-agent` | MCP server framework | `rmcp` or hand-roll | Emerging ecosystem, less mature |

**Prevention:** Budget extra time for the Confluence API client and Markdown converter. These are the two largest implementation efforts beyond a straightforward port.

---

### Pitfall 15: Inline Comment Marker Byte-Level Preservation

**What goes wrong:** The existing system's most critical invariant is that `ac:inline-comment-marker` elements must survive the LLM pipeline byte-for-byte. In Rust, string normalization (NFC/NFD), XML entity encoding/decoding, and even line ending normalization (`\r\n` vs `\n`) can silently alter these markers.

**Prevention:**

- Extract markers as raw byte slices before any processing.
- Compare markers after LLM round-trip using byte equality, not string equality.
- Write a dedicated test that feeds real inline comment markers through the full pipeline and asserts `==` on the output.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Confluence API client in Rust | No existing crate; must build from scratch with reqwest | Start here; it unblocks everything. Use the Python `confluence.py` as the spec -- it is only ~130 lines. |
| Markdown to storage format converter | No Rust equivalent of md2conf | Consider keeping Python converter as subprocess for phase 1; port later. |
| Anthropic API integration (raw HTTP) | SSE streaming complexity, tool use response parsing | Use non-streaming for the LLM chain (simpler, no latency benefit for sequential calls). Reserve streaming for future interactive mode. |
| Per-comment parallel evaluation | Token cost explosion, rate limits, partial failures | Implement with semaphore-bounded concurrency, batch evaluation, and conservative partial-failure handling from day one. |
| Cross-platform binary distribution | OpenSSL linking, binary size, CI matrix | Use rustls, cargo-zigbuild, and strip+LTO from the start. |
| Async architecture | Borrow checker fights with async closures | Start sync, add async only for HTTP/parallel. Use channels over shared state. |

## Sources

- Training data (May 2025 cutoff) -- MEDIUM confidence for all findings
- Direct codebase analysis of the existing Python implementation -- HIGH confidence for project-specific pitfalls
- WebSearch and WebFetch were unavailable during this research session; all external claims are based on training knowledge and should be independently verified against current documentation for:
  - Anthropic API docs (<https://docs.anthropic.com/en/api/messages>)
  - Confluence REST API docs (<https://developer.atlassian.com/cloud/confluence/rest/v1/>)
  - tokio, reqwest, clap, pulldown-cmark crate documentation
