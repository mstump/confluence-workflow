# Confluence Agent Workflow

This project implements an agentic workflow to update a Confluence page from a
local markdown file. It uses `mcp-agent` to expose this workflow as an MCP
server.

The agent intelligently merges the new content from the markdown file into the
existing Confluence page, preserving Confluence-specific elements like macros,
attachments, and inline comments by leveraging an LLM.

## Prerequisites

- Python 3.9+
- [uv](https://github.com/astral-sh/uv) package manager
- [pandoc](https://pandoc.org/installing.html)

## Setup

1. **Clone the repository:**

    ```bash
    git clone <repository-url>
    cd confluence-agent-workflow
    ```

2. **Create a virtual environment and install dependencies:**

    ```bash
    uv venv
    source .venv/bin/activate
    uv pip install -e .[dev]
    ```

3. **Configure environment variables:**
    Copy the example environment file and fill in your credentials.

    ```bash
    cp .env.example .env
    ```

    Edit `.env` with your Confluence URL, username, API token, and OpenAI API
    key.

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

### Starting the MCP Server

To run the agent as an MCP server, use the `mcp-agent` CLI:

```bash
uvx mcp-agent serve confluence_agent
```

The server will be available on `localhost:8000` by default.

### Interacting with the Agent

You can interact with the running agent's tool `update_confluence_page` using an
MCP client or by sending a direct HTTP request.

**Example using `curl`:**

```bash
curl -X POST http://localhost:8000/tools/update_confluence_page/invoke \
-H "Content-Type: application/json" \
-d '{
    "input": {
        "markdown_content": "# My New Section\n\nThis is the updated content.",
        "page_url": "https://your-domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Your+Page+Title"
    }
}'
```

Make sure to replace the `page_url` with the URL of your target Confluence
page and modify the `markdown_content` with your desired input.
