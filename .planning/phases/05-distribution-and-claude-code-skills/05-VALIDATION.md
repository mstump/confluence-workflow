---
phase: 5
slug: distribution-and-claude-code-skills
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-20
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test + shell scripts |
| **Config file** | Cargo.toml (existing) |
| **Quick run command** | `cargo build --release` |
| **Full suite command** | `cargo test && cargo build --release && ls -lh target/release/confluence-agent` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo build --release`
- **After every plan wave:** Run `cargo test && cargo build --release`
- **Before `/gsd-verify-work`:** Full suite must be green + binary size verified
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 1 | DIST-01 | — | N/A | build | `cargo build --release 2>&1 \| grep -v warning` | ✅ | ⬜ pending |
| 05-01-02 | 01 | 1 | DIST-01 | — | N/A | manual | `cargo install --path . && confluence-agent --help` | ✅ | ⬜ pending |
| 05-01-03 | 01 | 1 | DIST-04 | — | N/A | shell | `ls -lh target/release/confluence-agent \| awk '{print $5}'` | ✅ | ⬜ pending |
| 05-02-01 | 02 | 1 | DIST-02 | — | N/A | file | `test -f skills/confluence-update/SKILL.md` | ❌ W0 | ⬜ pending |
| 05-02-02 | 02 | 1 | DIST-02 | — | N/A | grep | `grep 'disable-model-invocation: true' skills/confluence-update/SKILL.md` | ❌ W0 | ⬜ pending |
| 05-03-01 | 03 | 2 | DIST-03 | — | N/A | file | `test -f .github/workflows/release.yml` | ❌ W0 | ⬜ pending |
| 05-03-02 | 03 | 2 | DIST-03 | — | N/A | grep | `grep 'macos-latest' .github/workflows/release.yml` | ❌ W0 | ⬜ pending |
| 05-03-03 | 03 | 2 | DIST-03 | — | N/A | grep | `grep 'ubuntu' .github/workflows/release.yml` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `skills/confluence-update/SKILL.md` — create skills directory stub
- [ ] `skills/confluence-upload/SKILL.md` — create skills directory stub
- [ ] `.github/workflows/release.yml` — create CI workflow stub

*Wave 0 creates the files that tasks will populate with content.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `cargo install confluence-agent` from crates.io works | DIST-01 | Requires crates.io publish (credentials + human approval) | After publish: `cargo install confluence-agent && confluence-agent --help` |
| Claude Code skill appears in `/` menu | DIST-02 | Requires live Claude Code instance and skill installation | Copy skill to `~/.claude/skills/confluence-update/`; open Claude Code; verify skill appears |
| GitHub Actions CI runs on tag push | DIST-03 | Requires a real git tag push to GitHub | Push a `v0.0.1-test` tag; verify Actions run and produce artifacts |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
