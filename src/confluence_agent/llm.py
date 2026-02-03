from typing import Any, Type
from tiktoken import get_encoding
from mcp_agent.workflows.llm.augmented_llm import AugmentedLLM

from confluence_agent.patched_providers import (
    ChunkAwareGoogleAugmentedLLM,
    ChunkAwareOpenAIAugmentedLLM,
)


class UnsupportedProviderError(Exception):
    """Raised when an unsupported LLM provider is requested."""

    pass


def get_llm_provider(provider_name: str) -> Type[AugmentedLLM[Any, Any]]:
    """
    Factory function to get an instance of an LLM provider.
    Args:
        provider_name: The name of the provider (e.g., 'openai').
    Returns:
        An instance of the LLM provider.
    Raises:
        UnsupportedProviderError: If the provider is not supported.
    """
    if provider_name == "openai":
        return ChunkAwareOpenAIAugmentedLLM
    if provider_name == "google":
        return ChunkAwareGoogleAugmentedLLM
    raise UnsupportedProviderError(f"Provider '{provider_name}' is not supported.")


async def get_token_count(provider_name: str, text: str) -> int:
    """
    Calculates the token count for a given text and provider.
    Args:
        provider_name: The name of the provider (e.g., 'openai' or 'google').
        text: The text to calculate the token count for.
    Returns:
        The number of tokens in the text.
    Raises:
        UnsupportedProviderError: If the provider is not supported.
    """
    if provider_name == "openai":
        # Using cl100k_base encoding as it's standard for GPT-3.5 and GPT-4 models
        encoding = get_encoding("cl100k_base")
        return len(encoding.encode(text))
    elif provider_name == "google":
        # Gemini uses a different tokenization method. A rough approximation is to
        # divide the number of characters by 4. This is a heuristic and may not be
        # perfectly accurate, but it's a reasonable estimate for our purposes.
        return len(text) // 4
    else:
        raise UnsupportedProviderError(f"Provider '{provider_name}' is not supported.")
