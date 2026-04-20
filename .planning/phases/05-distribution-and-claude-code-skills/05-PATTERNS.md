# Phase 5: Distribution and Claude Code Skills - Pattern Map

**Mapped:** 2026-04-16
**Files analyzed:** 5
**Analogs found:** 3 / 5

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `Cargo.toml` | config | batch | `Cargo.toml` (self — existing file to modify) | exact |
| `LICENSE` | config | — | `/Users/matthewstump/src/confluence-workflow/LICENSE` (existing GPL-3 — to replace) | replace |
| `skills/confluence-update/SKILL.md` | utility | request-response | `.claude/commands/confluence-publish.md` | role-match |
| `skills/confluence-upload/SKILL.md` | utility | request-response | `.claude/commands/confluence-publish.md` | role-match |
| `.github/workflows/release.yml` | config | batch | `.github/workflows/publish.yml` | role-match |

## Pattern Assignments

### `Cargo.toml` (config — metadata addition)

**Analog:** `/Users/matthewstump/src/confluence-workflow/Cargo.toml` (existing `[package]` section, lines 1–5)

**Existing package block** (lines 1–5):

```toml
[package]
name = "confluence-agent"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
```

**Fields to add** (insert after `rust-version`, before `[[bin]]`):

```toml
description = "Merge Markdown files into Confluence pages while preserving inline comments"
license = "MIT OR Apache-2.0"
repository = "https://github.com/<owner>/confluence-workflow"
readme = "README.md"
keywords = ["confluence", "markdown", "atlassian", "cli", "merge"]
categories = ["command-line-utilities"]
exclude = ["tests/fixtures/**", ".planning/**", ".github/**"]
```

**Existing release profile** (lines 35–40 — already correct, do not modify):

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

**Key notes:**

- `license = "MIT OR Apache-2.0"` is the Rust ecosystem dual-license convention; requires both `LICENSE-MIT` and `LICENSE-APACHE` files, OR a single `LICENSE` file covering both
- The existing `LICENSE` file contains GPL-3, which is incompatible with crates.io `"MIT OR Apache-2.0"` declaration — the LICENSE file must be replaced
- `exclude` list prevents large test fixtures from bloating the `.crate` file past crates.io 10 MB limit
- Release profile is already optimal (4.0 MB stripped binary confirmed)

---

### `LICENSE` (config — replacement)

**Analog:** `/Users/matthewstump/src/confluence-workflow/LICENSE` (existing file — GPL-3, must be replaced)

**Critical finding:** The existing `LICENSE` is GNU GPL v3. This is incompatible with crates.io publication under `license = "MIT OR Apache-2.0"`. The planner must decide: either keep GPL-3 and set `license = "GPL-3.0"` in Cargo.toml, or replace the LICENSE file with MIT/Apache-2.0 text.

**If choosing MIT OR Apache-2.0 (Rust ecosystem convention):**

- Replace `LICENSE` with MIT license text (single file approach), or
- Create `LICENSE-MIT` and `LICENSE-APACHE` as separate files (dual-file approach)
- crates.io accepts either; the single `LICENSE` field in Cargo.toml must match exactly what is in the file

**If keeping GPL-3 (simplest — no file change needed):**

```toml
# In Cargo.toml [package]:
license = "GPL-3.0"
```

- No LICENSE file changes required
- crates.io accepts GPL-3.0
- Caveat: GPL-3.0 is copyleft; users must open-source derivative works

**Recommendation from RESEARCH.md (open question A2):** Use `"MIT OR Apache-2.0"` for Rust ecosystem compatibility. This requires replacing the existing LICENSE file.

---

### `skills/confluence-update/SKILL.md` (utility, request-response)

**Analog:** `/Users/matthewstump/src/confluence-workflow/.claude/commands/confluence-publish.md`

**Existing commands format pattern** (entire file — lines 1–29):

```markdown
Publish the current markdown file to Confluence using the confluence-agent CLI.

**Arguments:** `$ARGUMENTS` (optional: a Confluence page URL)

## Steps

1. Identify the markdown file to publish:
   ...
4. Run the update command using the intelligent LLM merge workflow:
   ```bash
   export LOG_LEVEL='INFO'
   export PYTHONPATH=./src
   uv run python -m confluence_agent.cli update '<markdown_path>' '<page_url>'
   ```

```

**Key structural differences for the new skill format:**
- Old format (`.claude/commands/*.md`): no frontmatter, freeform prose, invokes Python CLI via `uv run`
- New format (`.claude/skills/*/SKILL.md`): YAML frontmatter required, invokes Rust binary directly

**New skill frontmatter pattern** (from RESEARCH.md Pattern 1):
```yaml
---
name: confluence-update
description: Update a Confluence page from a local Markdown file using the full merge pipeline (preserves inline comments). Use when the user wants to sync a .md file to Confluence.
argument-hint: "<markdown_path> <confluence_page_url>"
disable-model-invocation: true
allowed-tools: Bash(confluence-agent *)
---
```

**New skill invocation pattern** (Rust binary, not Python):

```bash
confluence-agent update "$0" "$1" --output json
```

**Success/failure reporting pattern** (from main.rs JSON output, lines 30–43):

- On success: JSON contains `page_url`, `comments_kept`, `comments_dropped`
- On failure: JSON contains error message; exit code 1

**Key differences from existing `.claude/commands/confluence-publish.md`:**

- Existing command: interactive multi-step workflow, asks user for confirmation, uses Python `uv run`
- New skill: single-command execution, `disable-model-invocation: true`, uses Rust binary directly, `--output json` for machine-readable output
- New skill uses `$0`/`$1` positional argument indexing from `$ARGUMENTS`
- `allowed-tools: Bash(confluence-agent *)` scopes shell access to only the binary (not blanket Bash)

---

### `skills/confluence-upload/SKILL.md` (utility, request-response)

**Analog:** `/Users/matthewstump/src/confluence-workflow/.claude/commands/confluence-publish.md`

Same structural pattern as `confluence-update/SKILL.md` above, with these differences:

**Frontmatter:**

```yaml
---
name: confluence-upload
description: Upload a Markdown file directly to a Confluence page, overwriting existing content without LLM merge. Use when the user wants a fast overwrite with no comment preservation.
argument-hint: "<markdown_path> <confluence_page_url>"
disable-model-invocation: true
allowed-tools: Bash(confluence-agent *)
---
```

**Invocation command** (uses `upload` subcommand, not `update`):

```bash
confluence-agent upload "$0" "$1" --output json
```

**Success JSON shape** (from main.rs lines 60–62 — simpler than update):

- On success: JSON contains only `page_url` (no comment counts)
- On failure: JSON contains error message; exit code 1

---

### `.github/workflows/release.yml` (config, batch)

**Analog:** `/Users/matthewstump/src/confluence-workflow/.github/workflows/publish.yml`

**Existing workflow structural pattern** (lines 1–43 — Docker publish workflow):

```yaml
name: Publish Docker image

on:
  push:
    branches:
      - main

jobs:
  build_and_publish:
    name: Build and publish Docker image
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - name: Checkout
        uses: actions/checkout@v4
      ...
      - uses: docker/build-push-action@v5
        with:
          ...
```

**Pattern elements to carry forward from existing workflow:**

- `uses: actions/checkout@v4` — same version already in use
- Single-job structure (existing) becomes two-job structure (build matrix + release)
- `permissions:` block at job level — existing workflow uses this pattern

**New release workflow structural differences:**

- Trigger: `push: tags: - "v*"` instead of branch push
- `permissions: contents: write` (release creation requires write, not read)
- Matrix strategy with `fail-fast: false` (no analog in existing workflow)
- Two jobs (`build` + `release`) with `needs: build` dependency (existing has single job)
- Uses `houseabsolute/actions-rust-cross@v1` instead of Docker actions
- Uses `softprops/action-gh-release@v2` for release creation (no analog)
- Uses `actions/upload-artifact@v4` + `actions/download-artifact@v4` for inter-job artifact passing

**Complete new workflow pattern** (from RESEARCH.md Pattern 2):

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  build:
    name: Build - ${{ matrix.platform.name }}
    strategy:
      fail-fast: false
      matrix:
        platform:
          - name: macOS-arm64
            runs-on: macos-latest
            target: aarch64-apple-darwin
            archive: confluence-agent-aarch64-apple-darwin.tar.gz

          - name: macOS-x86_64
            runs-on: macos-latest
            target: x86_64-apple-darwin
            archive: confluence-agent-x86_64-apple-darwin.tar.gz

          - name: Linux-x86_64
            runs-on: ubuntu-24.04
            target: x86_64-unknown-linux-musl
            archive: confluence-agent-x86_64-unknown-linux-musl.tar.gz

    runs-on: ${{ matrix.platform.runs-on }}
    steps:
      - uses: actions/checkout@v4

      - name: Build binary
        uses: houseabsolute/actions-rust-cross@v1
        with:
          command: build
          target: ${{ matrix.platform.target }}
          args: "--locked --release"
          strip: true

      - name: Package archive
        run: |
          cp target/${{ matrix.platform.target }}/release/confluence-agent .
          tar czf ${{ matrix.platform.archive }} confluence-agent
          rm confluence-agent

      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.platform.archive }}
          path: ${{ matrix.platform.archive }}

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          merge-multiple: true
          path: artifacts/

      - uses: softprops/action-gh-release@v2
        with:
          files: artifacts/*
```

---

## Shared Patterns

### Binary Invocation (CLI subcommands and flags)

**Source:** `/Users/matthewstump/src/confluence-workflow/src/main.rs` lines 29–43
**Apply to:** Both SKILL.md files

```rust
// JSON output mode (--output json):
// Success: prints JSON to stdout, exits 0
// Failure: prints JSON error to stdout, exits 1
//
// The binary subcommands are: update, upload, convert
// --output json flag enables machine-readable output
```

Skills must use `--output json` so Claude can parse the structured result rather than scraping human-readable text.

### GitHub Actions Checkout Version

**Source:** `/Users/matthewstump/src/confluence-workflow/.github/workflows/publish.yml` line 17
**Apply to:** `.github/workflows/release.yml`

```yaml
- uses: actions/checkout@v4
```

The existing workflow already pins to `@v4` — maintain this version for consistency.

### GitHub Actions Permissions Block

**Source:** `/Users/matthewstump/src/confluence-workflow/.github/workflows/publish.yml` lines 11–13
**Apply to:** `.github/workflows/release.yml`

```yaml
permissions:
  contents: read   # existing pattern (read only)
  packages: write
```

The new release workflow needs `contents: write` instead of `contents: read` to create GitHub Releases.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `skills/confluence-update/SKILL.md` (new format) | utility | request-response | The `.claude/commands/confluence-publish.md` analog uses the legacy commands format and Python CLI; the SKILL.md YAML frontmatter and binary invocation are new patterns with no existing analog in `.claude/skills/` (directory does not exist yet) |

## Critical Pre-Implementation Findings

1. **LICENSE conflict:** The existing `LICENSE` is GPL-3. crates.io `license = "MIT OR Apache-2.0"` will be rejected if the actual `LICENSE` file contains GPL-3 text. The planner must include a decision step on license choice before the Cargo.toml metadata task.

2. **`.claude/skills/` directory does not exist:** The `skills/` directory in the repo is also new. Both must be created as part of the plan. The existing `.claude/commands/` directory is the closest structural analog.

3. **`.github/workflows/` exists** with `publish.yml` — the `release.yml` file is an additive new file in the same directory, not a modification of the existing workflow.

4. **`skills/` in repo vs. `~/.claude/skills/` on user machine:** Skill files will live at `skills/confluence-update/SKILL.md` in the repo for distribution, but users must manually copy them to `~/.claude/skills/` (or the README must document this). The planner should include a documentation step.

## Metadata

**Analog search scope:** `/Users/matthewstump/src/confluence-workflow/` — Cargo.toml, LICENSE, .github/workflows/, .claude/commands/, src/main.rs
**Files scanned:** 6
**Pattern extraction date:** 2026-04-16
