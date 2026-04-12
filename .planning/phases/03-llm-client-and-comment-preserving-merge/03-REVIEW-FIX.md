---
phase: 03-llm-client-and-comment-preserving-merge
fixed_at: 2026-04-12T23:31:35Z
review_path: .planning/phases/03-llm-client-and-comment-preserving-merge/03-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 03: Code Review Fix Report

**Fixed at:** 2026-04-12T23:31:35Z
**Source review:** .planning/phases/03-llm-client-and-comment-preserving-merge/03-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4
- Fixed: 4
- Skipped: 0

## Fixed Issues

### WR-04: Wrong error variant for invalid CONFLUENCE_URL scheme

**Files modified:** `src/error.rs`, `src/config.rs`
**Commit:** c1463d4
**Applied fix:** Added `ConfigError::Invalid { name: &'static str, reason: &'static str }` variant to the `ConfigError` enum in `error.rs`. Updated `config.rs` line 93 to return `ConfigError::Invalid { name: "CONFLUENCE_URL", reason: "must start with https://" }` instead of the semantically incorrect `ConfigError::Missing`. Updated the `test_confluence_url_must_be_https` test to pattern-match on `ConfigError::Invalid` and assert both `name` and `reason` fields.

### WR-01/WR-02/WR-03: Injector Strategy-1 wrong occurrence, Strategy-2 stale anchor text, offset invalidation

**Files modified:** `src/merge/injector.rs`
**Commit:** 57d5932
**Applied fix:**

- **WR-03 (offset invalidation):** At the top of each marker loop iteration, `extract_sections(&result)` is now called to produce `current_sections` from the already-mutated string. This replaces all downstream uses of the `new_sections` parameter within the loop, so offsets are always valid regardless of earlier insertions. The `new_sections` parameter is renamed to `_new_sections` and kept for API compatibility.

- **WR-01 (wrong occurrence):** Strategy-1 now resolves the old section containing the marker, finds the matching new section by heading within `current_sections`, and restricts `find()` to that section's byte range `[start_offset..end_offset]`. Falls back to searching the whole result only if no section match exists. This prevents picking up the same anchor text from an unrelated earlier section.

- **WR-02 (stale anchor text in Strategy-2 fallback):** The `else` branch of Strategy-2 (reached when anchor text was NOT found in new content) now inserts a self-closing `<ac:inline-comment-marker ac:ref="..."/>` element instead of injecting `wrapper_open + marker.anchor_text + wrapper_close`. This preserves the comment thread reference without corrupting the document with text that no longer exists. The `test_inject_fallback_to_section_start` test was updated to assert a self-closing element (matching the corrected behaviour).

All 8 injector unit tests pass; `cargo check` clean.

---

_Fixed: 2026-04-12T23:31:35Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
