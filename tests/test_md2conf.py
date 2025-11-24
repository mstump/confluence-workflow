import re
import tempfile
from pathlib import Path

import pytest
from confluence_agent.md2conf import markdown_to_confluence_storage


def strip_whitespace(text: str) -> str:
    return re.sub(r"\s+", "", text)


def test_markdown_to_confluence_storage_success() -> None:
    """Tests that the markdown_to_confluence_storage function successfully converts markdown."""
    markdown_input = "# Title"
    expected_output = "<h1>Title</h1>"
    result = markdown_to_confluence_storage(markdown_input)
    assert strip_whitespace(result) == strip_whitespace(expected_output)


def test_markdown_to_confluence_storage_with_complex_markdown() -> None:
    """Tests conversion with more complex markdown."""
    markdown_input = "## Subtitle\n\n- Item 1\n- Item 2"
    result = markdown_to_confluence_storage(markdown_input)
    assert "<h2>Subtitle</h2>" in result
    assert "<li><p>Item 1</p></li>" in result
    assert "<li><p>Item 2</p></li>" in result


def test_markdown_to_confluence_storage_empty_input() -> None:
    """Tests that an empty string is handled correctly."""
    markdown_input = ""
    expected_output = ""
    result = markdown_to_confluence_storage(markdown_input)
    assert result == expected_output


def test_markdown_to_confluence_storage_with_real_markdown_features() -> None:
    """Tests conversion with various markdown features."""
    markdown_input = """
# Main Heading

This is a paragraph with **bold text** and *italic text*.

- List item 1
- List item 2

```python
def hello():
    print("Hello, World!")
```
"""
    result = markdown_to_confluence_storage(markdown_input)
    assert "<h1>Main Heading</h1>" in result
    assert "<strong>bold text</strong>" in result
    assert "<em>italic text</em>" in result
    assert "<li><p>List item 1</p></li>" in result
    assert '<ac:structured-macro ac:name="code"' in result
    assert 'ac:name="language">py</ac:parameter>' in result
    assert "<ac:plain-text-body><![CDATA[def hello():" in result
