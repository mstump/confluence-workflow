import inspect


def test_patched_providers_does_not_reference_nonexistent_providers_module() -> None:
    import confluence_agent.patched_providers as patched

    source = inspect.getsource(patched)
    assert "mcp_agent.workflows.llm.providers" not in source
