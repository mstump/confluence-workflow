import tempfile
from pathlib import Path
import re

from md2conf.domain import ConfluenceDocumentOptions
from md2conf.local import LocalConverter
from md2conf.metadata import ConfluenceSiteMetadata


def markdown_to_confluence_storage(markdown_content: str) -> str:
    """
    Converts a markdown string to Confluence storage format (XHTML) using md2conf library.
    """
    if not markdown_content.strip():
        return ""

    with tempfile.TemporaryDirectory() as temp_dir_str:
        temp_dir = Path(temp_dir_str)
        input_file = temp_dir / "input.md"
        input_file.write_text(markdown_content, encoding="utf-8")

        options = ConfluenceDocumentOptions(generated_by=None)
        site = ConfluenceSiteMetadata(
            domain="example.com", base_path="/", space_key="TEST"
        )
        converter = LocalConverter(options=options, site=site, out_dir=temp_dir)
        converter.process(input_file)

        output_file = temp_dir / "input.csf"
        if not output_file.exists():
            # If the output file is not created, it might be because the markdown is empty
            # or only contains comments. In this case, return an empty string.
            if not markdown_content.strip():
                return ""
            raise FileNotFoundError("Conversion failed, no output file generated.")

        xhtml_content = output_file.read_text(encoding="utf-8")

        # Post-process to remove anchor macros.
        # Using regex as the XHTML is not well-formed for standard XML parsers.
        # <ac:structured-macro ac:name="anchor" ...>...</ac:structured-macro>
        anchor_pattern = (
            r'<ac:structured-macro ac:name="anchor".*?</ac:structured-macro>'
        )
        cleaned_content = re.sub(anchor_pattern, "", xhtml_content, flags=re.DOTALL)

        return cleaned_content
