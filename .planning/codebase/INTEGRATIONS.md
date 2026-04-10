# External Integrations

**Analysis Date:** 2026-04-10

## APIs & External Services

**Confluence Cloud:**

- Confluence Cloud REST API - Document storage and page management
  - SDK/Client: `atlassian-python-api==3.41.16` (Confluence class)
  - Auth: HTTP Basic Auth via `CONFLUENCE_USERNAME` and `CONFLUENCE_API_TOKEN` env vars
  - Config location: `src/confluence_agent/confluence.py` (ConfluenceClient class)
  - Operations: Get page content, update page content, upload attachments
  - URL pattern: Extracts page ID from URL patterns like `/pages/{id}`, `/pages/edit-v2/{id}`, `?pageId={id}`

**Large Language Models:**

- **OpenAI API (GPT models)**
  - Default provider (configurable via `LLM_PROVIDER` env var)
  - SDK: `openai==2.8.1`
  - Auth: `OPENAI__API_KEY` env var (nested format)
  - Default model: `gpt-5-nano` (configurable via `OPENAI__DEFAULT_MODEL`)
  - Implementation: `src/confluence_agent/patched_providers.py` (ChunkAwareOpenAIAugmentedLLM)
  - Token counting: Via `tiktoken` with `cl100k_base` encoding

- **Google Generative AI (Gemini models)**
  - Alternative provider (switch via `LLM_PROVIDER=google`)
  - SDK: `google-genai==1.52.0`
  - Auth: `GOOGLE__API_KEY` env var (nested format)
  - Default model: `gemini-2.5-flash-lite` (per config.py) - Note: produces poor results per CLAUDE.md
  - Implementation: `src/confluence_agent/patched_providers.py` (ChunkAwareGoogleAugmentedLLM)
  - Token counting: Heuristic approximation (characters / 4)

## Data Storage

**Databases:**

- None - Stateless workflow (reads from Confluence, processes via LLM, writes back)

**File Storage:**

- **Local filesystem (during processing)**
  - PlantUML/Mermaid diagrams rendered to temporary files via `tempfile` module
  - SVG outputs written to working directory before upload
  - Location: `src/confluence_agent/converter.py` (process_markdown_puml, render_mermaid_to_svg functions)

- **Confluence Cloud (persistent storage)**
  - Converted markdown → Confluence storage format XML
  - Rendered diagrams uploaded as page attachments (attachment upload via atlassian-python-api)

**Caching:**

- None - No explicit caching layer

## Authentication & Identity

**Auth Providers:**

- Custom via HTTP Basic Auth (Confluence)
- API Key Auth (OpenAI, Google Gemini)

**Implementation:**

- Confluence: `CONFLUENCE_USERNAME` + `CONFLUENCE_API_TOKEN` passed to Confluence(url, username, password, cloud=True)
- OpenAI: API key injected via mcp-agent OpenAISettings
- Google: API key injected via mcp-agent GoogleSettings
- All configured in `src/confluence_agent/config.py` via pydantic-settings from `.env`

## Monitoring & Observability

**Error Tracking:**

- None configured - Errors logged via Python stdlib logging

**Logs:**

- **Transport:** Console (configured in `mcp_agent.config.yaml`)
- **Handling:** Python `logging` module with configurable `LOG_LEVEL` env var (default: INFO)
  - Structured: `"%(asctime)s - %(name)s - %(levelname)s - %(message)s"`
  - Log level controlled via `LOG_LEVEL` environment variable in `src/confluence_agent/agent.py`
  - RUST_LOG also supported (legacy, for mcp-agent framework)

**Tracing:**

- OpenTelemetry enabled in `mcp_agent.config.yaml`
- Exporter: Console (no external tracing backend)
- Span tracking for LLM operations via mcp-agent built-in tracing

## CI/CD & Deployment

**Hosting:**

- Docker container registry: GitHub Container Registry (GHCR)
- Image naming: `ghcr.io/{github-repo-path}:{tag}`

**CI Pipeline:**

- GitHub Actions workflow: `.github/workflows/publish.yml`
- Trigger: Push to `main` branch
- Steps:
  1. Checkout code
  2. Log in to GHCR (uses GITHUB_TOKEN)
  3. Extract metadata (latest + short SHA tags)
  4. Build and push Docker image
- Permissions: `contents: read`, `packages: write`

**Container Configuration:**

- Base image: `python:3.11-slim`
- System dependencies installed:
  - OpenJDK 21 JRE (for PlantUML)
  - PlantUML package
  - Node.js + npm (for mermaid-cli)
  - Chromium + fonts-liberation (for Puppeteer/mermaid rendering)
  - wget
- Python dependencies: Installed via `uv sync` and project installed with `uv pip install .`
- Entrypoint: `confluence-agent` CLI command

## Environment Configuration

**Required env vars (from `.env.example`):**

- `CONFLUENCE_URL` - Confluence instance URL (e.g., <https://domain.atlassian.net/wiki>)
- `CONFLUENCE_USERNAME` - Confluence user email
- `CONFLUENCE_API_TOKEN` - Confluence API token (from account settings)
- `LLM_PROVIDER` - Provider to use: `openai` (default) or `google`
- `OPENAI__API_KEY` - OpenAI API key if using OpenAI provider (sk-prefixed)
- `OPENAI__DEFAULT_MODEL` - OpenAI model to use (default: gpt-5-nano)
- `GOOGLE__API_KEY` - Google API key if using Google provider
- `GOOGLE__DEFAULT_MODEL` - Google model to use (default: gemini-2.5-flash-lite)

**Optional env vars:**

- `LOG_LEVEL` - Logging level (default: INFO)
- `RUST_LOG` - Legacy mcp-agent logging level
- `PLANTUML_JAR_PATH` - Path to plantuml.jar (default: plantuml.jar)
- `PLANTUML_JAVA_PATH` - Path to java executable (default: java)
- `MERMAID_CLI_PATH` - Path to mermaid-cli/mmdc (default: mmdc)
- `MERMAID_PUPPETEER_CONFIG` - Path to Puppeteer config JSON (Docker: /etc/puppeteer.json)

**Secrets location:**

- `.env` file (git-ignored, not committed)
- Environment variables in CI/CD via GitHub Actions secrets
- Dockerfile sets some defaults (PLANTUML_JAR_PATH, PUPPETEER_EXECUTABLE_PATH, MERMAID_PUPPETEER_CONFIG)

## Webhooks & Callbacks

**Incoming:**

- None - This is a push-based workflow (CLI or scheduled job can trigger updates)

**Outgoing:**

- None - Direct Confluence API calls only

## External Tools/Services

**Diagram Rendering:**

- **PlantUML** - Java subprocess call to render UML diagrams
  - Invoked via subprocess with `-jar {jar_path} -tsvg -pipe` for SVG output
  - Implementation: `src/confluence_agent/converter.py` (render_puml_to_svg function)

- **Mermaid CLI (mmdc)** - Node.js CLI tool for rendering Mermaid diagrams
  - Subprocess call with Puppeteer integration
  - Chromium required in Docker; optional in development
  - Implementation: `src/confluence_agent/converter.py` (render_mermaid_to_svg function)
  - Puppeteer config: Via `MERMAID_PUPPETEER_CONFIG` env var (Docker runs with --no-sandbox)

**Content Conversion:**

- **markdown-to-confluence** - Python package for markdown → Confluence storage XML conversion
  - Handles standard Markdown syntax + Confluence macros
  - Strips YAML frontmatter before conversion (Obsidian support)
  - Replaces fenced code blocks with Confluence code macros
  - Width attribute handling for `ac:image` tags via `str.replace` (commit 38c0779)

---

Integration audit: 2026-04-10
