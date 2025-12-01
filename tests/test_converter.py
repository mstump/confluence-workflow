import asyncio
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
import anyio
from confluence_agent.converter import (
    convert_markdown_to_storage,
    render_puml_to_svg,
    process_markdown_puml,
    convert_wiki_to_storage,
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
    async def run_test():
        markdown_content = """
# Title

```plantuml
@startuml
A -> B
@enduml
```
"""
        mock_confluence_client = MagicMock()
        mock_confluence_client.confluence = MagicMock()

        with (
            patch(
                "confluence_agent.converter.render_puml_to_svg"
            ) as mock_render_puml_to_svg,
            patch(
                "confluence_agent.converter.convert_wiki_to_storage"
            ) as mock_convert_wiki,
        ):
            mock_render_puml_to_svg.return_value = b"<svg>diagram</svg>"
            mock_convert_wiki.return_value = "<p>Storage Format</p>"

            storage_format, attachments, _ = await convert_markdown_to_storage(
                markdown_content, mock_confluence_client, mock_settings
            )

            assert storage_format == "<p>Storage Format</p>"
            assert len(attachments) == 1
            assert attachments[0][0] == "diagram_1.svg"
            assert attachments[0][1] == b"<svg>diagram</svg>"
            mock_convert_wiki.assert_called_once()

    anyio.run(run_test)


def test_convert_wiki_to_storage_async_path():
    """Tests the asynchronous path for wiki to storage conversion."""

    async def run_test():
        mock_confluence = MagicMock()
        mock_confluence.post.return_value = {"asyncId": "123"}
        mock_confluence.get.return_value = {
            "status": "COMPLETED",
            "value": "<p>Converted</p>",
        }

        with patch("asyncio.sleep", new_callable=AsyncMock):
            result = await convert_wiki_to_storage(mock_confluence, "h1. Test")

        assert result == "<p>Converted</p>"
        mock_confluence.post.assert_called_once_with(
            "rest/api/contentbody/convert/storage",
            data={"value": "h1. Test", "representation": "wiki"},
        )
        mock_confluence.get.assert_called_once_with(
            "rest/api/contentbody/convert/async/123"
        )

    anyio.run(run_test)


def test_convert_wiki_to_storage_sync_path():
    """Tests the synchronous path for wiki to storage conversion."""

    async def run_test():
        mock_confluence = MagicMock()
        mock_confluence.post.return_value = {
            "status": "COMPLETED",
            "value": "<p>Converted Immediately</p>",
        }

        result = await convert_wiki_to_storage(mock_confluence, "h1. Test")
        assert result == "<p>Converted Immediately</p>"
        mock_confluence.post.assert_called_once_with(
            "rest/api/contentbody/convert/storage",
            data={"value": "h1. Test", "representation": "wiki"},
        )
        mock_confluence.get.assert_not_called()

    anyio.run(run_test)
