# Confluence Agent Workflow

An intelligent agentic workflow that converts local Markdown files into Confluence pages. It leverages Large Language Models (LLMs) to intelligently merge content, preserving existing page context like inline comments and macros, while automatically handling diagrams.

This project uses [mcp-agent](https://pypi.org/project/mcp-agent/) to expose this workflow as a Model Context Protocol (MCP) server.

## Motivation

I write Confluence pages in markdown, typically using Cursor as my editor, and then publish to Confluence. When publishing I routinely ran into two problems:

- My diagrams didn't render correctly. I had to manually re-add diagram macros after publishing.
- Every time I published, I lost the location context of inline comments.

This workflow solves both of these issues:

- It renders diagrams as SVGs and inserts an inline image above the associated code block. When publishing, the SVG files are uploaded as attachments.
- It uses an LLM to get the existing page state and merge in any comment location markup so that when publishing, comments are retained.

## Key Features

- **Intelligent Merging**: Uses a multi-step LLM process (Merge -> Reflect -> Critic) to update pages without overwriting existing context.
- **Comment Preservation**: Retains the location context of inline comments on Confluence pages, solving a common pain point when publishing from Markdown.
- **Diagram Support**: Automatically renders [PlantUML](https://plantuml.com/) diagrams as SVGs and uploads them as attachments.
- **Markdown Compatibility**: Built on [markdown-to-confluence](https://pypi.org/project/markdown-to-confluence/) for robust conversion to Confluence Storage Format.

## Prerequisites

- **Python 3.10+**
- **uv**: Recommended for dependency management (or standard pip/venv).
- **Confluence Cloud**: Access to an instance with API token credentials.
- **LLM API Key**: Access to an LLM provider (OpenAI or Google).

## Installation

1. **Clone the repository:**

    ```bash
    git clone https://github.com/mstump/confluence-workflow
    cd confluence-workflow
    ```

2. **Set up the environment:**

    Using `uv` (recommended):

    ```bash
    uv venv
    source .venv/bin/activate
    uv pip install -e '.[dev]'
    ```

3. **Configure Credentials:**

    Copy the example environment file and add your API keys:

    ```bash
    cp .env.example .env
    ```

    Edit `.env` with your details:
    - `CONFLUENCE_URL` (e.g., `https://your-domain.atlassian.net/wiki`)
    - `CONFLUENCE_USERNAME` (Email address)
    - `CONFLUENCE_API_TOKEN` (Create one at Atlassian Account Settings)
    - `OPENAI_API_KEY` (or Google equivalent)

### Verified LLM Providers

This workflow is verified with:

- **OpenAI**: `gpt-5` (Configured as default)
- **Google**: `gemini-2.5-pro`

*Note: When using Google's models, `gemini-2.5-flash-lite` produced unsatisfactory results.*

## Usage

The tool can be used via the Command-Line Interface (CLI) or as an MCP Server.

### CLI Commands

For development, it is recommended to run commands using `python -m` to ensure the local source is used.

First, export the necessary variables:

```bash
export LOG_LEVEL='INFO'
export PYTHONPATH=./src
```

#### 1. Update a Page (Recommended)

Updates a Confluence page using the intelligent LLM merge agent. This preserves comments and handles conflicts.

```bash
uv run python -m confluence_agent.cli update 'path/to/document.md' 'https://your-domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Title'
```

**Options:**

- `--provider` / `-p`: Override the LLM provider (e.g., `-p google`).

#### 2. Upload (Direct)

Converts and uploads the file, **overwriting** existing content (no LLM merge). Use this for initial page creation or when context preservation isn't needed.

```bash
uv run python -m confluence_agent.cli upload 'path/to/document.md' 'https://your-domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Title'
```

#### 3. Convert Only

Converts Markdown to Confluence Storage Format locally for inspection.

```bash
uv run python -m confluence_agent.cli convert 'path/to/document.md' './output_dir'
```

### MCP Server

Run the agent as an MCP server to integrate with AI coding assistants (like Cursor) or other MCP clients.

```bash
uvx mcp-agent serve confluence_agent
```

The server will run on `localhost:8000` (default).

**Available Tool:** `update_confluence_page`

- **Inputs**:
  - `markdown_content` (string): The new content.
  - `page_url` (string): Target Confluence page.
  - `provider` (string): `openai` or `google`.

## Docker Support

A Docker image is available containing all dependencies (Python, Java, PlantUML, Pandoc).

- **Registry**: [ghcr.io/mstump/confluence-workflow](https://github.com/users/mstump/packages/container/package/confluence-workflow)
- **Build**: `docker build -t confluence-agent .`

### Run with Docker

Mount your local documents directory to `/app` in the container:

```bash
docker run --rm -it \
  --env-file .env \
  -v "$(pwd)/docs:/app/docs" \
  ghcr.io/mstump/confluence-workflow:latest \
  update /app/docs/page.md 'https://your-domain.atlassian.net/wiki/...'
```

## Development

### Linting & Formatting

This project uses `pre-commit` to enforce code style.

```bash
pre-commit install
pre-commit run --all-files
```

### Running Tests

```bash
uv run pytest
```

### Re-installing CLI

If modifying `src/confluence_agent/cli.py`, re-install the package to update the `confluence-agent` entrypoint:

```bash
uv pip uninstall confluence-agent && uv pip install -e '.[dev]'
```
