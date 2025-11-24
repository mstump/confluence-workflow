from abc import ABC, abstractmethod
from typing import Any

import openai
from confluence_agent.llm_prompts import MERGE_PROMPT


class LLMProvider(ABC):
    """Abstract base class for LLM providers."""

    @abstractmethod
    def merge_content(self, original_content: str, new_content_markdown: str) -> str:
        """
        Merges new markdown content into existing Confluence storage format content.

        Args:
            original_content: The original content in Confluence storage format.
            new_content_markdown: The new content in markdown format.

        Returns:
            The merged content in Confluence storage format.
        """
        pass

    @abstractmethod
    def reflect_and_correct(
        self, original_content: str, new_content_storage: str, merged_content: str
    ) -> str:
        """
        Reflects on and corrects the merged content.

        Args:
            original_content: The original content in Confluence storage format.
            new_content_storage: The new content in storage format.
            merged_content: The initially merged content.

        Returns:
            The refined and corrected content in Confluence storage format.
        """
        pass

    @abstractmethod
    def critique_content(
        self,
        original_content: str,
        new_content_storage: str,
        final_proposed_content: str,
    ) -> str:
        """
        Critiques the final proposed content.

        Args:
            original_content: The original content in Confluence storage format.
            new_content_storage: The new content in storage format.
            final_proposed_content: The content after the reflection step.

        Returns:
            The content if it's perfect, or "REJECT" if there are any issues.
        """
        pass


class OpenAIProvider(LLMProvider):
    """LLM provider for OpenAI models."""

    def __init__(self, api_key: str, model: str):
        self.client = openai.OpenAI(api_key=api_key)
        self.model = model

    def merge_content(self, original_content: str, new_content_storage: str) -> str:
        prompt = MERGE_PROMPT.format(
            original_content=original_content, new_content_storage=new_content_storage
        )
        response = self.client.chat.completions.create(
            model=self.model,
            messages=[
                {"role": "system", "content": "You are an expert content moderator."},
                {"role": "user", "content": prompt},
            ],
        )
        return response.choices[0].message.content or ""

    def reflect_and_correct(
        self, original_content: str, new_content_storage: str, merged_content: str
    ) -> str:
        prompt = REFLECTION_PROMPT.format(
            original_content=original_content,
            new_content_storage=new_content_storage,
            merged_content=merged_content,
        )
        response = self.client.chat.completions.create(
            model=self.model,
            messages=[
                {
                    "role": "system",
                    "content": "You are a quality assurance expert specializing in Confluence content.",
                },
                {"role": "user", "content": prompt},
            ],
        )
        return response.choices[0].message.content or ""

    def critique_content(
        self,
        original_content: str,
        new_content_storage: str,
        final_proposed_content: str,
    ) -> str:
        prompt = CRITIC_PROMPT.format(
            original_content=original_content,
            new_content_storage=new_content_storage,
            final_proposed_content=final_proposed_content,
        )
        response = self.client.chat.completions.create(
            model=self.model,
            messages=[
                {
                    "role": "system",
                    "content": "You are the final gatekeeper for Confluence page updates. Your standards are exceptionally high.",
                },
                {"role": "user", "content": prompt},
            ],
        )
        return response.choices[0].message.content or ""


class UnsupportedProviderError(Exception):
    """Raised when an unsupported LLM provider is requested."""

    pass


def get_llm_provider(provider_name: str, **kwargs: Any) -> LLMProvider:
    """
    Factory function to get an instance of an LLM provider.

    Args:
        provider_name: The name of the provider (e.g., 'openai').
        **kwargs: Keyword arguments to pass to the provider's constructor.

    Returns:
        An instance of the LLM provider.

    Raises:
        UnsupportedProviderError: If the provider is not supported.
    """
    if provider_name == "openai":
        return OpenAIProvider(api_key=kwargs["api_key"], model=kwargs["model"])
    raise UnsupportedProviderError(f"Provider '{provider_name}' is not supported.")
