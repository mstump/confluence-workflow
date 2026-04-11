---
phase: 01-project-scaffolding-and-confluence-api-client
plan: 02
subsystem: infra
tags: [rust, config, credentials, dotenvy, dirs, waterfall]

# Dependency graph
requires:
  - 01-01 (ConfigError, CliOverrides shape from cli.rs)
provides:
  - Config struct with load() waterfall (CLI > env > .env > ~/.claude/)
  - CliOverrides struct mirroring CLI optional fields
  - load_from_claude_config() best-effort ~/.claude/settings.json stub
affects:
  - 01-03 (will use Config::load() to obtain credentials for Confluence API)
  - all subsequent phases that need credentials

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Waterfall credential resolution: CLI > env var > dotenvy .env > ~/.claude/ stub"
    - "Test isolation: load_with_home() separates dotenvy from resolution for unit testability"
    - "Threat T-01-04: https:// validation before use"

key-files:
  created:
    - src/config.rs
  modified:
    - src/lib.rs

key-decisions:
  - "Separated dotenvy::dotenv() call from load_with_home() to enable test isolation without .env interference"
  - "~/.claude/ stub reads settings.json top-level key; no error if absent — best-effort only"
  - "https:// validation added per threat model T-01-04 (not in original plan but required for security)"

requirements-completed: [SCAF-03, SCAF-04]

# Metrics
duration: 8min
completed: 2026-04-10
---

# Phase 1 Plan 02: Config Struct and Credential Waterfall Loader Summary

**Config struct with waterfall credential loading (CLI > env > .env > ~/.claude/ stub), https:// validation, and 10 unit tests — all passing**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-04-10T23:00:25Z
- **Completed:** 2026-04-10T23:08:03Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments

- Implemented `Config` and `CliOverrides` structs in `src/config.rs`
- Waterfall resolution: CLI flag → environment variable → `.env` (dotenvy) → `~/.claude/settings.json` stub
- Required fields (`confluence_url`, `confluence_username`, `confluence_api_token`) produce `ConfigError::Missing` with the exact field name when absent from all sources
- `anthropic_api_key` is `Option<String>` — absent does not error (Phase 3 will require it)
- `confluence_url` normalized: trailing slash stripped, must start with `https://`
- `load_from_claude_config()` reads `~/.claude/settings.json` top-level keys; gracefully returns `None` if file absent or malformed
- Exported `pub mod config` from `src/lib.rs`
- 10 unit tests covering all waterfall steps, all error cases, and threat mitigations

## Task Commits

Each task was committed atomically:

1. **Task 1: Config struct and credential waterfall loader** - `ce3109b` (feat)

## Files Created/Modified

- `src/config.rs` — `CliOverrides`, `Config`, `Config::load()`, `Config::load_with_home()`, `load_from_claude_config()`, and 10 unit tests
- `src/lib.rs` — added `pub mod config;`

## Decisions Made

- Separated `dotenvy::dotenv()` call from `load_with_home()` to enable test isolation: the public `load()` calls dotenvy once, then delegates to `load_with_home()`. Tests call `load_with_home()` directly with a non-existent home path to prevent real credentials from leaking in.
- `~/.claude/settings.json` stub reads top-level JSON keys by the env var name (e.g., `CONFLUENCE_URL`). This is a stub for Phase 1; a more complete implementation may use nested keys in a future phase.
- `https://` validation added per threat model T-01-04 — confluence_url must use HTTPS to prevent accidental unencrypted credential transmission.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Security] Added https:// validation per threat model T-01-04**

- **Found during:** Task 1 implementation
- **Issue:** Plan's threat model listed T-01-04 with disposition `mitigate` — validate confluence_url starts with "https://"
- **Fix:** Added scheme check after trailing-slash normalization; returns `ConfigError::Missing` with descriptive name if http:// or other scheme used
- **Files modified:** `src/config.rs`
- **Commit:** `ce3109b`

**2. [Rule 1 - Bug] Separated dotenvy call from load_with_home() to fix test isolation**

- **Found during:** Task 1 test execution (GREEN phase)
- **Issue:** `dotenvy::dotenv().ok()` inside the internal resolution function caused tests to re-load the real `.env` file even after `std::env::remove_var()`, making "missing field" tests fail with real credentials
- **Fix:** Moved `dotenvy::dotenv().ok()` to `Config::load()` only; `load_with_home()` (used by tests) does not call dotenvy
- **Files modified:** `src/config.rs`
- **Commit:** `ce3109b`

## Known Stubs

- `load_from_claude_config()` reads `~/.claude/settings.json` top-level keys. Real Claude Code config stores the API key differently; this stub will need updating in Phase 3 when `anthropic_api_key` becomes required.

## Self-Check: PASSED

All created files verified present. Task commit `ce3109b` verified in git history.
