from unittest.mock import patch, MagicMock

import pytest

from confluence_agent.llm import (
    LLMProvider,
    OpenAIProvider,
    get_llm_provider,
    UnsupportedProviderError,
)


class MockLLMProvider(LLMProvider):
    def merge_content(self, original_content: str, new_content: str) -> str:
        return "merged"


def test_get_llm_provider_openai():
    """Tests that the correct provider is returned for 'openai'."""
    provider = get_llm_provider("openai", api_key="test_key", model="gpt-4")
    assert isinstance(provider, OpenAIProvider)


def test_get_llm_provider_unsupported():
    """Tests that an error is raised for an unsupported provider."""
    with pytest.raises(UnsupportedProviderError):
        get_llm_provider("unsupported_provider")


@patch("openai.OpenAI")
def test_openai_provider_merge_content(mock_openai_class):
    """Tests the merge_content method of the OpenAIProvider."""
    mock_openai_instance = MagicMock()
    mock_chat_completion = MagicMock()
    mock_chat_completion.choices = [
        MagicMock(message=MagicMock(content="<p>Merged Content</p>"))
    ]
    mock_openai_instance.chat.completions.create.return_value = mock_chat_completion
    mock_openai_class.return_value = mock_openai_instance

    provider = OpenAIProvider(api_key="test_key", model="gpt-4")
    result = provider.merge_content("<p>Original</p>", "## New\n\nContent")

    assert result == "<p>Merged Content</p>"
    mock_openai_instance.chat.completions.create.assert_called_once()
    call_args = mock_openai_instance.chat.completions.create.call_args
    assert call_args.kwargs["model"] == "gpt-4"
    assert (
        "You are an expert content moderator."
        in call_args.kwargs["messages"][0]["content"]
    )
