# About This Project

This project provides a `confluence-agent` that converts Jira markup into Confluence Wiki format and uploads it to a specified Confluence space. It leverages an LLM to assist with the conversion process and includes features for handling images and diagrams.

## Codebase Structure

The codebase is organized into the following key directories and files:

- `src/confluence_agent/`: Contains the main source code for the `confluence-agent`.
  - `agent.py`: The core logic for the agent, orchestrating the conversion and upload process.
  - `cli.py`: The command-line interface for the agent, built using `typer`.
  - `config.py`: Handles loading and validation of configuration from a `.env` file.
  - `confluence.py`: A client for interacting with the Confluence API (e.g., uploading pages, attachments).
  - `converter.py`: The logic for converting Jira markup to Confluence Wiki format.
  - `llm.py`: Interacts with a Large Language Model (LLM) to assist in conversion.
  - `llm_prompts.py`: Contains the prompts used for the LLM.
  - `md2conf.py`: A module for converting Markdown to Confluence format.
  - `models.py`: Defines the data models used throughout the application.
- `tests/`: Contains the automated tests for the project.
  - `test_*.py`: Individual test files corresponding to the modules in `src/confluence_agent/`.
- `pyproject.toml`: The project's build and dependency configuration file.
- `README.md`: The main project documentation.
- `AGENTS.md`: This file, providing instructions for AI agents.

## Common Tasks

### Running Tests

To run the full test suite, use the following command:

```bash
uv run pytest
```

### Type Checking

To validate type hints, use `mypy`:

```bash
uv run mypy .
```

### Formatting Code

To format the Python code using `black`:

```bash
uv run black .
```

### Linting Markdown Files

To lint and fix Markdown files:

```bash
markdownlint --fix .
```
