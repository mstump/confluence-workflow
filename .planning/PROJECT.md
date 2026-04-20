# Confluence Agent (Rust)

## What This Is

A Rust CLI tool that converts Markdown files to Confluence pages using an intelligent
LLM-driven merge pipeline. It preserves inline Confluence comments through the merge by
evaluating each comment in parallel with a focused context window. Distributed as a
standalone binary and as Claude Code skills.

## Core Value

Merge new Markdown content into an existing Confluence page without destroying inline
comments — everything else is infrastructure around that guarantee.

## Requirements

### Validated

- ✓ Convert Markdown to Confluence storage XML format — existing
- ✓ Intelligently merge new content with existing page (LLM pipeline) — existing
- ✓ Preserve `<ac:inline-comment-marker>` elements byte-for-byte through merge — existing
- ✓ Direct upload mode (overwrite without LLM, no merge) — existing
- ✓ Local convert-only mode (no Confluence upload, output to directory) — existing
- ✓ PlantUML diagram rendering to SVG — existing
- ✓ Mermaid diagram rendering to SVG — existing
- ✓ Strip Obsidian YAML frontmatter before conversion — existing
- ✓ CLI interface with `update`, `upload`, `convert` commands — existing

### Active

- [ ] Full Rust rewrite — replace all Python (CLI, LLM calls, Confluence API, converter)
- [ ] Anthropic (Claude) as primary LLM provider, credentials loaded from `~/.claude/` config or `ANTHROPIC_API_KEY` env var
- [ ] Per-comment parallel evaluation: each inline comment gets its own focused LLM call
      to determine whether it survives the merge (content proximity — does the new content
      still warrant this comment at this location?)
- [ ] Confluence REST API via reqwest + serde (no Python dependency)
- ✓ Standalone binary distribution + Claude Code skills that delegate to it — Validated in Phase 05: distribution-and-claude-code-skills
- [ ] PlantUML configurable as either jar path (`plantuml.jar`) or HTTP server URL

### Out of Scope

- MCP server — dropped; Claude Code skills replace the agent integration use case
- OpenAI / Google Gemini providers at launch — Anthropic only; can be added later
- Backward compatibility with Python CLI flags/config — clean break, users migrate manually
- Python runtime dependency of any kind in the shipped binary

## Context

The existing Python implementation works but has two significant pain points:

1. **Serial LLM pipeline with large context**: The Merge/Reflect/Critic chain processes
   the entire page in each phase. For pages with many inline comments, this is slow and
   the large context degrades merge quality.

2. **MCP server dependency**: The `mcp-agent` framework adds complexity and a server
   process that isn't necessary for the primary use case (CLI invoked from Claude Code).

The Rust rewrite is a clean break — same external behavior (Confluence page updated with
comments preserved), different implementation. The per-comment parallel strategy replaces
the monolithic pipeline: each comment is evaluated independently in a small context,
results assembled before upload.

**Credential loading**: The user wants to load the Anthropic API key from the Claude Code
config file at `~/.claude/` (same credentials the user already has for Claude Code), not
require a separate `ANTHROPIC_API_KEY` setup. This is a key UX improvement.

## Constraints

- **Tech Stack**: Rust only — no Python runtime in the final binary
- **LLM Provider**: Anthropic at launch — use the claude SDK or raw HTTP via reqwest
- **Credentials**: Must read from `~/.claude/` config file (Claude Code credential location)
- **Distribution**: Must work as a standalone `cargo install`-able binary AND be callable
  from Claude Code skills
- **Diagram rendering**: PlantUML support required; configurable as jar or server URL

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Full Rust rewrite, clean break | Performance, single binary distribution, no Python runtime | — In Progress |
| Drop MCP server | Complexity not justified; Claude Code skills cover the use case | — Pending |
| Per-comment parallel evaluation | Smaller context = better LLM results; parallel = faster runtime | — Pending |
| Anthropic-only at launch | User already has Claude Code credentials; simplest path to working | — Pending |
| Credentials from `~/.claude/` config | Reuse existing Claude Code setup, no extra env var needed | — Pending |
| reqwest + serde for Confluence API | No Python dependency; Confluence REST API is stable and well-documented | — Pending |
| PlantUML: jar or server URL configurable | Flexibility without requiring Java in PATH for server users | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):

1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):

1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---

Last updated: 2026-04-10 after initialization
