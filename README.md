# Confluence Agent Workflow

This project implements an agentic workflow to update a Confluence page from a
local markdown file. It uses `mcp-agent` to expose this workflow as an MCP
server.

The agent intelligently merges new content into the existing Confluence page.
It uses a multi-step process involving an initial merge, a reflection step to
correct errors, and a final critic step to ensure quality before updating the
page. This process preserves Confluence-specific elements like macros,
attachments, and inline comments by leveraging an LLM.

## Prerequisites

* Python 3.10+
* An active virtual environment (e.g., venv, conda).
* Access to a Confluence Cloud instance with API token credentials.
* An LLM provider API key (e.g., OpenAI, Anthropic, Google).

### Installation

1. **Clone the repository:**

    ```bash
    git clone https://github.com/mstump/confluence-workflow
    cd confluence-agent-workflow
    ```

2. **Create a virtual environment and install dependencies:**

    ```bash
    uv venv
    source .venv/bin/activate
    uv pip install -e '.[dev]'
    ```

3. **Configure environment variables:**
    Copy the example environment file and fill in your credentials.

    ```bash
    cp .env.example .env
    ```

    Edit `.env` with your Confluence URL, username, API token, and OpenAI API
    key.

## Verified LLM Providers

This workflow has been verified with the following LLM provider and model:

* **Provider**: `openai`
* **Model**: `gpt-5-nano`

Note: When using Google's models, `gemini-2.5-flash-lite` produced unsatisfactory results, and other Google models have not yet been tested.

## Usage

### Setting Up Pre-commit Hooks

This project uses `pre-commit` to enforce code style and quality checks. To set
it up, run the following command after installing the dev dependencies:

```bash
pre-commit install
```

This will install git hooks that run automatically before each commit.

You can also run the hooks manually on all files at any time:

```bash
pre-commit run --all-files
```

### Running Tests

To ensure everything is set up correctly, run the test suite:

```bash
pytest
```

Note: The `pytest` command is configured to be run via `uv run pytest`.

### Starting the MCP Server

To run the agent as an MCP server, use the `mcp-agent` CLI:

```bash
uvx mcp-agent serve confluence_agent
```

The server will be available on `localhost:8000` by default.

### Command-Line Interface (CLI)

The `confluence-agent` provides a command-line interface for interacting with Confluence.

While the package installs a `confluence-agent` command, for development it is often more reliable to invoke the CLI via `python -m`. Here is the recommended format:

```bash
LOG_LEVEL='INFO' PYTHONPATH=./src uv run python -m confluence_agent.cli [COMMAND] [ARGS]
```

#### `update`

Updates a Confluence page with the content of a local markdown file, using an LLM to merge the content intelligently.

**Example:**

```bash
LOG_LEVEL='INFO' PYTHONPATH=./src uv run python -m confluence_agent.cli update 'path/to/your/document.md' 'https://your-domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Your+Page+Title'
```

**Arguments:**

* `MARKDOWN_PATH`: The local path to the markdown file.
* `PAGE_URL`: The URL of the Confluence page to update.

**Options:**

* `--provider` / `-p`: Specify the LLM provider to use (`openai` or `google`). Overrides the `LLM_PROVIDER` environment variable.

#### `upload`

Converts a local markdown file to Confluence storage format and uploads it, overwriting the existing content. This command does not use an LLM for merging.

**Example:**

```bash
LOG_LEVEL='INFO' PYTHONPATH=./src uv run python -m confluence_agent.cli upload 'path/to/your/document.md' 'https://your-domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Your+Page+Title'
```

**Arguments:**

* `MARKDOWN_PATH`: The local path to the markdown file.
* `PAGE_URL`: The URL of the Confluence page to update.

#### `convert`

Converts a local markdown file to Confluence storage format and saves it locally without uploading.

**Example:**

```bash
LOG_LEVEL='INFO' PYTHONPATH=./src uv run python -m confluence_agent.cli convert 'path/to/your/document.md' 'path/to/output/dir'
```

**Arguments:**

* `MARKDOWN_PATH`: The local path to the markdown file.
* `OUTPUT_DIR`: The directory to save the converted file and any generated diagrams.

### Interacting with the Agent

When running as an MCP server, the agent exposes the `update_confluence_page` tool, which can be invoked via an MCP client or a direct HTTP request.

#### Tool: `update_confluence_page`

Updates a Confluence page with the content of a markdown string. This tool performs the same intelligent merging as the `update` CLI command.

**Input:**

* `markdown_content` (string): The markdown content to update the page with.
* `page_url` (string): The URL of the target Confluence page.
* `provider` (string): The LLM provider to use (`openai` or `google`).

**Example using `curl`:**

```bash
curl -X POST http://localhost:8000/tools/update_confluence_page/invoke \
-H "Content-Type: application/json" \
-d '{
    "input": {
        "markdown_content": "# My New Section\n\nThis is the updated content.",
        "page_url": "https://your-domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Your+Page+Title",
        "provider": "openai"
    }
}'
```

Make sure to replace the `page_url` with the URL of your target Confluence page and modify the `markdown_content` with your desired input.

## Developer Notes

When modifying the command-line interface, particularly `src/confluence_agent/cli.py`, you may need to force a reinstallation of the package for your changes to take effect on the `confluence-agent` command. This is because the entrypoint script is generated during installation.

You can do this by running:

```bash
uv pip uninstall confluence-agent && uv pip install -e '.[dev]'
```
