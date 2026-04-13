# Requirements: Confluence Agent (Rust)

**Defined:** 2026-04-10
**Core Value:** Merge new Markdown content into an existing Confluence page without destroying inline comments

## v1 Requirements

### Project Scaffolding

- [ ] **SCAF-01**: Rust workspace builds cleanly with `cargo build`
- [ ] **SCAF-02**: CLI binary accepts `update`, `upload`, `convert` subcommands via clap; `--anthropic-api-key` flag supported
- [ ] **SCAF-03**: Credentials loaded via waterfall: CLI flag → env var (`ANTHROPIC_API_KEY`) → `~/.claude/` config file; CLI flag functional for all credentials including Anthropic API key
- [ ] **SCAF-04**: Configuration supports Confluence base URL, API token, username, and space key
- [ ] **SCAF-05**: Structured error types with `thiserror`; user-facing errors have clear messages

### Confluence API Client

- [ ] **CONF-01**: Fetch existing page content (storage XML) and version number via REST API v1
- [ ] **CONF-02**: Update page content with incremented version number (conflict detection via version field)
- [ ] **CONF-03**: Upload SVG attachments to a page
- [ ] **CONF-04**: Extract page ID from a Confluence page URL
- [ ] **CONF-05**: Client is trait-based (`ConfluenceApi` trait) for testability

### Markdown Conversion

- [ ] **CONV-01**: Convert Markdown to Confluence storage XML format (Confluence XHTML with `ac:*` elements)
- [ ] **CONV-02**: Strip Obsidian YAML frontmatter before conversion
- [ ] **CONV-03**: Render PlantUML diagrams to SVG — configurable as JAR path or HTTP server URL
- [ ] **CONV-04**: Render Mermaid diagrams to SVG via mermaid-cli
- [ ] **CONV-05**: Converter is trait-based for testability

### Comment-Preserving Merge

- [ ] **MERGE-01**: Extract all `<ac:inline-comment-marker>` elements from existing page with their surrounding section context
- [ ] **MERGE-02**: Deterministic short-circuits: comment in unchanged section → KEEP; comment section deleted → DROP (no LLM call)
- [ ] **MERGE-03**: Per-comment LLM evaluation for ambiguous cases: given old section + new section, classify KEEP or DROP
- [ ] **MERGE-04**: Comment evaluations run in parallel (bounded concurrency via tokio semaphore)
- [ ] **MERGE-05**: Surviving comment markers injected back into new content XML at correct locations
- [ ] **MERGE-06**: Empty page or page with no comments → skip merge, use new content directly

### LLM Client

- [ ] **LLM-01**: Hand-rolled Anthropic Messages API client over reqwest (no Python dependency)
- [ ] **LLM-02**: Structured output via Claude tool_use (function calling) for KEEP/DROP classifications
- [ ] **LLM-03**: Retry with exponential backoff on rate limit (429) and transient errors
- [ ] **LLM-04**: LLM client is trait-based (`LlmClient` trait) for testability

### CLI Commands

- [ ] **CLI-01**: `update <markdown_path> <page_url>` — full merge pipeline (convert → fetch → merge → upload)
- [ ] **CLI-02**: `upload <markdown_path> <page_url>` — direct overwrite without LLM merge
- [ ] **CLI-03**: `convert <markdown_path> <output_dir>` — local conversion only, no Confluence upload
- [ ] **CLI-04**: `--verbose` flag for debug output; structured logs via `tracing`
- [ ] **CLI-05**: JSON output mode (`--output json`) for machine-readable results (Claude Code skill integration)

### Distribution

- [ ] **DIST-01**: Binary installable via `cargo install confluence-agent`
- [ ] **DIST-02**: Claude Code skill at `~/.claude/commands/confluence-update.md` that invokes the binary
- [ ] **DIST-03**: CI/CD builds release binaries for macOS (arm64, x86_64) and Linux (x86_64)
- [ ] **DIST-04**: Binary size under 15 MB stripped

## v2 Requirements

### Additional LLM Providers

- **LLM2-01**: OpenAI provider support (GPT models)
- **LLM2-02**: Google Gemini provider support
- **LLM2-03**: Provider selection via config or CLI flag

### Advanced Comment Handling

- **MERGE2-01**: RELOCATE support — move surviving comment to closest semantically equivalent location in new content
- **MERGE2-02**: Fuzzy anchor text matching for reassembly (Levenshtein distance threshold)
- **MERGE2-03**: User-visible report of dropped comments (orphaned comment log)
- **MERGE2-04**: Batch comment evaluation (group 5–10 comments per LLM call to reduce API calls)

### Docker / Container Support

- **DIST2-01**: Dockerfile with Rust binary, Java JRE (PlantUML), Node.js (mermaid-cli)
- **DIST2-02**: Published to GitHub Container Registry

## Out of Scope

| Feature | Reason |
|---------|--------|
| MCP server | Dropped; Claude Code skills replace the agent integration use case |
| Python runtime dependency | Clean break rewrite; no subprocess bridge to Python |
| Backward compatibility with Python CLI | Clean break; users migrate manually |
| OpenAI / Google providers at launch | Anthropic-only initially; v2 can add more |
| Real-time streaming LLM output | Not needed for non-interactive CLI; non-streaming is simpler |
| Confluence Server (on-prem) | Cloud REST API v1 only for now |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SCAF-01 | Phase 1 | Pending |
| SCAF-02 | Phase 6 | Pending |
| SCAF-03 | Phase 6 | Pending |
| SCAF-04 | Phase 1 | Pending |
| SCAF-05 | Phase 1 | Pending |
| CONF-01 | Phase 1 | Pending |
| CONF-02 | Phase 1 | Pending |
| CONF-03 | Phase 1 | Pending |
| CONF-04 | Phase 1 | Pending |
| CONF-05 | Phase 1 | Pending |
| CONV-01 | Phase 2 | Pending |
| CONV-02 | Phase 2 | Pending |
| CONV-03 | Phase 2 | Pending |
| CONV-04 | Phase 2 | Pending |
| CONV-05 | Phase 2 | Pending |
| LLM-01 | Phase 3 | Pending |
| LLM-02 | Phase 3 | Pending |
| LLM-03 | Phase 3 | Pending |
| LLM-04 | Phase 3 | Pending |
| MERGE-01 | Phase 3 | Pending |
| MERGE-02 | Phase 3 | Pending |
| MERGE-03 | Phase 3 | Pending |
| MERGE-04 | Phase 3 | Pending |
| MERGE-05 | Phase 3 | Pending |
| MERGE-06 | Phase 3 | Pending |
| CLI-01 | Phase 4 | Pending |
| CLI-02 | Phase 4 | Pending |
| CLI-03 | Phase 4 | Pending |
| CLI-04 | Phase 4 | Pending |
| CLI-05 | Phase 4 | Pending |
| DIST-01 | Phase 5 | Pending |
| DIST-02 | Phase 5 | Pending |
| DIST-03 | Phase 5 | Pending |
| DIST-04 | Phase 5 | Pending |

**Coverage:**

- v1 requirements: 34 total
- Mapped to phases: 34
- Unmapped: 0 ✓

---

Requirements defined: 2026-04-10
Last updated: 2026-04-10 after initial definition
