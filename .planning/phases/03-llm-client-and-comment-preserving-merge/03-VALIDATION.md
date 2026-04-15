---
phase: 3
slug: llm-client-and-comment-preserving-merge
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-10
audited: 2026-04-14
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) + wiremock 0.6 + insta 1 |
| **Config file** | Cargo.toml [dev-dependencies] |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | MERGE-01 | — | N/A | unit | `cargo test merge::extractor::tests` | ❌ W0 | ⬜ pending |
| 03-01-02 | 01 | 1 | MERGE-02 | — | N/A | unit | `cargo test merge::matcher::tests` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 2 | LLM-01 | API key not logged | `x-api-key` never logged | integration | `cargo test --test llm_integration` | ❌ W0 | ⬜ pending |
| 03-02-02 | 02 | 2 | LLM-02 | Prompt injection via comment | tool_use constrains to KEEP/DROP | unit | `cargo test llm::tests` | ❌ W0 | ⬜ pending |
| 03-02-03 | 02 | 2 | LLM-03 | Rate limit DoS | Semaphore + backoff | integration | `cargo test --test llm_integration` | ❌ W0 | ⬜ pending |
| 03-02-04 | 02 | 2 | LLM-04 | — | N/A | unit | `cargo test llm::tests::test_mock_client` | ❌ W0 | ⬜ pending |
| 03-03-01 | 03 | 3 | MERGE-03 | — | N/A | unit (mock) | `cargo test merge::tests` | ❌ W0 | ⬜ pending |
| 03-03-02 | 03 | 3 | MERGE-04 | Rate limit DoS | Bounded concurrency | integration | `cargo test merge::tests::test_bounded_concurrency` | ❌ W0 | ⬜ pending |
| 03-03-03 | 03 | 3 | MERGE-05 | — | N/A | unit | `cargo test merge::injector::tests` | ❌ W0 | ⬜ pending |
| 03-03-04 | 03 | 3 | MERGE-06 | — | N/A | unit | `cargo test merge::tests::test_empty_skip` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/llm/mod.rs` — LlmClient trait + AnthropicClient struct + unit test stubs
- [ ] `src/llm/types.rs` — serde types for Anthropic API request/response
- [ ] `src/merge/mod.rs` — MergeEngine entry point + test stubs
- [ ] `src/merge/extractor.rs` — Comment extraction + test stubs
- [ ] `src/merge/matcher.rs` — Section extraction + matching + test stubs
- [ ] `src/merge/injector.rs` — Comment re-injection + test stubs
- [ ] `tests/llm_integration.rs` — wiremock-based integration tests for Anthropic client

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| API key never appears in log output | LLM-01 (security) | Requires visual inspection of tracing output | Run with `RUST_LOG=trace cargo run -- update doc.md <url>`, grep output for API key value — must not appear |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
