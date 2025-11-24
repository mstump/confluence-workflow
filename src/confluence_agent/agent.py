from mcp_agent.app import MCPApp
import logging
import tempfile
import os
import re

from confluence_agent.config import Settings
from confluence_agent.confluence import ConfluenceClient
from confluence_agent.pandoc import markdown_to_confluence_storage
from confluence_agent.llm import get_llm_provider

# Configure structured logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

app = MCPApp(name="confluence_agent")


def _is_content_empty(content: str) -> bool:
    """
    Checks if Confluence storage content is effectively empty.

    An empty page can be an empty string, whitespace, or a self-closing <p/> tag.
    """
    if not content or content.isspace():
        return True

    stripped_content = content.strip()
    # Matches <p/>, <p />, <p attr="..."/>
    if re.fullmatch(r"<p(\s+[^>]*)?/>", stripped_content):
        return True

    return False


@app.tool()
async def update_confluence_page(markdown_content: str, page_url: str) -> str:
    """
    Updates a Confluence page with the content of a markdown string.

    Args:
        markdown_content: The markdown content as a string.
        page_url: The URL of the Confluence page to update.

    Returns:
        A string indicating the result of the operation.
    """
    try:
        logger.info("Starting Confluence page update process.")

        # 1. Load configuration
        settings = Settings()
        logger.info("Configuration loaded.")

        # 2. Initialize Confluence client
        confluence_client = ConfluenceClient(
            url=settings.confluence_url,
            username=settings.confluence_username,
            api_token=settings.confluence_api_token,
        )
        logger.info("Confluence client initialized.")

        # 3. Convert markdown to Confluence storage format
        logger.info("Converting markdown content to Confluence storage format.")
        temp_file_path = ""
        try:
            with tempfile.NamedTemporaryFile(
                mode="w", delete=False, suffix=".md", encoding="utf-8"
            ) as temp_file:
                temp_file.write(markdown_content)
                temp_file_path = temp_file.name

            new_content_storage = markdown_to_confluence_storage(temp_file_path)
            logger.info("Markdown conversion successful.")
        finally:
            if temp_file_path:
                os.remove(temp_file_path)

        # 4. Get current page content from Confluence
        page_id = ConfluenceClient._get_page_id_from_url(page_url)
        logger.info(f"Fetching content for page ID: {page_id}")
        original_content, version, title = (
            confluence_client.get_page_content_and_version(page_id)
        )
        logger.info(f"Successfully fetched page '{title}' (version {version}).")

        # 5. Use LLM to merge new content, or upload directly if page is empty
        merged_content: str
        if _is_content_empty(original_content):
            logger.info(
                "Original page is empty. Bypassing LLM and using new content directly."
            )
            merged_content = new_content_storage
        else:
            llm_provider = get_llm_provider(
                settings.llm_provider,
                api_key=settings.openai_api_key,
                model=settings.openai_model,
            )
            logger.info(f"LLM provider initialized: {settings.llm_provider}.")
            logger.info("Merging new content with original using LLM.")
            merged_content = llm_provider.merge_content(
                original_content, new_content_storage
            )
            logger.info("Content merge successful.")

        # 6. Update the Confluence page
        new_version = version + 1
        logger.info(f"Updating page ID {page_id} to version {new_version}.")
        confluence_client.update_page_content(
            page_id, title, merged_content, new_version
        )
        logger.info("Page update successful.")

        return f"Page '{title}' (ID: {page_id}) updated successfully to version {new_version}."

    except Exception as e:
        logger.error(f"An error occurred during the update process: {e}", exc_info=True)
        return f"Error: {e}"
