---
phase: 2
slug: markdown-to-confluence-storage-format-converter
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-10
audited: 2026-04-14
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

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
| 02-01-T1 | 01 | 1 | CONV-01, CONV-02 | T-02-01 | XML-escape text nodes | unit | `cargo test converter::tests::test_mock_converter -q` | No W0 | pending |
| 02-01-T2 | 01 | 1 | CONV-01, CONV-02 | T-02-01, T-02-02, T-02-03 | XML-escape, CDATA split, attr escape | unit + snapshot | `cargo test converter -q` | No W0 | pending |
| 02-02-T1 | 02 | 2 | CONV-01, CONV-02, CONV-05 | T-02-04, T-02-05 | Path traversal prevention, href escape | unit | `cargo test converter -q` | No W0 | pending |
| 02-02-T2 | 02 | 2 | CONV-01, CONV-02, CONV-05 | — | N/A | integration + snapshot | `cargo test converter -q` | No W0 | pending |
| 02-03-T1 | 03 | 3 | CONV-03, CONV-04 | T-02-06, T-02-07, T-02-08 | No shell invoke, subprocess timeout, secure tempfile | unit + integration | `cargo test converter::diagrams -q` | No W0 | pending |
| 02-03-T2 | 03 | 3 | CONV-03, CONV-04 | T-02-09 | Large output bounded by diagram complexity | integration | `cargo test converter -q` | No W0 | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] `src/converter/tests.rs` -- stubs for CONV-01 through CONV-05 (heading, code block, table, link, image, trait mock tests)
- [ ] `src/converter/diagrams.rs` -- stubs for CONV-03, CONV-04 (PlantUML and Mermaid rendering tests)
- [ ] `tests/fixtures/` -- sample markdown documents for spike comparison tests

*Existing infrastructure (cargo test) covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Output renders correctly in Confluence editor | CONV-01 | Requires live Confluence instance | Paste generated XML into Confluence page editor, verify visual rendering |
| PlantUML SVG renders correctly | CONV-03 | Requires live Confluence + PlantUML attachment | Upload SVG attachment, verify image displays |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
