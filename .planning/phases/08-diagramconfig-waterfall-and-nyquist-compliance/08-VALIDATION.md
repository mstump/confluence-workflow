---
phase: 08
slug: diagramconfig-waterfall-and-nyquist-compliance
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-14
audited: 2026-04-15
---

# Phase 08 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust / cargo test |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test --lib config::tests -- --test-threads=1` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~40 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib config::tests -- --test-threads=1`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~5 seconds (unit tests only)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 08-01-01 | 01 | 1 | SCAF-03 | T-08-01 / T-08-02 | plantuml_path from CLI overrides env var; never injected from untrusted source | unit | `cargo test --lib config::tests::test_plantuml_path_cli_override -- --exact` | ✅ | ✅ green |
| 08-01-01 | 01 | 1 | SCAF-03 | T-08-01 / T-08-02 | mermaid_path from CLI overrides env var | unit | `cargo test --lib config::tests::test_mermaid_path_cli_override -- --exact` | ✅ | ✅ green |
| 08-01-01 | 01 | 1 | SCAF-03 | — | Default paths ("plantuml"/"mmdc") used when no CLI or env override | unit | `cargo test --lib config::tests::test_diagram_config_defaults_when_no_override -- --exact` | ✅ | ✅ green |
| 08-01-02 | 01 | 1 | SCAF-03 | T-08-01 | --plantuml-path and --mermaid-path flags accepted by convert arm; page.xml written | integration | `cargo test test_convert_with_diagram_path_flags` | ✅ | ✅ green |
| 08-02-01 | 02 | 1 | SCAF-03 | — | N/A | verification | `cargo test --lib config::tests && cargo test converter && cargo test --lib merge && cargo test --lib llm` | ✅ | ✅ green |
| 08-02-02 | 02 | 1 | SCAF-03 | — | N/A | artifact | `grep -c "nyquist_compliant: true" .planning/phases/0[123]-*/0[123]-VALIDATION.md` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Audit 2026-04-15

| Metric | Count |
|--------|-------|
| Gaps found | 1 |
| Resolved | 1 |
| Escalated | 0 |

Gap resolved: Added `test_convert_with_diagram_path_flags` in `tests/cli_integration.rs` to verify `--plantuml-path` / `--mermaid-path` CLI flags are accepted and wired through the convert arm.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 40s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-04-15
