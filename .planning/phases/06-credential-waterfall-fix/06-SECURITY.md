---
phase: 06-credential-waterfall-fix
plan: 01
type: security-audit
asvs_level: 1
audited: 2026-04-13
auditor: gsd-secure-phase
verdict: SECURED
---

# Phase 06 Plan 01: Security Audit

## Summary

**Phase:** 06 — credential-waterfall-fix
**Threats Closed:** 3/3
**ASVS Level:** 1
**Block On:** high (no high-severity open threats)

---

## Threat Verification

| Threat ID | Category | Disposition | Status | Evidence |
|-----------|----------|-------------|--------|----------|
| T-06-01 | Information Disclosure | accept | CLOSED | Acceptance documented below. Env var path is the recommended secure method; clap `env` annotation on `anthropic_api_key` field in `src/cli.rs:30-31` mirrors the identical pattern already accepted for `CONFLUENCE_API_TOKEN` at `src/cli.rs:26-27`. |
| T-06-02 | Information Disclosure | mitigate | CLOSED | No tracing or logging statement references `api_key` or `anthropic_api_key` as a field value anywhere in the call path. The key travels `src/cli.rs:31` → `src/lib.rs:88` (CliOverrides construction) → `src/config.rs:114-118` (resolve_optional) → `src/lib.rs:93` (ok_or_else extraction) → `src/lib.rs:116` (moved into AnthropicClient::new). All tracing statements in `src/lib.rs` log only `page_id`, `kept`, `dropped`, `llm_evaluated`, and `filename` — never the key value. `src/config.rs` tracing emits only path-not-found messages with no key value. |
| T-06-03 | Tampering | accept | CLOSED | Acceptance documented below. The HTTPS guard at `src/config.rs:93-98` rejects any `confluence_url` not starting with `https://` before Config::load returns. This phase does not modify that guard. Integration test fix (commit 22f5945) strengthened coverage by switching test URLs to `https://localhost:19999`, exercising the ANTHROPIC_API_KEY error path that sits behind the HTTPS guard. |

---

## Detailed Evidence

### T-06-02 Call-Path Tracing Audit

The `api_key` local variable (bound at `src/lib.rs:93`) is passed directly into `AnthropicClient::new(api_key, ...)` at `src/lib.rs:115-118`. It is stored in the `AnthropicClient.api_key` field (`src/llm/mod.rs:44`) and used only as an HTTP request header value (`src/llm/mod.rs:100`). No `tracing::`, `println!`, or `eprintln!` call in any of the three audited files (`src/cli.rs`, `src/lib.rs`, `src/config.rs`) emits a value derived from the key.

The `Config` struct derives `Debug` (`src/config.rs:55`) which would print `anthropic_api_key` if the struct were passed to a debug formatter. No `tracing::debug!` or `tracing::info!` call logs `Config` or `CliOverrides` as a structured field anywhere in the call path — confirmed by exhaustive grep across all `.rs` files in `src/`.

Mitigation verdict: **present and effective**.

### T-06-03 HTTPS Guard Location

Guard text in `src/config.rs:92-98`:

```
if !confluence_url.starts_with("https://") {
    return Err(ConfigError::Invalid {
        name: "CONFLUENCE_URL",
        reason: "must start with https://",
    });
}
```

This fires before any credential is consumed, making HTTPS enforcement unconditional regardless of how `confluence_url` arrives (CLI flag, env var, or `.env` file). Phase 06-01 makes no changes to this guard.

---

## Accepted Risks

| Risk ID | Threat | Rationale | Residual Risk | Owner |
|---------|--------|-----------|---------------|-------|
| T-06-01 | API key value visible in `ps aux` / `/proc/pid/cmdline` when passed via `--anthropic-api-key` | This is an inherent, unfixable OS-level property of CLI process arguments. The clap `env = "ANTHROPIC_API_KEY"` annotation (`src/cli.rs:30`) means users who set the env var instead of the flag are not exposed. The env var path is the recommended method. This pattern is identical to the already-accepted risk for `CONFLUENCE_API_TOKEN` (`src/cli.rs:26-27`). ASVS Level 1 does not require mitigation of OS process-listing exposure for CLI tools. | Low — exposure requires local access to the machine running the process | Project owner |
| T-06-03 | HTTPS bypass via CLI-supplied `confluence_url` | Cannot be bypassed — `Config::load` validates the scheme unconditionally at `src/config.rs:93-98` before returning. Disposition is `accept` only in the sense that the guard is an existing control not modified by this phase; the actual risk is fully mitigated by the guard. | None | N/A |

---

## Unregistered Threat Flags

None. The SUMMARY.md `## Threat Surface Scan` section confirms no new network endpoints, auth paths, file access patterns, or schema changes were introduced by this phase.

---

## Audit Scope

Files audited for T-06-02 tracing verification:

- `/Users/matthewstump/src/confluence-workflow/src/cli.rs`
- `/Users/matthewstump/src/confluence-workflow/src/lib.rs`
- `/Users/matthewstump/src/confluence-workflow/src/config.rs`
- `/Users/matthewstump/src/confluence-workflow/src/llm/mod.rs` (AnthropicClient.api_key field and header use)

Files audited for T-06-03 guard verification:

- `/Users/matthewstump/src/confluence-workflow/src/config.rs` (lines 92-98)
