---
phase: 8
slug: diagramconfig-waterfall-and-nyquist-compliance
status: verified
threats_open: 0
asvs_level: 1
created: 2026-04-15
---

# Phase 8 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| User CLI input → subprocess spawn | `--plantuml-path` / `--mermaid-path` values passed to `Command::new()` | Local filesystem path (user-supplied string) |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-08-01 | Tampering | `--plantuml-path` CLI flag | accept | Path supplied by authenticated local user with same privilege level as the process. `Command::new(plantuml_path)` pattern in `converter/diagrams.rs` is unchanged — this phase only changes where the value originates (CLI flag vs env var), not how it is consumed. No additional validation needed. | closed |
| T-08-02 | Elevation of Privilege | `--plantuml-path` pointing to malicious binary | accept | Same trust boundary as T-08-01. User who supplies the flag can already execute arbitrary binaries. Standard CLI behavior comparable to `$PATH` resolution. | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-08-01 | T-08-01 | CLI tool runs with the same privileges as the invoking user. Accepting arbitrary binary execution via `--plantuml-path` is equivalent to accepting `$PATH` resolution — standard for CLI tools. No privilege boundary crossed. | gsd-security-auditor | 2026-04-15 |
| AR-08-02 | T-08-02 | EoP threat is subsumed by AR-08-01: a user who can supply `--plantuml-path` already has full shell access at the same privilege level. No escalation path exists. | gsd-security-auditor | 2026-04-15 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-04-15 | 2 | 2 | 0 | gsd-security-auditor (automated, /gsd-secure-phase 8) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-04-15
