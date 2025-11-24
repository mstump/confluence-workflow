import anyio
from unittest.mock import MagicMock, patch, AsyncMock
from typing import Any

import pytest
from mcp_agent.app import MCPApp

from confluence_agent.models import ConfluenceContent, CriticResponse

# This is a placeholder for the actual agent implementation.
# We are creating the file and a basic test structure.
# The actual agent logic will be tested via an integration-style test.


@pytest.fixture
def app_instance() -> MCPApp:
    """Fixture to create an instance of the MCPApp for testing."""
    return MCPApp(name="test_confluence_agent")


@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.markdown_to_confluence_storage")
@patch("confluence_agent.agent.get_llm_provider")
@pytest.mark.anyio
async def test_update_confluence_page_tool(
    mock_get_llm_provider: MagicMock,
    mock_markdown_to_confluence_storage: MagicMock,
    mock_confluence_client: MagicMock,
    mock_settings: MagicMock,
    app_instance: MCPApp,
) -> None:
    """
    Integration-style test for the update_confluence_page tool.
    This test mocks all external dependencies to verify the orchestration logic.
    """
    # Setup mocks
    mock_settings.return_value.confluence_url = "https://fake.url"
    mock_settings.return_value.confluence_username = "user"
    mock_settings.return_value.confluence_api_token = "token"
    mock_settings.return_value.llm_provider = "openai"
    mock_settings.return_value.openai_api_key = "key"
    mock_settings.return_value.openai_model = "model"

    mock_confluence_instance = mock_confluence_client.return_value
    mock_confluence_instance.get_page_content_and_version.return_value = (
        "<p>Old</p>",
        1,
        "Title",
    )

    mock_confluence_client._get_page_id_from_url.return_value = "12345"

    mock_markdown_to_confluence_storage.return_value = "<h2>New</h2>"

    mock_llm = AsyncMock()
    mock_llm.name = "model"
    mock_llm.provider = "openai"
    mock_llm.generate_structured.side_effect = [
        ConfluenceContent(content="<p>Merged</p>"),
        ConfluenceContent(content="<p>Corrected</p>"),
        CriticResponse(decision="APPROVE", content="<p>Final</p>"),
    ]

    mock_llm_provider_class = MagicMock()
    mock_llm_provider_class.return_value = mock_llm
    mock_get_llm_provider.return_value = mock_llm_provider_class

    from confluence_agent import agent

    markdown_content = "# New Content"
    page_url = "https://fake.url/wiki/spaces/SPACE/pages/12345/Test+Page"

    result = await agent.update_confluence_page(
        markdown_content, page_url, provider="openai"
    )
    assert "success" in result.lower()

    mock_confluence_client._get_page_id_from_url.assert_called_once_with(page_url)
    mock_markdown_to_confluence_storage.assert_called_once_with(markdown_content)

    mock_confluence_instance.get_page_content_and_version.assert_called_once_with(
        "12345"
    )
    mock_get_llm_provider.assert_called_once_with("openai")

    mock_confluence_instance.update_page_content.assert_called_once_with(
        "12345", "Title", "<p>Final</p>", 2
    )
    assert result == f"Page 'Title' (ID: 12345) updated successfully to version 2."


@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.markdown_to_confluence_storage")
@patch("confluence_agent.agent.get_llm_provider")
@pytest.mark.anyio
async def test_update_confluence_page_tool_empty_page(
    mock_get_llm_provider: MagicMock,
    mock_markdown_to_confluence_storage: MagicMock,
    mock_confluence_client: MagicMock,
    mock_settings: MagicMock,
    app_instance: MCPApp,
) -> None:
    """
    Tests that the LLM is bypassed when the original Confluence page is empty.
    """
    # Setup mocks
    mock_settings.return_value.confluence_url = "https://fake.url"
    mock_settings.return_value.confluence_username = "user"
    mock_settings.return_value.confluence_api_token = "token"
    mock_settings.return_value.llm_provider = "openai"

    mock_confluence_instance = mock_confluence_client.return_value
    mock_confluence_instance.get_page_content_and_version.return_value = (
        "",
        1,
        "Empty Page",
    )

    mock_confluence_client._get_page_id_from_url.return_value = "12345"
    mock_markdown_to_confluence_storage.return_value = "<h2>New Content</h2>"

    mock_llm_provider = MagicMock()
    mock_get_llm_provider.return_value = mock_llm_provider

    from confluence_agent import agent

    markdown_content = "# New Content"
    page_url = "https://fake.url/wiki/spaces/SPACE/pages/12345/Empty+Page"

    await agent.update_confluence_page(markdown_content, page_url, provider="openai")

    mock_get_llm_provider.assert_not_called()
    mock_llm_provider.merge_content.assert_not_called()
    mock_confluence_instance.update_page_content.assert_called_once_with(
        "12345", "Empty Page", "<h2>New Content</h2>", 2
    )


@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.markdown_to_confluence_storage")
@patch("confluence_agent.agent.get_llm_provider")
@pytest.mark.anyio
async def test_update_confluence_page_tool_empty_p_tag(
    mock_get_llm_provider: MagicMock,
    mock_markdown_to_confluence_storage: MagicMock,
    mock_confluence_client: MagicMock,
    mock_settings: MagicMock,
    app_instance: MCPApp,
) -> None:
    """
    Tests that the LLM is bypassed when the page contains only a self-closing p tag.
    """
    # Setup mocks
    mock_settings.return_value.confluence_url = "https://fake.url"
    mock_settings.return_value.confluence_username = "user"
    mock_settings.return_value.confluence_api_token = "token"
    mock_settings.return_value.llm_provider = "openai"

    mock_confluence_instance = mock_confluence_client.return_value
    mock_confluence_instance.get_page_content_and_version.return_value = (
        '<p local-id="50851407-c63c-4ebb-aae7-c965f5a959ad" />',
        1,
        "Emptyish Page",
    )

    mock_confluence_client._get_page_id_from_url.return_value = "12345"
    mock_markdown_to_confluence_storage.return_value = "<h2>New Content</h2>"

    mock_llm_provider = MagicMock()
    mock_get_llm_provider.return_value = mock_llm_provider

    from confluence_agent import agent

    markdown_content = "# New Content"
    page_url = "https://fake.url/wiki/spaces/SPACE/pages/12345/Emptyish+Page"

    await agent.update_confluence_page(markdown_content, page_url, provider="openai")

    mock_llm_provider.merge_content.assert_not_called()
    mock_confluence_instance.update_page_content.assert_called_once_with(
        "12345", "Emptyish Page", "<h2>New Content</h2>", 2
    )
