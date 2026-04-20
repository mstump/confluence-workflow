---
phase: 03-llm-client-and-comment-preserving-merge
reviewed: 2026-04-11T00:00:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - src/config.rs
  - src/error.rs
  - src/lib.rs
  - src/llm/mod.rs
  - src/llm/types.rs
  - src/merge/extractor.rs
  - src/merge/injector.rs
  - src/merge/matcher.rs
  - src/merge/mod.rs
  - tests/llm_integration.rs
findings:
  critical: 0
  warning: 4
  info: 9
  total: 13
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-04-11T00:00:00Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

This phase implements the Anthropic LLM client and the comment-preserving merge
pipeline. The architecture is well-structured: the `LlmClient` trait cleanly
decouples HTTP from business logic, the three-strategy injector provides graceful
degradation, and the deterministic short-circuit classifier minimises unnecessary
LLM calls. Test coverage is thorough, including wiremock integration tests for
retry/backoff behaviour.

Three related warnings in `src/merge/injector.rs` together represent the most
significant risk: the injector mutates a string in-place while relying on
substring offsets derived from the original, and its section-fallback branch
inserts stale anchor text into the new document rather than attaching to existing
content. These can produce corrupt XML under realistic inputs. One additional
warning flags semantic misuse of an error variant in `src/config.rs`.

---

## Warnings

### WR-01: Injector Strategy-1 may wrap wrong occurrence when anchor text appears in multiple sections

**File:** `src/merge/injector.rs:47`
**Issue:** `result.find(&marker.anchor_text)` always selects the *first* occurrence of the anchor text in the entire (already-mutated) result string. When the same anchor text appears in an earlier unrelated section, the marker is injected at the wrong location. This is silent — no warning is emitted and the counts appear correct.

**Fix:** Narrow the search to the section that originally contained the marker. Locate the new section matching the old section's heading first, then call `.find()` within that section's byte range only.

```rust
// After finding old_section and new_section via classify_comment path:
let section_start_in_result = result.find(&new_sec.content).unwrap_or(0);
let search_range = &result[section_start_in_result..section_start_in_result + new_sec.content.len()];
if let Some(rel_pos) = search_range.find(&marker.anchor_text) {
    let abs_pos = section_start_in_result + rel_pos;
    // inject at abs_pos
}
```

---

### WR-02: Injector Strategy-2 fallback inserts stale anchor text into new document

**File:** `src/merge/injector.rs:83-92`
**Issue:** The Strategy-2 (section-start) fallback is reached precisely when Strategy-1 failed — meaning `marker.anchor_text` was NOT found in the new content. Despite this, the code builds `wrapped = wrapper_open + marker.anchor_text + wrapper_close` and inserts it directly at the paragraph start. This injects old text that does not exist in the updated document, producing malformed/misleading XML.

A self-closing marker (empty anchor text) at lines 77-82 is handled correctly; the bug only affects paired markers landing in this branch.

**Fix:** For the section fallback with non-empty anchor text, either use a self-closing marker (the comment reference is preserved, the anchor is lost) or skip injection and emit a warning. Do not insert the old anchor text verbatim.

```rust
// Replace lines 83-92 with:
} else {
    // Anchor text not in new content — inject self-closing to preserve the
    // comment thread reference without corrupting new document text.
    let self_closing = format!(
        r#"<ac:inline-comment-marker ac:ref="{}"/>"#,
        marker.ac_ref
    );
    result.insert_str(insert_pos, &self_closing);
}
```

---

### WR-03: Injector offset invalidation — `new_sec.content` search fails after earlier mutations

**File:** `src/merge/injector.rs:71`
**Issue:** `result.find(&new_sec.content)` searches for the verbatim section content derived from the *original* `new_content`. After earlier markers are injected into `result`, the string `result` no longer contains that exact substring, so the find returns `None` and the marker is silently dropped via Strategy 3 even when a section match exists.

This is a systematic failure: every second-or-later marker that requires the section-fallback path will be silently lost.

**Fix:** Recompute `new_sections` from `result` at the start of each iteration (expensive but correct), or track a mutable byte-offset delta and adjust `new_sec.start_offset` rather than searching by content string.

```rust
// At the top of the for loop body, refresh sections from current result:
let current_sections = crate::merge::matcher::extract_sections(&result);
// Then search for matching section by heading rather than by content substring.
if let Some(new_sec) = find_matching_section(&old_sec.heading, &current_sections) {
    if let Some(p_match) = P_OPEN_RE.find(&result[new_sec.start_offset..]) {
        let insert_pos = new_sec.start_offset + p_match.end();
        // ...
    }
}
```

---

### WR-04: Wrong error variant for invalid CONFLUENCE_URL scheme

**File:** `src/config.rs:93-96`
**Issue:** When `CONFLUENCE_URL` begins with `http://` instead of `https://`, the code returns `ConfigError::Missing { name: "CONFLUENCE_URL (must start with https://)" }`. `Missing` semantically means "the value was not provided." Here the value was provided but is invalid. Using the wrong variant misleads both users (who may search for the missing variable) and callers who pattern-match on `ConfigError`.

A dedicated `ConfigError::Invalid` variant is already absent from the enum, but can be added.

**Fix:** Add an `Invalid` variant and use it here:

```rust
// In error.rs:
#[error("Invalid configuration value for {name}: {reason}")]
Invalid { name: &'static str, reason: &'static str },

// In config.rs line 93:
return Err(ConfigError::Invalid {
    name: "CONFLUENCE_URL",
    reason: "must start with https://",
});
```

---

## Info

### IN-01: Dead code — `ConfigError::FileRead` and `ConfigError::NoHomeDir` are never constructed

**File:** `src/error.rs:91-98`
**Issue:** `ConfigError::FileRead`, `ConfigError::JsonParse`, and `ConfigError::NoHomeDir` are defined but never returned — `load_from_claude_config` swallows IO and JSON parse errors via `tracing::debug!` and returns `None`. These variants are dead code.

**Fix:** Either remove the unused variants, or wire `load_from_claude_config` to propagate the errors via `Result` (which would change the function signature and callers).

---

### IN-02: Dead code — unreachable post-loop `RateLimitExhausted` return

**File:** `src/llm/mod.rs:170-172`
**Issue:** The loop body returns early via `if attempt == MAX_RETRIES { return Err(...) }` at line 134. The post-loop `Err(LlmError::RateLimitExhausted { ... })` at line 170 can never be reached. Rust does not warn about this because the loop bound uses a runtime constant.

**Fix:** Replace with `unreachable!("loop exits via early return on final attempt")` to make the intent explicit, or restructure the loop to eliminate the guard.

---

### IN-03: Panic-safe task error drops comment without incrementing `dropped` counter

**File:** `src/merge/mod.rs:169-178`
**Issue:** When a spawned LLM-evaluation task panics (`Err(join_err)` arm), the marker is silently discarded — it is not added to `keep_list` and `dropped` is not incremented. `llm_evaluated` was already counted. The `MergeResult` totals will not sum to `markers.len()`, which may confuse callers.

**Fix:** Increment `dropped` in the panic arm:

```rust
Err(join_err) => {
    tracing::warn!(error = %join_err, "LLM evaluation task panicked, defaulting to DROP");
    dropped += 1;
}
```

---

### IN-04: Blocking file read on async executor in `lib.rs`

**File:** `src/lib.rs:37-38`
**Issue:** `std::fs::read_to_string(&markdown_path)` is called directly on the async executor thread. For typical markdown file sizes this is inconsequential, but it is a pattern that should be replaced with `tokio::fs::read_to_string` for consistency with the async runtime.

**Fix:**

```rust
let markdown = tokio::fs::read_to_string(&markdown_path)
    .await
    .map_err(AppError::Io)?;
```

---

### IN-05: Semaphore acquire unwrap inside spawned task

**File:** `src/merge/mod.rs:144`
**Issue:** `sem.acquire().await.unwrap()` — `Semaphore::acquire` returns `Err` only if the semaphore has been closed. The semaphore is locally owned and never explicitly closed, so this is safe. However, `.unwrap()` inside a `tokio::spawn` closure causes the task to panic, which would then propagate as a `JoinError` (triggering IN-03 above).

**Fix:** Use `expect` with a message, or map the error:

```rust
let _permit = sem.acquire().await.expect("semaphore closed unexpectedly");
```

---

### IN-06: `ToolChoice.name` is always required in the serialized struct

**File:** `src/llm/types.rs:23-27`
**Issue:** The `ToolChoice` struct requires `name: String` unconditionally. The Anthropic API only requires `name` when `type == "tool"`. If the code is extended to support `type == "auto"` or `type == "any"`, the struct will serialize a `name` field that is invalid for those modes.

**Fix:** Make `name` optional: `pub name: Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]`.

---

### IN-07: Env var mutation in config unit tests is not parallel-safe

**File:** `src/config.rs:266-279`
**Issue:** Tests `test_fallthrough_to_env_vars` and `test_env_vars_used_when_cli_absent` call `std::env::set_var`/`remove_var`. Rust unit tests run in parallel by default; concurrent mutation of environment variables is undefined behaviour on many platforms (the standard library marks `set_var` as unsafe in Rust 2024 edition). If another config test reads the same variable concurrently, the test can produce false positives or panics.

**Fix:** Annotate these tests with `#[serial_test::serial]` (add the `serial_test` crate as a dev-dependency), or restructure to use `load_with_home` with a fake path and inject values only via `CliOverrides`.

---

### IN-08: Security test only checks `tracing::info!` and `tracing::debug!` for api_key leakage

**File:** `tests/llm_integration.rs:423-434`
**Issue:** The compile-time source scan checks only `tracing::info!` and `tracing::debug!` for references to `api_key`. It does not check `tracing::warn!` or `tracing::error!`. Currently no `warn!`/`error!` calls reference `api_key`, so there is no actual leakage, but the test gives false assurance for those levels.

**Fix:** Expand the check to all tracing levels:

```rust
if line.contains("api_key")
    && (line.contains("tracing::info!")
        || line.contains("tracing::debug!")
        || line.contains("tracing::warn!")
        || line.contains("tracing::error!"))
{
    panic!(...);
}
```

---

### IN-09: `retry-after` header delay is not capped at `MAX_BACKOFF_MS`

**File:** `src/llm/mod.rs:141-145`
**Issue:** When a `retry-after` header is present, the delay is taken directly from the header value (in seconds, converted to ms) with jitter applied but no upper-bound cap. A server could return `retry-after: 86400` (one day), causing the client to sleep indefinitely. The exponential backoff path correctly caps at `MAX_BACKOFF_MS` (32 seconds), but the `retry-after` path bypasses this cap.

**Fix:** Apply the cap after computing `delay_ms`:

```rust
let delay_ms = if let Some(retry_secs) = retry_after {
    ((retry_secs * 1000.0) as u64).min(MAX_BACKOFF_MS)
} else {
    backoff_ms
};
```

---

*Reviewed: 2026-04-11T00:00:00Z*
*Reviewer: Claude (gsd-code-reviewer)*
*Depth: standard*
