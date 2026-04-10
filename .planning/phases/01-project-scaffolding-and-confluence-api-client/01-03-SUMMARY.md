---
phase: 01-project-scaffolding-and-confluence-api-client
plan: 03
subsystem: confluence-api
tags: [rust, reqwest, wiremock, async-trait, base64, regex, tdd]

# Dependency graph
requires:
  - 01-01 (ConfluenceError, AppError, CLI structure)
  - 01-02 (Config::load, CliOverrides)
provides:
  - ConfluenceApi trait with get_page, update_page, upload_attachment
  - ConfluenceClient implementing ConfluenceApi via reqwest
  - update_page_with_retry with TOCTOU-safe re-fetch semantics
  - extract_page_id for /pages/ID, /pages/edit-v2/ID, and pageId=ID URL patterns
  - Page, PageBody, StorageRepresentation, PageVersion serde structs
  - upload command wired end-to-end in lib.rs
affects:
  - Phase 2 (converter will call upload_attachment for SVG attachments)
  - Phase 3 (update command will use ConfluenceApi trait via mock in tests)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ConfluenceApi trait with async_trait enables mock substitution in all future tests"
    - "OnceLock<Regex> for zero-overhead compiled regex reuse across calls"
    - "wiremock MockServer for integration tests: up_to_n_times(1) controls response sequencing"
    - "Retry pattern: re-fetch then retry on 409 (TOCTOU mitigation for version conflicts)"
    - "X-Atlassian-Token: nocheck header required for all attachment uploads"

key-files:
  created:
    - src/confluence/mod.rs
    - src/confluence/types.rs
    - src/confluence/url.rs
    - src/confluence/client.rs
  modified:
    - src/lib.rs

key-decisions:
  - "OnceLock<Regex> preferred over lazy_static for regex compilation (no extra dependency)"
  - "update_page_with_retry is a free function taking &dyn ConfluenceApi (not a method) to enable mock injection in tests"
  - "Upload command uploads raw markdown as placeholder content — Phase 2 converter will produce proper storage XML"
  - "check edit-v2 pattern before /pages/ pattern to avoid false partial matches on more-specific URLs"

requirements-completed: [CONF-01, CONF-02, CONF-03, CONF-04, CONF-05]

# Metrics
duration: 15min
completed: 2026-04-10
---

# Phase 1 Plan 03: Confluence API Client Summary

**Trait-based Confluence REST API client with get_page, update_page, upload_attachment, retry-on-409 TOCTOU mitigation, URL page ID extraction, and upload command wired end-to-end — 20 tests all passing**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-04-10
- **Completed:** 2026-04-10
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Defined `ConfluenceApi` async trait enabling mock substitution in all future tests
- Implemented `ConfluenceClient` with `get_page`, `update_page`, `upload_attachment` against real Confluence REST API v1
- Implemented `update_page_with_retry` free function with TOCTOU-safe re-fetch-then-retry on 409 version conflicts (up to configurable max_retries)
- Implemented `extract_page_id` handling all three Confluence URL patterns with compiled regex via `OnceLock`
- Implemented typed serde structs for Confluence page responses (`Page`, `PageBody`, `StorageRepresentation`, `PageVersion`)
- Wired the `upload` CLI command end-to-end in `lib.rs` — loads config, builds client, extracts page ID, uploads with retry
- Added `confluence` module to `src/lib.rs`
- 20 total tests passing: 5 URL extraction, 1 type deserialization, 1 mock trait compilation, 8 wiremock integration, 5 config (pre-existing)

## Task Commits

Each task was committed atomically:

1. **Task 1: ConfluenceApi trait, types, URL extraction, and mock client** — `0cb2f12` (feat)
2. **Task 2: ConfluenceClient implementation with retry-on-409** — `dd46f74` (feat)
3. **Task 3: Wire upload command end-to-end in lib.rs** — `2357136` (feat)

## Files Created/Modified

- `src/confluence/mod.rs` — `ConfluenceApi` trait definition and module re-exports; `MockConfluenceClient` test helper
- `src/confluence/types.rs` — `Page`, `PageBody`, `StorageRepresentation`, `PageVersion` serde structs
- `src/confluence/url.rs` — `extract_page_id` with three URL pattern regexes via `OnceLock`
- `src/confluence/client.rs` — `ConfluenceClient`, `impl ConfluenceApi for ConfluenceClient`, `update_page_with_retry`, 8 wiremock tests
- `src/lib.rs` — Added `pub mod confluence;`, replaced upload stub with end-to-end implementation

## Decisions Made

- `OnceLock<Regex>` preferred over `lazy_static` for compiled regex reuse — avoids extra dependency while providing same zero-overhead initialization
- `update_page_with_retry` is a free function taking `&dyn ConfluenceApi` (not a method) to enable mock injection in tests without coupling to `ConfluenceClient`
- Check `edit-v2` URL pattern before `/pages/` pattern — more specific pattern must be checked first to avoid false partial matches
- Upload command uploads raw markdown as placeholder content in Phase 1 — Phase 2 converter will produce proper Confluence storage XML

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

- `Commands::Update` in `src/lib.rs` prints "update command: not yet implemented" — intentional per plan; Phase 3 will implement the LLM merge pipeline
- `Commands::Convert` in `src/lib.rs` prints "convert command: not yet implemented" — intentional per plan; Phase 2 will implement the markdown converter
- Upload command sends raw markdown text rather than Confluence storage XML — intentional stub documented in plan; Phase 2 converter resolves this

These stubs do not prevent the plan's goal from being achieved — the upload command is wired end-to-end and the converter integration is explicitly deferred to Phase 2.

## Threat Surface Scan

All threats from the plan's threat model are mitigated:

| Threat | Mitigation Applied |
|--------|--------------------|
| T-01-05: Auth over network | reqwest with rustls; Basic Auth header only sent over HTTPS |
| T-01-06: URL injection via extract_page_id | Regex extracts only `\d+` digits; no code injection possible |
| T-01-07: Version conflict TOCTOU | `update_page_with_retry` re-fetches version before each retry |
| T-01-08: API availability | reqwest 30s timeout; typed errors surface failures clearly |
| T-01-09: Response deserialization | Typed serde structs for all API responses; failures produce `ConfluenceError::Deserialize` |

No new security-relevant surfaces were introduced beyond what the plan's threat model covered.

## Self-Check: PASSED

All created files verified present. All task commits verified in git history.
