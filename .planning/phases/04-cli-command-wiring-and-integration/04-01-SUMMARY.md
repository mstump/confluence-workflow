---
phase: 04-cli-command-wiring-and-integration
plan: 01
subsystem: cli
tags: [clap, tokio, async, pipeline-wiring]

# Dependency graph
requires:
  - phase: 01-project-scaffolding-and-confluence-api-client
    provides: ConfluenceClient, Config, CliOverrides, update_page_with_retry
  - phase: 02-markdown-to-confluence-storage-format-converter
    provides: MarkdownConverter, Converter trait, ConvertResult
  - phase: 03-llm-client-and-comment-preserving-merge
    provides: AnthropicClient, merge::merge, MergeResult
provides:
  - Full pipeline wiring for update/upload/convert commands in lib.rs
  - CommandResult enum for structured output formatting
  - OutputFormat enum (Human/Json) with --output global CLI flag
affects: [04-02-output-formatting]

# Tech tracking
tech-stack:
  added: []
  patterns: [command-result-enum, deferred-config-load]

key-files:
  created: []
  modified:
    - src/cli.rs
    - src/lib.rs
    - src/main.rs

key-decisions:
  - "Config::load() called per-command (not globally) so convert works without Confluence credentials"
  - "CommandResult enum returned from run() for Plan 02 output formatting layer"
  - "main.rs uses temporary println until Plan 02 adds proper formatting"

patterns-established:
  - "Deferred config load: only commands needing credentials call Config::load()"
  - "Structured result: run() returns CommandResult, not unit, for downstream formatting"

requirements-completed: [CLI-01, CLI-02, CLI-03, CLI-05]

# Metrics
duration: 20min
completed: 2026-04-13
---

# Phase 4 Plan 1: CLI Command Wiring Summary

**All three CLI commands wired through full pipeline with CommandResult enum and OutputFormat flag for Plan 02 formatting layer**

## Performance

- **Duration:** 20 min
- **Started:** 2026-04-13T15:55:13Z
- **Completed:** 2026-04-13T16:15:12Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Wired update command through converter, merge engine (with LLM), attachment upload, and page update with retry
- Wired upload command through converter, attachment upload, and direct page overwrite (no LLM)
- Wired convert command to write storage XML and attachments to local disk without needing Confluence credentials
- Added OutputFormat enum (Human/Json) with --output global flag for Plan 02
- Added CommandResult enum so run() returns structured data instead of printing directly

## Task Commits

Each task was committed atomically:

1. **Task 1: Add OutputFormat enum and --output flag to cli.rs** - `29839d5` (feat)
2. **Task 2: Wire all three commands in lib.rs** - `d9c0c79` (feat)

## Files Created/Modified

- `src/cli.rs` - Added OutputFormat enum (Human/Json) with ValueEnum derive, added --output global flag
- `src/lib.rs` - Complete rewrite of run() with all three command pipelines, CommandResult enum
- `src/main.rs` - Updated to handle CommandResult instead of anyhow::Result

## Decisions Made

- Config::load() is called per-command rather than globally, so the convert command works without Confluence credentials
- CommandResult enum provides structured output data for the formatting layer in Plan 02
- main.rs uses temporary println statements until Plan 02 adds proper output formatting
- Added ConfluenceApi trait import to lib.rs (required for trait method dispatch on ConfluenceClient)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing ConfluenceApi trait import**

- **Found during:** Task 2 (cargo check)
- **Issue:** ConfluenceClient method calls (get_page, upload_attachment) require ConfluenceApi trait to be in scope
- **Fix:** Added `ConfluenceApi` to the confluence import line in lib.rs
- **Files modified:** src/lib.rs
- **Verification:** cargo check passes
- **Committed in:** d9c0c79 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary import for trait method resolution. No scope creep.

## Issues Encountered

None beyond the auto-fixed trait import.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- CommandResult enum is ready for Plan 02 to implement output formatting
- OutputFormat enum is ready for Plan 02 to match on for Human vs JSON output
- All 115 existing lib tests pass with the new wiring

---
*Phase: 04-cli-command-wiring-and-integration*
*Completed: 2026-04-13*
