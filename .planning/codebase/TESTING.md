# Testing Patterns

**Analysis Date:** 2026-04-10

## Test Framework

**Runner:**

- `pytest==9.0.1`
- Config: `pyproject.toml` under `[tool.pytest.ini_options]`
- PYTHONPATH configured to include `src/` directory

**Assertion Library:**

- Native pytest assertions (no external library needed)

**Async Support:**

- `pytest-anyio==0.0.0` for async test execution
- Decorator: `@pytest.mark.anyio` on async test functions

**Run Commands:**

```bash
# Run all tests
uv run pytest

# Run single test file
uv run pytest tests/test_agent.py

# Run single test function
uv run pytest tests/test_agent.py::test_function_name -v

# Watch mode (not configured in repo)
```

## Test File Organization

**Location:**

- Co-located in `tests/` directory at project root (separate from `src/`)
- Tests excluded from mypy strict mode (configured in `pyproject.toml`)

**Naming:**

- `test_<module>.py` pattern
- Test functions: `test_<feature_or_scenario>()`
- Example files:
  - `tests/test_agent.py` - Agent orchestration and LLM pipeline
  - `tests/test_cli.py` - CLI commands
  - `tests/test_config.py` - Settings loading
  - `tests/test_confluence.py` - Confluence API wrapper
  - `tests/test_converter.py` - Markdown conversion
  - `tests/test_llm.py` - LLM provider factory

## Test Structure

**Suite Organization:**

```python
# Imports first
import pytest
from unittest.mock import MagicMock, patch, AsyncMock

# Fixtures
@pytest.fixture
def mock_confluence_api() -> Generator[MagicMock, None, None]:
    """Fixture to create a mock Confluence API object."""
    with patch("confluence_agent.confluence.Confluence") as mock_confluence_class:
        instance = MagicMock()
        mock_confluence_class.return_value = instance
        yield instance

# Test functions
@patch("confluence_agent.confluence.Confluence")
def test_something(mock_confluence: MagicMock) -> None:
    """Test description."""
    # Arrange
    mock_confluence.return_value.method.return_value = "expected"

    # Act
    from confluence_agent.confluence import ConfluenceClient
    client = ConfluenceClient("url", "user", "token")
    result = client.method()

    # Assert
    assert result == "expected"
    mock_confluence.assert_called_once()
```

**Patterns:**

1. **Fixtures with context managers:**

```python
@pytest.fixture
def mock_settings():
    settings = MagicMock()
    settings.plantuml_java_path = "java"
    settings.plantuml_jar_path = "plantuml.jar"
    return settings
```

1. **Patch decorators stacked (note: reverse order in function params):**

```python
@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.convert_markdown_to_storage")
@pytest.mark.anyio
async def test_update_confluence_page_tool(
    mock_convert_markdown_to_storage: MagicMock,
    mock_confluence_client: MagicMock,
    mock_settings: MagicMock,
) -> None:
```

1. **AsyncMock for async functions:**

```python
@patch("confluence_agent.agent._process_content_with_llm", new_callable=AsyncMock)
@pytest.mark.anyio
async def test_something(mock_process_with_llm: AsyncMock) -> None:
    mock_process_with_llm.return_value = "result"
    result = await mock_process_with_llm()
```

## Mocking

**Framework:** `unittest.mock` (standard library)

**Patterns:**

**Mock initialization:**

```python
from unittest.mock import MagicMock, patch, AsyncMock

mock_settings = MagicMock()
mock_settings.confluence_url = "https://fake.url"
mock_settings.openai.api_key = "key"
```

**Side effects for sequential returns:**

```python
mock_llm.generate_structured.side_effect = [
    ConfluenceContent(content="<p>Merged</p>"),
    ConfluenceContent(content="<p>Corrected</p>"),
    CriticResponse(decision="APPROVE", content="<p>Final</p>"),
]
```

**AsyncMock for async functions:**

```python
from unittest.mock import AsyncMock
mock_llm = AsyncMock()
mock_llm.generate_structured = AsyncMock(return_value=ConfluenceContent(content="..."))
```

**What to Mock:**

- External service calls: Confluence API, LLM providers, file I/O
- Settings/configuration: Always mock `Settings` to avoid env var dependency
- Subprocess calls: PlantUML and Mermaid rendering

**What NOT to Mock:**

- Internal helper functions that are well-tested
- Pydantic model instantiation
- String/regex operations
- Pure logic (token counting heuristics)

## Fixtures and Factories

**Test Data:**

```python
@pytest.fixture
def mock_confluence_api() -> Generator[MagicMock, None, None]:
    with patch("confluence_agent.confluence.Confluence") as mock_class:
        instance = MagicMock()
        mock_class.return_value = instance
        yield instance
```

**Location:**

- Defined in each test file near the top
- Shared fixtures could be in `conftest.py` (currently not used)

**Examples from test_agent.py:**

```python
mock_settings.return_value.confluence_url = "https://fake.url"
mock_confluence_instance = mock_confluence_client.return_value
mock_confluence_instance.get_page_content_and_version.return_value = (
    '<p>Old <ac:inline-comment-marker ac:ref="abc-123">text</ac:inline-comment-marker></p>',
    1,
    "Title",
)
```

## Coverage

**Requirements:** Not enforced (no pytest-cov configuration detected)

**View Coverage:** Not configured

**Current Test Coverage Gaps:**

- Async execution with real LLM providers (mocked in tests)
- Real Confluence API calls (mocked)
- PlantUML/Mermaid subprocess rendering (partially tested with subprocess.run mock)
- End-to-end integration tests with actual markdown file

## Test Types

**Unit Tests (majority):**

- Test individual functions in isolation
- Mock all external dependencies
- Example: `test_get_llm_provider_openai()` in `test_llm.py`
- Example: `test_extract_inline_comment_markers_self_closing()` in `test_agent.py`

**Integration-style Tests:**

- Test orchestration across multiple modules
- Still use mocks for external services
- Example: `test_update_confluence_page_tool()` in `test_agent.py` - verifies merge, reflect, critic chain

**Configuration Tests:**

- Test environment variable loading
- Example: `test_settings_load_from_env()` in `test_config.py`

**No E2E Tests:**

- No end-to-end tests that call real Confluence or LLM APIs
- Would require live credentials and is avoided in CI

## Common Patterns

**Async Testing:**

```python
@pytest.mark.anyio
async def test_llm_merge_content(mock_generate_structured: AsyncMock) -> None:
    mock_llm = MagicMock()
    mock_generate_structured.return_value = ConfluenceContent(content="<p>Merged</p>")

    from confluence_agent.agent import _llm_merge_content
    result = await _llm_merge_content(mock_llm, "<p>Old</p>", "<h2>New</h2>")

    assert result == "<p>Merged</p>"
    mock_generate_structured.assert_called_once()
```

**Error Testing:**

```python
def test_get_page_id_from_url_invalid() -> None:
    with pytest.raises(ValueError, match="Could not extract page ID from URL"):
        ConfluenceClient._get_page_id_from_url("https://invalid.url")

@pytest.mark.anyio
async def test_llm_critic_content_reject(mock_generate_structured: AsyncMock) -> None:
    mock_generate_structured.return_value = CriticResponse(
        decision="REJECT", reasoning="It's bad"
    )

    with pytest.raises(
        Exception, match="Critic rejected the proposed content. Reason: It's bad"
    ):
        await _llm_critic_content(mock_llm, "<p>Old</p>", "<h2>New</h2>", "<p>Corrected</p>")
```

**Subprocess Mocking:**

```python
def test_render_puml_to_svg_success(mock_settings):
    with patch("subprocess.run") as mock_run:
        mock_process = MagicMock()
        mock_process.stdout = b"<svg>diagram</svg>"
        mock_run.return_value = mock_process

        result = render_puml_to_svg(puml_content, mock_settings)
        assert result == b"<svg>diagram</svg>"
        mock_run.assert_called_once()
```

**File I/O Testing with tmp_path:**

```python
def test_process_markdown_puml_retains_block(mock_settings, tmp_path):
    with patch("confluence_agent.converter.render_puml_to_svg") as mock_render:
        mock_render.return_value = b"<svg>diagram</svg>"
        processed_markdown, attachments = process_markdown_puml(
            markdown_content, mock_settings, tmp_path
        )

        assert "![diagram_1.svg](./diagram_1.svg)" in processed_markdown
        assert (tmp_path / "diagram_1.svg").exists()  # Verify file written
```

## Pre-commit Integration

**Test running:**

- Tests run as part of pre-commit hooks
- Configured in `.pre-commit-config.yaml` under `local` hooks
- Entry: `bash -c 'source .venv/bin/activate && pytest'`
- Prevents committing code that fails tests

---

Testing analysis: 2026-04-10
