---
phase: 2
slug: markdown-to-confluence-storage-format-converter
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-10
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
| 02-01-01 | 01 | 0 | CONV-01 | — | N/A | unit | `cargo test converter::spike` | ❌ W0 | ⬜ pending |
| 02-02-01 | 02 | 1 | CONV-01 | — | N/A | unit | `cargo test converter::tests` | ❌ W0 | ⬜ pending |
| 02-02-02 | 02 | 1 | CONV-02 | — | N/A | unit | `cargo test converter::tests::frontmatter` | ❌ W0 | ⬜ pending |
| 02-02-03 | 02 | 2 | CONV-03 | — | N/A | unit | `cargo test converter::tests::images` | ❌ W0 | ⬜ pending |
| 02-02-04 | 02 | 2 | CONV-04 | — | N/A | unit | `cargo test converter::tests::trait_mock` | ❌ W0 | ⬜ pending |
| 02-03-01 | 03 | 1 | CONV-05 | — | N/A | unit | `cargo test diagram::tests` | ❌ W0 | ⬜ pending |
| 02-03-02 | 03 | 2 | CONV-05 | — | N/A | integration | `cargo test diagram::tests::mermaid` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/converter/tests.rs` — stubs for CONV-01 through CONV-04 (heading, code block, table, link, image conversion tests)
- [ ] `src/diagram/tests.rs` — stubs for CONV-05 (PlantUML and Mermaid rendering tests)
- [ ] `tests/fixtures/` — sample markdown documents for spike comparison tests

*Existing infrastructure (cargo test) covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Output renders correctly in Confluence editor | CONV-01 | Requires live Confluence instance | Paste generated XML into Confluence page editor, verify visual rendering |
| PlantUML SVG renders correctly | CONV-05 | Requires live Confluence + PlantUML attachment | Upload SVG attachment, verify image displays |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
