import asyncio
import logging
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import List, Tuple
import uuid
from xml.sax.saxutils import escape

import pypandoc  # type: ignore
from atlassian import Confluence
from atlassian.errors import ApiError

from confluence_agent.confluence import ConfluenceClient

from .config import Settings

logger = logging.getLogger(__name__)


async def convert_wiki_to_storage(confluence: Confluence, wiki_markup: str) -> str:
    """
    Converts wiki markup to Confluence storage format asynchronously.
    """
    if not wiki_markup.strip():
        return ""

    # Step 1: Start the conversion
    start_url = "rest/api/contentbody/convert/storage"
    payload = {"value": wiki_markup, "representation": "wiki"}
    start_response = confluence.post(start_url, data=payload)

    # Handle immediate response
    if "value" in start_response:
        return start_response["value"]

    async_id = start_response.get("asyncId")
    if not async_id:
        raise ApiError(
            "Failed to start wiki to storage conversion: No asyncId or immediate value in response."
        )

    # Step 2: Poll for the result
    result_url = f"rest/api/contentbody/convert/async/{async_id}"
    for _ in range(10):  # Poll for up to 10 seconds
        await asyncio.sleep(1)
        result_response = confluence.get(result_url)
        if result_response.get("status") == "COMPLETED":
            return result_response.get("value", "")
    raise ApiError("Conversion timed out or failed.")


def replace_fenced_code_with_jira_macro(markdown_text: str) -> str:
    """
    Replaces markdown fenced code blocks with Jira's code block macro.
    """

    def replacer(match: "re.Match[str]") -> str:
        lang = match.group(1).strip()
        code = match.group(2)
        if lang:
            return f"{{code:language={lang}}}\n{code}\n{{code}}"
        return f"{{code}}\n{code}\n{{code}}"

    pattern = re.compile(r"```(.*?)\n(.*?)\n```", re.DOTALL)
    return pattern.sub(replacer, markdown_text)


def render_puml_to_svg(puml_content: str, settings: Settings) -> bytes:
    """Renders PlantUML content to SVG using a direct subprocess call."""
    cmd = [
        settings.plantuml_java_path,
        "-jar",
        settings.plantuml_jar_path,
        "-tsvg",
        "-pipe",
    ]
    try:
        process = subprocess.run(
            cmd,
            input=puml_content.encode("utf-8"),
            capture_output=True,
            check=True,
            text=False,
        )
        return process.stdout
    except FileNotFoundError:
        logger.error(f"Java executable not found at: {settings.plantuml_java_path}")
        raise
    except subprocess.CalledProcessError as e:
        logger.error(f"PlantUML rendering failed: {e.stderr.decode('utf-8')}")
        raise
    except Exception as e:
        logger.error(f"An unexpected error occurred during PlantUML rendering: {e}")
        raise


def process_markdown_puml(
    markdown_content: str, settings: Settings
) -> Tuple[str, List[Tuple[str, bytes]]]:
    """
    Processes markdown content to render PlantUML diagrams and replace
    them with image tags.
    """
    puml_blocks = re.findall(r"```plantuml\n(.*?)\n```", markdown_content, re.DOTALL)
    attachments = []
    for i, puml_block in enumerate(puml_blocks):
        svg_content = render_puml_to_svg(puml_block, settings)
        image_name = f"diagram_{i + 1}.svg"
        attachments.append((image_name, svg_content))
        image_tag = f"![{image_name}](./{image_name})"
        original_block = f"```plantuml\n{puml_block}\n```"
        replacement = f"{image_tag}\n\n{original_block}"
        markdown_content = markdown_content.replace(original_block, replacement)
    return markdown_content, attachments


async def convert_markdown_to_storage(
    markdown_content: str, confluence_client: ConfluenceClient, settings: Settings
) -> Tuple[str, List[Tuple[str, bytes]], str]:
    """
    Converts markdown to Confluence storage format, handling PlantUML diagrams.
    """
    processed_markdown, attachments = process_markdown_puml(markdown_content, settings)

    def create_code_block_macro(match: "re.Match[str]") -> str:
        lang = match.group(1).strip()
        code = match.group(2)

        # Use xml.sax.saxutils.escape to be safe, though CDATA should handle most things.
        escaped_code = escape(code)

        if lang:
            return f"""<ac:structured-macro ac:name="code" ac:schema-version="1">
<ac:parameter ac:name="language">{lang}</ac:parameter>
<ac:plain-text-body><![CDATA[{escaped_code}]]></ac:plain-text-body>
</ac:structured-macro>"""
        else:
            return f"""<ac:structured-macro ac:name="code" ac:schema-version="1">
<ac:plain-text-body><![CDATA[{escaped_code}]]></ac:plain-text-body>
</ac:structured-macro>"""

    pattern = re.compile(r"```(.*?)\n(.*?)\n```", re.DOTALL)
    xml_enhanced_markdown = pattern.sub(create_code_block_macro, processed_markdown)

    wiki_markup = pypandoc.convert_text(xml_enhanced_markdown, "jira", format="gfm")
    wiki_markup = re.sub(r"\{anchor:.*?\}", "", wiki_markup)

    storage_format = await convert_wiki_to_storage(
        confluence_client.confluence, wiki_markup
    )
    return storage_format, attachments, wiki_markup


async def main() -> None:
    """Main function for testing the converter module."""
    logging.basicConfig(level=logging.INFO)
    settings = Settings(
        confluence_url="http://localhost:8090",
        confluence_username="admin",
        confluence_api_token="token",
        plantuml_jar_path="plantuml.jar",
    )
    confluence_client = ConfluenceClient(
        url=settings.confluence_url,
        username=settings.confluence_username,
        api_token=settings.confluence_api_token,
    )

    markdown_example = """
# My Document

Here is a diagram:

```puml
@startuml
Alice -> Bob: Authentication Request
Bob --> Alice: Authentication Response
@enduml
```
"""
    storage_content, puml_attachments, wiki_markup = await convert_markdown_to_storage(
        markdown_example, confluence_client, settings
    )
    print("----- Storage Format -----")
    print(storage_content)
    print("\n----- Wiki Markup -----")
    print(wiki_markup)
    print("\n----- Attachments -----")
    for name, _ in puml_attachments:
        print(name)


if __name__ == "__main__":
    asyncio.run(main())
