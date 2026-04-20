---
phase: 10
slug: tech-debt-integration-test-coverage-and-api-cleanup
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-20
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test -- --include-ignored` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test -- --include-ignored`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 10-01-01 | 01 | 1 | CLI-01 | — | mock server isolates network | integration | `cargo test test_upload_command_happy_path` | ✅ | ⬜ pending |
| 10-01-02 | 01 | 1 | CLI-01 | — | mock server isolates network | integration | `cargo test test_update_command_happy_path` | ❌ W0 | ⬜ pending |
| 10-02-01 | 02 | 2 | CLI-02 | — | N/A | unit | `cargo test` | ✅ | ⬜ pending |
| 10-02-02 | 02 | 2 | CLI-02 | — | N/A | unit | `cargo build` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/cli_integration.rs` — add `test_update_command_happy_path` stub

*Existing infrastructure covers all other phase requirements.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
