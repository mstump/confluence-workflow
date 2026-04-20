---
phase: 9
slug: convert-waterfall-fix-and-phase-08-verification
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-20
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust, serial_test for env-var isolation) |
| **Config file** | `.cargo/config.toml` (enforces `-D warnings`) |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && markdownlint --fix .planning/phases/08-diagramconfig-waterfall-and-nyquist-compliance/08-VERIFICATION.md` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test && cargo build`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 09-01-01 | 01 | 1 | SCAF-03 | — | N/A | unit | `cargo test test_diagram_config` | ✅ | ⬜ pending |
| 09-01-02 | 01 | 1 | SCAF-03 | — | N/A | integration | `cargo test test_convert_with_diagram_path_flags` | ✅ | ⬜ pending |
| 09-01-03 | 01 | 1 | SCAF-03 | — | N/A | integration | `cargo test test_convert_with_env_var_diagram_paths` | ❌ W0 | ⬜ pending |
| 09-02-01 | 02 | 1 | SCAF-03 | — | N/A | artifact | `test -f .planning/phases/08-diagramconfig-waterfall-and-nyquist-compliance/08-VERIFICATION.md` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/cli_integration.rs` — add stub `test_convert_with_env_var_diagram_paths` (D-06)
- [ ] `.planning/phases/08-diagramconfig-waterfall-and-nyquist-compliance/08-VERIFICATION.md` — created by Plan 09-02

*Existing infrastructure (cargo test, serial_test, assert_cmd) covers all other phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| VERIFICATION.md accurately reflects Phase 08 codebase state | D-07 | Document authorship requires human judgment on scoring | Read `08-VERIFICATION.md` and spot-check 2–3 evidence citations against the actual source files |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
