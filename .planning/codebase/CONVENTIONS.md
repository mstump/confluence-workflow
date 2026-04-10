# Coding Conventions

**Analysis Date:** 2026-04-10

## Naming Patterns

**Files:**

- Lowercase with underscores: `agent.py`, `converter.py`, `config.py`
- Test files: `test_<module>.py` (co-located in `tests/` directory)
- No file name abbreviations

**Functions:**

- Lowercase with underscores: `get_llm_provider()`, `_extract_inline_comment_markers()`, `get_page_content_and_version()`
- Private functions prefixed with single underscore: `_is_content_empty()`, `_llm_merge_content()`, `_process_content_with_llm()`
- Async functions follow same pattern: `async def _llm_merge_content()`

**Variables:**

- Lowercase with underscores: `original_content`, `new_content_storage`, `page_id`, `settings`
- Type variables use `T`: `T = TypeVar("T", bound=BaseModel)`
- Constants/module-level: `MERGE_PROMPT`, `REFLECTION_PROMPT`, `CRITIC_PROMPT` (all caps)

**Types and Classes:**

- PascalCase: `Settings`, `ConfluenceClient`, `ConfluenceContent`, `CriticResponse`, `TokenMonitor`
- Exception classes: `UnsupportedProviderError`
- Data models inherit from Pydantic `BaseModel`: `class ConfluenceContent(BaseModel)`

## Code Style

**Formatting:**

- Tool: `black` (enforced via pre-commit hook)
- Line length: 88 characters (configured in `pyproject.toml`)
- Target version: Python 3.9+

**Linting:**

- Tool: `mypy` (strict mode enforced for src, excluded tests)
- All type hints required in source code
- Tests have `# type: ignore` comments where needed for mocking
- Mypy config excludes tests and has overrides for external libraries (`mcp_agent`, `atlassian`, `openai`, `google`)

**Markdown:**

- Tool: `markdownlint`
- All markdown files auto-fixed by pre-commit hook
- Line length rule (MD013) disabled

## Import Organization

**Order (observed in actual files):**

1. External library imports: `from mcp_agent.app import MCPApp`
2. Standard library imports: `import logging`, `import os`, `import re`
3. Type hints and annotations: `from typing import Any, Type, TypeVar`
4. More standard library: `from pathlib import Path`, `from pydantic import BaseModel`
5. Local imports: `from confluence_agent.config import Settings`

**Path Aliases:**

- No alias imports used; full relative imports: `from confluence_agent.config import Settings`
- Relative imports within package: `from .config import Settings` (in `converter.py`)

## Error Handling

**Patterns:**

**Exception re-raising with context:**

```python
try:
    process = subprocess.run(...)
except FileNotFoundError:
    logger.error(f"Java executable not found at: {settings.plantuml_java_path}")
    raise
except subprocess.CalledProcessError as e:
    logger.error(f"PlantUML rendering failed: {e.stderr.decode('utf-8')}")
    raise
except Exception as e:
    logger.error(f"An unexpected error occurred during PlantUML rendering: {e}")
    raise
```

**Validation errors with structured output:**

- Custom retry logic in `_generate_structured_with_retry()` catches `ValidationError` from Pydantic and retries up to 3 times with 1s delay
- Logs warning on each attempt, raises original exception after max retries

**API errors with re-wrapping:**

```python
try:
    page = self.confluence.get_page_by_id(page_id, expand="body.storage,version")
except ApiError as e:
    raise ApiError(f"Failed to get page content for page ID {page_id}: {e}")
```

**CLI error handling:**

- Catches `FileNotFoundError`, logs with `console.print()` (Rich), exits with code 1
- Generic exceptions caught and logged with `console.print()`

## Logging

**Framework:** Python `logging` module

**Setup (agent.py):**

```python
log_level_str = os.getenv("LOG_LEVEL", "INFO").upper()
log_level = getattr(logging, log_level_str, logging.INFO)
logging.basicConfig(
    level=log_level, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)
```

**Patterns:**

- `logger.info()`: Major steps (page fetches, content merge, updates)
- `logger.debug()`: Detailed data (content values, converted formats)
- `logger.warning()`: Retryable failures, validation attempts
- `logger.error()`: Unrecoverable errors with `exc_info=True` for stack traces
- Named loggers per module: `logger = logging.getLogger(__name__)`

**Examples from code:**

```python
logger.info("Starting Confluence page update process.")
logger.debug(f"Initial markdown content: {markdown_content}")
logger.warning(f"Validation failed on attempt {attempt + 1}/{max_retries}. Retrying in {delay}s. Error: {e}")
logger.error(f"An error occurred during the update process: {e}", exc_info=True)
```

## Comments

**When to Comment:**

- Complex regex patterns (documented inline)
- Non-obvious logic (inline comment preservation token handling)
- Important architectural decisions (LLM provider selection)
- Workarounds and hacks (token scaling calculation)

**Examples:**

```python
# Prefer a single combined regex so results are returned in source order.
# Matches both self-closing and paired tags
pattern = re.compile(r"...", flags=re.DOTALL)
```

**Docstrings:**

- Used for all public functions and classes
- Format: Triple-quoted strings immediately after function/class definition
- Include Args, Returns, and Raises sections for public APIs
- No docstrings for private functions (leading underscore)

**Example:**

```python
def get_page_content_and_version(self, page_id: str) -> Tuple[str, int, str]:
    """
    Retrieves the content, version, and title of a Confluence page.

    Args:
        page_id: The ID of the page to retrieve.

    Returns:
        A tuple containing the page content in storage format, the current version number, and the title.

    Raises:
        ConfluenceError: If the API call fails.
    """
```

## Function Design

**Size:** Functions tend toward single responsibility (50-100 lines typical for main logic)

**Parameters:**

- Explicit type hints on all parameters
- No default values for critical paths (e.g., `url`, `username`, `api_token` required)
- Optional parameters use `Optional[Type]`: `provider: Optional[str] = None`
- TypeVar for generics: `response_model: Type[T]` where `T = TypeVar("T", bound=BaseModel)`

**Return Values:**

- Explicit return type hints on all functions
- Tuple unpacking common: `original_content, version, title = confluence_client.get_page_content_and_version(page_id)`
- Async functions return type: `async def _llm_merge_content(...) -> str:`

## Module Design

**Exports:**

- Modules export main classes and functions used by other modules
- Private helpers prefixed with underscore (not exported from `__init__.py`)
- Example: `agent.py` exports `update_confluence_page()` tool, but keeps `_llm_merge_content()` private

**Barrel Files:**

- No barrel files (`__init__.py`) used; direct imports from modules

**Module Responsibilities:**

- `agent.py`: LLM orchestration pipeline and MCP tool registration
- `cli.py`: Typer CLI commands (update, upload, convert)
- `confluence.py`: Confluence API wrapper
- `converter.py`: Markdown to storage format conversion, diagram rendering
- `config.py`: Pydantic settings and environment loading
- `llm.py`: LLM provider factory and token counting
- `models.py`: Pydantic data models for structured output
- `llm_prompts.py`: Prompt template strings

---

Convention analysis: 2026-04-10
