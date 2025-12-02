from mcp_agent.app import MCPApp
import logging
import tempfile
import os
import re
import asyncio
import functools
from typing import Type, TypeVar
from pydantic import BaseModel
from pydantic_core import ValidationError

from mcp_agent.agents.agent import Agent
from mcp_agent.workflows.llm.augmented_llm import AugmentedLLM, RequestParams
from confluence_agent.config import Settings
from confluence_agent.confluence import ConfluenceClient
from confluence_agent.md2conf import markdown_to_confluence_storage
from confluence_agent.llm import get_llm_provider, get_token_count
from confluence_agent.llm_prompts import MERGE_PROMPT, REFLECTION_PROMPT, CRITIC_PROMPT
from confluence_agent.models import ConfluenceContent, CriticResponse

# Determine log level from environment variable
log_level_str = os.getenv("LOG_LEVEL", "INFO").upper()
log_level = getattr(logging, log_level_str, logging.INFO)

# Configure structured logging
logging.basicConfig(
    level=log_level, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

app = MCPApp(name="confluence_agent")

T = TypeVar("T", bound=BaseModel)


class TokenMonitor:
    async def on_token_update(self, node, usage):
        logger.info(
            f"[{node.name}] total_tokens={usage.total_tokens} prompt_tokens={usage.prompt_tokens} completion_tokens={usage.completion_tokens}"
        )


async def _generate_structured_with_retry(
    llm: AugmentedLLM,
    prompt: str,
    response_model: Type[T],
    max_retries: int = 3,
    delay: float = 1.0,
) -> T:
    """
    Calls llm.generate_structured with retry logic for handling ValidationErrors.
    """
    last_exception = None
    for attempt in range(max_retries):
        try:
            return await llm.generate_structured(
                message=prompt, response_model=response_model
            )
        except ValidationError as e:
            logger.warning(
                f"Validation failed on attempt {attempt + 1}/{max_retries}. Retrying in {delay}s. Error: {e}"
            )
            last_exception = e
            await asyncio.sleep(delay)
    if last_exception:
        raise last_exception
    raise Exception("Unknown error in _generate_structured_with_retry")


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
async def update_confluence_page(
    markdown_content: str, page_url: str, provider: str
) -> str:
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
        logger.debug(f"Initial markdown content: {markdown_content}")

        # 1. Load configuration
        settings = Settings()  # type: ignore
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
        new_content_storage = markdown_to_confluence_storage(markdown_content)
        logger.info("Markdown conversion successful.")
        logger.debug(f"Converted new content storage: {new_content_storage}")

        # 4. Get current page content from Confluence
        page_id = ConfluenceClient._get_page_id_from_url(page_url)
        logger.info(f"Fetching content for page ID: {page_id}")
        original_content, version, title = (
            confluence_client.get_page_content_and_version(page_id)
        )
        logger.info(f"Successfully fetched page '{title}' (version {version}).")
        logger.debug(f"Original page content: {original_content}")

        # 5. Use LLM to merge new content, or upload directly if page is empty
        final_content_storage: str
        if _is_content_empty(original_content):
            logger.info(
                "Original page is empty. Bypassing LLM and using new content directly."
            )
            final_content_storage = new_content_storage
        else:
            async with app.run() as agent_app:
                token_counter = agent_app.context.token_counter
                monitor = TokenMonitor()
                watch_id = await token_counter.watch(
                    callback=monitor.on_token_update,
                    node_type="llm",
                    threshold=1_000,
                    include_subtree=True,
                )
                try:
                    llm_agent = Agent(
                        name="llm_agent",
                        instruction="You are an agent with access to LLMs.",
                    )
                    async with llm_agent:
                        LLMProviderClass = get_llm_provider(provider)

                        # Calculate token count for scaling max_tokens
                        content_to_merge = original_content + new_content_storage
                        content_token_count = await get_token_count(
                            provider, content_to_merge
                        )
                        scaled_max_tokens = int(content_token_count * 1.5) + 1024
                        logger.info(f"Scaled max_tokens to {scaled_max_tokens}")

                        ConfiguredLLMProvider = functools.partial(
                            LLMProviderClass,
                            default_request_params=RequestParams(
                                maxTokens=scaled_max_tokens
                            ),
                        )

                        llm = await llm_agent.attach_llm(ConfiguredLLMProvider)

                        # 5.1: Initial Merge
                        logger.info("Merging new content with original using LLM.")
                        prompt = MERGE_PROMPT.format(
                            original_content=original_content,
                            new_content_storage=new_content_storage,
                        )
                        merged_response = await _generate_structured_with_retry(
                            llm, prompt, ConfluenceContent
                        )
                        merged_content = merged_response.content
                        logger.info("Content merge successful.")
                        logger.debug(f"Merged content: {merged_content}")

                        # 5.2: Reflection Step
                        logger.info("Reflecting on and correcting the merged content.")
                        prompt = REFLECTION_PROMPT.format(
                            original_content=original_content,
                            new_content_storage=new_content_storage,
                            merged_content=merged_content,
                        )
                        corrected_response = await _generate_structured_with_retry(
                            llm, prompt, ConfluenceContent
                        )
                        corrected_content = corrected_response.content
                        logger.info("Reflection and correction step complete.")
                        logger.debug(f"Corrected content: {corrected_content}")

                        # 5.3: Critic Step
                        logger.info("Critiquing final content before update.")
                        prompt = CRITIC_PROMPT.format(
                            original_content=original_content,
                            new_content_storage=new_content_storage,
                            final_proposed_content=corrected_content,
                        )
                        critic_response = await _generate_structured_with_retry(
                            llm, prompt, CriticResponse
                        )

                        if critic_response.decision == "REJECT":
                            logger.error(
                                "Critic agent rejected the final content. Aborting update."
                            )
                            raise Exception(
                                "Critic agent rejected the final content. Please review the logs."
                            )

                        if critic_response.content is None:
                            logger.error(
                                "Critic agent approved but did not provide content. Aborting update."
                            )
                            raise Exception(
                                "Critic agent approved but did not provide content."
                            )

                        final_content_storage = critic_response.content
                        logger.info("Critic agent approved the final content.")
                        logger.debug(
                            f"Final content from critic: {final_content_storage}"
                        )
                finally:
                    await token_counter.unwatch(watch_id)

        # 6. Update the Confluence page
        new_version = version + 1
        logger.info(f"Updating page ID {page_id} to version {new_version}.")
        confluence_client.update_page_content(
            page_id, title, final_content_storage, new_version
        )
        logger.info("Page update successful.")

        return f"Page '{title}' (ID: {page_id}) updated successfully to version {new_version}."

    except Exception as e:
        logger.error(f"An error occurred during the update process: {e}", exc_info=True)
        return f"Error: {e}"
