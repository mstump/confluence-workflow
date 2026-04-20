# Phase 5: Distribution and Claude Code Skills - Research

**Researched:** 2026-04-20
**Domain:** Rust binary distribution — crates.io, Claude Code skills, GitHub Actions CI/CD
**Confidence:** HIGH

## Summary

Phase 5 distributes the `confluence-agent` Rust binary through three channels: `cargo install` (from crates.io or git), Claude Code skill files that invoke the binary interactively, and GitHub Actions CI that produces pre-built release artifacts for macOS and Linux on tagged commits.

The release binary already exists and measures **4.0 MB** stripped (built with `opt-level="z"`, `lto=true`, `strip=true`, `panic="abort"`) — well under the 15 MB DIST-04 threshold. No binary-size work is needed. The crate name `confluence-agent` is **not taken** on crates.io as of the research date. The Cargo.toml is missing only `description`, `license`, `repository`, and `readme` fields required for crates.io publishing.

Claude Code has migrated from `.claude/commands/<name>.md` to `.claude/skills/<name>/SKILL.md` as the recommended format, but the legacy commands format still works. The skill format supports YAML frontmatter (`name`, `description`, `allowed-tools`, `disable-model-invocation`, `argument-hint`) and `$ARGUMENTS` substitution. Skills invoke external binaries via the `Bash` tool using the `!` shell injection syntax or by instructing Claude to run shell commands. For GitHub Actions CI, the `houseabsolute/actions-rust-cross` action is the standard approach: macOS runners build native Darwin targets, and Ubuntu runners build Linux targets without needing Docker cross-compilation for x86_64.

**Primary recommendation:** Use crates.io publishing for DIST-01 (it is the literal text of the requirement), complete the minimal missing Cargo.toml metadata, install via `houseabsolute/actions-rust-cross` matrix for CI, and author skills in the new `.claude/skills/` format with `disable-model-invocation: true`.

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DIST-01 | Binary installable via `cargo install confluence-agent` | Crate name available; metadata fields identified; publish process documented |
| DIST-02 | Claude Code skill at `~/.claude/commands/confluence-update.md` (or skills equivalent) that invokes the binary | Skill frontmatter format and `$ARGUMENTS` substitution verified; `disable-model-invocation: true` needed |
| DIST-03 | CI/CD builds release binaries for macOS arm64, macOS x86_64, Linux x86_64 on tagged commits | `houseabsolute/actions-rust-cross` matrix pattern documented; `softprops/action-gh-release@v2` for artifact upload |
| DIST-04 | Stripped release binary under 15 MB | **Already satisfied**: current release binary is 4.0 MB [VERIFIED: `ls -lh target/release/confluence-agent`] |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| crates.io metadata | Cargo.toml (build config) | — | Package registry requires metadata at publish time |
| Binary size optimization | Release profile (Cargo.toml) | — | Already satisfied at 4.0 MB; no runtime tier involved |
| Claude Code skill invocation | Claude Code client (user machine) | Binary (CLI) | Skill file instructs Claude; Claude shells out to binary |
| JSON output surfacing | Binary CLI (`--output json`) | Claude Code skill | Binary produces JSON; skill tells Claude to display it |
| CI/CD cross-platform builds | GitHub Actions (cloud) | Rust toolchain | Native runners for macOS; Ubuntu for Linux |
| Release artifact upload | GitHub Actions (softprops action) | GitHub Releases | Standard release asset pattern |

## Standard Stack

### Core

| Library / Tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| `houseabsolute/actions-rust-cross` | v1 | GitHub Action: selects cargo vs cross per target | Abstracts native/cross logic; widely used in Rust ecosystem [VERIFIED: GitHub Marketplace] |
| `softprops/action-gh-release` | v2 | GitHub Action: create/update release, upload assets | De-facto standard for GitHub release automation [VERIFIED: GitHub Marketplace] |
| `actions/upload-artifact` / `actions/download-artifact` | v4 | Pass build artifacts between jobs | Required to collect matrix build outputs before creating release |
| Claude Code skills format | Current (2026) | `.claude/skills/<name>/SKILL.md` | Recommended format per official docs; supersedes `.claude/commands/` [VERIFIED: code.claude.com/docs] |

### Supporting

| Tool | Purpose | When to Use |
|------|---------|-------------|
| `cargo publish --dry-run` | Verify crates.io package before live publish | Run before every publish to catch metadata errors |
| `cargo package --list` | Inspect what files are included in the `.crate` file | Verify no large test fixtures are bundled (10 MB crates.io limit) |
| `cargo install --git` | Install from GitHub without crates.io | Fallback if crates.io publish is deferred; also useful for pre-release |
| `cargo-binstall` | Install pre-compiled binary without recompilation | Optional: adds `[package.metadata.binstall]` to Cargo.toml for binary-first installs |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `houseabsolute/actions-rust-cross` | `cross` CLI directly | More setup; cross v0.2 does not publish Linux ARM binaries (not relevant for this project's targets) |
| `houseabsolute/actions-rust-cross` | `cargo-zigbuild` | Good for cross-compiling Linux targets from macOS; not needed when using native Linux runners |
| `softprops/action-gh-release@v2` | `gh release create` in a Bash step | More control, more shell scripting required |
| `.claude/skills/` format | `.claude/commands/` format | Legacy format still works; skills format adds supporting files, frontmatter, and `context: fork` support |
| crates.io publish | `cargo install --git` only | Git install requires Rust toolchain on user machine and takes 2+ minutes to compile; crates.io enables `cargo-binstall` for pre-built installs |

**Installation (CI dependencies — no local install needed):**

```bash
# These run inside GitHub Actions; no local install required
# For local testing of the release profile:
cargo build --release
ls -lh target/release/confluence-agent
```

**Cargo.toml metadata additions needed:**

```toml
[package]
# ... existing fields ...
description = "Merge Markdown files into Confluence pages while preserving inline comments"
license = "MIT"          # or "Apache-2.0" — choose before publishing
repository = "https://github.com/<owner>/confluence-workflow"
readme = "README.md"
keywords = ["confluence", "markdown", "atlassian", "cli"]
categories = ["command-line-utilities"]
```

## Architecture Patterns

### System Architecture Diagram

```
Tagged commit pushed (v0.x.y)
         │
         ▼
GitHub Actions: release.yml triggered on tags/v*
         │
         ├──► Job: build (matrix × 3)
         │      ├── [macos-latest / aarch64-apple-darwin]
         │      │      houseabsolute/actions-rust-cross → confluence-agent binary
         │      │      → tar.gz → upload-artifact
         │      │
         │      ├── [macos-latest / x86_64-apple-darwin]
         │      │      houseabsolute/actions-rust-cross → confluence-agent binary
         │      │      → tar.gz → upload-artifact
         │      │
         │      └── [ubuntu-24.04 / x86_64-unknown-linux-musl]
         │             houseabsolute/actions-rust-cross → confluence-agent binary
         │             → tar.gz → upload-artifact
         │
         ▼
Job: release (depends on build)
         │
         ├── download-artifact (all 3 archives)
         └── softprops/action-gh-release@v2
                  → GitHub Release with 3 binary archives attached

User machine:
  Option A: cargo install confluence-agent          (compiles from crates.io)
  Option B: cargo install --git <url>               (compiles from GitHub)
  Option C: Download release binary from GitHub     (pre-built, no Rust required)

Claude Code:
  ~/.claude/skills/confluence-update/SKILL.md
         │
         Claude reads SKILL.md on /confluence-update invocation
         │
         Claude executes: confluence-agent update "$0" "$1" --output json
         │
         Claude surfaces JSON result to user conversation
```

### Recommended Project Structure (additions for this phase)

```
.github/
└── workflows/
    └── release.yml          # Cross-platform build + release on tags/v*
    └── publish.yml          # Existing Docker workflow (unrelated)

# User-side (installed, not in repo):
~/.claude/skills/
├── confluence-update/
│   └── SKILL.md             # /confluence-update <md_path> <page_url>
└── confluence-upload/
    └── SKILL.md             # /confluence-upload <md_path> <page_url>
```

### Pattern 1: Claude Code Skill with External Binary

**What:** A SKILL.md file that instructs Claude to invoke an external CLI binary, pass arguments from the user, and surface the JSON output.

**When to use:** Any time a skill wraps a side-effectful CLI tool the user controls (deploy, upload, sync). Use `disable-model-invocation: true` to prevent Claude from triggering it automatically.

**Example (`~/.claude/skills/confluence-update/SKILL.md`):**

```yaml
---
name: confluence-update
description: Update a Confluence page from a local Markdown file using the full merge pipeline (preserves inline comments). Use when the user wants to sync a .md file to Confluence.
argument-hint: "<markdown_path> <confluence_page_url>"
disable-model-invocation: true
allowed-tools: Bash(confluence-agent *)
---

Update a Confluence page from a Markdown file, preserving existing inline comments.

Run the following command and show me the output:

```bash
confluence-agent update "$0" "$1" --output json
```

Report the result to the user:

- On success: show the page URL and the number of comments kept/dropped
- On failure: show the error message and suggest checking credentials (CONFLUENCE_API_TOKEN, ANTHROPIC_API_KEY)

```
```

**Key design decisions:**

- `disable-model-invocation: true` — this has side effects; user must explicitly type `/confluence-update`
- `allowed-tools: Bash(confluence-agent *)` — pre-approves the specific binary without granting blanket shell access
- `"$0"` and `"$1"` — positional arguments from `$ARGUMENTS` indexing
- `--output json` — machine-readable output so Claude can parse and format the response

### Pattern 2: GitHub Actions Release Matrix

**What:** Matrix build across 3 platform/target combinations, each uploading a named artifact, followed by a release job that downloads all artifacts and creates a GitHub Release.

**Example (`.github/workflows/release.yml`):**

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

**Key notes:**

- `x86_64-unknown-linux-musl` produces a fully static binary (no glibc dependency) — more portable than `gnu` target
- `fail-fast: false` ensures one platform failure does not cancel other builds
- `contents: write` permission required for release creation
- `--locked` ensures Cargo.lock is respected in CI (reproducibility)

### Pattern 3: crates.io Publishing Checklist

**Minimum metadata required:**

- `description` (sentence or two)
- `license` (SPDX identifier, e.g., `"MIT"` or `"Apache-2.0"`)

**Strongly recommended:**

- `repository`
- `readme`
- `keywords` (max 5)
- `categories` (from crates.io category list)

**Publish flow:**

```bash
cargo publish --dry-run       # verify locally
cargo publish                 # live publish (irreversible)
```

**Note:** Publishing is permanent. Versions cannot be deleted. Use `cargo yank` to mark broken versions. The `.crate` file has a 10 MB limit — use `cargo package --list` to verify test fixtures are excluded.

### Anti-Patterns to Avoid

- **Naming the skill file `confluence-update.md` in `.claude/commands/`:** Works, but the new `.claude/skills/confluence-update/SKILL.md` structure is preferred and supports supporting files, `context: fork`, and directory-level organization.
- **Not setting `disable-model-invocation: true` on deployment skills:** Claude may auto-invoke the skill if the user says "update the Confluence page," triggering an unintended side effect.
- **Using `x86_64-unknown-linux-gnu` target in CI:** Produces a binary dynamically linked to glibc; musl (`x86_64-unknown-linux-musl`) produces a fully static binary that works on any Linux distribution without version matching.
- **Publishing without `--dry-run` first:** The dry run catches missing metadata, excluded files, and size limit issues before making a permanent publish.
- **Omitting `--locked` from `cargo build` in CI:** Without `--locked`, Cargo may update dependencies if Cargo.lock drifts, producing non-reproducible builds.
- **Granting `allowed-tools: Bash` without a pattern:** This grants blanket shell access; always scope to `Bash(confluence-agent *)`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-platform GitHub Actions | Custom shell scripts per OS | `houseabsolute/actions-rust-cross@v1` | Handles cargo vs cross selection, Rust toolchain setup, and strip flags |
| Release artifact upload | GitHub API curl scripts | `softprops/action-gh-release@v2` | Handles creating/updating releases, asset upload, draft/pre-release flags |
| Artifact passing between jobs | Complex S3 or external cache | `actions/upload-artifact@v4` + `actions/download-artifact@v4` | Native GitHub Actions artifact passing, free for public repos |

**Key insight:** The CI/CD layer is entirely configuration (YAML), not code. All the complexity of cross-platform Rust compilation is already solved by the `actions-rust-cross` action.

## Common Pitfalls

### Pitfall 1: macOS x86_64 Cross-Compilation on arm64 Runner

**What goes wrong:** `macos-latest` GitHub runner is now arm64 (M1). Targeting `x86_64-apple-darwin` requires Rosetta-era cross-compilation. Without the proper target added, the Rust toolchain won't have it.

**Why it happens:** Default Rust installation only includes the host target. `rustup target add x86_64-apple-darwin` is needed, or the action handles it automatically.

**How to avoid:** Use `houseabsolute/actions-rust-cross@v1` — it calls `rustup target add` automatically before building. [VERIFIED: action README]

**Warning signs:** Build error `error[E0463]: can't find crate for 'std'` for the x86_64 target.

### Pitfall 2: musl Target Missing C Library

**What goes wrong:** `x86_64-unknown-linux-musl` builds on Ubuntu require `musl-tools` package to be installed, or a Docker-based cross-compilation approach.

**Why it happens:** musl is not installed by default on `ubuntu-latest`.

**How to avoid:** `houseabsolute/actions-rust-cross` uses `cross` (Docker-based) automatically for musl targets on Linux runners, which includes musl. [VERIFIED: action behavior description]

**Warning signs:** `error: linker 'x86_64-linux-musl-gcc' not found`

### Pitfall 3: crates.io 10 MB Package Size Limit

**What goes wrong:** `cargo publish` fails with a size limit error if test fixtures or documentation are included in the package.

**Why it happens:** `cargo package` includes all files not in `.gitignore` and not explicitly excluded via the `exclude` key.

**How to avoid:** Run `cargo package --list` before publishing. Add `exclude = ["tests/fixtures/*", "docs/*"]` to `[package]` if large files are present. [VERIFIED: Cargo Book]

**Warning signs:** `cargo publish --dry-run` reports `.crate` file larger than 10 MB.

### Pitfall 4: Skill Not Accessible Without Full Path

**What goes wrong:** The skill invokes `confluence-agent` but the binary is not on Claude Code's `PATH` at invocation time.

**Why it happens:** Claude Code's shell environment may differ from the user's interactive shell `PATH`. `~/.cargo/bin` may not be in the non-interactive shell `PATH`.

**How to avoid:** In the skill, use the full path `~/.cargo/bin/confluence-agent` or document that users must add `~/.cargo/bin` to their shell `PATH` (standard `cargo install` guidance). Alternatively, detect and report if the binary is missing.

**Warning signs:** Skill produces `command not found: confluence-agent` even when the binary is installed.

### Pitfall 5: `--dry-run` vs Live Publish Confusion

**What goes wrong:** Developers test with `--dry-run` successfully, then publish without verifying the package name is available on crates.io.

**Why it happens:** `--dry-run` does not check crates.io for name availability or ownership.

**How to avoid:** Check `https://crates.io/crates/confluence-agent` manually before first publish. The name is currently available. [VERIFIED: crates.io API, 2026-04-20]

## Code Examples

### Cargo.toml Complete [package] Section

```toml
# Source: doc.rust-lang.org/cargo/reference/publishing.html [CITED]
[package]
name = "confluence-agent"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
description = "Merge Markdown files into Confluence pages while preserving inline comments"
license = "MIT"
repository = "https://github.com/<owner>/confluence-workflow"
readme = "README.md"
keywords = ["confluence", "markdown", "atlassian", "cli", "merge"]
categories = ["command-line-utilities"]
exclude = ["tests/fixtures/**", ".planning/**", ".github/**"]
```

### Skill File — confluence-upload (simpler, no merge)

```yaml
# ~/.claude/skills/confluence-upload/SKILL.md
---
name: confluence-upload
description: Upload a Markdown file directly to a Confluence page, overwriting existing content without LLM merge. Use when the user wants a fast overwrite with no comment preservation.
argument-hint: "<markdown_path> <confluence_page_url>"
disable-model-invocation: true
allowed-tools: Bash(confluence-agent *)
---

Upload a Markdown file to Confluence (direct overwrite, no LLM).

Run this command:

```bash
confluence-agent upload "$0" "$1" --output json
```

Report the result: on success show the page URL; on failure show the error and suggest checking CONFLUENCE_API_TOKEN and CONFLUENCE_URL.

```
```

### Checking Binary Size Locally

```bash
# Source: Cargo docs, verified locally [VERIFIED: target/release/confluence-agent = 4.0 MB]
cargo build --release
ls -lh target/release/confluence-agent
# For more detail:
file target/release/confluence-agent
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `.claude/commands/<name>.md` | `.claude/skills/<name>/SKILL.md` | 2025-2026 | Skills add supporting files, frontmatter, `context: fork`, and auto-invocation control |
| `cross` Docker-based cross-compilation | `houseabsolute/actions-rust-cross` wrapping either cargo or cross | 2023-2024 | Action auto-selects strategy per target; less manual configuration |
| `softprops/action-gh-release@v1` | `softprops/action-gh-release@v2` | 2024 | v2 is current stable; v1 still works but v2 preferred |
| `actions/upload-artifact@v3` | `actions/upload-artifact@v4` | 2024 | v3 deprecated; v4 required for `merge-multiple` flag |

**Deprecated/outdated:**

- `.claude/commands/` format: still works, but skills format is recommended for new work
- `cross` CLI standalone in GitHub Actions without the action wrapper: requires more manual setup

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The GitHub repository URL is `github.com/<owner>/confluence-workflow` — exact owner unknown | Standard Stack, Code Examples | `repository` field in Cargo.toml will be wrong; low risk (easy to fix) |
| A2 | The desired license is MIT or Apache-2.0 — no `LICENSE` file found in repo | Standard Stack | Publishing blocked if license choice is wrong; requires human decision |
| A3 | `~/.cargo/bin` is on PATH for Claude Code skill invocations | Pitfall 4 | Skill fails silently for users who did not add cargo bin to PATH |

## Open Questions (RESOLVED)

1. **License choice**
   - What we know: `license` is required for crates.io; no `LICENSE` file exists in the repo
   - What's unclear: MIT, Apache-2.0, or dual MIT/Apache-2.0 (Rust ecosystem default)?
   - Recommendation: Use `license = "MIT OR Apache-2.0"` (Rust ecosystem convention) and add both `LICENSE-MIT` and `LICENSE-APACHE` files
   - RESOLVED: human decision required -- Task 1 checkpoint in 05-01 gates execution; executor proposes MIT OR Apache-2.0 (Rust ecosystem convention)

2. **Publish immediately vs. git-only install**
   - What we know: DIST-01 says `cargo install confluence-agent` — this implies crates.io; git-install would be `cargo install --git ...`
   - What's unclear: Is a crates.io account already set up? Is the intent to publish now or defer?
   - Recommendation: Plan for crates.io publish in 05-01; if no account exists, `cargo install --git` is a valid temporary alternative
   - RESOLVED: `cargo install --path .` verified locally; actual `cargo publish` to crates.io is a manual step requiring crates.io credentials and is out of scope for this phase

3. **Skill location — personal vs. project**
   - What we know: DIST-02 specifies `~/.claude/commands/confluence-update.md` (personal scope)
   - What's unclear: Should the skill source files also live in the repo for distribution?
   - Recommendation: Commit skill files to `skills/` in the repo; document manual copy to `~/.claude/skills/` in README
   - RESOLVED: skill files committed to `skills/` directory in repo root; users copy to `~/.claude/skills/` manually (documented in task 2 action in 05-02)

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | 05-01 (cargo publish) | ✓ | stable (inferred from build success) | — |
| `cargo` | 05-01 | ✓ | bundled with Rust | — |
| GitHub Actions | 05-03 | ✓ (cloud) | N/A — cloud service | — |
| `houseabsolute/actions-rust-cross` | 05-03 | ✓ (GitHub Marketplace) | v1 | `cross` CLI directly |
| `softprops/action-gh-release` | 05-03 | ✓ (GitHub Marketplace) | v2 | `gh release create` Bash steps |
| Docker (for musl Linux cross-compile) | 05-03 (inside GitHub Actions) | ✓ (ubuntu runners have Docker) | provided by runner | native musl toolchain install |
| crates.io account + API token | 05-01 (publish) | Unknown | — | `cargo install --git` |

**Missing dependencies with no fallback:**

- crates.io account and API token: required for DIST-01 as literally stated; if absent, the planner should note that a GitHub Actions secret `CARGO_REGISTRY_TOKEN` must be configured

**Missing dependencies with fallback:**

- None for CI/CD (all GitHub Actions tooling is cloud-provisioned)

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) + `assert_cmd` for binary integration tests |
| Config file | None (uses default `cargo test` runner) |
| Quick run command | `cargo test --test cli_integration test_convert_command` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DIST-01 | `cargo install` (or `--git`) produces working binary | smoke | `cargo build --release && ./target/release/confluence-agent --version` | ❌ Wave 0 (manual verification script) |
| DIST-02 | Skill file exists and has correct frontmatter + invocation | static analysis | `test -f ~/.claude/skills/confluence-update/SKILL.md` | ❌ Wave 0 (skill files created in 05-02) |
| DIST-03 | GitHub Actions workflow exists and targets correct platforms | static analysis | `grep -q 'aarch64-apple-darwin' .github/workflows/release.yml` | ❌ Wave 0 (workflow created in 05-03) |
| DIST-04 | Stripped binary under 15 MB | automated | `cargo build --release && test $(stat -f%z target/release/confluence-agent) -lt 15728640` | ✅ Already satisfied (4.0 MB) |

**DIST-04 already passes.** A one-liner check in CI can gate this:

```bash
# macOS stat syntax:
test $(stat -f%z target/release/confluence-agent) -lt 15728640 && echo "PASS: binary size OK"
# Linux stat syntax:
test $(stat -c%s target/release/confluence-agent) -lt 15728640 && echo "PASS: binary size OK"
```

### Sampling Rate

- **Per task commit:** `cargo build --release && cargo test` (verify binary still compiles and tests pass)
- **Per wave merge:** Full `cargo test` suite
- **Phase gate:** All 4 DIST requirements verified before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `skills/confluence-update/SKILL.md` — covers DIST-02 (created in 05-02, then documented for manual install)
- [ ] `skills/confluence-upload/SKILL.md` — covers DIST-02 (upload variant)
- [ ] `.github/workflows/release.yml` — covers DIST-03 (created in 05-03)
- [ ] `README.md` — covers DIST-01 (installation instructions); likely needs creation or update

*(DIST-04 has no Wave 0 gap — already satisfied by existing release profile)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A — phase is distribution only, no auth added |
| V3 Session Management | No | N/A |
| V4 Access Control | Partial | GitHub Actions: `contents: write` permission scoped minimally; no `write-all` |
| V5 Input Validation | No | Binary already validates inputs in prior phases |
| V6 Cryptography | No | N/A |

### Known Threat Patterns for CI/CD Distribution

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Compromised CARGO_REGISTRY_TOKEN | Elevation of privilege | Store only in GitHub Actions secret; never commit; use scoped token for publish-only |
| Supply chain: pinned action versions | Tampering | Pin actions to SHA digest (`@v1` is acceptable; SHA pinning is paranoid-mode best practice) |
| Artifact substitution between jobs | Tampering | `actions/upload-artifact`/`download-artifact` use signed URLs tied to the workflow run |
| Skill invoking arbitrary shell commands | Tampering | Scope `allowed-tools` to `Bash(confluence-agent *)` — not blanket `Bash` |

## Sources

### Primary (HIGH confidence)

- `code.claude.com/docs/en/slash-commands` — Claude Code skills format, frontmatter fields, `$ARGUMENTS` substitution, `allowed-tools` scoping, `.claude/skills/` vs `.claude/commands/` [VERIFIED: WebFetch 2026-04-20]
- `doc.rust-lang.org/cargo/reference/publishing.html` — Required/recommended crates.io metadata fields, publish process, size limits [VERIFIED: WebFetch 2026-04-20]
- `target/release/confluence-agent` — Binary size: 4.0 MB [VERIFIED: `ls -lh` 2026-04-20]
- `crates.io/api/v1/crates/confluence-agent` — Name availability: NOT FOUND (available) [VERIFIED: curl + JSON parse 2026-04-20]

### Secondary (MEDIUM confidence)

- `github.com/houseabsolute/actions-rust-cross` — Matrix build pattern with `houseabsolute/actions-rust-cross@v1` [CITED: GitHub README, verified via WebFetch]
- `ahmedjama.com/blog/2025/12/cross-platform-rust-pipeline-github-actions/` — macOS-latest for Darwin targets, ubuntu-24.04 for Linux, `use_cross: false` for x86_64 [CITED: 2025-12 blog]
- `softprops/action-gh-release` v2 — GitHub Releases artifact upload pattern [CITED: GitHub Marketplace]

### Tertiary (LOW confidence)

- None — all critical claims verified via official sources

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — all tools verified via official sources or codebase inspection
- Architecture: HIGH — binary size confirmed; crate name confirmed available; skill format confirmed via live docs
- Pitfalls: MEDIUM — pitfalls derived from Cargo Book and action documentation; musl linker behavior is ASSUMED based on known Linux cross-compilation behavior

**Research date:** 2026-04-20
**Valid until:** 2026-07-20 (stable ecosystem; crate name availability changes if someone publishes first)
