---
phase: 4
slug: cli-command-wiring-and-integration
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-12
updated: 2026-04-13
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) + wiremock 0.6 + assert_cmd 2 |
| **Config file** | none — Cargo.toml workspace |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 04-01-01 | 01 | 1 | CLI-01 | — | update rejects missing ANTHROPIC_API_KEY with config error; exits 1 | integration | `cargo test --test cli_integration -- test_update_command_missing_api_key` | ✅ | ✅ green |
| 04-01-02 | 01 | 1 | CLI-02 | T-01-04 | upload rejects http:// URL (security guard) and missing credentials; exits 1 | integration | `cargo test --test cli_integration -- test_upload_command_rejects_http_url test_upload_command_missing_credentials` | ✅ | ✅ green |
| 04-01-03 | 01 | 1 | CLI-03 | — | convert writes storage XML to output dir; exits 0; stdout contains "Converted to:" | integration | `cargo test --test cli_integration -- test_convert_command` | ✅ | ✅ green |
| 04-02-01 | 02 | 2 | CLI-04 | — | --verbose sends tracing to stderr, not stdout; default mode produces empty stderr | integration | `cargo test --test output_format` | ✅ | ✅ green |
| 04-02-02 | 02 | 2 | CLI-05 | — | convert --output json emits valid JSON with success, output_dir, files fields | integration | `cargo test --test cli_integration -- test_json_output_mode` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/cli_integration.rs` — integration tests for CLI-01 (update error path), CLI-02 (upload security guard + missing credentials), CLI-03 (convert happy path + error path)
- [x] `tests/cli_integration.rs` — JSON output tests for CLI-05 (success schema + error schema)
- [x] `tests/output_format.rs` — CLI-04 stderr routing (verbose routes to stderr, default mode is silent)

*assert_cmd 2 and wiremock 0.6 are already in [dev-dependencies] — no new deps needed.*

---

## Constraint Notes

### CLI-02 Upload Happy-Path (end-to-end)

The `Config::load()` function enforces that `CONFLUENCE_URL` must start with `https://` (threat T-01-04). wiremock only binds to `http://` and cannot serve TLS. As a result, the upload command happy-path (successful page overwrite) cannot be tested at the binary level with wiremock alone. Covered by:

- `test_upload_command_rejects_http_url` — verifies the security guard fires correctly
- `test_upload_command_missing_credentials` — verifies credential validation
- Unit tests in `src/confluence/client.rs` — full HTTP layer covered via wiremock (GET/PUT/attachment)
- `test_upload_command_happy_path` — `#[ignore]`-tagged stub noting the TLS constraint

### CLI-01 Update Happy-Path (end-to-end)

Requires both a TLS Confluence mock and an Anthropic API mock (LLM merge step). The error-path test (`test_update_command_missing_api_key`) covers the credential-validation boundary. Full happy-path is manual-only per the Manual-Only Verifications table.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end update against live Confluence | CLI-01 | Requires live Confluence + ANTHROPIC_API_KEY + real page | Run `confluence-agent update doc.md <real_page_url>` and verify page updates |
| End-to-end upload against live Confluence | CLI-02 | Requires live https:// Confluence instance | Run `confluence-agent upload doc.md <real_page_url>` and verify page overwrites |

---

## Validation Sign-Off

- [x] All tasks have automated verify or documented constraint explanation
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** 2026-04-13 — Nyquist auditor (gaps filled)
