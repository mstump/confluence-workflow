---
phase: 7
slug: test-scaffold-completion
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-13
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + `serial_test` crate (to be added) |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 7-01-01 | 01 | 1 | CLI-01–05 | — | N/A | integration | `cargo test --test cli_integration` | ✅ | ⬜ pending |
| 7-01-02 | 01 | 1 | CLI-01–05 | — | N/A | integration | `cargo test --test output_format` | ✅ | ⬜ pending |
| 7-02-01 | 02 | 1 | CLI-01–05 | — | N/A | unit | `cargo test` (parallel, must pass) | ✅ | ⬜ pending |
| 7-02-02 | 02 | 1 | CLI-01–05 | — | N/A | build | `cargo build` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* Test files `tests/cli_integration.rs` and `tests/output_format.rs` already exist and pass. Wave 0 only needs `serial_test` crate added.

- [ ] `serial_test = "3.4.0"` added to `[dev-dependencies]` in `Cargo.toml`

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
