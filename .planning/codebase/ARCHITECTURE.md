# Architecture

**Analysis Date:** 2026-04-10

## Pattern Overview

**Overall:** Multi-stage LLM agent pipeline (Merge → Reflect → Critic) with Confluence API integration

**Key Characteristics:**

- Three-phase LLM chain with structured validation at each stage
- Markdown-to-Confluence conversion pipeline with diagram rendering
- Async/await throughout with MCP (Model Context Protocol) agent integration
- Inline comment preservation as immutable tokens through LLM processing
- Conditional LLM processing—bypassed when page is empty or has no comments

## Layers

**Orchestration Layer:**

- Purpose: Coordinates the full update workflow and manages LLM phases
- Location: `src/confluence_agent/agent.py`
- Contains: `update_confluence_page()` tool, `_process_content_with_llm()`, phase functions
- Depends on: Confluence client, converter, LLM provider, config
- Used by: CLI and MCP server endpoints

**Content Conversion Layer:**

- Purpose: Transforms markdown to Confluence storage XML format and renders diagrams
- Location: `src/confluence_agent/converter.py`
- Contains: PlantUML/Mermaid rendering, markdown-to-storage conversion, diagram processing
- Depends on: Settings (for tool paths), subprocess for PlantUML/Mermaid CLIs, md2conf library
- Used by: Orchestration layer

**Confluence Integration Layer:**

- Purpose: API client for reading and updating Confluence pages, managing attachments
- Location: `src/confluence_agent/confluence.py`
- Contains: Page fetch, content update, attachment upload, page ID extraction
- Depends on: atlassian-python-api library
- Used by: Orchestration layer

**LLM Provider Layer:**

- Purpose: Abstracts LLM provider selection and token counting
- Location: `src/confluence_agent/llm.py`, `src/confluence_agent/patched_providers.py`
- Contains: Provider factory, token counting, custom chunked-response handlers
- Depends on: OpenAI, Google Generative AI, tiktoken
- Used by: Orchestration layer for phase execution

**Configuration Layer:**

- Purpose: Loads environment variables and settings for all subsystems
- Location: `src/confluence_agent/config.py`
- Contains: Pydantic Settings extending mcp-agent base configuration
- Depends on: pydantic-settings, environment variables
- Used by: All layers

**Data Models:**

- Purpose: Pydantic models for structured LLM responses
- Location: `src/confluence_agent/models.py`
- Contains: `ConfluenceContent`, `CriticResponse` models
- Used by: LLM phases for response validation

**CLI Interface:**

- Purpose: User-facing command line for manual operations
- Location: `src/confluence_agent/cli.py`
- Contains: `update`, `upload`, `convert` commands via Typer
- Used by: Direct command-line invocation

## Data Flow

**Main Update Flow:**

```text
Markdown file (from user)
    ↓
[CLI/Agent entry point]
    ↓
Load Settings (config.py)
    ↓
Initialize ConfluenceClient (confluence.py)
    ↓
Convert markdown to storage format (converter.py)
    ├─ Strip Obsidian frontmatter
    ├─ Render PlantUML diagrams to SVG
    ├─ Render Mermaid diagrams to SVG
    └─ Produce Confluence storage XML + attachment list
    ↓
Fetch existing page from Confluence (confluence.py)
    ↓
Check if page is empty or has no inline comments
    ├─ YES: Use new content directly
    └─ NO: Run LLM pipeline
        ├─ Phase 1 (Merge): Intelligently merge new with original
        ├─ Phase 2 (Reflect): Validate and correct merged content
        └─ Phase 3 (Critic): Final QA and approval
    ↓
Upload attachments (confluence.py)
    ↓
Update Confluence page with final content (confluence.py)
```

**State Management:**

- Original page content, new converted content, and merged versions flow through LLM phases
- Inline comment markers are extracted as immutable tokens and passed as a section in each prompt
- Token count computed from combined content to scale `max_tokens` parameter (4x content tokens + 1024)
- Version number incremented locally; Confluence API handles conflict detection

## Key Abstractions

**ConfluenceClient:**

- Purpose: Encapsulates Confluence API operations
- Examples: `src/confluence_agent/confluence.py`
- Pattern: Thin wrapper over atlassian-python-api with page ID extraction, attachment handling

**Converter Pipeline:**

- Purpose: Modular markdown-to-storage transformation with optional diagram rendering
- Examples: `process_markdown_puml()`, `process_markdown_mermaid()`, `convert_markdown_to_storage()`
- Pattern: Functional pipeline—each step returns modified markdown + attachments list

**LLM Provider Factory:**

- Purpose: Polymorphic LLM selection and configuration
- Examples: `get_llm_provider()`, `ChunkAwareOpenAIAugmentedLLM`, `ChunkAwareGoogleAugmentedLLM`
- Pattern: Factory function returning provider class; custom classes override chunked response handling

**Structured Output Extraction:**

- Purpose: Robustly parse JSON from LLM responses that may be chunked across multiple parts
- Examples: `src/confluence_agent/structured_output.py`
- Pattern: Provider-specific extractors (Google vs OpenAI shapes) with fallback logic

## Entry Points

**CLI: `update` command:**

- Location: `src/confluence_agent/cli.py::update()`
- Triggers: `confluence-agent update <markdown_path> <page_url>`
- Responsibilities: Read file, invoke async LLM-driven update, report results

**CLI: `upload` command:**

- Location: `src/confluence_agent/cli.py::upload()`
- Triggers: `confluence-agent upload <markdown_path> <page_url>`
- Responsibilities: Convert and upload directly without LLM merge (overwrites page)

**CLI: `convert` command:**

- Location: `src/confluence_agent/cli.py::convert()`
- Triggers: `confluence-agent convert <markdown_path> <output_dir>`
- Responsibilities: Local markdown-to-storage conversion only, no Confluence upload

**MCP Tool: `update_confluence_page`:**

- Location: `src/confluence_agent/agent.py::update_confluence_page()`
- Triggers: MCP server registration via `@app.tool()` decorator
- Responsibilities: Full orchestration (same as CLI update, but accessible via MCP protocol)

## Error Handling

**Strategy:** Exceptions raised at each layer propagate upward; orchestration layer catches and returns error string

**Patterns:**

- Validation errors during structured output parsing → retry with exponential backoff (up to 3 retries)
- Confluence API errors (`ApiError`) → logged and re-raised with context
- LLM provider errors → logged with token/model details
- PlantUML/Mermaid rendering failures → caught, logged, re-raised
- Configuration missing → ValueError at Settings instantiation

## Cross-Cutting Concerns

**Logging:**

- Structured logging via Python `logging` module
- Log level controlled via `LOG_LEVEL` environment variable (default: INFO)
- Token usage monitored and logged for all LLM calls via `TokenMonitor` callback

**Validation:**

- Pydantic models (`ConfluenceContent`, `CriticResponse`) validate LLM JSON responses
- Retry loop in `_generate_structured_with_retry()` handles transient validation failures
- Empty page detection via `_is_content_empty()` avoids unnecessary LLM calls

**Inline Comment Preservation:**

- Markers extracted from original via regex: `<ac:inline-comment-marker .../>` or paired tags
- Formatted as immutable token list and injected into all merge/reflect/critic prompts
- LLM instructed to copy byte-for-byte and verify count matches original
- Actual markers passed verbatim (not XML-escaped) so LLM sees exact bytes to preserve

---

Architecture analysis: 2026-04-10
