"""Tests for the LLM prompts."""

from confluence_agent.llm_prompts import MERGE_PROMPT, REFLECTION_PROMPT, CRITIC_PROMPT


def test_critic_prompt_ignores_macro_id() -> None:
    """Tests that the critic prompt instructs the LLM to ignore ac:macro-id."""
    assert "ignore any inconsistencies in the `ac:macro-id` attribute" in CRITIC_PROMPT


def test_critic_prompt_ignores_preexisting_errors() -> None:
    """Tests that the critic prompt instructs the LLM to ignore pre-existing errors."""
    assert "Do not reject a page for pre-existing errors." in CRITIC_PROMPT


def test_merge_prompt_authoritative_source() -> None:
    """Tests that the merge prompt treats new content as authoritative."""
    instructions = [
        "Treat the `NEW_CONTENT` as the authoritative source",
        "If sections are removed or significantly reformatted in `NEW_CONTENT`",
        "reflect those changes",
    ]
    for instruction in instructions:
        assert instruction.lower() in MERGE_PROMPT.lower()


def test_merge_prompt_preserves_code_line_breaks() -> None:
    """Tests that the merge prompt instructs to preserve line breaks in code blocks."""
    assert "line breaks" in MERGE_PROMPT.lower()
    assert (
        "code blocks" in MERGE_PROMPT.lower() or "code sections" in MERGE_PROMPT.lower()
    )


def test_reflection_prompt_authoritative_source() -> None:
    """Tests that the reflection prompt treats new content as authoritative."""
    assert "authoritative source" in REFLECTION_PROMPT.lower()


def test_critic_prompt_authoritative_source() -> None:
    """Tests that the critic prompt treats new content as authoritative."""
    assert "authoritative source" in CRITIC_PROMPT.lower()


def test_prompts_ignore_version_at_save() -> None:
    """Tests that all prompts instruct the LLM to ignore ri:version-at-save."""
    assert "ri:version-at-save" in MERGE_PROMPT
    assert "ri:version-at-save" in REFLECTION_PROMPT
    assert "ri:version-at-save" in CRITIC_PROMPT


def test_prompts_strict_semantic_tags() -> None:
    """Tests that all prompts instruct the LLM to strictly follow semantic tags."""
    # MERGE_PROMPT
    assert "strictly use the exact semantic tags" in MERGE_PROMPT
    assert "Do not convert `<strong>` to `<b>`" in MERGE_PROMPT

    # REFLECTION_PROMPT
    assert "match exactly those in `NEW_CONTENT`" in REFLECTION_PROMPT
