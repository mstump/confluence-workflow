# Confluence Agent

A Rust CLI that converts Markdown files into Confluence pages. It uses a multi-step LLM pipeline
(Merge → Reflect → Critic) to intelligently merge new content with an existing page, preserving
inline comments and rendering diagrams (PlantUML, Mermaid).

## Motivation

I write Confluence pages in Markdown (typically in Cursor) and publish to Confluence. Two problems
kept coming up:

- Diagrams didn't render — I had to manually re-add diagram macros after every publish.
- Every publish wiped out the location context of inline comments.

This tool solves both:

- PlantUML and Mermaid blocks are rendered to SVG, uploaded as attachments, and inserted as
  inline images above their source blocks.
- An LLM fetches the existing page state and re-injects `<ac:inline-comment-marker>` elements so
  comments survive every publish.

## Prerequisites

- **Rust** (1.80+): [rustup.rs](https://rustup.rs)
- **PlantUML** (optional): `plantuml` CLI on `$PATH`, or a path to `plantuml.jar`
- **mermaid-cli** (optional): `mmdc` on `$PATH` — `npm install -g @mermaid-js/mermaid-cli`
- **Confluence Cloud**: an instance with API token credentials
- **Anthropic API key**: required only for the `update` command's LLM merge — run
  `claude setup-token` if you have Claude installed but no API key yet

## Installation

### From crates.io (recommended)

```bash
cargo install confluence-workflow
```

### From source

```bash
git clone https://github.com/mstump/confluence-workflow
cd confluence-workflow
cargo build --release
# Binary is at target/release/confluence-workflow
```

## Configuration

Auth credentials (`CONFLUENCE_URL`, `CONFLUENCE_USERNAME`, `CONFLUENCE_API_TOKEN`,
`ANTHROPIC_API_KEY`) are resolved in this order (first non-empty wins):

1. CLI flag
2. Environment variable
3. `~/.claude/settings.json` (top-level key matching the env var name — session auth only)

| CLI flag                 | Env var                | Required      | Description                                        |
| ------------------------ | ---------------------- | ------------- | -------------------------------------------------- |
| `--confluence-url`       | `CONFLUENCE_URL`       | Yes           | Base URL, e.g. `https://your-domain.atlassian.net` |
| `--confluence-username`  | `CONFLUENCE_USERNAME`  | Yes           | Email address                                      |
| `--confluence-api-token` | `CONFLUENCE_API_TOKEN` | Yes           | Atlassian API token                                |
| `--anthropic-api-key`    | `ANTHROPIC_API_KEY`    | `update` only | Anthropic API key                                  |
| `--plantuml-path`        | `PLANTUML_PATH`        | No            | Path to plantuml (default: `plantuml`)             |
| `--mermaid-path`         | `MERMAID_PATH`         | No            | Path to mmdc (default: `mmdc`)                     |

Additional env vars (no CLI flag):

| Env var                    | Default                     | Description                                          |
| -------------------------- | --------------------------- | ---------------------------------------------------- |
| `ANTHROPIC_MODEL`          | `claude-haiku-4-5-20251001` | Model used for the LLM pipeline                      |
| `ANTHROPIC_CONCURRENCY`    | `5`                         | Max concurrent LLM requests                          |
| `MERMAID_PUPPETEER_CONFIG` | —                           | Path to puppeteer config file for mmdc               |
| `DIAGRAM_TIMEOUT`          | `30`                        | Seconds before a diagram render subprocess is killed |

The simplest setup is a `.env` file in the working directory (loaded automatically):

```env
CONFLUENCE_URL=https://your-domain.atlassian.net
CONFLUENCE_USERNAME=you@example.com
CONFLUENCE_API_TOKEN=your-token
ANTHROPIC_API_KEY=sk-ant-...
```

## Usage

### `update` — merge with LLM (recommended)

Fetches the existing page, runs the Merge → Reflect → Critic pipeline to merge your Markdown into
it, preserves inline comments, and publishes the result.

```bash
confluence-workflow update doc.md 'https://your-domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Title'
```

### `upload` — direct overwrite

Converts and uploads without any LLM merge. Useful for initial page creation or when you don't
care about preserving existing content or comments.

```bash
confluence-workflow upload doc.md 'https://your-domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Title'
```

### `convert` — local conversion only

Converts Markdown to Confluence Storage Format XML and writes the output locally. No network
requests; useful for debugging the conversion step.

```bash
confluence-workflow convert doc.md ./output-dir
```

### Global flags

| Flag                      | Description                      |
| ------------------------- | -------------------------------- |
| `--verbose` / `-v`        | Enable debug logging             |
| `--output human\|json`    | Output format (default: `human`) |

## Claude Code Skills

This repo ships a `/confluence-publish` Claude Code slash command. Because the command is most
useful in your own projects (where your Markdown files live), install it globally after running
`cargo install`:

```bash
mkdir -p ~/.claude/commands
curl -o ~/.claude/commands/confluence-publish.md \
  https://raw.githubusercontent.com/mstump/confluence-workflow/main/.claude/commands/confluence-publish.md
```

Once installed, the command is available in every Claude Code session.

### `/confluence-publish`

Publishes the currently open Markdown file to Confluence using the `update` command (LLM merge
by default, `upload` if you request a direct overwrite).

**Usage:**

```text
/confluence-publish
/confluence-publish https://your-domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Title
```

- If a URL is passed as an argument, that page is targeted.
- Otherwise the skill looks for a `confluence_url:` key in the file's YAML frontmatter.
- If neither is found, the skill prompts for a URL before running.

The skill confirms the file path and target URL with you before executing.

## YAML Frontmatter

YAML frontmatter (`---` blocks at the top of a Markdown file) is stripped during conversion and
never appears in the published Confluence page.

One frontmatter key has functional meaning for the `/confluence-publish` Claude Code skill:

| Key               | Description                                                                                         |
| ----------------- | --------------------------------------------------------------------------------------------------- |
| `confluence_url:` | Full URL of the target Confluence page — lets `/confluence-publish` skip the "which page?" prompt   |

Example:

```yaml
---
title: My Design Doc
confluence_url: https://your-domain.atlassian.net/wiki/spaces/ENG/pages/12345/My+Design+Doc
---

## Introduction

...
```

With this key present, running `/confluence-publish` in Claude Code automatically targets the
correct page without prompting for a URL.

## Development

```bash
# Build
cargo build

# Run all tests
cargo test

# Run integration tests only
cargo test --test cli_integration

# Lint markdown
markdownlint --fix .
```
