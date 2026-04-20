---
phase: 02-markdown-to-confluence-storage-format-converter
plan: 03
subsystem: converter
tags: [diagrams, plantuml, mermaid, subprocess, async]
dependency_graph:
  requires: ["02-02"]
  provides: ["diagram-rendering", "converter-full-feature-set"]
  affects: ["src/converter/diagrams.rs", "src/converter/mod.rs", "src/config.rs"]
tech_stack:
  added: ["tempfile (runtime)", "tokio process+time features"]
  patterns: ["async subprocess rendering", "tempfile I/O for mermaid", "placeholder replacement"]
key_files:
  created:
    - src/converter/diagrams.rs
    - tests/fixtures/plantuml_diagram.md
    - tests/fixtures/mermaid_diagram.md
  modified:
    - src/converter/mod.rs
    - src/converter/tests.rs
    - src/config.rs
    - Cargo.toml
decisions:
  - "Combined env var config tests into single test to avoid parallel race conditions"
  - "Mermaid integration tests gracefully skip when Chrome/puppeteer not configured"
  - "PlantUML integration tests skip when plantuml binary not available"
metrics:
  duration: "342s (~6 min)"
  completed: "2026-04-11"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 7
---

# Phase 02 Plan 03: Diagram Rendering (PlantUML + Mermaid) Summary

Async subprocess rendering for PlantUML and Mermaid diagrams with DiagramConfig env-var loading, tokio timeout safety, and ac:image placeholder replacement in MarkdownConverter.

## What Was Done

### Task 1: DiagramConfig and async subprocess rendering

- Added `DiagramConfig` struct to `src/config.rs` with env var loading (`PLANTUML_PATH`, `MERMAID_PATH`, `DIAGRAM_TIMEOUT`, `MERMAID_PUPPETEER_CONFIG`)
- Created `src/converter/diagrams.rs` with `render_plantuml` (CLI + JAR modes via stdin pipe) and `render_mermaid` (tempfile I/O with optional puppeteer config)
- Both renderers wrapped in `tokio::time::timeout` for subprocess safety (default 30s)
- Error handling returns `ConversionError::DiagramError` or `ConversionError::DiagramTimeout`
- Added `tempfile` to runtime dependencies, `process` and `time` features to tokio
- Created test fixtures for plantuml and mermaid diagram markdown

### Task 2: Wire diagram rendering into MarkdownConverter

- Updated `MarkdownConverter` to accept `DiagramConfig` and render diagram blocks during `convert()`
- PlantUML/puml blocks rendered via `render_plantuml`, mermaid via `render_mermaid`
- `DIAGRAM_PLACEHOLDER_N` comments replaced with `<ac:image>/<ri:attachment>` references
- Attachments returned with sequential filenames (`diagram_0.svg`, `diagram_1.svg`, etc.)
- Added integration tests: plantuml rendering, mermaid rendering, placeholder replacement, no-diagrams case
- Updated all existing tests to use `DiagramConfig::default()`

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | b06f626 | feat(02-03): add DiagramConfig and async PlantUML/Mermaid subprocess rendering |
| 2 | aa6b4bc | feat(02-03): wire diagram rendering into MarkdownConverter with placeholder replacement |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Missing tokio features for process and time**

- **Found during:** Task 1
- **Issue:** `tokio::process::Command` and `tokio::time::timeout` require `process` and `time` cargo features
- **Fix:** Added `process` and `time` to tokio features in Cargo.toml
- **Files modified:** Cargo.toml

**2. [Rule 1 - Bug] Env var race conditions in config tests**

- **Found during:** Task 1
- **Issue:** Separate env var tests race under parallel `cargo test` (same known issue as Phase 1)
- **Fix:** Combined all DiagramConfig env tests into a single test with save/restore pattern
- **Files modified:** src/converter/diagrams.rs

**3. [Rule 1 - Bug] Mermaid integration test fails without Chrome/puppeteer**

- **Found during:** Task 1
- **Issue:** `mmdc` requires Chrome headless shell which may not be installed
- **Fix:** Integration tests gracefully handle Chrome/puppeteer errors instead of panicking
- **Files modified:** src/converter/diagrams.rs, src/converter/tests.rs

## Verification

- `cargo build` exits 0 with zero warnings
- `cargo test converter` passes all 45 tests
- `cargo test -- --test-threads=1` passes all 70 tests (single-threaded avoids pre-existing env var race)
- Pre-existing config test flake (`test_fallthrough_to_env_vars`) occurs under parallel execution -- not caused by this plan

## Self-Check: PASSED

All 7 files verified present. Both commits (b06f626, aa6b4bc) verified in git log.
