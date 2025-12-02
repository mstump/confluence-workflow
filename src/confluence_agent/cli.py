import typer
from rich.console import Console
from typing import Optional
import os
import asyncio
from pathlib import Path
import tempfile

from confluence_agent.agent import update_confluence_page
from confluence_agent.config import Settings
from confluence_agent.confluence import ConfluenceClient
from confluence_agent.converter import convert_markdown_to_storage

app = typer.Typer()
console = Console()


@app.callback()
def callback() -> None:
    """
    A CLI for updating Confluence pages from local markdown files.
    """


@app.command()
def update(
    markdown_path: str = typer.Argument(
        ..., help="The local path to the markdown file."
    ),
    page_url: str = typer.Argument(
        ..., help="The URL of the Confluence page to update."
    ),
    provider: Optional[str] = typer.Option(
        None,
        "--provider",
        "-p",
        help="The LLM provider to use ('openai' or 'google'). Overrides LLM_PROVIDER env var.",
    ),
) -> None:
    """
    Updates a Confluence page with the content of a markdown file.
    """
    settings = Settings()  # type: ignore
    llm_provider = provider if provider else settings.llm_provider

    async def run_update() -> None:
        with console.status(
            "[bold green]Updating Confluence page...", spinner="dots"
        ) as status:
            status.update(f"[bold green]Reading markdown file from: {markdown_path}...")
            try:
                with open(markdown_path, "r", encoding="utf-8") as f:
                    markdown_content = f.read()
            except FileNotFoundError:
                console.print(
                    f"[bold red]Error:[/bold red] Markdown file not found at: {markdown_path}"
                )
                raise typer.Exit(code=1)

            status.update(
                f"[bold green]Updating Confluence page: {page_url} using {llm_provider}..."
            )
            result = await update_confluence_page(
                markdown_content, page_url, llm_provider
            )

            if "Error" in result:
                console.print(f"[bold red]Failed to update page:[/bold red] {result}")
                raise typer.Exit(code=1)
            else:
                console.print(f"[bold green]Success:[/bold green] {result}")

    asyncio.run(run_update())


@app.command()
def upload(
    markdown_path: str = typer.Argument(
        ..., help="The local path to the markdown file."
    ),
    page_url: str = typer.Argument(
        ..., help="The URL of the Confluence page to update."
    ),
) -> None:
    """
    Converts a local markdown file and uploads it to a Confluence page.
    """
    settings = Settings()  # type: ignore
    confluence_client = ConfluenceClient(
        url=settings.confluence_url,
        username=settings.confluence_username,
        api_token=settings.confluence_api_token,
    )

    console.print(f"Reading markdown file from: {markdown_path}")
    try:
        with open(markdown_path, "r", encoding="utf-8") as f:
            markdown_content = f.read()
    except FileNotFoundError:
        console.print(
            f"[bold red]Error:[/bold red] Markdown file not found at: {markdown_path}"
        )
        raise typer.Exit(code=1)

    try:
        console.print("Converting markdown to Confluence storage format...")
        page_id = ConfluenceClient._get_page_id_from_url(page_url)
        console.print(f"Fetching details for page ID: {page_id}")
        content, version, title = confluence_client.get_page_content_and_version(
            page_id
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            work_dir = Path(temp_dir)
            storage_format, attachments = convert_markdown_to_storage(
                markdown_content, settings, work_dir
            )

            if attachments:
                console.print(f"Uploading {len(attachments)} attachments...")
                confluence_client.upload_attachments(
                    page_id,
                    [
                        (os.path.join(work_dir, filename), content)
                        for filename, content in attachments
                    ],
                )

            console.print(f"Uploading content to page '{title}' (ID: {page_id})...")
            new_version = version + 1
            confluence_client.update_page_content(
                page_id, title, storage_format, new_version
            )

        console.print(
            f"[bold green]Success:[/bold green] Page '{title}' updated to version {new_version}."
        )
    except Exception as e:
        console.print(f"[bold red]Error:[/bold red] {e}")
        raise typer.Exit(code=1)


@app.command()
def convert(
    markdown_path: str = typer.Argument(
        ..., help="The local path to the markdown file."
    ),
    output_dir: str = typer.Argument(
        ..., help="The output directory for the converted file and diagrams."
    ),
) -> None:
    """
    Converts a local markdown file to Confluence storage format.
    """
    settings = Settings()  # type: ignore
    confluence_client = ConfluenceClient(
        url=settings.confluence_url,
        username=settings.confluence_username,
        api_token=settings.confluence_api_token,
    )

    console.print(f"Reading markdown file from: {markdown_path}")
    try:
        with open(markdown_path, "r", encoding="utf-8") as f:
            markdown_content = f.read()
    except FileNotFoundError:
        console.print(
            f"[bold red]Error:[/bold red] Markdown file not found at: {markdown_path}"
        )
        raise typer.Exit(code=1)

    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    storage_format, attachments = convert_markdown_to_storage(
        markdown_content, settings, output_path
    )

    input_file_stem = Path(markdown_path).stem
    storage_output_path = output_path / f"{input_file_stem}.storage.html"

    with open(storage_output_path, "w", encoding="utf-8") as f:
        f.write(storage_format)

    console.print(
        f"[bold green]Success:[/bold green] Converted content and diagrams saved to: {output_dir}"
    )


if __name__ == "__main__":
    app()
