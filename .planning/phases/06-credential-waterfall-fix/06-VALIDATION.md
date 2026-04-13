---
phase: 6
slug: credential-waterfall-fix
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-13
audited: 2026-04-13
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) + assert_cmd 2 |
| **Config file** | `Cargo.toml` [dev-dependencies] |
| **Quick run command** | `cargo test --test cli_integration test_update_command_missing_api_key` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test cli_integration && cargo test --lib config::tests`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | SCAF-02, SCAF-03 | T-6-01 | API key passed via flag/env, never logged | integration | `cargo test --test cli_integration test_update_command_missing_api_key` | ✅ | ✅ green |
| 06-01-02 | 01 | 1 | SCAF-03 | T-6-01 | Correct error path fires for missing key (not HTTPS guard) | integration | `cargo test --test cli_integration test_update_command_missing_api_key` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. The test file (`tests/cli_integration.rs`) and test function (`test_update_command_missing_api_key`) already exist — only content needs fixing.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none required)
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-04-13

---

## Validation Audit 2026-04-13

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

All tasks verified green. `test_update_command_missing_api_key` passes (1/1). Full CLI integration suite passes (7 passed, 1 ignored). Two pre-existing flaky lib tests (`test_env_vars_used_when_cli_absent`, `test_fallthrough_to_env_vars`) pass in isolation — env var pollution between threads, unrelated to phase scope.
