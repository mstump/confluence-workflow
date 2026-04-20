---
phase: 03-llm-client-and-comment-preserving-merge
verified: 2026-04-11T22:30:00Z
status: passed
score: 18/18 must-haves verified
overrides_applied: 0
---

# Phase 03: LLM Client and Comment-Preserving Merge Verification Report

**Phase Goal:** Per-comment parallel LLM evaluation determines which inline comments survive a content merge, with deterministic short-circuits for trivial cases and bounded concurrency for LLM calls
**Verified:** 2026-04-11T22:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Given a page with inline comments where the underlying text is unchanged, all comments are preserved with zero LLM calls (deterministic short-circuit) | VERIFIED | `classify_comment()` compares `strip_markers(old_section.content)` == `strip_markers(new_section.content)` and returns `Some(Keep)`. `test_merge_unchanged_section_keeps_all_markers_no_llm` passes, verifying zero LLM calls via empty call log. |
| 2 | Given a page with inline comments where a section was deleted, comments in that section are dropped with zero LLM calls | VERIFIED | `classify_comment()` returns `Some(Drop)` when `find_matching_section` returns `None`. `test_merge_deleted_section_drops_marker_no_llm` verifies dropped=1 and zero LLM calls. |
| 3 | Given a page with inline comments in changed sections, each ambiguous comment triggers exactly one focused LLM call that returns KEEP or DROP | VERIFIED | `classify_comment()` returns `None` for changed sections; merge() spawns exactly one tokio task per ambiguous marker. `test_merge_ambiguous_calls_llm_once` verifies `result.llm_evaluated == 1` and call log length is 1. |
| 4 | Parallel LLM evaluations are bounded by a configurable concurrency limit (default 5) using a tokio semaphore | VERIFIED | `merge()` creates `Semaphore::new(concurrency_limit)`. `test_merge_bounded_concurrency` verifies that peak concurrent calls do not exceed the limit of 3 using `AtomicUsize` peak tracking across 10 markers. Config defaults to 5 via `anthropic_concurrency`. |
| 5 | If an individual comment evaluation fails, that comment defaults to KEEP and a warning is logged — the overall update proceeds | VERIFIED | `Ok((marker, Err(e)))` branch in join result handling pushes to `keep_list` and calls `tracing::warn!`. `test_merge_llm_error_defaults_to_keep` verifies kept=1 on `LlmError::RateLimitExhausted`. |

**Score:** 5/5 roadmap success criteria verified

### Plan 01 Must-Haves

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Comment markers are extracted from Confluence storage XML with ac:ref UUID and anchor text | VERIFIED | `extract_markers()` in `extractor.rs` uses LazyLock regex singletons. 6 unit tests cover paired, self-closing, multiple, multiline, no-marker, and byte-offset cases. All pass. |
| 2 | Content is split into heading-scoped sections (h1-h6 to next heading or end-of-document) | VERIFIED | `extract_sections()` in `matcher.rs` handles h1-h6 via `HEADING_OPEN_RE` with programmatic close-tag search. `test_extract_sections_h1_through_h6` and `test_extract_sections_two_headings` verify. |
| 3 | Sections between old and new content are matched by heading text | VERIFIED | `find_matching_section()` performs exact string match on `section.heading`. Tests cover found and not-found cases. |
| 4 | Unchanged section content triggers deterministic KEEP short-circuit | VERIFIED | `classify_comment()` strips markers from both sections before comparing. `test_classify_comment_keep_unchanged` verifies `Some(Keep)`. |
| 5 | Deleted section heading triggers deterministic DROP short-circuit | VERIFIED | `classify_comment()` returns `Some(Drop)` when `find_matching_section` returns `None`. `test_classify_comment_drop_section_deleted` verifies. |

### Plan 02 Must-Haves

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | AnthropicClient sends POST to /v1/messages with correct headers and tool_use schema | VERIFIED | `test_sends_correct_headers` and `test_sends_tool_use_schema` wiremock tests pass. `anthropic-version: 2023-06-01` and `content-type: application/json` in default headers; `x-api-key` per-request. |
| 2 | tool_use response is parsed to CommentDecision::Keep or CommentDecision::Drop | VERIFIED | `test_keep_response_returns_keep` and `test_drop_response_returns_drop` wiremock tests pass. Decision parsed from `EvaluateCommentInput.decision` field. |
| 3 | Malformed response (no tool_use block) defaults to Keep with a warning log | VERIFIED | `test_no_tool_use_block_defaults_to_keep` wiremock test passes. `tracing::warn!("No tool_use block in response, defaulting to KEEP")` emitted. |
| 4 | 429 and 5xx responses trigger retry with exponential backoff and jitter | VERIFIED | `test_429_triggers_retry_then_succeeds` and `test_529_overloaded_triggers_retry` pass. Backoff: 1s start, 2x, cap 32s; jitter via `rand::rng().random_range(0.75..=1.25)`. |
| 5 | retry-after header is honored when present | VERIFIED | `test_retry_after_header_is_respected` passes. Code reads `retry-after` header before consuming body (RESEARCH.md pitfall avoidance confirmed at lines 107-111). |
| 6 | Rate limit exhaustion after 5 retries returns LlmError::RateLimitExhausted | VERIFIED | `test_five_consecutive_429s_returns_rate_limit_exhausted` passes with 6 mock calls (1 initial + 5 retries). `MAX_RETRIES = 5`. |
| 7 | LlmClient trait can be mocked in tests without HTTP | VERIFIED | `MockLlmClient` in `llm::tests` implements `LlmClient` trait. Used throughout `merge::tests`. |
| 8 | API key is never logged (redacted in tracing output) | VERIFIED | `test_api_key_not_in_debug_output` passes. `grep` of `src/llm/mod.rs` for `tracing::` + `api_key` returns empty — key is only in `.header("x-api-key", &self.api_key)` call, not in any log format string. |

### Plan 03 Must-Haves

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Empty page or page with no comment markers skips merge entirely and returns new content | VERIFIED | Three short-circuits at top of `merge()`: empty/whitespace/`<p/>` old content, empty new content, and `markers.is_empty()`. Tests `test_merge_empty_old_content_returns_new_unchanged`, `test_merge_p_slash_old_content_returns_new_unchanged`, `test_merge_no_markers_returns_new_unchanged`, `test_merge_empty_new_content_returns_empty` all pass. |
| 2 | Ambiguous comments (changed sections) trigger exactly one LLM call each via LlmClient | VERIFIED | Each ambiguous marker spawns exactly one `tokio::spawn` task. `test_merge_ambiguous_calls_llm_once` verifies call log length is 1. |
| 3 | Parallel LLM evaluations are bounded by a configurable semaphore (default 5) | VERIFIED | `Semaphore::new(concurrency_limit)` at line 137. Default 5 from `Config::anthropic_concurrency`. `test_merge_bounded_concurrency` verifies. |
| 4 | Individual comment evaluation failure defaults to KEEP with a warning log | VERIFIED | `Ok((marker, Err(e)))` branch logs `tracing::warn!` and pushes to `keep_list`. `test_merge_llm_error_defaults_to_keep` passes. |
| 5 | Surviving KEEP markers are re-injected into new content XML by exact anchor text match | VERIFIED | `inject_markers()` strategy 1: `result.find(&marker.anchor_text)` wraps with `<ac:inline-comment-marker ac:ref="...">`. `test_inject_exact_anchor_text_match` passes. |
| 6 | KEEP markers with no anchor text match fall back to section-start injection | VERIFIED | Strategy 2 in `inject_markers()` finds matching section, locates first `<p>` tag, injects at that position. `test_inject_fallback_to_section_start` and `test_inject_self_closing_marker_uses_section_fallback` pass. |
| 7 | DROP markers are discarded entirely | VERIFIED | `Some(CommentDecision::Drop)` branch increments `dropped` counter without adding to `keep_list`. `test_merge_llm_drop_omits_marker` verifies dropped=1, kept=0. |
| 8 | The merge function returns final XML with all surviving markers re-injected | VERIFIED | `inject_markers(new_content, &keep_list, &old_sections, &new_sections)` called at end of `merge()`. Returns `MergeResult { content, kept, dropped, llm_evaluated }`. |

**Combined Score:** 18/18 must-haves verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|---------|--------|---------|
| `src/merge/extractor.rs` | CommentMarker extraction from storage XML | VERIFIED | `pub fn extract_markers(content: &str) -> Vec<CommentMarker>` exists, substantive (6 tests), wired via `extractor::extract_markers` call in `merge/mod.rs` line 82 |
| `src/merge/matcher.rs` | Section extraction and short-circuit classification | VERIFIED | `pub fn extract_sections`, `pub fn find_matching_section`, `pub fn strip_markers`, `pub fn classify_comment` all exist, substantive (11 tests), wired in `merge/mod.rs` |
| `src/merge/mod.rs` | Module re-exports, shared types, and merge() function | VERIFIED | `pub mod extractor`, `pub mod injector`, `pub mod matcher`; `pub struct CommentMarker`, `pub enum CommentDecision`, `pub struct MergeResult`, `pub async fn merge` all present |
| `src/llm/mod.rs` | LlmClient trait + AnthropicClient struct | VERIFIED | `pub trait LlmClient: Send + Sync` with `evaluate_comment`; `pub struct AnthropicClient` with `new`, `with_endpoint`, `request_with_retry`; `impl LlmClient for AnthropicClient` |
| `src/llm/types.rs` | Serde types for Anthropic Messages API | VERIFIED | `MessageRequest`, `MessageResponse`, `ContentBlock`, `ToolDefinition`, `ToolChoice`, `Message`, `EvaluateCommentInput` all present with correct serde derives |
| `src/merge/injector.rs` | Comment marker re-injection into new content XML | VERIFIED | `pub fn inject_markers` exists, substantive (8 tests, 3-strategy implementation), wired via `injector::inject_markers` in `merge/mod.rs` line 186 |
| `tests/llm_integration.rs` | wiremock-based integration tests | VERIFIED | 12 tests, uses `wiremock::MockServer`, covers all required behaviors |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/merge/extractor.rs` | `src/merge/mod.rs` | `CommentMarker` struct defined in mod.rs | WIRED | `use super::CommentMarker;` at line 4 of extractor.rs |
| `src/merge/matcher.rs` | `src/merge/mod.rs` | `Section` struct, `CommentDecision` | WIRED | `use super::{CommentDecision, CommentMarker};` at line 4 of matcher.rs |
| `src/llm/mod.rs` | `src/llm/types.rs` | `AnthropicClient` uses `MessageRequest/MessageResponse` | WIRED | `use types::{ContentBlock, EvaluateCommentInput, Message, MessageRequest, MessageResponse, ToolChoice, ToolDefinition};` in mod.rs |
| `src/llm/mod.rs` | `src/merge/mod.rs` | `LlmClient::evaluate_comment` uses `CommentMarker`, `CommentDecision` | WIRED | `use crate::merge::{CommentDecision, CommentMarker};` in mod.rs |
| `src/llm/mod.rs` | `src/error.rs` | Returns `Result<CommentDecision, LlmError>` | WIRED | `use crate::error::LlmError;` in mod.rs |
| `src/merge/mod.rs` | `src/llm/mod.rs` | `merge()` takes `Arc<dyn LlmClient>` | WIRED | `use crate::llm::LlmClient;` at line 9; `Arc<dyn LlmClient>` parameter at line 55 |
| `src/merge/mod.rs` | `src/merge/extractor.rs` | `merge()` calls `extract_markers` | WIRED | `extractor::extract_markers(old_content)` at line 82 |
| `src/merge/mod.rs` | `src/merge/matcher.rs` | `merge()` calls `classify_comment` | WIRED | `matcher::classify_comment(marker, &old_sections, &new_sections)` at line 105 |
| `src/merge/mod.rs` | `src/merge/injector.rs` | `merge()` calls `inject_markers` | WIRED | `injector::inject_markers(new_content, &keep_list, &old_sections, &new_sections)` at line 186 |
| `src/lib.rs` | `src/merge/mod.rs` | `pub mod merge` | WIRED | `pub mod merge;` at line 7 of lib.rs |
| `src/lib.rs` | `src/llm/mod.rs` | `pub mod llm` | WIRED | `pub mod llm;` at line 6 of lib.rs |

### Data-Flow Trace (Level 4)

This phase produces a merge engine, not a rendering component. The data flow is library-internal:

- `old_content` (Confluence storage XML) flows through `extract_markers` → `CommentMarker` structs with real byte positions
- Markers flow through `classify_comment` → deterministic decisions or `ambiguous_list` with real section content references
- Ambiguous markers flow through `LlmClient::evaluate_comment` → `CommentDecision` based on actual LLM response parsing
- Keep list flows through `inject_markers` → transformed `new_content` string with re-injected XML elements

All data transformations are substantive (no static stubs). Tests verify real transformations against concrete XML fixtures.

| Data Path | Data Flows | Status |
|-----------|------------|--------|
| XML → CommentMarker | Regex match, real byte offsets | FLOWING |
| CommentMarker → CommentDecision (deterministic) | Section comparison after marker stripping | FLOWING |
| CommentMarker → CommentDecision (LLM) | HTTP POST via AnthropicClient, tool_use parsing | FLOWING (verified via wiremock) |
| CommentMarker → re-injected XML | Anchor text search + section fallback | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All lib unit tests pass | `cargo test --lib` | 115 passed, 0 failed | PASS |
| LLM integration tests (wiremock) | `cargo test --test llm_integration` | 12 passed, 0 failed | PASS |
| Deterministic KEEP fires for unchanged sections | `cargo test merge::tests::test_merge_unchanged_section_keeps_all_markers_no_llm` | ok | PASS |
| Deterministic DROP fires for deleted sections | `cargo test merge::tests::test_merge_deleted_section_drops_marker_no_llm` | ok | PASS |
| Semaphore bounds concurrency | `cargo test merge::tests::test_merge_bounded_concurrency` | ok (peak <= 3 verified) | PASS |
| API key absent from tracing output | `cargo test llm_integration::test_api_key_not_in_debug_output` | ok | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| MERGE-01 | 03-01 | Extract all `<ac:inline-comment-marker>` elements with section context | SATISFIED | `extractor::extract_markers()` extracts markers with ac_ref, anchor_text, position. `matcher::extract_sections()` provides section context. 6 extractor tests + 8 section tests pass. |
| MERGE-02 | 03-01 | Deterministic short-circuits: unchanged section → KEEP; deleted section → DROP | SATISFIED | `classify_comment()` implements both short-circuits. Tests verify each. |
| MERGE-03 | 03-03 | Per-comment LLM evaluation for ambiguous cases | SATISFIED | `merge()` fans out ambiguous markers to `LlmClient::evaluate_comment`. Fail-safe KEEP default. Tests verify. |
| MERGE-04 | 03-03 | Comment evaluations run in parallel with bounded concurrency via tokio semaphore | SATISFIED | `Semaphore::new(concurrency_limit)` in `merge()`. `test_merge_bounded_concurrency` verifies bounded peak. |
| MERGE-05 | 03-03 | Surviving comment markers injected back into new content XML | SATISFIED | `inject_markers()` with exact anchor text match + section-start fallback + drop-with-warning. 8 injector tests pass. |
| MERGE-06 | 03-03 | Empty page or page with no comments → skip merge | SATISFIED | Three early returns in `merge()` for empty/whitespace/`<p/>`/no-markers cases. 4 tests verify. |
| LLM-01 | 03-02 | Hand-rolled Anthropic Messages API client over reqwest | SATISFIED | `AnthropicClient` in `src/llm/mod.rs` uses `reqwest::Client` directly with no Python dependency. |
| LLM-02 | 03-02 | Structured output via Claude tool_use for KEEP/DROP | SATISFIED | `ToolDefinition` + `ToolChoice { type: "tool", name: "evaluate_comment" }` forces tool_use response. `test_sends_tool_use_schema` verifies request structure. |
| LLM-03 | 03-02 | Retry with exponential backoff on rate limit (429) and transient errors | SATISFIED | `request_with_retry()` retries on [429, 500, 502, 503, 529] with 1s/2x/32s backoff + jitter. Wiremock tests verify retry and exhaustion. |
| LLM-04 | 03-02 | LlmClient trait-based for testability | SATISFIED | `#[async_trait] pub trait LlmClient: Send + Sync` defined. `MockLlmClient` used in merge tests without HTTP. |

All 10 Phase 03 requirements covered. No orphaned requirements (REQUIREMENTS.md maps exactly MERGE-01 through MERGE-06 and LLM-01 through LLM-04 to Phase 3).

### Anti-Patterns Found

None found. Scanned all Phase 03 source files for TODO/FIXME/placeholder comments, empty implementations, and hardcoded empty data. Zero matches.

Notable security pattern confirmed: `x-api-key` header is added per-request via `.header("x-api-key", &self.api_key)` at line 100 of `src/llm/mod.rs` and does not appear in any `tracing::` format string in that file. The integration test `test_api_key_not_in_debug_output` enforces this at the test level.

### Human Verification Required

None. All success criteria are fully verifiable programmatically via the test suite. No visual output, real-time behavior, or external service integration that cannot be covered by the existing wiremock-based tests.

## Gaps Summary

No gaps. All 18 must-haves verified. All 10 requirements satisfied. All tests pass (115 lib + 12 integration = 127 total). No anti-patterns or stubs detected. The merge pipeline is complete and wired end-to-end.

---

_Verified: 2026-04-11T22:30:00Z_
_Verifier: Claude (gsd-verifier)_
