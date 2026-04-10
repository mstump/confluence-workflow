# Codebase Structure

**Analysis Date:** 2026-04-10

## Directory Layout

```text
confluence-workflow/
├── src/confluence_agent/         # Main package
│   ├── __init__.py               # Package marker
│   ├── agent.py                  # Orchestration & LLM pipeline
│   ├── cli.py                    # Typer CLI commands
│   ├── confluence.py             # Confluence API client
│   ├── config.py                 # Pydantic settings/environment
│   ├── converter.py              # Markdown→Storage conversion
│   ├── llm.py                    # LLM provider factory
│   ├── llm_prompts.py            # Prompt templates
│   ├── models.py                 # Pydantic response models
│   ├── patched_providers.py      # Custom LLM wrappers
│   └── structured_output.py      # JSON extraction helpers
├── tests/                         # Test suite
│   ├── test_agent.py             # Orchestration tests
│   ├── test_cli.py               # CLI command tests
│   ├── test_config.py            # Configuration tests
│   ├── test_confluence.py        # Confluence API tests
│   ├── test_converter.py         # Converter tests
│   ├── test_google_generate_structured_chunking.py
│   ├── test_llm.py               # LLM provider tests
│   ├── test_llm_prompts.py       # Prompt template tests
│   ├── test_patched_providers_import_paths.py
│   ├── test_safe_preview.py      # Output formatting tests
│   └── fixtures/                 # Test fixture files
├── pyproject.toml                # Project metadata & dependencies
├── pytest.ini                    # Pytest configuration
├── CLAUDE.md                     # Developer instructions
├── README.md                     # Project documentation
├── Dockerfile                    # Docker build config
├── .env.example                  # Environment template
├── .pre-commit-config.yaml       # Pre-commit hooks
└── .markdownlint.yaml            # Markdown linting rules
```

## Directory Purposes

**`src/confluence_agent/`:**

- Purpose: Core application logic
- Contains: Python modules implementing workflow stages
- Key files: `agent.py` (main orchestration), `cli.py` (user interface), `converter.py` (content pipeline)

**`tests/`:**

- Purpose: Automated test suite
- Contains: Unit and integration tests for all modules
- Key files: `test_agent.py` (workflow tests), `test_converter.py` (content transformation tests)
- Test data: `fixtures/` directory for mock data and sample files

**Root directory:**

- Configuration: `pyproject.toml`, `pytest.ini`, `.pre-commit-config.yaml`
- Documentation: `README.md`, `CLAUDE.md`
- Deployment: `Dockerfile`, `.env.example`

## Key File Locations

**Entry Points:**

- `src/confluence_agent/cli.py`: CLI commands (`update`, `upload`, `convert`)
- `src/confluence_agent/agent.py::update_confluence_page()`: MCP tool entrypoint
- `pyproject.toml [project.scripts]`: CLI binary registration as `confluence-agent`

**Configuration:**

- `src/confluence_agent/config.py`: Settings class (environment → Pydantic)
- `.env` (at runtime): Environment variables for Confluence, LLM providers, tool paths
- `.env.example`: Template showing required variables

**Core Logic:**

- `src/confluence_agent/agent.py`: Orchestration, LLM pipeline phases (merge/reflect/critic)
- `src/confluence_agent/converter.py`: Markdown→Confluence storage format, diagram rendering
- `src/confluence_agent/confluence.py`: Confluence API client (fetch, update, attachments)
- `src/confluence_agent/llm.py`: LLM provider factory and token counting

**LLM Prompts:**

- `src/confluence_agent/llm_prompts.py`: Three prompt templates (MERGE_PROMPT, REFLECTION_PROMPT, CRITIC_PROMPT)
- Each prompt includes inline comment marker preservation instructions

**Data Models:**

- `src/confluence_agent/models.py`: Pydantic models for LLM response validation
- `src/confluence_agent/structured_output.py`: JSON extraction from chunked LLM responses

## Naming Conventions

**Files:**

- Modules: `lowercase_with_underscores.py` (e.g., `confluence.py`, `llm_prompts.py`)
- Test files: `test_<module>.py` (e.g., `test_agent.py`, `test_converter.py`)

**Functions:**

- Public functions: `lowercase_with_underscores()` (e.g., `convert_markdown_to_storage()`)
- Private functions: `_leading_underscore()` (e.g., `_is_content_empty()`, `_llm_merge_content()`)
- Async functions: `async def function_name()` (all LLM operations are async)

**Variables:**

- camelCase for MCP/framework types: `RequestParams`, `MCPApp`, `AugmentedLLM`
- snake_case for local variables and function parameters
- UPPERCASE_WITH_UNDERSCORES for constants (e.g., `MERGE_PROMPT`, `REFLECTION_PROMPT`)

**Types:**

- Pydantic models: `PascalCase` (e.g., `ConfluenceContent`, `CriticResponse`)
- Exception classes: `PascalCase` ending in `Error` (e.g., `UnsupportedProviderError`)

## Where to Add New Code

**New Feature (adding a CLI command):**

- Primary code: `src/confluence_agent/cli.py::@app.command()`
- Support logic: New module in `src/confluence_agent/` as needed
- Tests: `tests/test_cli.py` with command-specific test functions

**New LLM Phase (adding a processing step):**

- Prompt: Add template to `src/confluence_agent/llm_prompts.py`
- Phase function: Add `async def _llm_<phase_name>()` to `src/confluence_agent/agent.py`
- Response model: Add class to `src/confluence_agent/models.py`
- Integration: Wire into `_process_content_with_llm()` call chain
- Tests: Add test case to `tests/test_agent.py`

**New Confluence Feature (different API operation):**

- Client method: Add to `ConfluenceClient` class in `src/confluence_agent/confluence.py`
- Tests: Add test case to `tests/test_confluence.py`

**New Markdown/Diagram Support (e.g., new diagram type):**

- Processing function: Add `process_markdown_<format>()` to `src/confluence_agent/converter.py`
- Rendering function: Add `render_<format>_to_svg()` handler
- Integration: Call from `convert_markdown_to_storage()`
- Tests: Add test case to `tests/test_converter.py` with fixture in `tests/fixtures/`

**Utilities/Shared Helpers:**

- Shared helpers: `src/confluence_agent/structured_output.py` (or new module if large)

## Special Directories

**`tests/fixtures/`:**

- Purpose: Test fixture files (sample markdown, HTML, expected outputs)
- Generated: No (committed to repo)
- Committed: Yes

**`tmp/`:**

- Purpose: Temporary runtime files and local testing artifacts
- Generated: Yes (created during testing)
- Committed: No (in .gitignore)

**`.planning/codebase/`:**

- Purpose: Architecture and structure documentation
- Generated: Yes (by GSD analysis tools)
- Committed: Yes

## Module Import Patterns

**Standard imports in modules:**

```python
# External/stdlib first
import asyncio
import logging
from typing import Any, Type, TypeVar
from pathlib import Path

# Third-party framework
from mcp_agent.app import MCPApp
from pydantic import BaseModel

# Internal relative imports
from confluence_agent.config import Settings
from confluence_agent.confluence import ConfluenceClient
```

**Circular dependency prevention:**

- `config.py` has no internal imports (only Pydantic/mcp-agent)
- `models.py` has no internal imports (only Pydantic)
- Orchestration (`agent.py`) imports all other modules; no other module imports `agent.py`

---

Structure analysis: 2026-04-10
