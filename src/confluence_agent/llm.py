from typing import Type

from mcp_agent.workflows.llm.augmented_llm import AugmentedLLM
from mcp_agent.workflows.llm.augmented_llm_openai import OpenAIAugmentedLLM
from mcp_agent.workflows.llm.augmented_llm_google import GoogleAugmentedLLM


class UnsupportedProviderError(Exception):
    """Raised when an unsupported LLM provider is requested."""

    pass


def get_llm_provider(provider_name: str) -> Type[AugmentedLLM]:
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
        return OpenAIAugmentedLLM
    if provider_name == "google":
        return GoogleAugmentedLLM
    raise UnsupportedProviderError(f"Provider '{provider_name}' is not supported.")
