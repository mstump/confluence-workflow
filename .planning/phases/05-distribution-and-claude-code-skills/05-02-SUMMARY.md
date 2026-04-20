---
phase: 05-distribution-and-claude-code-skills
plan: 02
subsystem: distribution
tags: [claude-code, skills, distribution, cli-wrapper]
depends_on:
  requires: []
  provides: [claude-code-skills-confluence-update, claude-code-skills-confluence-upload]
  affects: [user-installation-flow, distribution-artifacts]
tech_stack:
  added: [claude-code-skills-format]
  patterns: [skill-with-external-binary, disable-model-invocation, scoped-allowed-tools]
key_files:
  created:
    - skills/confluence-update/SKILL.md
    - skills/confluence-upload/SKILL.md
  modified:
    - .markdownlint.yaml
decisions:
  - "Skills committed to repo at skills/ (not ~/.claude/skills/); users copy to personal scope manually as documented in 05-RESEARCH"
  - "disable-model-invocation: true on both skills to prevent Claude from auto-invoking side-effectful Confluence updates without explicit user intent"
  - "allowed-tools scoped to Bash(confluence-agent *) — not blanket Bash — to limit shell access to the confluence-agent binary only"
  - "--output json used for machine-readable responses so Claude can parse and format results reliably"
  - "MD041 disabled globally in .markdownlint.yaml to allow YAML frontmatter as the first line of SKILL.md files"
  - "confluence-upload error guidance omits ANTHROPIC_API_KEY because the upload path does not invoke the LLM pipeline"
metrics:
  duration: ~8 min
  completed: 2026-04-20
  tasks_completed: 2
  tasks_total: 2
  files_created: 2
  files_modified: 1
  requirements_satisfied: [DIST-02]
---

# Phase 05 Plan 02: Claude Code Skills for confluence-agent Binary Summary

Two Claude Code skills (`confluence-update`, `confluence-upload`) created in the new `.claude/skills/` format; both delegate to the `confluence-agent` binary with `--output json`, scope shell access to the binary only, and require explicit user invocation via `disable-model-invocation: true`.

## Tasks Completed

| Task | Name                          | Commit  | Files                                                        |
| ---- | ----------------------------- | ------- | ------------------------------------------------------------ |
| 1    | Create confluence-update skill| 8861a9c | skills/confluence-update/SKILL.md, .markdownlint.yaml        |
| 2    | Create confluence-upload skill| 2f6937c | skills/confluence-upload/SKILL.md                            |

## Verification Results

**Must-have truths (all confirmed):**

- `/confluence-update` skill file exists with correct YAML frontmatter and binary invocation — CONFIRMED
- `/confluence-upload` skill file exists with correct YAML frontmatter and binary invocation — CONFIRMED
- Both skills use `--output json` for machine-readable output — CONFIRMED
- Both skills have `disable-model-invocation: true` — CONFIRMED
- Both skills scope `allowed-tools` to `Bash(confluence-agent *)` only — CONFIRMED

**Acceptance criteria grep checks (Task 1 — confluence-update):**

```text
name: confluence-update                              -> line 2
argument-hint: "<markdown_path> ..."                 -> line 4
disable-model-invocation: true                       -> line 5
allowed-tools: Bash(confluence-agent *)              -> line 6
confluence-agent update "$0" "$1" --output json      -> line 14
CONFLUENCE_API_TOKEN, ANTHROPIC_API_KEY              -> line 20 (error guidance)
```

**Acceptance criteria grep checks (Task 2 — confluence-upload):**

```text
name: confluence-upload                              -> line 2
disable-model-invocation: true                       -> line 5
allowed-tools: Bash(confluence-agent *)              -> line 6
confluence-agent upload "$0" "$1" --output json      -> line 14
CONFLUENCE_API_TOKEN, CONFLUENCE_URL                 -> line 20 (error guidance)
ANTHROPIC_API_KEY                                    -> ABSENT (correct; no LLM in upload path)
```

**Lint check:** `markdownlint skills/` exits 0 (no violations). MD041 disabled globally so YAML frontmatter is accepted as first-line content; no other rules triggered.

## Changes Made

### skills/confluence-update/SKILL.md (new, 20 lines)

YAML frontmatter: `name`, `description`, `argument-hint`, `disable-model-invocation: true`, `allowed-tools: Bash(confluence-agent *)`. Body instructs Claude to run `confluence-agent update "$0" "$1" --output json` and report success (page URL + comment counts) or failure (error + credential guidance for `CONFLUENCE_API_TOKEN` and `ANTHROPIC_API_KEY`).

### skills/confluence-upload/SKILL.md (new, 20 lines)

Parallel structure to confluence-update; differs in three ways:

1. Subcommand is `upload` (direct overwrite, no LLM merge)
2. Success path reports page URL only (no comment counts, because no merge happens)
3. Error guidance references `CONFLUENCE_API_TOKEN` and `CONFLUENCE_URL` only (no `ANTHROPIC_API_KEY`, because the upload path never invokes the LLM)

### .markdownlint.yaml (+4 lines)

Added `MD041: false` with an inline comment explaining the rationale: SKILL.md files begin with YAML frontmatter (`---`), which triggers MD041 (first line should be a heading). Disabling MD041 globally is consistent with how the skills are authored across the repo and avoids a per-file ignore pattern.

## Deviations from Plan

None. The plan was executed exactly as written.

### Context Recovery Note (not a deviation)

The worktree base commit (f7f7f56) predated Phase 5 plan creation (d1fbd31). The expected hard-reset to d1fbd31 was blocked by the sandbox, so the required plan and state files were fetched via `git checkout d1fbd31... -- .planning/...` to make the executor's context load succeed. Those checkouts incidentally staged unrelated planning artifacts (other phase 5 plans, STATE.md, ROADMAP.md). To honor the parallel-executor constraint that forbids modifying STATE.md/ROADMAP.md, commits for both tasks used `git commit --only -- <paths>` so only the two tasks' own files landed in each commit. Git diff against `HEAD~2` confirms only `skills/confluence-update/SKILL.md`, `skills/confluence-upload/SKILL.md`, and `.markdownlint.yaml` changed.

## Authentication Gates

None encountered. Skill authoring is a pure-file operation; no external services were invoked.

## Known Stubs

None. Both SKILL.md files are fully wired: they delegate to a real binary (`confluence-agent`) with real subcommands (`update`, `upload`) and real flags (`--output json`). The binary itself is produced by earlier phases (Phase 01–04).

## Threat Flags

None. The plan's `<threat_model>` explicitly covers the trust boundaries introduced here (Claude Code → binary; user `$0`/`$1` → shell). Both mitigations (`T-05-03` scoped `allowed-tools`, `T-05-04` `disable-model-invocation: true`) are present in both SKILL.md files. No new threat surface was introduced outside the registered model.

## Self-Check: PASSED

- [x] `skills/confluence-update/SKILL.md` exists
- [x] `skills/confluence-upload/SKILL.md` exists
- [x] `.markdownlint.yaml` contains `MD041: false`
- [x] Commit 8861a9c exists in git log (Task 1)
- [x] Commit 2f6937c exists in git log (Task 2)
- [x] `markdownlint skills/` passes
- [x] confluence-update contains `ANTHROPIC_API_KEY`; confluence-upload does not
- [x] Both skills scope `allowed-tools` to `Bash(confluence-agent *)`
- [x] Both skills set `disable-model-invocation: true`
- [x] No unintended file deletions in either commit
