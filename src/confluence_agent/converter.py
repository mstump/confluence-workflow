import logging
import os
import re
import subprocess
from typing import List, Tuple
from xml.sax.saxutils import escape

from md2conf.collection import ConfluencePageCollection
from md2conf.converter import ConfluenceDocument
from md2conf.domain import ConfluenceDocumentOptions
from md2conf.metadata import ConfluenceSiteMetadata
from md2conf.scanner import ScannedDocument
from pathlib import Path


from .config import Settings

logger = logging.getLogger(__name__)


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


def convert_markdown_to_storage(
    markdown_content: str, settings: Settings
) -> Tuple[str, List[Tuple[str, bytes]]]:
    """
    Converts markdown to Confluence storage format, handling PlantUML diagrams.
    """
    processed_markdown, attachments = process_markdown_puml(markdown_content, settings)

    scanned_document = ScannedDocument(
        page_id=None,
        space_key=None,
        generated_by=None,
        title=None,
        tags=None,
        synchronized=None,
        properties=None,
        alignment=None,
        text=processed_markdown,
    )
    options = ConfluenceDocumentOptions(ignore_invalid_url=True, generated_by=None)
    # fake path needed for link and image resolution, though we don't have any in our case
    path = Path.cwd()
    root_dir = Path.cwd()
    site_metadata = ConfluenceSiteMetadata(
        domain="localhost", base_path="/", space_key="TEST"
    )
    page_metadata = ConfluencePageCollection()

    confluence_document = ConfluenceDocument(
        path,
        scanned_document,
        options,
        root_dir,
        site_metadata,
        page_metadata,
    )
    storage_format = confluence_document.xhtml()

    return storage_format, attachments


def main() -> None:
    """Main function for testing the converter module."""
    logging.basicConfig(level=logging.INFO)
    settings = Settings(
        confluence_url="http://localhost:8090",
        confluence_username="admin",
        confluence_api_token="token",
        plantuml_jar_path="plantuml.jar",
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

And a code block:

```python
def hello():
    print("Hello, World!")
```
"""
    storage_content, puml_attachments = convert_markdown_to_storage(
        markdown_example, settings
    )
    print("----- Storage Format -----")
    print(storage_content)
    print("\n----- Attachments -----")
    for name, _ in puml_attachments:
        print(name)


if __name__ == "__main__":
    main()
