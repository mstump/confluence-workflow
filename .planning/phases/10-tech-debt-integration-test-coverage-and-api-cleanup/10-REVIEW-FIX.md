---
phase: 10-tech-debt-integration-test-coverage-and-api-cleanup
fixed_at: 2026-04-20T00:00:00Z
review_path: .planning/phases/10-tech-debt-integration-test-coverage-and-api-cleanup/10-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 10: Code Review Fix Report

**Fixed at:** 2026-04-20
**Source review:** .planning/phases/10-tech-debt-integration-test-coverage-and-api-cleanup/10-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4
- Fixed: 4
- Skipped: 0

## Fixed Issues

### WR-01: Diagram placeholder replacement breaks when there are 10 or more diagrams

**Files modified:** `src/converter/renderer.rs`, `src/converter/mod.rs`
**Commit:** 9a4fba8
**Applied fix:** Changed the placeholder format from `<!-- DIAGRAM_PLACEHOLDER_{i} -->` to `<!-- DIAGRAM_PLACEHOLDER_{i:04} -->` (zero-padded to 4 digits) in both the renderer (where placeholders are inserted) and in mod.rs (where they are substituted). This ensures `DIAGRAM_PLACEHOLDER_0001` cannot be a substring of `DIAGRAM_PLACEHOLDER_0010`, eliminating the prefix-collision bug for documents with 10+ diagrams.

---

### WR-02: Mermaid temp file leaked on subprocess timeout

**Files modified:** `src/converter/diagrams.rs`
**Commit:** 49df706
**Applied fix:** Restructured the mermaid timeout handling from a chained `.map_err` into an explicit `match` on the `timeout_result`. The `Err(_)` (timeout) arm now calls `let _ = std::fs::remove_file(&output_path)` before returning `DiagramTimeout`, cleaning up any partially-written SVG file that `mmdc` may have created before the timeout fired.

---

### WR-03: `ANTHROPIC_CONCURRENCY=0` silently accepted

**Files modified:** `src/config.rs`
**Commit:** 84ebbfc
**Applied fix:** Added `.max(1)` between `.unwrap_or(5)` and `.min(50)` in the `ANTHROPIC_CONCURRENCY` parsing chain. This ensures a configured value of `0` is clamped to `1`, preventing a zero-permit semaphore deadlock at the use site.

---

### WR-04: `AnthropicClient::with_endpoint` panics on reqwest client build failure

**Files modified:** `src/error.rs`, `src/llm/mod.rs`, `src/lib.rs`, `tests/llm_integration.rs`
**Commit:** 787b380
**Applied fix:**
- Added `InitError(String)` variant to `LlmError` in `src/error.rs`.
- Changed `AnthropicClient::new` and `AnthropicClient::with_endpoint` signatures from `-> Self` to `-> Result<Self, LlmError>`, replacing `.expect("Failed to build reqwest client")` with `.map_err(|e| LlmError::InitError(e.to_string()))?`.
- Updated the call site in `src/lib.rs` to propagate the error with `?`.
- Updated all `AnthropicClient::with_endpoint(...)` calls in `tests/llm_integration.rs` to append `.unwrap()` (appropriate in test context).

---

_Fixed: 2026-04-20_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
