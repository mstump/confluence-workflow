---
phase: 03-llm-client-and-comment-preserving-merge
plan: 01
subsystem: merge-engine
tags: [comment-extraction, section-matching, short-circuit, regex]
dependency_graph:
  requires: []
  provides: [CommentMarker, Section, CommentDecision, extract_markers, extract_sections, classify_comment]
  affects: [src/merge, src/error.rs, src/lib.rs]
tech_stack:
  added: []
  patterns: [LazyLock regex singletons, heading-scoped section extraction, deterministic short-circuit classification]
key_files:
  created:
    - src/merge/mod.rs
    - src/merge/extractor.rs
    - src/merge/matcher.rs
  modified:
    - src/error.rs
    - src/lib.rs
key_decisions:
  - "Used LazyLock<Regex> singletons for compiled regex patterns to avoid recompilation"
  - "Replaced backreference regex with programmatic close-tag search since Rust regex crate does not support backreferences"
metrics:
  duration: 5m 19s
  completed: "2026-04-11T01:33:28Z"
  tasks_completed: 2
  tasks_total: 2
  test_count: 17
---

# Phase 03 Plan 01: Comment Marker Extraction and Section Classification Summary

Comment marker extraction from Confluence storage XML with heading-scoped section splitting and deterministic KEEP/DROP short-circuit classification for the merge engine.

## What Was Built

### Task 1: Shared types and comment marker extraction

- **CommentMarker** struct in `src/merge/mod.rs` with fields: full_match, ac_ref, anchor_text, position
- **CommentDecision** enum with Keep and Drop variants
- **extract_markers()** in `src/merge/extractor.rs` using combined regex for paired and self-closing `<ac:inline-comment-marker>` elements
- **LlmError** and **MergeError** enums added to `src/error.rs`
- **pub mod merge** added to `src/lib.rs`
- 6 tests covering paired markers, self-closing markers, multiple markers, multiline anchor text, no markers, and byte offset preservation

### Task 2: Section extraction and deterministic short-circuit classification

- **Section** struct in `src/merge/matcher.rs` with heading, heading_level, content, start_offset, end_offset
- **extract_sections()** splitting HTML by h1-h6 headings with preamble support (heading="" level=0 for content before first heading)
- **find_matching_section()** for exact heading text lookup
- **strip_markers()** removing marker tags while preserving anchor text
- **classify_comment()** implementing deterministic short-circuits: KEEP (unchanged section), DROP (deleted section), None (ambiguous, needs LLM)
- 11 tests covering section extraction, heading matching, strip markers, and all three classification outcomes

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed backreference regex incompatibility**

- **Found during:** Task 2
- **Issue:** Plan specified heading regex `<h([1-6])\b[^>]*>(.*?)</h\1>` which uses a backreference (`\1`). Rust's `regex` crate does not support backreferences.
- **Fix:** Changed to match only the opening `<h[1-6]>` tag with regex, then programmatically search for the corresponding `</hN>` close tag using `str::find()`.
- **Files modified:** src/merge/matcher.rs
- **Commit:** 4bee361

**2. [Rule 1 - Bug] Fixed byte offset assertion in test**

- **Found during:** Task 1
- **Issue:** Test asserted marker position as 12 bytes but `<p>Some text` is 13 bytes.
- **Fix:** Corrected assertion to 13.
- **Files modified:** src/merge/extractor.rs
- **Commit:** a036a1e

## Commit Log

| Task | Commit | Message |
|------|--------|---------|
| 1 | a036a1e | feat(03-01): add comment marker extraction and merge error types |
| 2 | 4bee361 | feat(03-01): add section extraction and deterministic short-circuit classification |

## Verification Results

- `cargo test merge::extractor::tests` -- 6/6 passed
- `cargo test merge::matcher::tests` -- 11/11 passed
- `cargo test --lib` -- 87/87 passed (zero regressions)
- `cargo clippy -- -D warnings -A clippy::uninlined-format-args` -- clean (pre-existing `uninlined-format-args` in converter/client files are out of scope)

## Self-Check: PASSED

All files exist. All commits verified.
