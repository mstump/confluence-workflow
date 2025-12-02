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


def test_process_markdown_puml_retains_block(mock_settings):
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
            markdown_content, mock_settings
        )

        assert "![diagram_1.svg](./diagram_1.svg)" in processed_markdown
        assert "```plantuml\n@startuml\nA -> B\n@enduml\n```" in processed_markdown
        assert len(attachments) == 1


def test_convert_markdown_to_storage(mock_settings):
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
        mock_doc_instance.xhtml.return_value = "<p>Storage Format</p>"
        mock_confluence_document.return_value = mock_doc_instance

        storage_format, attachments = convert_markdown_to_storage(
            markdown_content, mock_settings
        )

        assert storage_format == "<p>Storage Format</p>"
        assert len(attachments) == 1
        assert attachments[0][0] == "diagram_1.svg"
        assert attachments[0][1] == b"<svg>diagram</svg>"
        mock_doc_instance.xhtml.assert_called_once()
