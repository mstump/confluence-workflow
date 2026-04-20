# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Confluence Agent is a Rust CLI tool that converts Markdown files to Confluence pages. It uses a
multi-step LLM pipeline (Merge → Reflect → Critic) to intelligently merge new content with an
existing Confluence page while preserving inline comments and rendering diagrams (PlantUML, Mermaid).

## Commands

```bash
# Build
cargo build

# Run all tests
cargo test

# Run a single test
cargo test test_function_name

# Run integration tests only
cargo test --test cli_integration

# Lint markdown (required after markdown changes)
markdownlint --fix .

# CLI usage (development)
cargo run -- update 'doc.md' 'https://domain.atlassian.net/wiki/...'
cargo run -- upload 'doc.md' 'https://...'   # Direct upload, no LLM
cargo run -- convert 'doc.md' './output'      # Local conversion only
```

## Architecture

### Core Processing Pipeline

The `update` command runs a 3-phase LLM chain (`src/llm/mod.rs`):

1. **Merge**: Intelligently merges new markdown-converted content with the existing Confluence page
2. **Reflect**: Validates and corrects the merged output
3. **Critic**: Final QA pass — approves or rejects with revisions

### Module Responsibilities

- **src/lib.rs**: Top-level `run()` function; dispatches update / upload / convert commands
- **src/cli.rs**: Clap CLI definitions (`update`, `upload`, `convert` subcommands)
- **src/config.rs**: `Config` struct; waterfall loader (CLI flags → env vars → `~/.claude/settings.json` → defaults)
- **src/error.rs**: Unified `LlmError` / `AppError` types
- **src/confluence/**: REST API client — fetch page, update page, upload attachments
- **src/converter/**: Markdown → Confluence storage XML; PlantUML/Mermaid → SVG via subprocess
- **src/llm/**: `AnthropicClient`; Merge/Reflect/Critic pipeline; structured output types
- **src/merge/**: Extract, match, and re-inject `<ac:inline-comment-marker>` elements

### Data Flow

```text
Markdown file
    ↓ src/converter/ (render diagrams, convert to Confluence storage XML)
    ↓
Storage XML + SVG attachments
    ↓ src/lib.rs (fetch existing page, run LLM pipeline if page non-empty)
    ↓
Merged storage XML
    ↓ src/confluence/ (upload attachments, update page)
    ↓
Updated Confluence page
```

## Key Implementation Details

- **Inline comment preservation**: `<ac:inline-comment-marker>` elements are extracted before the
  LLM pipeline and re-injected afterward — they must survive byte-for-byte
- **Config waterfall**: `DiagramConfig` and credentials follow a strict precedence chain; never
  bypass it by constructing structs with `Default::default()` — use `Config::load()` or the
  explicit constructors
- **Diagram rendering**: PlantUML and Mermaid are rendered via subprocess; output SVGs are
  uploaded as Confluence attachments and referenced by placeholder tokens in the storage XML
- **`ANTHROPIC_BASE_URL`**: Overrides the LLM endpoint — used in integration tests to redirect
  traffic to a wiremock server; do not hardcode the production URL

## Code Quality Requirements

- After modifying Rust files: `cargo test` must pass, `cargo build` must be warning-free
- After modifying Markdown files: `markdownlint --fix .`
- Pin all dependency versions in `Cargo.toml` (e.g., `package = "=1.2.3"`)
- Do not add `impl Default` to `DiagramConfig` or `MarkdownConverter` — explicit construction
  is required to keep the config waterfall honest
