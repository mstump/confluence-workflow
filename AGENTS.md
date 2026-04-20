# About This Project

`confluence-agent` is a Rust CLI tool that converts Markdown files to Confluence storage format and publishes them to a Confluence space. It uses a multi-step LLM pipeline (Merge → Reflect → Critic) to intelligently merge new content with an existing page while preserving inline comments and rendering diagrams (PlantUML, Mermaid).

## Codebase Structure

```text
src/
  main.rs               — Binary entry point
  lib.rs                — Top-level orchestration (run function, command dispatch)
  cli.rs                — Clap-based CLI (update, upload, convert subcommands)
  config.rs             — Config struct; waterfall loader (CLI → env → ~/.claude/settings.json → defaults)
  error.rs              — Unified LlmError / AppError types
  confluence/
    client.rs           — Confluence REST API client (get/update page, upload attachments)
    types.rs            — Page, attachment, and API response types
    url.rs              — URL validation and parsing helpers
  converter/
    mod.rs              — MarkdownConverter: Markdown → Confluence storage XML
    diagrams.rs         — PlantUML / Mermaid → SVG rendering
    renderer.rs         — Diagram placeholder substitution
    tests.rs            — Unit tests for converter
  llm/
    mod.rs              — AnthropicClient; Merge/Reflect/Critic LLM pipeline
    types.rs            — Request/response types, structured output models
  merge/
    extractor.rs        — Extract inline-comment markers from existing page XML
    injector.rs         — Re-inject markers into merged output
    matcher.rs          — Fuzzy marker matching
    mod.rs              — Merge orchestration

tests/
  cli_integration.rs    — Happy-path integration tests (wiremock, assert_cmd)
  llm_integration.rs    — LLM client tests (wiremock)
  output_format.rs      — Stdout/stderr routing tests
```

## Common Tasks

### Build

```bash
cargo build
```

### Run tests

```bash
cargo test
```

### Run a single test

```bash
cargo test test_name
```

### CLI usage

```bash
# Merge markdown into an existing Confluence page (LLM pipeline)
confluence-agent update doc.md https://domain.atlassian.net/wiki/...

# Direct upload, no LLM
confluence-agent upload doc.md https://domain.atlassian.net/wiki/...

# Local conversion only (no network)
confluence-agent convert doc.md ./output
```

### Environment variables

| Variable | Purpose |
| --- | --- |
| `ANTHROPIC_API_KEY` | Required for LLM pipeline |
| `ANTHROPIC_BASE_URL` | Override LLM endpoint (used in tests) |
| `CONFLUENCE_URL` | Base URL (alternative to CLI flag) |
| `CONFLUENCE_USERNAME` | Atlassian account email |
| `CONFLUENCE_API_TOKEN` | Atlassian API token |
| `PLANTUML_PATH` | Path to `plantuml` binary (default: `plantuml`) |
| `MERMAID_PATH` | Path to `mmdc` binary (default: `mmdc`) |
| `DIAGRAM_TIMEOUT` | Subprocess timeout seconds (default: 30) |
| `ANTHROPIC_CONCURRENCY` | Max parallel LLM requests (default: 5, min: 1, max: 50) |
