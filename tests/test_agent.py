import anyio
from unittest.mock import MagicMock, patch

import pytest
from mcp_agent.app import MCPApp

# This is a placeholder for the actual agent implementation.
# We are creating the file and a basic test structure.
# The actual agent logic will be tested via an integration-style test.


@pytest.fixture
def app_instance():
    """Fixture to create an instance of the MCPApp for testing."""
    return MCPApp(name="test_confluence_agent")


@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.markdown_to_confluence_storage")
@patch("confluence_agent.agent.get_llm_provider")
@patch("confluence_agent.agent.tempfile.NamedTemporaryFile")
@patch("confluence_agent.agent.os.remove")
@pytest.mark.anyio
async def test_update_confluence_page_tool(
    mock_os_remove,
    mock_tempfile,
    mock_get_llm_provider,
    mock_markdown_to_confluence_storage,
    mock_confluence_client,
    mock_settings,
    app_instance,
):
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

    mock_llm_provider = MagicMock()
    mock_llm_provider.merge_content.return_value = "<p>Merged</p>"
    mock_get_llm_provider.return_value = mock_llm_provider

    mock_temp_file = MagicMock()
    mock_temp_file.__enter__.return_value.name = "/tmp/somefile.md"
    mock_tempfile.return_value = mock_temp_file

    from confluence_agent import agent

    markdown_content = "# New Content"
    page_url = "https://fake.url/wiki/spaces/SPACE/pages/12345/Test+Page"

    result = await agent.update_confluence_page(markdown_content, page_url)

    mock_confluence_client._get_page_id_from_url.assert_called_once_with(page_url)
    mock_temp_file.__enter__().write.assert_called_once_with(markdown_content)
    mock_markdown_to_confluence_storage.assert_called_once_with("/tmp/somefile.md")
    mock_os_remove.assert_called_once_with("/tmp/somefile.md")

    mock_confluence_instance.get_page_content_and_version.assert_called_once_with(
        "12345"
    )
    mock_get_llm_provider.assert_called_once_with(
        "openai", api_key="key", model="model"
    )
    mock_llm_provider.merge_content.assert_called_once_with(
        "<p>Old</p>", "<h2>New</h2>"
    )
    mock_confluence_instance.update_page_content.assert_called_once_with(
        "12345", "Title", "<p>Merged</p>", 2
    )
    assert result == f"Page 'Title' (ID: 12345) updated successfully to version 2."


@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.markdown_to_confluence_storage")
@patch("confluence_agent.agent.get_llm_provider")
@patch("confluence_agent.agent.tempfile.NamedTemporaryFile")
@patch("confluence_agent.agent.os.remove")
@pytest.mark.anyio
async def test_update_confluence_page_tool_empty_page(
    mock_os_remove,
    mock_tempfile,
    mock_get_llm_provider,
    mock_markdown_to_confluence_storage,
    mock_confluence_client,
    mock_settings,
    app_instance,
):
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

    mock_temp_file = MagicMock()
    mock_temp_file.__enter__.return_value.name = "/tmp/somefile.md"
    mock_tempfile.return_value = mock_temp_file

    from confluence_agent import agent

    markdown_content = "# New Content"
    page_url = "https://fake.url/wiki/spaces/SPACE/pages/12345/Empty+Page"

    await agent.update_confluence_page(markdown_content, page_url)

    mock_get_llm_provider.assert_not_called()
    mock_llm_provider.merge_content.assert_not_called()
    mock_confluence_instance.update_page_content.assert_called_once_with(
        "12345", "Empty Page", "<h2>New Content</h2>", 2
    )


@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.markdown_to_confluence_storage")
@patch("confluence_agent.agent.get_llm_provider")
@patch("confluence_agent.agent.tempfile.NamedTemporaryFile")
@patch("confluence_agent.agent.os.remove")
@pytest.mark.anyio
async def test_update_confluence_page_tool_empty_p_tag(
    mock_os_remove,
    mock_tempfile,
    mock_get_llm_provider,
    mock_markdown_to_confluence_storage,
    mock_confluence_client,
    mock_settings,
    app_instance,
):
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

    mock_temp_file = MagicMock()
    mock_temp_file.__enter__.return_value.name = "/tmp/somefile.md"
    mock_tempfile.return_value = mock_temp_file

    from confluence_agent import agent

    markdown_content = "# New Content"
    page_url = "https://fake.url/wiki/spaces/SPACE/pages/12345/Emptyish+Page"

    await agent.update_confluence_page(markdown_content, page_url)

    mock_llm_provider.merge_content.assert_not_called()
    mock_confluence_instance.update_page_content.assert_called_once_with(
        "12345", "Emptyish Page", "<h2>New Content</h2>", 2
    )
