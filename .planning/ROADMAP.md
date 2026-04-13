# Roadmap: Confluence Agent (Rust)

## Overview

Rewrite the Python Confluence Agent as a standalone Rust binary. The project progresses through five phases following component dependencies: the Confluence API client unblocks everything, the Markdown converter produces the storage XML that downstream phases consume, the per-comment parallel LLM evaluation is the novel architectural core, CLI wiring assembles the full pipeline, and distribution packages the result for cargo install and Claude Code skills.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Project Scaffolding and Confluence API Client** - Cargo workspace, config/credential loading, trait-based Confluence REST client, working `upload` command
- [x] **Phase 2: Markdown-to-Confluence Storage Format Converter** - pulldown-cmark visitor emitting Confluence storage XML, diagram rendering, frontmatter stripping
- [ ] **Phase 3: LLM Client and Comment-Preserving Merge** - Hand-rolled Anthropic client, per-comment parallel evaluation with bounded concurrency, comment re-injection into new content
- [ ] **Phase 4: CLI Command Wiring and Integration** - Wire update/upload/convert commands through the full pipeline, structured logging, JSON output mode
- [ ] **Phase 5: Distribution and Claude Code Skills** - cargo install, Claude Code skill definitions, CI/CD cross-platform builds
- [ ] **Phase 6: Credential Waterfall Fix** - Add --anthropic-api-key CLI flag, wire through CliOverrides, fix broken credential waterfall for Anthropic key (gap closure)
- [ ] **Phase 7: Test Scaffold Completion** - Create missing cli_integration.rs and output_format.rs test stubs, fix config test race condition, remove unused anyhow dependency (gap closure)
- [ ] **Phase 8: DiagramConfig Waterfall and Nyquist Compliance** - Add diagram path CLI flags, integrate DiagramConfig into Config waterfall, achieve Nyquist compliance for Phases 01–03 (gap closure)

## Phase Details

### Phase 1: Project Scaffolding and Confluence API Client
**Goal**: A buildable Rust workspace with credential loading, a trait-based Confluence client, and a working direct-upload path (no LLM) against a real Confluence instance
**Depends on**: Nothing (first phase)
**Requirements**: SCAF-01, SCAF-02, SCAF-03, SCAF-04, SCAF-05, CONF-01, CONF-02, CONF-03, CONF-04, CONF-05
**Success Criteria** (what must be TRUE):
  1. `cargo build` succeeds with zero warnings on a clean checkout
  2. Running the binary with `upload` subcommand against a real Confluence page overwrites that page's content and returns success
  3. Credentials are loaded from `ANTHROPIC_API_KEY` env var or `~/.claude/` config file without requiring both
  4. Confluence API errors (auth failure, 404, 409 version conflict) produce clear, actionable error messages -- not raw HTTP status codes
  5. A mock `ConfluenceApi` trait implementation can be substituted in tests without touching production code
**Plans**: 3 plans

Plans:
- [ ] 01-01: Cargo workspace setup, module layout, error types, clap skeleton with subcommand stubs
- [ ] 01-02: Config and credential loading (waterfall: CLI flag, env var, ~/.claude/ file) with Confluence connection settings
- [ ] 01-03: Confluence REST API client (get page, update page, upload attachment, extract page ID from URL) with trait boundary and retry-on-409

**Flags:**
- VERIFY BEFORE CODING: Inspect the actual `~/.claude/` directory on this machine to determine the credential file format (expected: `credentials.json` with an `oauth.accessToken` or `apiKey` field). The config module design depends on this.
- RISK (TOCTOU): The gap between fetching page version N and updating with N+1 can be 30-90 seconds during LLM processing. A concurrent human edit causes a 409 Conflict. Phase 1 must implement retry-with-re-fetch on 409 from day one (CONF-02). The Python codebase does not handle this.

### Phase 2: Markdown-to-Confluence Storage Format Converter
**Goal**: Markdown files convert to valid Confluence storage XML with code blocks, tables, images, PlantUML/Mermaid diagrams, and frontmatter stripping -- verified against the Python converter's output on real documents
**Depends on**: Phase 1 (uses error types, config for PlantUML settings)
**Requirements**: CONV-01, CONV-02, CONV-03, CONV-04, CONV-05
**Success Criteria** (what must be TRUE):
  1. Running `convert` on a Markdown file containing headings, code blocks, tables, links, and images produces Confluence storage XML that renders correctly when pasted into the Confluence editor
  2. PlantUML fenced blocks are rendered to SVG files (via configurable jar path or HTTP server URL) and referenced as `ac:image` attachments in the output
  3. Mermaid fenced blocks are rendered to SVG via mermaid-cli
  4. Obsidian YAML frontmatter is stripped before conversion without affecting document content
  5. A mock `Converter` trait implementation can be substituted in tests
**Plans**: 3 plans

Plans:
- [x] 02-01: Converter spike -- attempt pulldown-cmark visitor for 5 complex test documents, compare output against Python converter; decide approach (native Rust vs fallback)
- [x] 02-02: Markdown-to-storage-XML converter (pulldown-cmark visitor emitting ac:structured-macro, ac:image, ri:attachment elements) with frontmatter stripping
- [x] 02-03: Diagram rendering -- PlantUML (jar or HTTP server, configurable) and Mermaid (mermaid-cli) via tokio::process::Command

**Flags:**
- SPIKE REQUIRED: No Rust crate converts Markdown to Confluence storage format. Plan 02-01 is a spike to determine feasibility of a custom pulldown-cmark visitor. If the spike fails (cannot handle code blocks, tables, images within one week), the fallback is to bridge the Python converter as a subprocess and document the debt. The spike outcome determines the scope of 02-02.
- RISK (XML namespaces): Confluence storage format uses `ac:` and `ri:` namespace prefixes that are not declared in the fragment. quick-xml will reject them unless fragments are wrapped in a synthetic root with namespace declarations. Establish the wrap/unwrap pattern in this phase.

### Phase 3: LLM Client and Comment-Preserving Merge
**Goal**: Per-comment parallel LLM evaluation determines which inline comments survive a content merge, with deterministic short-circuits for trivial cases and bounded concurrency for LLM calls
**Depends on**: Phase 1 (Confluence client for fetching existing page), Phase 2 (converter for producing new content)
**Requirements**: LLM-01, LLM-02, LLM-03, LLM-04, MERGE-01, MERGE-02, MERGE-03, MERGE-04, MERGE-05, MERGE-06
**Success Criteria** (what must be TRUE):
  1. Given a page with inline comments where the underlying text is unchanged, all comments are preserved with zero LLM calls (deterministic short-circuit)
  2. Given a page with inline comments where a section was deleted, comments in that section are dropped with zero LLM calls
  3. Given a page with inline comments in changed sections, each ambiguous comment triggers exactly one focused LLM call that returns KEEP or DROP
  4. Parallel LLM evaluations are bounded by a configurable concurrency limit (default 5) using a tokio semaphore -- not unbounded fan-out
  5. If an individual comment evaluation fails (timeout, rate limit, malformed response), that comment defaults to KEEP and a warning is logged -- the overall update proceeds
**Plans**: 3 plans

Plans:
- [x] 03-01: XML comment extraction and section context -- extract ac:inline-comment-marker elements with surrounding section, match sections between old and new content
- [x] 03-02: Hand-rolled Anthropic Messages API client -- reqwest-based, tool_use for structured KEEP/DROP output, retry with exponential backoff on 429/5xx, LlmClient trait
- [x] 03-03: Per-comment parallel merge engine -- deterministic short-circuits, bounded parallel LLM fan-out, comment re-injection into new content XML, partial failure handling

**Flags:**
- RISK (token cost): N comments x ~2,500 tokens each = potentially 125K tokens for a 50-comment page. The deterministic short-circuit in MERGE-02 is critical to keep costs reasonable -- most real-world comments will be in unchanged sections. The semaphore bound (MERGE-04) also prevents rate-limit thundering herd. Consider adding a cost warning threshold for pages with >20 ambiguous comments.
- RISK (rate limits): Anthropic enforces both requests-per-minute and tokens-per-minute limits. The semaphore bounds concurrent requests, but exponential backoff with jitter on 429 responses is also required (LLM-03). Read the `retry-after` header.

### Phase 4: CLI Command Wiring and Integration
**Goal**: All three CLI commands (update, upload, convert) work end-to-end through the full pipeline with structured logging and machine-readable JSON output for Claude Code skill integration
**Depends on**: Phase 1 (Confluence client), Phase 2 (converter), Phase 3 (merge engine)
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04, CLI-05
**Success Criteria** (what must be TRUE):
  1. `confluence-agent update doc.md <page_url>` converts the markdown, fetches the existing page, runs per-comment merge, uploads attachments, and updates the page -- end to end
  2. `confluence-agent upload doc.md <page_url>` converts and overwrites the page without any LLM calls
  3. `confluence-agent convert doc.md ./output` writes storage XML and diagram SVGs to the output directory without requiring Confluence credentials
  4. `--verbose` flag produces structured tracing output showing each pipeline step; default output is minimal (success/failure + page URL)
  5. `--output json` produces machine-readable JSON on stdout with fields for success, page URL, comments kept/dropped, and errors
**Plans**: 2 plans

Plans:
- [x] 04-01-PLAN.md -- Wire all three commands (update, upload, convert) through the full pipeline; add OutputFormat enum to cli.rs
- [x] 04-02-PLAN.md -- Output formatting (JSON + human), tracing subscriber init, exit codes

### Phase 5: Distribution and Claude Code Skills
**Goal**: The binary is installable via cargo install, callable from Claude Code skills, and built automatically for macOS and Linux via CI/CD
**Depends on**: Phase 4 (stable, working binary)
**Requirements**: DIST-01, DIST-02, DIST-03, DIST-04
**Success Criteria** (what must be TRUE):
  1. `cargo install confluence-agent` from crates.io (or git) produces a working binary
  2. A Claude Code skill at `~/.claude/commands/confluence-update.md` invokes the binary and surfaces its JSON output to the user
  3. GitHub Actions CI produces release binaries for macOS arm64, macOS x86_64, and Linux x86_64 on tagged commits
  4. Stripped release binary is under 15 MB
**Plans**: 3 plans

Plans:
- [ ] 05-01: Cargo packaging -- Cargo.toml metadata for crates.io, release profile (LTO, strip, panic=abort), verify cargo install works
- [ ] 05-02: Claude Code skills -- confluence-update.md and confluence-upload.md skill definitions that invoke the binary with --output json
- [ ] 05-03: CI/CD -- GitHub Actions workflow for cross-platform builds (cargo-zigbuild or cross), release artifact upload on tag

### Phase 6: Credential Waterfall Fix
**Goal**: The `--anthropic-api-key` CLI flag is wired end-to-end so the CLI tier of the credential waterfall (CLI > env > .env > ~/.claude/) is functional for the Anthropic API key, satisfying SCAF-02 and SCAF-03
**Depends on**: Phase 4 (CLI and CliOverrides already defined)
**Requirements**: SCAF-02, SCAF-03
**Gap Closure:** Closes gaps from v1.0 audit
**Success Criteria** (what must be TRUE):
  1. `confluence-agent --anthropic-api-key sk-xxx update doc.md <url>` passes the key through to the LLM client without requiring `ANTHROPIC_API_KEY` in the environment
  2. `CliOverrides.anthropic_api_key` is `Some(key)` when the flag is provided, `None` when omitted — both update and upload arms in lib.rs
  3. `test_update_command_missing_api_key` asserts on the correct error path (missing API key, not HTTPS guard)
  4. Credential waterfall precedence for ANTHROPIC_API_KEY is: CLI flag > ANTHROPIC_API_KEY env var > .env file > ~/.claude/ config
**Plans**: 1 plan

Plans:
- [x] 06-01: Add --anthropic-api-key flag to cli.rs, wire through CliOverrides in lib.rs (update + upload arms), fix test_update_command_missing_api_key

### Phase 7: Test Scaffold Completion
**Goal**: All test stubs specified in Phase 4 plans exist and pass; Phase 1 config tests pass reliably under parallel `cargo test`; unused dependencies removed
**Depends on**: Phase 6 (credential fix may affect some test scenarios)
**Requirements**: CLI-01 (test coverage), CLI-02 (test coverage), CLI-03 (test coverage), CLI-04 (test coverage), CLI-05 (test coverage)
**Gap Closure:** Closes gaps from v1.0 audit
**Success Criteria** (what must be TRUE):
  1. `tests/cli_integration.rs` exists with passing implementations of test_update_command, test_upload_command, test_convert_command, test_json_output_mode, test_stderr_routing
  2. `tests/output_format.rs` exists with a passing test_default_silent_mode test
  3. `cargo test` passes with default parallelism (no `--test-threads=1` workaround needed for config tests)
  4. `anyhow` removed from `Cargo.toml`; `cargo build` still clean
**Plans**: 2 plans

Plans:
- [ ] 07-01: Create tests/cli_integration.rs and tests/output_format.rs with full test implementations
- [ ] 07-02: Fix src/config.rs parallel test race condition (serial_test crate or env isolation); remove unused anyhow dependency

### Phase 8: DiagramConfig Waterfall and Nyquist Compliance
**Goal**: DiagramConfig respects the same CLI > env > config waterfall as credentials; Phases 01–03 achieve Nyquist compliance with proper VALIDATION.md frontmatter
**Depends on**: Phase 6 (waterfall pattern established)
**Requirements**: SCAF-03 (waterfall consistency for diagram paths)
**Gap Closure:** Closes gaps from v1.0 audit
**Success Criteria** (what must be TRUE):
  1. `--plantuml-path` and `--mermaid-path` CLI flags override env vars `PLANTUML_PATH` / `MERMAID_PATH` when provided
  2. DiagramConfig is part of `Config::load()` waterfall, not loaded independently via `from_env()`
  3. Phases 01, 02, 03 each have a VALIDATION.md with `nyquist_compliant: true` and `wave_0_complete: true` frontmatter
**Plans**: 2 plans

Plans:
- [ ] 08-01: Add --plantuml-path and --mermaid-path CLI flags; integrate DiagramConfig into Config waterfall
- [ ] 08-02: Run /gsd-validate-phase for Phases 01, 02, 03 to achieve Nyquist compliance

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Scaffolding and Confluence API Client | 0/3 | Not started | - |
| 2. Markdown-to-Confluence Converter | 0/3 | Not started | - |
| 3. LLM Client and Comment-Preserving Merge | 0/3 | Not started | - |
| 4. CLI Command Wiring and Integration | 0/2 | Not started | - |
| 5. Distribution and Claude Code Skills | 0/3 | Not started | - |
| 6. Credential Waterfall Fix | 0/1 | Not started | - |
| 7. Test Scaffold Completion | 0/2 | Not started | - |
| 8. DiagramConfig Waterfall and Nyquist Compliance | 0/2 | Not started | - |
