# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Confluence Agent is an intelligent workflow that converts Markdown files to Confluence pages. It uses a multi-step LLM process (Merge → Reflect → Critic) to intelligently merge content while preserving inline comments and handling diagrams. The project exposes this workflow as both a CLI tool and an MCP server.

## Commands

```bash
# Run tests
uv run pytest

# Run a single test
uv run pytest tests/test_agent.py::test_function_name -v

# Format code (required after Python changes)
uv run black .

# Type checking (required after Python changes)
uv run mypy .

# Lint markdown (required after markdown changes)
markdownlint --fix .

# Run pre-commit hooks
pre-commit run --all-files

# CLI usage (development)
export LOG_LEVEL='INFO'
export PYTHONPATH=./src
uv run python -m confluence_agent.cli update 'doc.md' 'https://domain.atlassian.net/wiki/...'
uv run python -m confluence_agent.cli upload 'doc.md' 'https://...'  # Direct upload, no LLM
uv run python -m confluence_agent.cli convert 'doc.md' './output'    # Local conversion only

# MCP server
uvx mcp-agent serve confluence_agent

# Re-install CLI after modifying cli.py
uv pip uninstall confluence-agent && uv pip install -e '.[dev]'
```

## Architecture

### Core Processing Pipeline (agent.py)

The main intelligence is a 3-phase LLM chain:

1. **Merge** (`_llm_merge_content`): Intelligently merges new markdown-converted content with existing Confluence page
2. **Reflect** (`_llm_reflect_and_correct`): Validates and corrects the merged content
3. **Critic** (`_llm_critic_content`): Final QA phase that can approve or reject with revisions

Key architectural patterns:

- Uses `mcp-agent` framework's `MCPApp` for MCP server integration
- Structured output via Pydantic models (`ConfluenceContent`, `CriticResponse`)
- Inline comment markers are extracted via regex and passed to LLM as immutable tokens
- Token counting scales `max_tokens` dynamically based on content size

### Module Responsibilities

- **agent.py**: Orchestrates the LLM pipeline, MCP tool registration (`@app.tool()`)
- **cli.py**: Typer-based CLI with `update`, `upload`, `convert` commands
- **confluence.py**: Confluence API wrapper using `atlassian-python-api`
- **converter.py**: Markdown → Confluence storage format, PlantUML → SVG rendering
- **llm.py**: Provider factory (`get_llm_provider`) for OpenAI/Google, token counting
- **llm_prompts.py**: Prompt templates for merge/reflect/critic phases
- **patched_providers.py**: Custom LLM wrappers for chunked response handling
- **config.py**: Pydantic settings with environment variable loading

### Data Flow

```text
Markdown file
    ↓ converter.py (render PlantUML, convert to storage format)
    ↓
Confluence storage XML + attachments
    ↓ agent.py (fetch existing page, run LLM pipeline if non-empty)
    ↓
Merged content
    ↓ confluence.py (upload attachments, update page)
    ↓
Updated Confluence page
```

## Key Implementation Details

- **Inline comment preservation**: Confluence `<ac:inline-comment-marker>` elements are extracted and must be preserved byte-for-byte through the LLM pipeline
- **Empty page detection**: `_is_content_empty()` handles whitespace, empty strings, and `<p/>` tags
- **PlantUML**: Rendered to SVG via subprocess call to Java/PlantUML jar, uploaded as attachments
- **Provider support**: OpenAI (gpt-5) and Google (gemini-2.5-pro) verified working; gemini-2.5-flash-lite produces poor results

## Code Quality Requirements

- Always pin dependency versions in pyproject.toml (e.g., `package==1.2.3`)
- MyPy strict mode is enforced (tests excluded)
- Run black, mypy, and pytest after Python changes
- Run markdownlint after markdown changes
