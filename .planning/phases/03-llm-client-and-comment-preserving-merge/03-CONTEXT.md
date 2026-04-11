# Phase 03: LLM Client and Comment-Preserving Merge — Context

**Created:** 2026-04-10 (auto mode)
**Phase:** 03 — LLM Client and Comment-Preserving Merge
**Status:** Ready for research and planning

---

## Domain Boundary

This phase delivers two tightly coupled components:
1. **LlmClient** — hand-rolled Anthropic Messages API client (reqwest, no Python dependency)
2. **MergeEngine** — per-comment parallel KEEP/DROP evaluator with deterministic short-circuits

The phase does NOT wire these into the CLI commands (that's Phase 4). It builds the reusable components behind trait boundaries.

---

## Canonical Refs

- `src/confluence/mod.rs` — ConfluenceApi trait pattern to follow for LlmClient trait
- `src/converter/mod.rs` — Converter trait pattern (async_trait with Send + Sync bounds)
- `src/error.rs` — Error hierarchy to extend with LlmError, MergeError variants
- `src/config.rs` — Config struct (has `anthropic_api_key: Option<String>` already)
- `Cargo.toml` — Already has: reqwest 0.13, tokio, serde, serde_json, async-trait, regex, anyhow, tracing
- `src/confluence_agent/agent.py` — Python extraction patterns (lines 91–115): regex for both self-closing `<ac:inline-comment-marker .../>` and paired `<ac:inline-comment-marker ...>...</ac:inline-comment-marker>` tags
- `.planning/REQUIREMENTS.md` — LLM-01 to LLM-04, MERGE-01 to MERGE-06 are the requirements for this phase

---

## Prior Decisions (from Phases 1 & 2)

- **Trait pattern**: All components are trait-based with `async_trait` and `Send + Sync` bounds (established in Phase 1 for `ConfluenceApi`, Phase 2 for `Converter`)
- **Error types**: Use `thiserror` enums with user-facing messages; errors compose upward via `#[from]`
- **Testing**: `async_trait` mock pattern established; use same pattern for `LlmClient` mock
- **Credentials**: `Config.anthropic_api_key` is `Option<String>` — Phase 3 should require it (return error if None when LLM path is taken)
- **Anthropic-only**: No other LLM providers at launch; Anthropic API only

---

## Decisions

### 1. Section Context Extraction for LLM Calls

**Decision:** Extract heading-scoped section blocks as context.

A "section" is the text from the preceding `<h1>`–`<h6>` element (inclusive) to the next heading or end-of-document. When extracting context for a comment evaluation LLM call, pass:
- The **old section** (from the existing Confluence page) — the heading and its body text with the comment marker stripped from the XML for readability
- The **new section** (from the converted new content) — the closest matching section by heading text

For the deterministic short-circuit (MERGE-02):
- KEEP short-circuit: the section containing the comment marker, stripped of the marker itself, matches the corresponding section in new content with exact string equality
- DROP short-circuit: no section in new content has a heading that matches (or closely matches) the old section heading

**Why this scope:** Heading-to-heading sections give the LLM enough context to judge relevance without inflating token cost. Sending the whole page per comment would negate the parallelism benefit.

### 2. LLM Model Selection

**Decision:** Use `claude-haiku-4-5-20251001` by default, configurable via `ANTHROPIC_MODEL` env var.

KEEP/DROP classification is a simple binary task with focused context (~500–1000 tokens per call). Haiku is cost-optimal for this. The model should be overridable for users who want higher quality at higher cost.

Config priority: `ANTHROPIC_MODEL` env var → config file `anthropic_model` field → default `claude-haiku-4-5-20251001`

### 3. Tool_use Schema for Structured KEEP/DROP Output

**Decision:** Single `evaluate_comment` tool with a `decision` field.

```json
{
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
}
```

Parse `decision` from the `tool_use` content block. If the response does not contain a `tool_use` block (malformed), treat as KEEP and log a warning (fail-safe per MERGE-03 / success criterion 5).

### 4. Comment Re-injection Strategy for Surviving Markers

**Decision:** Exact anchor text match; fallback to section-start injection; RELOCATE is v2.

After per-comment evaluation, for all KEEP decisions:
1. **Deterministic KEEP (short-circuit):** Anchor text is unchanged by definition → find the exact comment marker text in new content XML and re-inject the `<ac:inline-comment-marker>` element wrapping it. This is a direct string replacement.
2. **LLM KEEP (ambiguous):** Anchor text may have changed. Strategy:
   - Try exact match first
   - If not found: find the section with the closest matching heading in new content, inject the marker at the start of that section's first `<p>` element
   - If no matching section found: log a warning, drop the marker (do not silently corrupt XML)
3. **DROP (any path):** Discard marker entirely, do not inject.

The `ac:inline-comment-marker` element wraps text: `<ac:inline-comment-marker ac:ref="uuid">anchor text</ac:inline-comment-marker>`. The "anchor text" is what we search for in new content.

**Note:** Fuzzy matching (Levenshtein) is v2 (MERGE2-02). For v1, exact match only.

### 5. Concurrency and Retry Configuration

**Decision:** Default semaphore bound of 5, configurable via `ANTHROPIC_CONCURRENCY` env var.

Retry policy:
- Retry on: 429 (rate limit), 500, 502, 503, 529 (overloaded)
- Read `retry-after` header when present; otherwise use exponential backoff
- Backoff: start 1s, multiply by 2 with ±25% jitter, max 32s
- Max retries: 5 per individual request
- On exhaustion: return an `LlmError::RateLimitExhausted` — the merge engine treats this as KEEP + warning (MERGE-03 success criterion 5)

### 6. Module Layout

**Decision:** New `src/llm/` module and `src/merge/` module.

```
src/
  llm/
    mod.rs       — LlmClient trait + AnthropicClient struct
    types.rs     — Request/response types (Message, Content, ToolUse, etc.)
  merge/
    mod.rs       — MergeEngine / merge() function
    extractor.rs — ac:inline-comment-marker extraction (regex, both paired and self-closing)
    matcher.rs   — section extraction, exact match, short-circuit logic
    injector.rs  — re-injection of surviving markers into new content
```

This mirrors the `src/confluence/` and `src/converter/` patterns established in Phases 1–2.

### 7. LlmClient Trait Interface

**Decision:** Minimal trait surface:

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn evaluate_comment(
        &self,
        old_section: &str,
        new_section: Option<&str>,  // None if section was deleted
        marker: &CommentMarker,
    ) -> Result<CommentDecision, LlmError>;
}
```

Where `CommentDecision` is `KEEP` or `DROP`. The trait is testable via mock without HTTP.

### 8. Empty Page / No Comments Short-Circuit

**Decision:** Skip the entire merge engine when:
- New page content is empty (reuse `_is_content_empty`-style check from Python)
- Existing page has no `<ac:inline-comment-marker>` elements

In both cases, return new content directly (MERGE-06). This check happens in the merge engine's entry point before any XML parsing.

---

## Deferred Ideas

- Batch comment evaluation (group 5–10 per LLM call) — v2 (MERGE2-04)
- Fuzzy anchor text matching for re-injection — v2 (MERGE2-02)
- User-visible report of dropped comments — v2 (MERGE2-03)
- OpenAI / Gemini providers — v2

---

## Implementation Notes for Downstream Agents

1. **No new crate additions needed for core logic** — regex, reqwest, serde_json, tokio, async-trait all already in Cargo.toml. May need to add `tokio::sync::Semaphore` (already in tokio).

2. **Regex for marker extraction** — replicate the Python pattern exactly:
   - Self-closing: `<ac:inline-comment-marker\b[^>]*?/>`
   - Paired: `<ac:inline-comment-marker\b[^>]*?>.*?</ac:inline-comment-marker>` (DOTALL)

3. **XML parsing approach** — Do NOT use a full XML parser for marker extraction; use regex as Python does (Confluence storage XML is not well-formed XML — it uses `ac:` prefixes without namespace declarations). For section extraction, use string scanning for heading tags (`<h1>` through `<h6>`).

4. **Anthropic API endpoint** — `https://api.anthropic.com/v1/messages`; requires headers `x-api-key: <key>`, `anthropic-version: 2023-06-01`, `content-type: application/json`

5. **Test strategy** — Integration tests should use `wiremock` (already in dev-dependencies) to mock the Anthropic API endpoint. Unit tests for extractor, matcher, and injector use string fixtures.
