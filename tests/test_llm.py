import pytest

from confluence_agent.patched_providers import (
    ChunkAwareGoogleAugmentedLLM,
    ChunkAwareOpenAIAugmentedLLM,
)

from confluence_agent.llm import (
    get_llm_provider,
    UnsupportedProviderError,
)


def test_get_llm_provider_openai() -> None:
    """Tests that the correct provider is returned for 'openai'."""
    provider_class = get_llm_provider("openai")
    assert provider_class == ChunkAwareOpenAIAugmentedLLM


def test_get_llm_provider_google() -> None:
    """Tests that the correct provider is returned for 'google'."""
    provider_class = get_llm_provider("google")
    assert provider_class == ChunkAwareGoogleAugmentedLLM


def test_get_llm_provider_unsupported() -> None:
    """Tests that an error is raised for an unsupported provider."""
    with pytest.raises(UnsupportedProviderError):
        get_llm_provider("unsupported_provider")
