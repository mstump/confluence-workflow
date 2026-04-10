# Codebase Concerns

**Analysis Date:** 2026-04-10

## Tech Debt

**External Subprocess Dependencies:**

- Issue: The codebase relies on external Java and Node.js binaries (PlantUML jar, mermaid-cli) for diagram rendering
- Files: `src/confluence_agent/converter.py` (lines 38-64, 94-125)
- Impact: Deployment requires complex setup with multiple runtime environments. Failures are hard to diagnose; subprocess errors are caught but not granularly handled
- Fix approach: Consider containerizing all dependencies or investigating pure-Python alternatives for diagram rendering. Add better subprocess error diagnostics and graceful fallbacks

**String-Based XML Manipulation:**

- Issue: Multiple `re.sub()` operations directly manipulate Confluence storage XML without proper parsing
- Files: `src/confluence_agent/converter.py` (lines 210, 213, 216, 219-227)
- Impact: Fragile to XML structure changes; no validation that output remains valid XML. Regex patterns could accidentally match unintended content
- Fix approach: Use a proper XML library (lxml, ElementTree) for structural changes. Validate output XML after transformations

**Heuristic Token Counting for Google:**

- Issue: Gemini tokenization uses character count divided by 4 as a rough estimate
- Files: `src/confluence_agent/llm.py` (lines 49-53)
- Impact: Token predictions for Google provider are inaccurate. Can cause `maxTokens` to be under/over-allocated, potentially truncating responses or wasting budget
- Fix approach: Use official Google GenAI token counting API if available, or maintain a calibration table

**Configuration Hardcoded Defaults in config.py:**

- Issue: Default model names and placeholder API keys are hardcoded in the Settings class
- Files: `src/confluence_agent/config.py` (lines 28-35)
- Impact: Misleading defaults ("sk-my-openai-api-key", "gpt-5-nano" which doesn't exist, "gemini-2.5-flash-lite" which is explicitly noted as unsatisfactory). Users may not override these properly
- Fix approach: Remove placeholder defaults; require explicit env vars or use `None` with validation that checks they're set before use

**Broad Exception Handling:**

- Issue: Multiple catch-all `except Exception` blocks that swallow errors without proper context
- Files: `src/confluence_agent/patched_providers.py` (lines 61, 266), `src/confluence_agent/structured_output.py` (line 22)
- Impact: Hard to debug; errors lose their stack traces and specificity
- Fix approach: Catch specific exceptions (ValidationError, ApiError, etc.) and re-raise with context or convert to domain-specific exceptions

**Manual Version Incrementing:**

- Issue: Page version numbers are incremented manually with `new_version = version + 1`
- Files: `src/confluence_agent/agent.py` (line 363), `src/confluence_agent/cli.py` (line 129)
- Impact: Race condition: if page is updated externally between fetch and update, the version will be stale and the Confluence API will reject the update. No retry logic
- Fix approach: Implement optimistic locking with retry on version conflict, or use Confluence's update endpoint that validates version atomically

---

## Known Bugs

**Broken Model Name in Default Config:**

- Symptoms: If user doesn't override OpenAI model, "gpt-5-nano" is used, which doesn't exist
- Files: `src/confluence_agent/config.py` (line 29)
- Trigger: Running with OpenAI provider without setting `OPENAI__DEFAULT_MODEL` env var
- Workaround: Always set `OPENAI__DEFAULT_MODEL=gpt-4o` in .env or override via code

**Mermaid Diagram Numbering Issue:**

- Symptoms: If both PlantUML and Mermaid diagrams exist, numbering can cause attachment collisions or confusion
- Files: `src/confluence_agent/converter.py` (lines 127-152)
- Trigger: Document with both ```plantuml and```mermaid blocks
- Workaround: Currently diagrams are numbered sequentially (diagram_1.svg, diagram_2.svg, etc.) so no collision, but fragile if multiple files generate diagrams with same numbering

**Inline Comment Marker Regex Assumes Single Occurrence Per Line:**

- Symptoms: If multiple `<ac:inline-comment-marker>` tags appear on the same line or nested, regex may not capture all instances
- Files: `src/confluence_agent/agent.py` (lines 107-114)
- Trigger: Confluence pages with densely packed inline comments
- Workaround: Regex uses `re.DOTALL` and should handle multi-line, but edge cases with unusual XML formatting may still fail

---

## Security Considerations

**No API Token Validation:**

- Risk: Confluence API token and LLM API keys are loaded but never validated before use. Invalid credentials fail silently during page fetch
- Files: `src/confluence_agent/config.py`, `src/confluence_agent/agent.py` (line 305)
- Current mitigation: Error is logged and returned as string result (line 374), but doesn't indicate credential issue vs. network issue
- Recommendations: Add a startup validation endpoint that checks credentials. Provide explicit error messages for auth failures

**Subprocess Injection Risk:**

- Risk: PlantUML and Mermaid content is passed directly to subprocess without sanitization
- Files: `src/confluence_agent/converter.py` (lines 48-54, 104-107)
- Current mitigation: Input is from user markdown files (not network); subprocess uses `check=True` and `capture_output=True`
- Recommendations: Still, validate that PlantUML/Mermaid content doesn't contain shell metacharacters. Use subprocess argument lists instead of shell=True (already done)

**No Rate Limiting for LLM Calls:**

- Risk: No throttling or rate limiting on LLM API calls (merge, reflect, critic = 3 API calls per page update)
- Files: `src/confluence_agent/agent.py` (lines 272-280)
- Current mitigation: None
- Recommendations: Add exponential backoff retry with jitter. Consider adding rate limit detection and circuit breaker pattern

**Secrets in Default Config:**

- Risk: Example `.env` or default config may be committed with test credentials
- Files: `src/confluence_agent/config.py` (lines 28-35) - currently has placeholder values, not real secrets
- Current mitigation: Placeholders are clearly fake ("sk-my-...", "gpt-5-nano")
- Recommendations: Ensure `.env` and `.env.example` are in `.gitignore`. Consider a `config.example.py` if needed

---

## Performance Bottlenecks

**LLM Token Scaling Linear with Content Size:**

- Problem: `maxTokens = (token_count * 4) + 1024` means large documents can require very large completion budgets
- Files: `src/confluence_agent/agent.py` (lines 245-248)
- Cause: 4x multiplier is conservative but may be wasteful for well-structured content
- Improvement path: Implement dynamic scaling based on actual response patterns. Cache token counts. Consider chunking very large documents

**Three Sequential LLM Calls Per Update:**

- Problem: Merge -> Reflect -> Critic means 3 API calls to LLM per page update, each waiting for the previous
- Files: `src/confluence_agent/agent.py` (lines 272-280)
- Cause: Architectural design choice for quality gates
- Improvement path: Consider parallel execution of reflect and critic if they don't depend on each other's output. Add option to skip reflection/critic for speed

**Regex Operations on Large XML:**

- Problem: Multiple `re.sub()` operations iterate over entire storage format string multiple times
- Files: `src/confluence_agent/converter.py` (lines 210, 213, 216, 219-227)
- Cause: Sequential transformations instead of single pass
- Improvement path: Combine regex operations into single pass or use DOM parsing

**File I/O in Converter:**

- Problem: Writes processed markdown and SVG files to disk even though they're immediately read back or uploaded
- Files: `src/confluence_agent/converter.py` (lines 83-85, 144-146, 175-177)
- Cause: Temporary file handling for subprocess communication
- Improvement path: Use pipes/streams instead of intermediate files for PlantUML/Mermaid rendering if possible

---

## Fragile Areas

**Inline Comment Marker Preservation Logic:**

- Files: `src/confluence_agent/agent.py` (lines 91-134, 144-146, 169-171, 194-196)
- Why fragile: Entirely dependent on LLM not modifying markers. Regex extraction only works if XML is well-formed. If LLM "normalizes" XML (e.g., re-quotes attributes), markers are lost
- Safe modification: Add stricter validation that counts markers before/after LLM calls and fails loudly if they don't match. Use XML parser to locate markers instead of regex
- Test coverage: Tests exist for marker extraction (`test_extract_inline_comment_markers_*`) but not for full merge cycle with markers preserved

**Empty Content Detection:**

- Files: `src/confluence_agent/agent.py` (lines 74-88)
- Why fragile: Checks for empty string, whitespace, and `<p/>` tag, but may miss other empty representations (e.g., `<p></p>`, `<div/>`, single space character)
- Safe modification: Define explicit whitespace constants. Add tests for all known empty representations
- Test coverage: Tests exist (`test_update_confluence_page_tool_empty_page`, `test_update_confluence_page_tool_empty_p_tag`)

**Confluence API Error Handling:**

- Files: `src/confluence_agent/confluence.py` (lines 70-79, 117-128)
- Why fragile: Catches `ApiError` but wraps it in another `ApiError` without checking the original error type. Generic error messages don't distinguish auth failure, missing page, permission denial
- Safe modification: Catch specific atlassian API error types and provide context-specific messages
- Test coverage: Unit tests mock the API, so real error scenarios aren't exercised

**Diagram Rendering Failures:**

- Files: `src/confluence_agent/converter.py` (lines 48-64, 104-125)
- Why fragile: If PlantUML or Mermaid rendering fails, the entire markdown conversion fails. No graceful degradation (e.g., keep code block, skip diagram)
- Safe modification: Catch subprocess errors, log warning, and continue with code block unchanged. Mark diagram as "failed to render"
- Test coverage: No test for render failures

---

## Scaling Limits

**Inline Comment Marker Regex on Very Large Documents:**

- Current capacity: No hard limit, but regex performance degrades O(n) with document size
- Limit: Documents with thousands of inline comments may cause noticeable regex parsing delay
- Scaling path: Implement iterator-based regex parsing instead of `findall()`. Cache results

**Token Counting Performance:**

- Current capacity: OpenAI uses tiktoken library (fast). Google uses character division (very fast)
- Limit: Very large documents (>1M characters) may still be acceptable, but repeated token counting in scaling loops could accumulate
- Scaling path: Cache token count results. Pre-calculate budgets once at start of pipeline

**Attachment Upload Throughput:**

- Current capacity: Uploads attachments sequentially in a loop
- Limit: Documents with 100+ diagrams would require 100+ sequential HTTP calls
- Scaling path: Implement concurrent uploads with thread pool or async await. Batch upload API if Confluence supports it

**Confluence API Rate Limits:**

- Current capacity: Single threaded, ~1 page update per minute expected
- Limit: No rate limiting; 3 LLM calls + 1-2 Confluence API calls per update could hit rate limits with concurrent users
- Scaling path: Implement queue-based submission with rate limiter. Add circuit breaker for Confluence API

---

## Dependencies at Risk

**mcp-agent Framework Dependency:**

- Risk: Project is tightly coupled to mcp-agent API (MCPApp, Agent, AugmentedLLM). Internal implementation details are overridden
- Impact: If mcp-agent changes its LLM provider interfaces, `patched_providers.py` becomes unmaintainable
- Migration plan: Monitor mcp-agent releases. Consider vendoring the LLM provider code if stability is critical. Abstract provider interface locally

**atlassian-python-api Version 3.41.16:**

- Risk: No constraint for newer versions. Confluence API may change; library may not support new features
- Impact: Breaking changes in newer versions could fail page updates
- Migration plan: Lock to specific version in pyproject.toml (already done). Add integration tests against Confluence API schema

**OpenAI SDK 2.8.1 (vs 3.x):**

- Risk: Major version is outdated. Library evolution may include breaking changes
- Impact: New OpenAI models or features may not be supported
- Migration plan: Evaluate upgrade to openai 3.x, test with current code. If breaking, update patched_providers.py accordingly

**google-genai 1.52.0:**

- Risk: Newer versions may change response types. Current extraction logic in `structured_output.py` assumes specific response shape
- Impact: If Google changes response format (e.g., `candidates[0].content.parts`), extraction fails silently
- Migration plan: Add version pinning. Monitor Google GenAI releases for API changes. Add robustness checks for response shapes

---

## Missing Critical Features

**No Dry-Run Mode:**

- Problem: Cannot preview the merged result before uploading to Confluence. Must commit to the merge
- Blocks: Safe iteration on merge/reflection logic without risking page updates

**No Rollback Capability:**

- Problem: If critic rejects content, the error is raised but no history is maintained
- Blocks: Cannot easily revert to previous version or retry with different prompt parameters

**No Merge Conflict UI:**

- Problem: If merge produces unexpected results (e.g., duplicate content), user only sees error string
- Blocks: Cannot visually inspect what went wrong before deciding to fix manually

**No Support for Confluence Page Labels/Tags:**

- Problem: Markdown metadata (e.g., front matter) is stripped but not converted to Confluence page properties
- Blocks: Cannot use Confluence filtering/organization features that depend on labels

---

## Test Coverage Gaps

**Untested Subprocess Failures:**

- What's not tested: PlantUML or Mermaid rendering when subprocess is not available
- Files: `src/confluence_agent/converter.py` (lines 38-64, 94-125)
- Risk: `FileNotFoundError` is logged but workflow fails; no graceful fallback
- Priority: **High** - Common in CI/CD or minimal environments

**Untested Confluence API Errors:**

- What's not tested: Real API errors (401, 403, 404, 409 version conflict, rate limit)
- Files: `src/confluence_agent/confluence.py` (lines 70-79, 117-128)
- Risk: All handled identically; user can't distinguish auth failure from missing page
- Priority: **High** - Impacts production troubleshooting

**Untested Large Document Handling:**

- What's not tested: Documents >100KB, >10K inline comments, >100 diagrams
- Files: `src/confluence_agent/agent.py` (lines 245-280)
- Risk: Token scaling, LLM timeout, or memory issues may occur at scale
- Priority: **Medium** - Edge case but possible with large wikis

**Untested Inline Comment Edge Cases:**

- What's not tested: Inline comments with nested tags, malformed markers, comments inside code blocks
- Files: `src/confluence_agent/agent.py` (lines 91-115)
- Risk: Markers are lost in edge cases; data corruption possible
- Priority: **Medium** - Specific to Confluence comments but critical if they exist

**Untested Concurrent Updates:**

- What's not tested: Multiple processes updating the same page simultaneously
- Files: `src/confluence_agent/agent.py` (line 363)
- Risk: Version conflict or last-write-wins overwrite
- Priority: **Low** - Requires distributed scenario; single-user typical

**No Integration Tests Against Real Confluence:**

- What's not tested: Full end-to-end flow with actual Confluence instance
- Files: All files under `src/confluence_agent/`
- Risk: Mocks hide real API incompatibilities or parameter mismatches
- Priority: **Medium** - Currently only unit/integration tests with mocks

---

Concerns audit: 2026-04-10
