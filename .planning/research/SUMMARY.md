# Project Research Summary

**Project:** Confluence Agent (Rust rewrite)
**Domain:** Rust CLI tool for Markdown-to-Confluence conversion with per-comment parallel LLM evaluation
**Researched:** 2026-04-10
**Confidence:** MEDIUM (no live web access during research; critical gaps noted)

## Executive Summary

This project is a Rust rewrite of a Python CLI tool that converts Markdown files to Confluence pages, with a new architectural goal: replace the current monolithic 3-phase LLM pipeline (Merge → Reflect → Critic) with a focused per-comment parallel evaluation approach. The new design is authoritative about content — new Markdown is always the source of truth — and uses the LLM only for the semantic binary question of whether each inline comment is still relevant after the content change. This dramatically reduces token consumption (from 30K–300K tokens per update to 5K–25K) and latency (from 15–60 seconds serial to 1–5 seconds parallel), while producing more intelligent per-comment decisions instead of brute-force preservation of all markers.

The recommended stack is conventional Rust async: tokio + reqwest + serde as the foundation, clap for CLI, quick-xml for Confluence storage format manipulation, and pulldown-cmark for Markdown parsing. No official Anthropic Rust SDK exists; a hand-rolled thin HTTP client over reqwest is the correct approach. The architecture should expose everything through traits to allow mocking in tests, keep `main.rs` as a thin shell, and use `futures::stream::buffer_unordered` with a semaphore for bounded parallel LLM calls.

The two highest-risk areas are: (1) the Markdown-to-Confluence storage format converter, where no Rust equivalent of `md2conf` exists and a custom pulldown-cmark visitor must be written or the Python converter bridged as a subprocess; and (2) the XML manipulation layer, where Confluence storage format is not valid standalone XML and requires namespace-wrapping before parsing. A phased approach — Confluence API client first, then converter, then per-comment LLM evaluation, then Claude Code skill integration — minimizes integration risk and ensures each component can be tested independently before the full pipeline is assembled.

## Key Findings

### Recommended Stack

The Rust ecosystem provides strong, opinionated choices for every layer of this tool. tokio is the only viable async runtime (reqwest depends on it), clap with derive macros is the dominant CLI framework, and serde + serde_json are the uncontested serialization standards. The only area of meaningful uncertainty is the Anthropic API client: no official Rust SDK exists, and community crates (`anthropic-rs`, `misanthropic`) carry bus-factor risk. Hand-rolling a thin reqwest-based client is ~150 lines and gives full control over streaming, retry, and structured output via tool use.

For XML, `quick-xml` is the correct choice — it handles both read and write with namespace support, unlike `roxmltree` (read-only) or `xml-rs` (less maintained). Structured output from Claude should use the tool_use API (function calling), not prompt-based JSON extraction, with serde-based deserialization and retry-on-failure matching the existing Python `_generate_structured_with_retry` pattern.

**Core technologies:**
- `tokio` (1.x): Async runtime — required by reqwest; `JoinSet` + `Semaphore` for parallel LLM fan-out
- `reqwest` (0.12, rustls-tls): HTTP client — used for both Anthropic API and Confluence REST API; single shared client instance
- `serde` + `serde_json` (1.x): Serialization — all JSON, API types, structured LLM output
- `clap` (4.x, derive): CLI framework — subcommands (`update`, `upload`, `convert`), env var fallbacks, help generation
- `pulldown-cmark` (0.11+): Markdown parsing — CommonMark standard; requires custom Confluence storage format visitor
- `quick-xml` (0.36+): XML read/write — Confluence storage format manipulation; namespace support required
- `thiserror` (2.x) + `anyhow` (1.x): Error handling — typed errors in library code, anyhow at CLI boundary
- `tracing` + `tracing-subscriber`: Logging — async-aware structured logging, RUST_LOG control
- `dirs` (5.x): Home directory resolution — cross-platform `~/.claude/` path lookup
- `futures` (0.3): Async stream utilities — `buffer_unordered` for bounded parallel comment evaluation

See `.planning/research/STACK.md` for full dependency list, dev dependencies, and alternative analysis.

### Expected Features

The core architectural shift replaces "preserve all comments as immutable tokens" with "evaluate each comment independently in parallel." The feature set is well-defined:

**Must have (table stakes):**
- Per-comment context extraction — isolate each `ac:inline-comment-marker` with its surrounding section for focused evaluation
- Deterministic short-circuit evaluation — exact-match KEEP, deleted-section DROP, unchanged-section KEEP with zero LLM calls
- Parallel LLM evaluation — fan out one call per ambiguous comment using `futures::buffer_unordered` with semaphore
- Document reassembly from verdicts — re-inject surviving markers into the new content using XML tree manipulation, not LLM generation
- Three CLI commands preserved: `update` (full pipeline), `upload` (direct, no LLM), `convert` (local only)
- Credential loading from `~/.claude/` files with env var override

**Should have (differentiators):**
- KEEP/DROP/RELOCATE verdict model — RELOCATE requires LLM to supply a new anchor text for repositioned comments
- Orphaned comment report — log which comments were dropped and why, rather than silent removal
- Cost estimator — calculate expected token usage before execution, prompt for confirmation on large pages
- Semaphore-bounded concurrency with configurable limit (default 5–10 parallel calls)
- JSON output mode for machine-readable results (Claude Code skill integration)

**Defer to v2+:**
- RELOCATE verdict support — hardest reassembly case; start with KEEP/DROP only
- SSE streaming for LLM responses — non-streaming is simpler and sufficient for batch processing
- Comment thread resolution — out of scope; only inline markers are in scope
- Automatic comment creation based on content changes — explicitly anti-feature

See `.planning/research/FEATURES.md` for full feature dependency graph and prior art analysis (Google Docs OT, CKEditor track changes).

### Architecture Approach

The architecture follows the established Rust pattern of a thin binary over a testable library. All orchestration, conversion, and LLM logic lives in `lib.rs` submodules exposed through async traits (`ConfluenceApi`, `LlmClient`, `DiagramRenderer`). The pipeline uses a builder pattern for the `UpdatePipeline` struct, taking generic trait implementations so tests can inject mocks. The `main.rs` entry point is 3–5 lines: parse args, build runtime, call `confluence_agent::run()`.

**Major components:**
1. `cli/` — clap argument parsing and command dispatch; no business logic
2. `config/` — credential waterfall loader: CLI flag → `ANTHROPIC_API_KEY` env → `~/.claude/credentials.json` → error
3. `confluence/` — reqwest-based REST API client (GET page, PUT update, POST attachment); typed serde structs for v1 API
4. `converter/` — Markdown-to-storage-XML pipeline: frontmatter strip → PlantUML render → pulldown-cmark → custom Confluence XML visitor
5. `llm/` — Anthropic Messages API hand-rolled client; tool_use for structured output; per-comment parallel evaluation via `buffer_unordered`
6. `xml/` — `quick-xml` event-based parser/writer for inline comment extraction, namespace-wrapped fragment handling, and comment re-injection
7. `error.rs` — unified `AppError` enum with `thiserror`, one variant per module error type

See `.planning/research/ARCHITECTURE.md` for Confluence REST API endpoint specifications, full reqwest client code, and testability patterns.

### Critical Pitfalls

1. **Confluence version conflict (TOCTOU race)** — The gap between page fetch and page update can be 30–90 seconds when LLM calls are involved. A concurrent edit returns 409. Prevention: implement retry-with-re-fetch on 409; add `--force` flag for explicit override. This is not currently handled in the Python code.

2. **Confluence storage format is not valid XML** — The `ac:` and `ri:` namespace prefixes are undefined in storage fragments. `quick-xml` will reject them without pre-wrapping in a synthetic root with namespace declarations. Prevention: always wrap/unwrap when parsing; write roundtrip byte-equality tests against real Confluence exports.

3. **No Rust `markdown-to-confluence` equivalent** — There is no crate that converts Markdown to Confluence storage format. `pulldown-cmark` outputs HTML; Confluence needs `ac:structured-macro`, `ac:image`, `ri:attachment` elements. Prevention: budget 2–3 weeks for a custom visitor, OR bridge the Python converter as a subprocess for Phase 1 and port it later.

4. **Token cost explosion with fan-out** — 50 comments × 2,500 tokens each = 125K tokens per update. Prevention: batch comments into groups of 5–10 per LLM call; add a configurable threshold above which the user is warned; implement short-circuit evaluation to eliminate LLM calls for trivial cases.

5. **Structured output fragility without an SDK** — Claude's tool_use response comes as a `content` block with `type: "tool_use"`, not `type: "text"`. Missing this causes silent data loss. Prevention: implement typed response parsing with `serde`, `#[serde(deny_unknown_fields)]`, and 3-attempt retry on deserialization failure — matching the Python `_generate_structured_with_retry` pattern.

See `.planning/research/PITFALLS.md` for 15 pitfalls total, including async translation hazards, rate limit thundering herd, cross-compilation issues, and partial failure handling.

## Implications for Roadmap

Based on combined research, the recommended phase structure follows component dependencies: the Confluence API client unblocks everything; the converter is the largest implementation risk and should be resolved early; the per-comment LLM evaluation is the novel architectural work; and Claude Code skill integration is a thin wrapper requiring a stable binary first.

### Phase 1: Confluence API Client and Project Scaffolding

**Rationale:** The Confluence API client has no crate equivalent and must be written from scratch. It is a dependency of every subsequent phase. Establishing the project structure (workspace, module layout, error types, trait boundaries) here prevents costly refactoring later. Getting the Confluence API right — including version increment, attachment upload with `X-Atlassian-Token: nocheck`, and Basic Auth — is the highest-confidence work in the project.

**Delivers:** Working `upload` command (direct overwrite, no LLM) against a real Confluence instance; Cargo workspace with all production dependencies pinned; `ConfluenceApi` trait with mock implementation for testing; credential loading from `~/.claude/` and env vars.

**Addresses:** FEATURES — `upload` command, credential loading
**Avoids:** Pitfall 1 (version conflict — build retry logic from day one), Pitfall 7 (attachment upload quirks — X-Atlassian-Token header, multipart encoding), Pitfall 11 (cross-compilation — configure rustls and release profile immediately)

**Research flag:** SKIP — Confluence REST API v1 is well-documented and stable. Credential file format needs local verification (inspect `~/.claude/` before coding).

### Phase 2: Markdown-to-Confluence Storage Format Converter

**Rationale:** The converter is the largest implementation uncertainty. No Rust crate exists for this. The decision between writing a custom pulldown-cmark visitor versus bridging the Python converter as a subprocess determines the scope of Phase 2. A spike (attempting conversion of the 5 most complex test Markdown files) should happen at the start of this phase to validate the approach before committing.

**Delivers:** Working `convert` command; Markdown → Confluence storage XML with code blocks, images, tables, frontmatter stripping, and PlantUML rendering via `tokio::process::Command`.

**Uses:** `pulldown-cmark`, `quick-xml`, `tokio::process` for PlantUML
**Avoids:** Pitfall 5 (no md2conf equivalent — commit to approach early), Pitfall 2 (storage format namespace handling — establish the wrap/unwrap pattern here)

**Research flag:** NEEDS RESEARCH — Start this phase with a spike. If the custom visitor approach is feasible in < 1 week, proceed in Rust. If not, bridge Python converter via subprocess and document the debt. The spike output determines scope.

### Phase 3: Per-Comment Parallel LLM Evaluation

**Rationale:** This is the novel architectural core of the rewrite. The Anthropic API client (hand-rolled), structured output via tool_use, and `buffer_unordered` parallel evaluation all come together here. The deterministic short-circuit cases (exact match → KEEP, deleted section → DROP) should be built and validated before the LLM-based evaluation path, since they handle the majority of real-world cases at zero cost.

**Delivers:** Working `update` command with per-comment evaluation; `LlmClient` trait with Anthropic implementation; short-circuit evaluation (no LLM); LLM-based KEEP/DROP evaluation for ambiguous cases; comment re-injection into new content via `quick-xml` write path; orphaned comment log.

**Uses:** `reqwest` (Anthropic client), `serde` (tool_use structured output), `tokio::sync::Semaphore` + `futures::buffer_unordered` (bounded parallel calls)
**Avoids:** Pitfall 4 (SSE complexity — use non-streaming), Pitfall 8 (token cost — batch + short-circuit), Pitfall 9 (rate limit thundering herd — semaphore + exponential backoff), Pitfall 10 (partial failures — default to KEEP, abort if > 30% fail), Pitfall 12 (structured output fragility — tool_use + retry)

**Research flag:** NEEDS RESEARCH — The `~/.claude/credentials.json` exact format must be verified on a real installation before coding the credential reader. The Anthropic tool_use request/response schema should be verified against current API docs before implementing the hand-rolled client.

### Phase 4: Claude Code Skill Integration and Distribution

**Rationale:** Once the binary works correctly from the CLI, wrapping it as a Claude Code skill is straightforward. This phase adds JSON output mode, `.claude/commands/` skill definitions, and distribution artifacts (GitHub Releases, shell installer). The binary design already accommodates this (stdout/stderr, exit codes, no interactive prompts).

**Delivers:** `.claude/commands/confluence-update.md` and `.claude/commands/confluence-upload.md` skills; `--output-format json` flag; GitHub Actions CI with cross-platform builds (macOS aarch64, macOS x86_64, Linux x86_64-musl); stripped release binary (4–6 MB).

**Uses:** `clap` output format flag, `cargo-zigbuild` for cross-compilation
**Avoids:** Pitfall 11 (cross-compilation — rustls + zigbuild configured in Phase 1)

**Research flag:** SKIP — Claude Code skill format is well-documented; cross-compilation patterns are established.

### Phase Ordering Rationale

- Phase 1 before all others: the Confluence API client is a hard dependency of every integration test in subsequent phases.
- Phase 2 before Phase 3: the converter produces the storage XML that the LLM evaluation layer consumes; Phase 3 cannot be end-to-end tested without it.
- Phase 3 is deliberately self-contained: the `update` command sits on top of Phase 1 (Confluence API) and Phase 2 (converter); parallelism and LLM logic are isolated in the `llm/` module.
- Phase 4 last: it adds no new domain logic; it only requires a stable, correct binary.
- RELOCATE verdict support is deferred past Phase 3: KEEP/DROP covers the majority of cases and RELOCATE requires the hardest reassembly path (finding new anchor positions after text rewriting).

### Research Flags

Phases needing deeper research during planning:
- **Phase 2 (Converter):** Spike required to determine feasibility of custom pulldown-cmark Confluence visitor vs. Python subprocess bridge. Outcome determines scope by 1–3 weeks.
- **Phase 3 (LLM Evaluation):** Verify `~/.claude/credentials.json` format on real installation before coding config module. Verify Anthropic tool_use schema against current API docs.

Phases with standard patterns (safe to skip per-phase research):
- **Phase 1 (Confluence API):** REST API v1 is stable and well-documented. Reqwest patterns are established.
- **Phase 4 (Distribution):** Cross-compilation and Claude Code skill patterns are established.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Core crates (tokio, reqwest, serde, clap, quick-xml) are undisputed ecosystem standards. No official Anthropic Rust SDK confirmed. Version numbers need crates.io verification. |
| Features | HIGH | Feature set derived directly from codebase analysis of existing Python implementation plus well-defined architectural goals. Per-comment evaluation design is well-specified. |
| Architecture | HIGH | Module layout, trait boundaries, Confluence REST API endpoints, and XML handling patterns are all established Rust practices. Only gap is `~/.claude/` credential file format. |
| Pitfalls | MEDIUM | Project-specific pitfalls (XML namespace handling, converter gap, version conflict) derived from codebase analysis — HIGH confidence. Ecosystem claims (rate limits, cross-compilation) from training data — MEDIUM. |

**Overall confidence:** MEDIUM-HIGH. The project has a clear domain (well-understood Confluence API, well-specified LLM task), strong stack choices, and identifiable risks. The two implementation unknowns (converter approach, credential file format) are resolvable early in execution.

### Gaps to Address

- **`~/.claude/credentials.json` exact format:** Must be inspected on a real installation before coding the config module in Phase 1. Try `~/.claude/credentials.json` → look for `$.oauth.accessToken`, `$.apiKey`, or `$.claudeApiKey`. Fall back to `ANTHROPIC_API_KEY` env var.
- **Markdown-to-Confluence converter approach:** Requires a spike in Phase 2. The decision (custom Rust visitor vs. Python subprocess bridge) materially affects Phase 2 scope. Budget 2–3 days for the spike before estimating Phase 2.
- **Anthropic tool_use schema for current API version:** The hand-rolled client must match the current `tool_use` content block structure exactly. Verify against `https://docs.anthropic.com/en/api/messages` before implementing.
- **Crate version accuracy:** All version numbers in STACK.md are from training data (mid-2025 cutoff). Verify on crates.io before pinning: pulldown-cmark, quick-xml, clap 4.x, reqwest 0.12.x, dirs, base64.
- **RELOCATE verdict scope:** Deferred from Phase 3 MVP. When added, it requires the hardest XML path (finding new anchor positions after text rewriting). Design the verdict enum to accommodate RELOCATE from Phase 3 even if the handler is a stub.

## Sources

### Primary (HIGH confidence)
- Existing Python implementation — direct codebase analysis of `agent.py`, `cli.py`, `confluence.py`, `converter.py`, `llm.py`, `llm_prompts.py`, `models.py`
- Confluence REST API v1 documentation — `https://developer.atlassian.com/cloud/confluence/rest/v1/` (endpoints verified stable)
- quick-xml crate documentation — `https://docs.rs/quick-xml/`

### Secondary (MEDIUM confidence)
- Training data (cutoff ~mid-2025) — Rust ecosystem crate landscape, tokio/reqwest patterns, async architecture best practices
- Claude Code documentation — `https://docs.anthropic.com/en/docs/claude-code` — credential file location and skill format
- Anthropic API documentation — `https://docs.anthropic.com/en/api/messages` — Messages API structure, tool_use format, SSE event schema

### Tertiary (LOW confidence)
- Community crate assessments (`anthropic-rs`, `misanthropic`, `clust`) — status as of mid-2025; verify before dismissing
- Model pricing and latency estimates — approximate as of early 2025; costs may have changed
- `~/.claude/` credential file JSON schema — inferred from training data; must be verified against real installation

---
*Research completed: 2026-04-10*
*Ready for roadmap: yes*
