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
@patch("confluence_agent.agent.convert_markdown_to_storage")
@patch("confluence_agent.agent.get_llm_provider")
@pytest.mark.anyio
async def test_update_confluence_page_tool(
    mock_get_llm_provider: MagicMock,
    mock_convert_markdown_to_storage: MagicMock,
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

    mock_convert_markdown_to_storage.return_value = ("<h2>New</h2>", [])

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

    with patch(
        "confluence_agent.agent._process_content_with_llm",
        new_callable=AsyncMock,
        return_value="<p>Final</p>",
    ) as mock_process_with_llm:
        result = await agent.update_confluence_page(
            markdown_content, page_url, provider="openai"
        )
        assert "success" in result.lower()

        mock_process_with_llm.assert_called_once_with(
            "<p>Old</p>", "<h2>New</h2>", "openai", mock_settings.return_value
        )

    mock_confluence_client._get_page_id_from_url.assert_called_once_with(page_url)
    mock_convert_markdown_to_storage.assert_called_once()

    mock_confluence_instance.get_page_content_and_version.assert_called_once_with(
        "12345"
    )

    mock_confluence_instance.update_page_content.assert_called_once_with(
        "12345", "Title", "<p>Final</p>", 2
    )
    assert result == f"Page 'Title' (ID: 12345) updated successfully to version 2."


@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.convert_markdown_to_storage")
@patch("confluence_agent.agent.get_llm_provider")
@pytest.mark.anyio
async def test_update_confluence_page_tool_empty_page(
    mock_get_llm_provider: MagicMock,
    mock_convert_markdown_to_storage: MagicMock,
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
    mock_convert_markdown_to_storage.return_value = ("<h2>New Content</h2>", [])

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
@patch("confluence_agent.agent.convert_markdown_to_storage")
@patch("confluence_agent.agent.get_llm_provider")
@pytest.mark.anyio
async def test_update_confluence_page_tool_empty_p_tag(
    mock_get_llm_provider: MagicMock,
    mock_convert_markdown_to_storage: MagicMock,
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
    mock_convert_markdown_to_storage.return_value = ("<h2>New Content</h2>", [])

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


@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.convert_markdown_to_storage")
@patch("confluence_agent.agent.get_llm_provider")
@pytest.mark.anyio
async def test_update_confluence_page_tool_critic_rejection(
    mock_get_llm_provider: MagicMock,
    mock_convert_markdown_to_storage: MagicMock,
    mock_confluence_client: MagicMock,
    mock_settings: MagicMock,
    app_instance: MCPApp,
) -> None:
    """
    Tests that the tool correctly handles a critic rejection.
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

    mock_convert_markdown_to_storage.return_value = ("<h2>New</h2>", [])

    mock_llm = AsyncMock()
    mock_llm.name = "model"
    mock_llm.provider = "openai"
    mock_llm.generate_structured.side_effect = [
        ConfluenceContent(content="<p>Merged</p>"),
        ConfluenceContent(content="<p>Corrected</p>"),
        CriticResponse(decision="REJECT", reasoning="The content is not good enough."),
    ]

    mock_llm_provider_class = MagicMock()
    mock_llm_provider_class.return_value = mock_llm
    mock_get_llm_provider.return_value = mock_llm_provider_class

    from confluence_agent import agent

    markdown_content = "# New Content"
    page_url = "https://fake.url/wiki/spaces/SPACE/pages/12345/Test+Page"

    with patch(
        "confluence_agent.agent._process_content_with_llm",
        new_callable=AsyncMock,
        side_effect=Exception(
            "Critic rejected the proposed content. Reason: The content is not good enough."
        ),
    ) as mock_process_with_llm:
        result = await agent.update_confluence_page(
            markdown_content, page_url, provider="openai"
        )

        assert "Error: Critic rejected the proposed content" in result
        assert "The content is not good enough." in result
        mock_confluence_instance.update_page_content.assert_not_called()
        mock_process_with_llm.assert_called_once_with(
            "<p>Old</p>", "<h2>New</h2>", "openai", mock_settings.return_value
        )


@patch("confluence_agent.agent.Settings")
@patch("confluence_agent.agent.ConfluenceClient", autospec=True)
@patch("confluence_agent.agent.convert_markdown_to_storage")
@patch("confluence_agent.agent.get_llm_provider")
@pytest.mark.anyio
async def test_update_confluence_page_tool_with_attachments(
    mock_get_llm_provider: MagicMock,
    mock_convert_markdown_to_storage: MagicMock,
    mock_confluence_client: MagicMock,
    mock_settings: MagicMock,
    app_instance: MCPApp,
) -> None:
    """
    Tests that attachments are uploaded after critic approval.
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

    mock_convert_markdown_to_storage.return_value = (
        "<h2>New</h2>",
        [("diagram.svg", b"svg content")],
    )

    mock_llm = AsyncMock()
    mock_llm.name = "model"
    mock_llm.provider = "openai"
    mock_llm.generate_structured.side_effect = [
        ConfluenceContent(content="<p>Merged</p>"),
        ConfluenceContent(content="<p>Corrected</p>"),
        CriticResponse(decision="APPROVE", content="<p>Final</p>"),
    ]

    def upload_attachments_side_effect(*args: Any, **kwargs: Any) -> None:
        assert mock_llm.generate_structured.call_count == 3

    mock_confluence_instance.upload_attachments.side_effect = (
        upload_attachments_side_effect
    )

    mock_llm_provider_class = MagicMock()
    mock_llm_provider_class.return_value = mock_llm
    mock_get_llm_provider.return_value = mock_llm_provider_class

    from confluence_agent import agent

    markdown_content = "# New Content with attachment"
    page_url = "https://fake.url/wiki/spaces/SPACE/pages/12345/Test+Page"

    with patch(
        "confluence_agent.agent._process_content_with_llm",
        new_callable=AsyncMock,
        return_value="<p>Final</p>",
    ) as mock_process_with_llm:
        await agent.update_confluence_page(
            markdown_content, page_url, provider="openai"
        )
        mock_process_with_llm.assert_called_once_with(
            "<p>Old</p>", "<h2>New</h2>", "openai", mock_settings.return_value
        )


@pytest.mark.anyio
async def test_process_content_with_llm_uses_provider_default_model() -> None:
    """
    _process_content_with_llm should force RequestParams.model from settings so mcp-agent's
    ModelSelector doesn't silently override it.
    """
    from types import SimpleNamespace
    from unittest.mock import AsyncMock, MagicMock, patch

    from confluence_agent.agent import _process_content_with_llm

    settings = SimpleNamespace(
        google=SimpleNamespace(default_model="gemini-3-flash-preview"),
        openai=SimpleNamespace(default_model="gpt-4o-mini"),
    )

    class DummyProvider:
        pass

    token_counter = SimpleNamespace(
        watch=AsyncMock(return_value="watch-id"),
        unwatch=AsyncMock(return_value=None),
    )
    agent_app = SimpleNamespace(context=SimpleNamespace(token_counter=token_counter))

    class _RunCtx:
        async def __aenter__(self) -> object:
            return agent_app

        async def __aexit__(self, exc_type, exc, tb) -> None:
            return None

    fake_llm = MagicMock()

    class FakeAgent:
        def __init__(self, *args, **kwargs) -> None:
            pass

        async def __aenter__(self) -> "FakeAgent":
            return self

        async def __aexit__(self, exc_type, exc, tb) -> None:
            return None

        async def attach_llm(self, provider_factory) -> object:
            # Ensure provider_factory was built with the expected RequestParams
            assert (
                provider_factory.keywords["default_request_params"].model
                == "gemini-3-flash-preview"
            )
            assert provider_factory.keywords["default_request_params"].maxTokens == 1424
            return fake_llm

    with (
        patch("confluence_agent.agent.get_llm_provider", return_value=DummyProvider),
        patch(
            "confluence_agent.agent.get_token_count", new=AsyncMock(return_value=100)
        ),
        patch("confluence_agent.agent.app.run", return_value=_RunCtx()),
        patch("confluence_agent.agent.Agent", FakeAgent),
        patch(
            "confluence_agent.agent._llm_merge_content",
            new=AsyncMock(return_value="<p>M</p>"),
        ),
        patch(
            "confluence_agent.agent._llm_reflect_and_correct",
            new=AsyncMock(return_value="<p>R</p>"),
        ),
        patch(
            "confluence_agent.agent._llm_critic_content",
            new=AsyncMock(return_value="<p>C</p>"),
        ),
    ):
        result = await _process_content_with_llm(
            "<p>Old</p>", "<h2>New</h2>", "google", settings
        )
        assert result == "<p>C</p>"


@patch("confluence_agent.agent._generate_structured_with_retry", new_callable=AsyncMock)
@pytest.mark.anyio
async def test_llm_merge_content(mock_generate_structured: AsyncMock) -> None:
    """Tests the _llm_merge_content function."""
    mock_llm = MagicMock()
    mock_generate_structured.return_value = ConfluenceContent(content="<p>Merged</p>")

    from confluence_agent.agent import _llm_merge_content

    result = await _llm_merge_content(mock_llm, "<p>Old</p>", "<h2>New</h2>")

    assert result == "<p>Merged</p>"
    mock_generate_structured.assert_called_once()


@patch("confluence_agent.agent._generate_structured_with_retry", new_callable=AsyncMock)
@pytest.mark.anyio
async def test_llm_reflect_and_correct(mock_generate_structured: AsyncMock) -> None:
    """Tests the _llm_reflect_and_correct function."""
    mock_llm = MagicMock()
    mock_generate_structured.return_value = ConfluenceContent(
        content="<p>Corrected</p>"
    )

    from confluence_agent.agent import _llm_reflect_and_correct

    result = await _llm_reflect_and_correct(
        mock_llm, "<p>Old</p>", "<h2>New</h2>", "<p>Merged</p>"
    )

    assert result == "<p>Corrected</p>"
    mock_generate_structured.assert_called_once()


@patch("confluence_agent.agent._generate_structured_with_retry", new_callable=AsyncMock)
@pytest.mark.anyio
async def test_llm_critic_content_approve(mock_generate_structured: AsyncMock) -> None:
    """Tests the _llm_critic_content function for an approval."""
    mock_llm = MagicMock()
    mock_generate_structured.return_value = CriticResponse(
        decision="APPROVE", content="<p>Final</p>"
    )

    from confluence_agent.agent import _llm_critic_content

    result = await _llm_critic_content(
        mock_llm, "<p>Old</p>", "<h2>New</h2>", "<p>Corrected</p>"
    )

    assert result == "<p>Final</p>"
    mock_generate_structured.assert_called_once()


@patch("confluence_agent.agent._generate_structured_with_retry", new_callable=AsyncMock)
@pytest.mark.anyio
async def test_llm_critic_content_reject(mock_generate_structured: AsyncMock) -> None:
    """Tests the _llm_critic_content function for a rejection."""
    mock_llm = MagicMock()
    mock_generate_structured.return_value = CriticResponse(
        decision="REJECT", reasoning="It's bad"
    )

    from confluence_agent.agent import _llm_critic_content

    with pytest.raises(
        Exception, match="Critic rejected the proposed content. Reason: It's bad"
    ):
        await _llm_critic_content(
            mock_llm, "<p>Old</p>", "<h2>New</h2>", "<p>Corrected</p>"
        )

    mock_generate_structured.assert_called_once()


@patch("confluence_agent.agent._generate_structured_with_retry", new_callable=AsyncMock)
@pytest.mark.anyio
async def test_llm_critic_content_approve_no_content(
    mock_generate_structured: AsyncMock,
) -> None:
    """Tests the _llm_critic_content function for an approval with no content."""
    mock_llm = MagicMock()
    mock_generate_structured.return_value = CriticResponse(
        decision="APPROVE", content=None
    )

    from confluence_agent.agent import _llm_critic_content

    with pytest.raises(
        Exception, match="Critic agent approved but did not provide content."
    ):
        await _llm_critic_content(
            mock_llm, "<p>Old</p>", "<h2>New</h2>", "<p>Corrected</p>"
        )

    mock_generate_structured.assert_called_once()


def test_extract_inline_comment_markers_empty() -> None:
    """Extracting markers from empty content should return an empty list."""
    from confluence_agent import agent

    assert agent._extract_inline_comment_markers("") == []


def test_extract_inline_comment_markers_self_closing() -> None:
    """Extracting self-closing markers should return verbatim tag strings."""
    from confluence_agent import agent

    original = (
        '<p>Hi <ac:inline-comment-marker ac:ref="abc-123"/> there</p>'
        '<p>And <ac:inline-comment-marker ac:ref="def-456" /> more</p>'
    )
    assert agent._extract_inline_comment_markers(original) == [
        '<ac:inline-comment-marker ac:ref="abc-123"/>',
        '<ac:inline-comment-marker ac:ref="def-456" />',
    ]


def test_extract_inline_comment_markers_paired() -> None:
    """Extracting paired markers should return the full element verbatim."""
    from confluence_agent import agent

    original = (
        "<p>Before "
        '<ac:inline-comment-marker ac:ref="uuid-1">commented text</ac:inline-comment-marker>'
        " after</p>"
    )
    assert agent._extract_inline_comment_markers(original) == [
        '<ac:inline-comment-marker ac:ref="uuid-1">commented text</ac:inline-comment-marker>',
    ]
