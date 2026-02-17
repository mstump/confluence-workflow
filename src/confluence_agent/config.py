from mcp_agent.config import Settings as McpSettings, OpenAISettings, GoogleSettings
from pydantic_settings import SettingsConfigDict


class Settings(McpSettings):
    """Manages application configuration using Pydantic."""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
        env_nested_delimiter="__",
    )

    # Confluence Configuration
    confluence_url: str
    confluence_username: str
    confluence_api_token: str
    plantuml_jar_path: str = "plantuml.jar"
    plantuml_java_path: str = "java"
    mermaid_cli_path: str = "mmdc"

    # LLM Provider Configuration
    llm_provider: str = "openai"

    # OpenAI Configuration
    openai: OpenAISettings = OpenAISettings(
        api_key="sk-my-openai-api-key", default_model="gpt-5-nano"
    )

    # Google Configuration
    google: GoogleSettings = GoogleSettings(
        api_key="sk-my-google-api-key", default_model="gemini-2.5-flash-lite"
    )
