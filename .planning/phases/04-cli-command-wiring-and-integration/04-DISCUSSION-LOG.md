# Phase 4: CLI Command Wiring and Integration — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-12
**Phase:** 04-cli-command-wiring-and-integration
**Areas discussed:** Output flag design, Progress during LLM calls, Tracing/logging format, Exit codes

---

## Output Flag Design

| Option | Description | Selected |
|--------|-------------|----------|
| Global flag | Add --output to top-level Cli struct, same as --verbose | ✓ |
| Per-subcommand | Each of Update/Upload has its own --output flag; Convert omits it | |

**User's choice:** Global flag

| Option | Description | Selected |
|--------|-------------|----------|
| File list | { success, output_dir, files: [...] } | ✓ |
| Success only | { success, error? } | |
| Same shape as update | { success, output_dir } with null fields | |

**User's choice:** File list for convert command JSON schema

---

## Progress During LLM Calls

| Option | Description | Selected |
|--------|-------------|----------|
| Silent until done | No output until complete, then success line | ✓ |
| Single status line | Print "Evaluating N comments..." and update on done | |
| Log each LLM result | Print a line per KEEP/DROP decision as they arrive | |

**User's choice:** Silent until done

---

## Tracing/Logging Format

| Option | Description | Selected |
|--------|-------------|----------|
| Human-readable pretty | tracing-subscriber default pretty format | ✓ |
| Compact human-readable | Single-line per event, less visual noise | |
| Structured JSON | tracing-bunyan-formatter JSON lines | |

**User's choice:** Human-readable pretty

| Option | Description | Selected |
|--------|-------------|----------|
| Stderr | Keeps stdout clean for JSON output or page URL | ✓ |
| Stdout | Simpler but mixes with machine-readable output | |

**User's choice:** Stderr

---

## Exit Codes

| Option | Description | Selected |
|--------|-------------|----------|
| Standard 0/1 only | 0 = success, 1 = any failure | ✓ |
| Granular codes | 0/1/2/3/4 for different error types | |

**User's choice:** Standard 0/1 only

---
