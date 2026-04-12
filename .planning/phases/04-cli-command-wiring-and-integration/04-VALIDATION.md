---
phase: 4
slug: cli-command-wiring-and-integration
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-12
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) + wiremock 0.6 + assert_cmd 2 |
| **Config file** | none — Cargo.toml workspace |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 04-01-01 | 01 | 1 | CLI-01 | — | update routes convert→fetch→merge→upload correctly | integration | `cargo test --test '*' -- update` | ❌ W0 | ⬜ pending |
| 04-01-02 | 01 | 1 | CLI-02 | — | upload converts and overwrites page without LLM | integration | `cargo test --test '*' -- upload` | ❌ W0 | ⬜ pending |
| 04-01-03 | 01 | 1 | CLI-03 | — | convert writes storage XML + SVGs to output dir | integration | `cargo test --test '*' -- convert` | ❌ W0 | ⬜ pending |
| 04-02-01 | 02 | 2 | CLI-04 | — | --verbose sends tracing spans to stderr, not stdout | unit | `cargo test --lib -- tracing` | ❌ W0 | ⬜ pending |
| 04-02-02 | 02 | 2 | CLI-05 | — | --output json emits valid JSON on stdout | unit/integration | `cargo test --lib -- json_output` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/cli_integration.rs` — integration stubs for CLI-01 (update), CLI-02 (upload), CLI-03 (convert) using wiremock + tempdir + assert_cmd
- [ ] `tests/cli_integration.rs` — JSON output parsing stubs for CLI-05
- [ ] Unit test stubs in `tests/output_format.rs` or `src/lib.rs` for CLI-04 (stderr routing verification)

*assert_cmd 2 and wiremock 0.6 are already in [dev-dependencies] — no new deps needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end update against live Confluence | CLI-01 | Requires live Confluence credentials and a real page | Run `confluence-agent update doc.md <real_page_url>` and verify page updates |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
