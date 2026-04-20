---
phase: 05-distribution-and-claude-code-skills
reviewed: 2026-04-20T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - .github/workflows/release.yml
  - .markdownlint.yaml
  - Cargo.toml
  - LICENSE
  - skills/confluence-update/SKILL.md
  - skills/confluence-upload/SKILL.md
findings:
  critical: 0
  warning: 4
  info: 2
  total: 6
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-04-20T00:00:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Reviewed distribution infrastructure (GitHub Actions release workflow, Cargo.toml) and two Claude Code skill definitions. The Rust build configuration is idiomatic and correct. The skill files are well-structured. The main concerns are: floating action version tags in the release workflow (supply chain risk), inconsistent runner pinning, overly broad permission scope, and potentially incorrect argument indexing in the skill command templates.

## Warnings

### WR-01: Floating action version tags — supply chain risk

**File:** `.github/workflows/release.yml:38,51,60,65`
**Issue:** Third-party actions are pinned to floating major version tags (`@v1`, `@v2`, `@v4`) rather than immutable commit SHAs. If a tag is silently moved (intentionally or via a compromised account), the workflow will execute different code without any visible diff. This is a common supply chain attack vector for CI/CD pipelines.

Affected lines:
- Line 35: `actions/checkout@v4`
- Line 38: `houseabsolute/actions-rust-cross@v1`
- Line 51: `actions/upload-artifact@v4`
- Line 60: `actions/download-artifact@v4`
- Line 65: `softprops/action-gh-release@v2`

**Fix:** Pin each action to a full commit SHA. Example:
```yaml
# Before
- uses: houseabsolute/actions-rust-cross@v1

# After — pin to a specific commit SHA
- uses: houseabsolute/actions-rust-cross@65fe0df7dd8169d1b5af6d362c13c0a2e71dc560  # v0.2.7
```
Run `gh api repos/houseabsolute/actions-rust-cross/git/refs/tags/v1` to resolve the current SHA for each action.

---

### WR-02: `contents: write` permission granted to all jobs, including build jobs that don't need it

**File:** `.github/workflows/release.yml:8-9`
**Issue:** The top-level `permissions: contents: write` applies to all jobs in the workflow, including the three `build` matrix jobs. Those jobs only compile and upload artifacts — they do not need write access to the repository. Only the `release` job (which calls `action-gh-release`) requires `contents: write`. Granting excess permissions violates the principle of least privilege and increases blast radius if any build-step action is compromised.

**Fix:** Move the permission to job level, scoping it only where needed:
```yaml
# Remove the top-level permissions block entirely, then add to the release job:
jobs:
  build:
    permissions:
      contents: read   # build jobs only need read

  release:
    permissions:
      contents: write  # only this job creates the release
```

---

### WR-03: Inconsistent runner pinning between build and release jobs

**File:** `.github/workflows/release.yml:59`
**Issue:** The Linux build job explicitly pins to `ubuntu-24.04` (line 29), but the `release` job uses `ubuntu-latest` (line 59). `ubuntu-latest` is a floating label that GitHub updates when a new LTS is promoted. This means the job that downloads and publishes artifacts may run on a different OS version than expected. For consistency and reproducibility, pin the release job to the same explicit version.

**Fix:**
```yaml
release:
  needs: build
  runs-on: ubuntu-24.04   # pin to match Linux build matrix
```

---

### WR-04: Skill command templates use `$0` for first user argument — likely incorrect

**File:** `skills/confluence-update/SKILL.md:14`, `skills/confluence-upload/SKILL.md:14`
**Issue:** Both skill command templates use `$0` as the first positional argument:
```bash
confluence-agent update "$0" "$1" --output json
confluence-agent upload "$0" "$1" --output json
```
In POSIX shells, `$0` is the name of the executing script or shell, not the first user-supplied argument. User arguments begin at `$1`. If the Claude Code skill system performs literal string substitution before handing off to bash, `$0` is used as a template placeholder and is intentional. However, if the system evaluates the bash snippet directly in a subshell, `$0` will expand to the shell binary path (e.g., `/bin/bash`) rather than the markdown file path, causing a silent wrong-argument bug.

Review the Claude Code skill argument substitution specification. If substitution is literal (template-style), this is fine. If the snippet runs in an evaluated shell context, change to:
```bash
# If shell-evaluated (standard bash semantics):
confluence-agent update "$1" "$2" --output json
```

## Info

### IN-01: `confluence-update` skill only mentions `ANTHROPIC_API_KEY` — misleading for multi-provider support

**File:** `skills/confluence-update/SKILL.md:20`
**Issue:** The failure hint says "suggest checking credentials (CONFLUENCE_API_TOKEN, ANTHROPIC_API_KEY)". Per CLAUDE.md, the project supports both OpenAI (`OPENAI_API_KEY`) and Google (`GOOGLE_API_KEY`) as LLM providers. Users running the Google Gemini backend will not see their key mentioned and may be confused about what to check.

**Fix:** Broaden the credential hint:
```
- On failure: show the error message and suggest checking credentials
  (CONFLUENCE_API_TOKEN plus one of: ANTHROPIC_API_KEY, OPENAI_API_KEY, or GOOGLE_API_KEY
  depending on configured LLM provider)
```

---

### IN-02: `reqwest` TLS feature name differs between reqwest 0.12 and 0.13 — confirmed working but worth noting

**File:** `Cargo.toml:21`
**Issue:** `reqwest` is declared with `features = ["json", "multipart", "rustls"]`. In reqwest 0.12 the TLS feature was named `rustls-tls`; in 0.13 it was renamed to `rustls`. The Cargo.lock confirms reqwest 0.13.2 is resolved and TLS dependencies (`hyper-rustls`, `tokio-rustls`) are present, so this is working correctly. However, the feature name change means that if the version constraint is ever relaxed downward to 0.12.x (e.g., via a resolver decision), the `rustls` feature would silently not enable TLS. The current lock file protects against this.

**Fix:** No immediate action required — the Cargo.lock pins to 0.13.2 and TLS is confirmed working. Document the version sensitivity if the version constraint is ever changed.

---

_Reviewed: 2026-04-20T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
