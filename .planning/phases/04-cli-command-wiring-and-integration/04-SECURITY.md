---
phase: 04
slug: cli-command-wiring-and-integration
status: verified
threats_open: 0
asvs_level: 1
created: 2026-04-13
---

# Phase 04 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| user input -> file system | markdown_path and output_dir come from CLI args (user controlled) | file paths, markdown content |
| file system -> converter | Markdown content read from disk fed to converter | document content |
| converter -> Confluence API | Storage XML uploaded to remote API | formatted page content |
| run() output -> stdout/stderr | Command results and error messages emitted to stdout/stderr | page URLs, counts, error messages |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-04-01 | Tampering | Convert command output_dir | accept | `create_dir_all` + `output_dir.join()` — user controls their own filesystem; accepted risk | closed |
| T-04-02 | Information Disclosure | API key in verbose logs | mitigate | Tracing spans in AnthropicClient log only `status`, `attempt`, `delay_ms` — no `api_key` field at any call site (`src/llm/mod.rs:155-161`) | closed |
| T-04-03 | Information Disclosure | Credentials in error messages | mitigate | `ConfigError::Missing` Display shows only field name (`&'static str`); `ConfluenceError::Unauthorized` shows no values (`src/error.rs:86-88, 120-123`) | closed |
| T-04-04 | Denial of Service | Large markdown files | accept | Single-user CLI; user controls input; no memory limits needed beyond OS defaults | closed |
| T-04-05 | Information Disclosure | Verbose tracing leaks API key | mitigate | `init_tracing()` routes exclusively to stderr via `.with_writer(std::io::stderr)` (`src/main.rs:15`) | closed |
| T-04-06 | Information Disclosure | JSON error includes sensitive info | mitigate | `error_to_json()` uses `error.to_string()` — no AppError/ConfigError Display impl includes credential values (`src/lib.rs:71-76`) | closed |
| T-04-07 | Information Disclosure | Credentials in JSON success output | mitigate | `CommandResult` contains no credential fields; `result_to_json()` emits only `page_url`, counts, `output_dir`, `files` (`src/lib.rs:22-66`) | closed |

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-04-01 | T-04-01 | Single-user CLI; user specifies output_dir and controls their own filesystem; no server-side exposure | plan author | 2026-04-13 |
| AR-04-02 | T-04-04 | Single-user CLI; markdown input is user-provided; OS memory limits are sufficient guardrail | plan author | 2026-04-13 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-04-13 | 7 | 7 | 0 | gsd-security-auditor (ASVS L1) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-04-13
