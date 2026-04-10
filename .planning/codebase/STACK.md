# Technology Stack

**Analysis Date:** 2026-04-10

## Languages

**Primary:**

- Python 3.10+ - Core application logic, LLM integration, Confluence API interactions
- Markdown - Documentation and content format

**Secondary:**

- YAML - Configuration files (mcp_agent.config.yaml, .pre-commit-config.yaml)
- XML - Confluence storage format output

## Runtime

**Environment:**

- Python 3.10 minimum (per pyproject.toml requires-python)
- Tested/deployed on Python 3.11 and 3.13 (Dockerfile uses 3.11-slim)

**Package Manager:**

- `uv` - Fast Python package installer and resolver
- Lockfile: `uv.lock` (present - pinned versions enforced)

## Frameworks

**Core:**

- `mcp-agent==0.2.5` - Model Context Protocol application framework for building agent workflows
- `typer==0.20.0` - CLI framework with rich output support
- `pydantic-settings==2.12.0` - Environment-based configuration management

**LLM Integration:**

- `openai==2.8.1` - OpenAI API client for GPT models
- `google-genai==1.52.0` - Google Generative AI client for Gemini models
- `tiktoken==0.8.0` - Token counting for OpenAI models

**Confluence Integration:**

- `atlassian-python-api==3.41.16` - Confluence Cloud API wrapper
- `markdown-to-confluence==0.4.8` - Markdown to Confluence storage format converter

## Key Dependencies

**Critical:**

- `mcp-agent==0.2.5` - Provides AugmentedLLM base classes and MCP server framework; core dependency for agent orchestration
- `openai==2.8.1` - Primary LLM provider (default); gpt-5-nano model specified in config
- `google-genai==1.52.0` - Secondary LLM provider; Gemini models available
- `atlassian-python-api==3.41.16` - Confluence Cloud API integration; handles page retrieval/update and attachment uploads
- `markdown-to-confluence==0.4.8` - Converts markdown to Confluence storage XML format

**Diagram Rendering:**

- PlantUML (Java-based, requires external JAR) - Renders PlantUML diagrams to SVG
- Mermaid CLI (npm package) - Renders Mermaid diagrams to SVG via mermaid-cli (`@mermaid-js/mermaid-cli`)

**Infrastructure:**

- `pydantic==2.x` (transitive) - Data validation and serialization via pydantic-settings and mcp-agent
- `rich==1.x` (transitive) - Terminal output formatting for CLI

## Development Dependencies

**Testing:**

- `pytest==9.0.1` - Test runner
- `pytest-anyio==0.0.0` - Async test support

**Code Quality:**

- `black==25.11.0` - Code formatter (line-length: 88, target Python 3.9+)
- `mypy==1.18.2` - Type checker (strict mode enabled, excludes tests/)
- `pre-commit==4.5.0` - Git hooks framework

**Additional:**

- `trio==0.32.0` - Async runtime for testing

## Configuration

**Environment:**

- Loaded via `.env` file using `pydantic-settings` with `env_file_encoding="utf-8"`
- Nested variables supported via `env_nested_delimiter="__"` (e.g., `OPENAI__API_KEY`)
- Key variables defined in `.env.example` (see INTEGRATIONS.md for full list)

**Build:**

- `pyproject.toml` - Single source of truth for dependencies and build metadata
- `Dockerfile` - Container build with Python 3.11-slim, Java 21 JRE, PlantUML, Node.js, mermaid-cli, and Chromium
- `mcp_agent.config.yaml` - MCP server configuration with app registration and OpenTelemetry settings

**Code Quality Configuration:**

- `.markdownlint.yaml` - Markdown linting rules
- `.pre-commit-config.yaml` - Pre-commit hooks: black, markdownlint, trailing-whitespace, end-of-file-fixer, check-yaml, pytest
- `pyproject.toml [tool.black]` - Line length 88, target Python 3.9
- `pyproject.toml [tool.mypy]` - Strict mode with Python 3.10, excludes tests/, ignores missing imports for third-party packages

## Platform Requirements

**Development:**

- Python 3.10+
- Java Runtime Environment (JRE) for PlantUML rendering
- Node.js and npm for mermaid-cli installation
- Optional: Chromium/Puppeteer for mermaid diagram rendering (configured via `MERMAID_PUPPETEER_CONFIG`)

**Production:**

- Deployment via Docker (ubuntu-latest base with Python 3.11-slim)
- Container registry: GitHub Container Registry (GHCR) at `ghcr.io/{repository}`
- CI/CD: GitHub Actions on `main` branch push (workflow: `.github/workflows/publish.yml`)
- Runs as entrypoint: `confluence-agent` CLI command

---

Stack analysis: 2026-04-10
