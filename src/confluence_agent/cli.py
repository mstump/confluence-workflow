import typer
import asyncio
from rich.console import Console
from typing import Optional

from confluence_agent.agent import update_confluence_page
from confluence_agent.config import Settings

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
    settings = Settings()
    llm_provider = provider if provider else settings.llm_provider

    console.print(f"Reading markdown file from: {markdown_path}")
    try:
        with open(markdown_path, "r", encoding="utf-8") as f:
            markdown_content = f.read()
    except FileNotFoundError:
        console.print(
            f"[bold red]Error:[/bold red] Markdown file not found at: {markdown_path}"
        )
        raise typer.Exit(code=1)

    console.print(f"Updating Confluence page: {page_url} using {llm_provider}")

    async def run_update() -> None:
        result = await update_confluence_page(markdown_content, page_url, llm_provider)
        if "Error" in result:
            console.print(f"[bold red]Failed to update page:[/bold red] {result}")
            raise typer.Exit(code=1)
        else:
            console.print(f"[bold green]Success:[/bold green] {result}")

    asyncio.run(run_update())


if __name__ == "__main__":
    app()
