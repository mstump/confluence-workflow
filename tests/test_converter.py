from unittest.mock import MagicMock, patch

import pytest
import anyio
from confluence_agent.converter import (
    convert_markdown_to_storage,
    render_puml_to_svg,
    process_markdown_puml,
)


@pytest.fixture
def mock_settings():
    settings = MagicMock()
    settings.plantuml_java_path = "java"
    settings.plantuml_jar_path = "plantuml.jar"
    return settings


def test_render_puml_to_svg_success(mock_settings):
    puml_content = "@startuml\nAlice -> Bob\n@enduml"
    with patch("subprocess.run") as mock_run:
        mock_process = MagicMock()
        mock_process.stdout = b"<svg>diagram</svg>"
        mock_run.return_value = mock_process

        result = render_puml_to_svg(puml_content, mock_settings)
        assert result == b"<svg>diagram</svg>"
        mock_run.assert_called_once()


def test_process_markdown_puml_retains_block(mock_settings, tmp_path):
    """
    Tests that the puml block is retained and the image is inserted before it.
    """
    markdown_content = """
# Title

```plantuml
@startuml
A -> B
@enduml
```
"""
    with patch("confluence_agent.converter.render_puml_to_svg") as mock_render:
        mock_render.return_value = b"<svg>diagram</svg>"
        processed_markdown, attachments = process_markdown_puml(
            markdown_content, mock_settings, tmp_path
        )

        assert "![diagram_1.svg](./diagram_1.svg)" in processed_markdown
        assert "```plantuml\n@startuml\nA -> B\n@enduml\n```" in processed_markdown
        assert len(attachments) == 1
        # Check that the file was written
        assert (tmp_path / "diagram_1.svg").exists()


def test_process_markdown_puml_supports_puml_tag(mock_settings, tmp_path):
    """
    Tests that ```puml blocks are also processed (not just ```plantuml).
    """
    markdown_content = """
# Title

```puml
@startuml
A -> B
@enduml
```
"""
    with patch("confluence_agent.converter.render_puml_to_svg") as mock_render:
        mock_render.return_value = b"<svg>diagram</svg>"
        processed_markdown, attachments = process_markdown_puml(
            markdown_content, mock_settings, tmp_path
        )

        assert "![diagram_1.svg](./diagram_1.svg)" in processed_markdown
        assert "```puml\n@startuml\nA -> B\n@enduml\n```" in processed_markdown
        assert len(attachments) == 1
        assert (tmp_path / "diagram_1.svg").exists()


def test_process_markdown_puml_mixed_tags(mock_settings, tmp_path):
    """
    Tests that both ```plantuml and ```puml blocks are processed in the same document.
    """
    markdown_content = """
```plantuml
@startuml
A -> B
@enduml
```

```puml
@startuml
C -> D
@enduml
```
"""
    with patch("confluence_agent.converter.render_puml_to_svg") as mock_render:
        mock_render.return_value = b"<svg>diagram</svg>"
        processed_markdown, attachments = process_markdown_puml(
            markdown_content, mock_settings, tmp_path
        )

        assert "![diagram_1.svg](./diagram_1.svg)" in processed_markdown
        assert "![diagram_2.svg](./diagram_2.svg)" in processed_markdown
        assert "```plantuml\n@startuml\nA -> B\n@enduml\n```" in processed_markdown
        assert "```puml\n@startuml\nC -> D\n@enduml\n```" in processed_markdown
        assert len(attachments) == 2


def test_convert_markdown_to_storage(mock_settings, tmp_path):
    markdown_content = """
# Title
```plantuml
@startuml
A -> B
@enduml
```
"""
    with (
        patch(
            "confluence_agent.converter.render_puml_to_svg"
        ) as mock_render_puml_to_svg,
        patch(
            "confluence_agent.converter.ConfluenceDocument"
        ) as mock_confluence_document,
    ):
        mock_render_puml_to_svg.return_value = b"<svg>diagram</svg>"
        mock_doc_instance = MagicMock()
        mock_doc_instance.xhtml.return_value = (
            "<p>Storage Format</p><ac:caption>diagram_1.svg</ac:caption>"
        )
        mock_confluence_document.return_value = mock_doc_instance

        storage_format, attachments = convert_markdown_to_storage(
            markdown_content, mock_settings, tmp_path
        )

        assert storage_format == "<p>Storage Format</p>"
        assert len(attachments) == 1
        assert attachments[0][0] == "diagram_1.svg"
        assert attachments[0][1] == b"<svg>diagram</svg>"
        mock_doc_instance.xhtml.assert_called_once()
        assert (tmp_path / "diagram_1.svg").exists()
        assert (tmp_path / "_processed.md").exists()


def test_convert_markdown_to_storage_removes_h1(mock_settings, tmp_path):
    """
    Tests that the first h1 header is removed from the storage format.
    Confluence pages have their title outside the content body.
    """
    markdown_content = "# Document Title\n\nSome content here."
    with patch(
        "confluence_agent.converter.ConfluenceDocument"
    ) as mock_confluence_document:
        mock_doc_instance = MagicMock()
        mock_doc_instance.xhtml.return_value = (
            "<h1>Document Title</h1><p>Some content here.</p>"
        )
        mock_confluence_document.return_value = mock_doc_instance

        storage_format, attachments = convert_markdown_to_storage(
            markdown_content, mock_settings, tmp_path
        )

        assert "<h1>" not in storage_format
        assert "Document Title" not in storage_format
        assert "<p>Some content here.</p>" in storage_format
        assert len(attachments) == 0


def test_convert_markdown_to_storage_removes_only_first_h1(mock_settings, tmp_path):
    """
    Tests that only the first h1 header is removed, preserving subsequent h1 tags.
    """
    markdown_content = "# Title\n\n# Another H1\n\nContent."
    with patch(
        "confluence_agent.converter.ConfluenceDocument"
    ) as mock_confluence_document:
        mock_doc_instance = MagicMock()
        mock_doc_instance.xhtml.return_value = (
            "<h1>Title</h1><h1>Another H1</h1><p>Content.</p>"
        )
        mock_confluence_document.return_value = mock_doc_instance

        storage_format, attachments = convert_markdown_to_storage(
            markdown_content, mock_settings, tmp_path
        )

        assert storage_format == "<h1>Another H1</h1><p>Content.</p>"
