from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Manages application configuration using Pydantic."""

    model_config = SettingsConfigDict(
        env_file=".env", env_file_encoding="utf-8", extra="ignore"
    )

    # Confluence Configuration
    confluence_url: str
    confluence_username: str
    confluence_api_token: str

    # LLM Provider Configuration
    llm_provider: str = "openai"

    # OpenAI Configuration
    openai_api_key: str
    openai_model: str = "gpt-5-mini"
