from __future__ import annotations

from collections.abc import Iterable
from typing import Any


def _join_and_strip(chunks: Iterable[str]) -> str | None:
    combined = "".join(chunks)
    stripped = combined.strip()
    return stripped or None


def safe_preview(value: Any, limit: int = 4000) -> str:
    """
    Best-effort, bounded string preview for logging/tracing.

    We avoid assuming provider response types are JSON-serializable, and we never
    return more than `limit` characters.
    """
    try:
        text = repr(value)
    except Exception:
        text = f"<unrepr-able {type(value)!r}>"

    if limit <= 0:
        return ""
    if len(text) <= limit:
        return text
    return text[: limit - 3] + "..."


def _extract_from_google_like(response: Any) -> str | None:
    """
    Extract JSON text from a Gemini-like response shape:
      response.candidates[0].content.parts[*].text
    """
    candidates = getattr(response, "candidates", None)
    if not candidates:
        return None
    cand0 = candidates[0]
    content = getattr(cand0, "content", None)
    if not content:
        return None
    parts = getattr(content, "parts", None)
    if not parts:
        return None

    chunks: list[str] = []
    for part in parts:
        text = getattr(part, "text", None)
        if isinstance(text, str):
            chunks.append(text)
    return _join_and_strip(chunks)


def _extract_from_openai_chat_completion_like(response: Any) -> str | None:
    """
    Extract JSON text from an OpenAI ChatCompletion-like response shape:
      response.choices[0].message.content

    `message.content` is typically a string, but some clients may represent it as a list
    of content blocks (e.g., {"type": "text", "text": "..."}). We concatenate text-like
    blocks in order.
    """
    choices = getattr(response, "choices", None)
    if not choices:
        return None
    choice0 = choices[0]
    message = getattr(choice0, "message", None)
    if not message:
        return None
    content = getattr(message, "content", None)
    if content is None:
        return None

    if isinstance(content, str):
        return content.strip() or None

    if isinstance(content, list):
        chunks: list[str] = []
        for block in content:
            if isinstance(block, str):
                chunks.append(block)
                continue
            if isinstance(block, dict):
                text = block.get("text")
                if isinstance(text, str):
                    chunks.append(text)
                    continue
            text_attr = getattr(block, "text", None)
            if isinstance(text_attr, str):
                chunks.append(text_attr)
        return _join_and_strip(chunks)

    return None


def extract_structured_json_text(response: Any) -> str | None:
    """
    Best-effort extraction of JSON text from provider responses that may be chunked.

    Currently supports:
    - Gemini / Google responses (candidates[0].content.parts[*].text)
    - OpenAI ChatCompletion responses (choices[0].message.content)
    """
    text = _extract_from_google_like(response)
    if text is not None:
        return text

    return _extract_from_openai_chat_completion_like(response)
