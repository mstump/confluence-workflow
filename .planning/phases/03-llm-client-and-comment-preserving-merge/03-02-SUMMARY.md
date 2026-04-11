---
phase: 03-llm-client-and-comment-preserving-merge
plan: 02
subsystem: llm-client
tags: [anthropic, llm, api-client, retry, tool-use]
dependency_graph:
  requires: [03-01]
  provides: [LlmClient-trait, AnthropicClient]
  affects: [src/llm, src/config.rs, Cargo.toml]
tech_stack:
  added: [rand-0.9, tokio-sync]
  patterns: [exponential-backoff, tool-use-structured-output, per-request-auth-header]
key_files:
  created:
    - src/llm/mod.rs
    - src/llm/types.rs
    - tests/llm_integration.rs
  modified:
    - src/config.rs
    - src/lib.rs
    - Cargo.toml
    - Cargo.lock
decisions:
  - Used rand 0.9 API (rand::rng() and random_range) for jitter calculation
  - API key added per-request via .header() call, never in default headers, to prevent accidental logging
  - Malformed responses (no tool_use block) default to KEEP with warning log (fail-safe)
metrics:
  duration: 494s
  completed: 2026-04-11T21:54:54Z
  tasks_completed: 2
  tasks_total: 2
  test_count: 20
  files_changed: 7
---

# Phase 03 Plan 02: Anthropic LLM Client with Retry and Tool Use Summary

Hand-rolled Anthropic Messages API client over reqwest with tool_use structured output for KEEP/DROP comment classification, exponential backoff retry on transient errors (429/5xx), and a testable LlmClient trait boundary.

## What Was Built

### LlmClient Trait and API Types (Task 1)
- Defined `LlmClient` trait with `evaluate_comment` method using `async_trait` with `Send + Sync` bounds
- Created serde types for the Anthropic Messages API: `MessageRequest`, `MessageResponse`, `ContentBlock` (tagged enum for text/tool_use), `ToolDefinition`, `ToolChoice`, `Message`, `EvaluateCommentInput`
- Extended `Config` struct with `anthropic_model` (default: `claude-haiku-4-5-20251001`) and `anthropic_concurrency` (default: 5)
- Added `tokio` sync feature and `rand` 0.9 dependency to Cargo.toml
- 8 unit tests: serde round-trips, MockLlmClient trait implementation

### AnthropicClient Implementation (Task 2)
- `AnthropicClient` struct with `reqwest::Client`, api_key, model, endpoint fields
- `with_endpoint` constructor for test injection (wiremock)
- `request_with_retry`: exponential backoff (1s start, 2x multiply, 32s cap) with +/-25% jitter
- Retries on 429, 500, 502, 503, 529; immediate failure on 400, 401, 403
- Honors `retry-after` header when present
- Returns `LlmError::RateLimitExhausted` after 5 retries
- `evaluate_comment` builds prompt with old/new section context and comment anchor text
- Forces tool_use response via `tool_choice` with `evaluate_comment` tool
- Parses KEEP/DROP from `ContentBlock::ToolUse`; defaults to KEEP on malformed response
- 12 wiremock integration tests covering: headers, tool_use schema, KEEP/DROP parsing, text-only fallback, 429 retry, 529 retry, 400 no-retry, rate limit exhaustion, retry-after header, deleted section prompt, API key not in debug output

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 3c2a5b0 | LlmClient trait, Anthropic API types, config extensions |
| 2 | 2c928ea | AnthropicClient with retry, backoff, wiremock integration tests |

## Deviations from Plan

None - plan executed exactly as written.

## Threat Mitigations Implemented

| Threat ID | Mitigation |
|-----------|------------|
| T-03-03 | API key added per-request via `.header("x-api-key", &self.api_key)`, never in default headers or tracing output. Integration test verifies key not in logs. |
| T-03-04 | tool_use constrains response to KEEP/DROP. Malformed response defaults to KEEP + warning. EvaluateCommentInput validates decision field. |
| T-03-05 | Bounded retries (max 5) with exponential backoff (1s-32s). Respects retry-after header. Returns LlmError::RateLimitExhausted on exhaustion. |
| T-03-06 | tool_choice type "tool" forces evaluate_comment tool. Free-text responses treated as malformed, default to KEEP. |

## Known Stubs

None - all functionality is fully implemented and wired.

## Verification Results

- `cargo test llm::tests` -- 8 tests pass
- `cargo test --test llm_integration` -- 12 tests pass
- `cargo test config::tests` -- 10 tests pass (2 flaky due to pre-existing env var race condition when run in parallel, pass individually)
- `cargo clippy -- -D warnings -A clippy::uninlined-format-args` -- zero warnings (uninlined-format-args are pre-existing in other modules)

## Self-Check: PASSED

All 6 key files verified present. Both commit hashes (3c2a5b0, 2c928ea) found in git log.
