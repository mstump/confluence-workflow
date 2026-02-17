def test_safe_preview_truncates() -> None:
    from confluence_agent.structured_output import safe_preview

    out = safe_preview("x" * 100, limit=10)
    assert out.endswith("...")
    assert len(out) == 10


def test_safe_preview_limit_zero() -> None:
    from confluence_agent.structured_output import safe_preview

    assert safe_preview("anything", limit=0) == ""
