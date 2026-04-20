# Phase 10: Tech Debt — Integration Test Coverage and API Cleanup — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-20
**Phase:** 10-tech-debt-integration-test-coverage-and-api-cleanup
**Areas discussed:** https constraint, DiagramConfig cleanup, update test mock depth

---

## https Constraint

| Option | Description | Selected |
|--------|-------------|----------|
| Trait injection | Add run_with_clients() entry point that skips Config::load; tests pass mock trait objects | |
| Localhost exemption | Relax https guard to allow http://localhost and http://127.0.0.1 | ✓ |
| Accept the limitation | Leave test_upload_command_happy_path as #[ignore]; write library-level tests instead | |

**User's choice:** Localhost exemption — simple targeted change, https invariant preserved for non-localhost URLs
**Notes:** ~2 lines in Config::load_with_home; existing unit test needs updating to reflect exemption

---

## DiagramConfig Cleanup

| Option | Description | Selected |
|--------|-------------|----------|
| Remove entirely | Delete from_env() and Default impl; update converter tests to explicit construction | ✓ |
| Make private (pub(crate)) | Keep but restrict visibility; lower risk | |

**User's choice:** Remove entirely
**Notes:** Callers found in converter/mod.rs, converter/tests.rs, converter/diagrams.rs tests — all updated to explicit DiagramConfig { plantuml_path, mermaid_path, ... } construction

---

## Update Test Mock Depth

| Option | Description | Selected |
|--------|-------------|----------|
| Wiremock for both | Two MockServer instances (Confluence + Anthropic); full binary path exercised | ✓ |
| Trait injection via run_with_clients() | Add library entry point; inject MockConfluenceClient + MockLlmClient | |

**User's choice:** Wiremock for both
**Notes:** Requires making AnthropicClient base URL configurable via ANTHROPIC_BASE_URL env var (D-03)

---

## Anthropic Base URL Configurability

| Option | Description | Selected |
|--------|-------------|----------|
| ANTHROPIC_BASE_URL env var | Read in AnthropicClient::new(); tests set env var to wiremock address | ✓ |
| Constructor parameter | Add base_url: Option<String> to AnthropicClient::new(); all call sites updated | |

**User's choice:** ANTHROPIC_BASE_URL env var — no CLI/Config changes needed

---

## Claude's Discretion

- Exact wiremock stub shape for Anthropic tool_use KEEP response (follow llm_integration.rs fixtures)
- Whether test_upload_command_happy_path is repurposed or replaced
- serial attribute usage for ANTHROPIC_BASE_URL env var mutation
- Updated assertion in test_confluence_url_must_be_https for localhost exemption
