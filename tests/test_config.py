import os
from unittest.mock import patch

from pydantic import ValidationError
import pytest

from confluence_agent.config import Settings


def test_settings_load_from_env():
    """Tests that settings are correctly loaded from environment variables."""
    env_vars = {
        "CONFLUENCE_URL": "https://test.atlassian.net",
        "CONFLUENCE_USERNAME": "testuser",
        "CONFLUENCE_API_TOKEN": "testtoken",
        "LLM_PROVIDER": "openai",
        "OPENAI_API_KEY": "testkey",
        "OPENAI_MODEL": "gpt-4",
    }

    with patch.dict(os.environ, env_vars):
        settings = Settings()
        assert settings.confluence_url == "https://test.atlassian.net"
        assert settings.confluence_username == "testuser"
        assert settings.confluence_api_token == "testtoken"
        assert settings.llm_provider == "openai"
        assert settings.openai_api_key == "testkey"
        assert settings.openai_model == "gpt-4"


def test_settings_missing_variables():
    """Tests that a validation error is raised if required environment variables are missing."""
    with patch.dict(os.environ, {}, clear=True):
        with pytest.raises(ValidationError):
            Settings()
