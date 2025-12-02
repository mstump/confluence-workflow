"""Tests for the LLM prompts."""

from confluence_agent.llm_prompts import CRITIC_PROMPT


def test_critic_prompt_ignores_macro_id() -> None:
    """Tests that the critic prompt instructs the LLM to ignore ac:macro-id."""
    assert "ignore any inconsistencies in the `ac:macro-id` attribute" in CRITIC_PROMPT


def test_critic_prompt_ignores_preexisting_errors() -> None:
    """Tests that the critic prompt instructs the LLM to ignore pre-existing errors."""
    assert "Do not reject a page for pre-existing errors." in CRITIC_PROMPT
