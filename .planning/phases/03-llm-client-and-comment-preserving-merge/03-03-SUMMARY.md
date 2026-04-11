---
phase: 03-llm-client-and-comment-preserving-merge
plan: 03
subsystem: merge
tags: [merge-engine, comment-preservation, llm-fanout, injection]
dependency_graph:
  requires:
    - 03-01 (extractor, matcher)
    - 03-02 (LlmClient trait, AnthropicClient)
  provides:
    - merge() async function orchestrating full comment-preserving merge pipeline
    - inject_markers() for re-injecting surviving markers into new content XML
    - MergeResult struct with kept/dropped/llm_evaluated counts
  affects:
    - Phase 04 CLI wiring (merge() will be called from update command)
tech_stack:
  added: []
  patterns:
    - Semaphore-bounded concurrent LLM fan-out (tokio::sync::Semaphore)
    - Fail-safe KEEP default on LLM errors
    - Exact anchor text match with section-start fallback for injection
key_files:
  created:
    - src/merge/injector.rs
  modified:
    - src/merge/mod.rs
decisions:
  - "LLM failures default to KEEP with tracing::warn (fail-safe per MERGE-03)"
  - "Markers processed forward with HashSet tracking to prevent double-injection"
  - "Self-closing markers always use section-start fallback (no anchor text to match)"
metrics:
  duration_seconds: 471
  completed: 2026-04-11T21:57:40Z
  tasks_completed: 2
  tasks_total: 2
  test_count: 20
  files_changed: 2
---

# Phase 03 Plan 03: Comment-Preserving Merge Engine Summary

Semaphore-bounded merge engine orchestrating extraction, deterministic short-circuits, parallel LLM fan-out for ambiguous comments, and anchor-text-based re-injection of surviving markers into new content XML.

## Task Completion

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Merge engine orchestration with bounded LLM fan-out | 6914726 | src/merge/mod.rs, src/merge/injector.rs (stub) |
| 2 | Comment marker re-injection into new content XML | d82832a | src/merge/injector.rs |

## Implementation Details

### Task 1: Merge Engine (src/merge/mod.rs)

Added `MergeResult` struct and `merge()` async function that orchestrates the full pipeline:

1. **Short-circuits (MERGE-06):** Empty/whitespace/`<p/>` old content, empty new content, and no-markers all return early without LLM calls
2. **Deterministic classification:** Uses `matcher::classify_comment()` for KEEP (unchanged sections) and DROP (deleted sections) without LLM
3. **Bounded LLM fan-out:** Ambiguous comments spawn `tokio::spawn` tasks gated by `Semaphore::new(concurrency_limit)` -- verified via AtomicUsize peak tracking test
4. **Fail-safe defaults:** LLM errors and JoinErrors both default to KEEP with `tracing::warn!`
5. **Injection:** Calls `injector::inject_markers()` with the keep list

12 unit tests covering all short-circuits, LLM keep/drop/error paths, and bounded concurrency.

### Task 2: Comment Marker Injector (src/merge/injector.rs)

Implements `inject_markers()` with three strategies per marker:

1. **Exact anchor text match:** Finds first occurrence of anchor text in new content, wraps with `<ac:inline-comment-marker ac:ref="uuid">...</ac:inline-comment-marker>`
2. **Section-start fallback:** Finds matching section by heading, injects at first `<p>` tag opening
3. **Drop with warning:** If neither strategy works, logs `tracing::warn!` and skips

Double-injection prevention via `HashSet<String>` tracking injected anchor texts. Self-closing markers (empty anchor text) always use section fallback.

8 unit tests covering exact match, multiple markers, fallback, no-match drop, empty list, double-injection prevention, XML validity, and self-closing markers.

## Deviations from Plan

None -- plan executed exactly as written.

## Verification Results

- `cargo test merge::tests` -- 12/12 passed
- `cargo test merge::injector::tests` -- 8/8 passed
- `cargo test --lib` -- 105/107 passed (2 pre-existing config test failures unrelated to this plan)
- `cargo clippy -- -D warnings` -- 0 warnings in merge files (4 pre-existing warnings in other modules)

## Self-Check: PASSED

- [x] src/merge/injector.rs exists
- [x] src/merge/mod.rs contains `pub async fn merge`
- [x] src/merge/mod.rs contains `pub struct MergeResult`
- [x] Commit 6914726 exists
- [x] Commit d82832a exists
