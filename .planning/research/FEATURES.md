# Feature Landscape: Intelligent Per-Comment Merge Evaluation

**Domain:** LLM-driven inline comment preservation during Confluence page merges
**Researched:** 2026-04-10

## Problem Statement

The current implementation treats ALL inline comments as immutable tokens that must survive every merge. This is correct for comments anchored to unchanged text, but produces stale or misleading comments when the underlying text is significantly rewritten or removed. The goal is to evaluate each comment independently, in parallel, with a small focused context -- NOT the entire page.

### Current Behavior (from codebase analysis)

1. `_extract_inline_comment_markers()` extracts every `<ac:inline-comment-marker>` verbatim
2. The full list is injected into MERGE, REFLECTION, and CRITIC prompts as "must preserve byte-for-byte"
3. The entire page (original + new content) is sent through a 3-phase LLM chain
4. No evaluation of whether a comment is still relevant to changed content

### Desired Behavior

1. Extract each comment with its local context
2. Evaluate in parallel: does this comment still make sense given the new content?
3. Produce a per-comment verdict: KEEP / DROP / RELOCATE
4. Reassemble the final document with only surviving comments in correct positions

---

## Feature 1: Per-Comment Context Extraction

### What It Is

A preprocessing step that isolates each inline comment marker along with the minimal surrounding context needed for an LLM to judge relevance.

### Minimal Context Window (Recommended: Section-level)

| Context Level | Tokens (~) | Pros | Cons |
|--------------|-----------|------|------|
| Anchor text only | 5-50 | Cheapest, fastest | Insufficient -- comment meaning often depends on paragraph context |
| Surrounding paragraph | 50-300 | Good signal-to-noise, cheap | Misses cases where section reorganization moves meaning elsewhere |
| **Containing section (recommended)** | 100-800 | Captures structural context, handles reorganizations | Slightly more tokens but still very small vs full page |
| Full page | 2000-50000+ | Complete context | Defeats the purpose; current approach already does this |

**Use section-level context because:**

- Confluence storage format has clear section boundaries (`<h1>` through `<h6>` tags)
- Comments almost always reference concepts within their section
- Section-level context is small enough for a cheap/fast model (gemini-2.5-flash, gpt-4o-mini)
- If the section was deleted entirely in the new content, that alone signals the comment should be dropped -- no LLM call needed

### Context Extraction Algorithm

For each inline comment marker in the original content:

1. **Parse the section boundary**: Walk up from the marker to the nearest `<hN>` tag, walk down to the next `<hN>` of equal or lesser depth (or end of document). This is the "original section."
2. **Extract the anchor text**: The text content wrapped by the `<ac:inline-comment-marker>` tag.
3. **Find the corresponding new section**: Match by heading text (fuzzy match, since markdown-to-storage may alter formatting slightly). If no match exists, the section was deleted.
4. **Bundle**: `{marker_xml, anchor_text, original_section_xml, new_section_xml | null}`

### Complexity: Medium

Requires an XML/HTML parser (use Python's `lxml` or `html.parser`) to walk the storage format tree. Regex-based section splitting is fragile for nested structures.

### Notes

- Self-closing comment markers (`<ac:inline-comment-marker ... />`) have no anchor text. These are rare but should be handled -- use surrounding 2-3 sentences as context instead.
- When a section heading changes but content is clearly recognizable (e.g., "Setup" renamed to "Getting Started"), fuzzy matching on the section body is more reliable than heading text alone.

---

## Feature 2: Per-Comment Relevance Evaluation Prompt

### What It Is

A focused LLM prompt that evaluates a single comment's relevance given the content change in its section.

### Prompt Strategy (Recommended: Structured Decision with Evidence)

**Use a structured output model, not free-form text.** The LLM should return:

```python
class CommentVerdict(BaseModel):
    comment_ref: str          # The ac:ref UUID
    decision: Literal["KEEP", "DROP", "RELOCATE"]
    confidence: float         # 0.0 - 1.0
    reasoning: str            # One sentence explaining why
    relocated_anchor: str | None  # New anchor text if RELOCATE
```

### Prompt Design

The prompt should be minimal and decision-focused. It should NOT ask the LLM to produce merged content -- only to judge comment relevance.

**Key prompt elements:**

1. **Role**: "You are evaluating whether an inline comment on a Confluence page is still relevant after a content update."
2. **Anchor text**: The exact text the comment was attached to.
3. **Original section**: The section in which the comment appeared (storage format).
4. **New section**: The corresponding section in the updated content (storage format), or explicit statement "This section was removed."
5. **Decision criteria**:
   - KEEP: The anchor text (or semantically equivalent text) still exists in the new section. The comment's context is preserved.
   - DROP: The anchor text was removed AND no semantically equivalent text exists. The concept the comment addressed no longer appears.
   - RELOCATE: The anchor text was reworded but the same concept exists. Provide the new anchor text.
6. **Bias toward KEEP**: When uncertain, prefer KEEP. Losing a stale comment is annoying; losing a relevant comment is destructive. Use confidence threshold (e.g., only DROP when confidence > 0.8).

### Model Selection

Use a small, fast model for per-comment evaluation. This is a classification task, not generation.

| Model | Suitability | Cost | Latency |
|-------|-------------|------|---------|
| **gpt-4o-mini** | Good -- strong at structured output, fast | Very low | ~200ms |
| **gemini-2.5-flash** | Good -- fast, cheap, decent at classification | Very low | ~150ms |
| gemini-2.5-flash-lite | Avoid -- CLAUDE.md notes it "produces poor results" | Lowest | ~100ms |
| gpt-5 / gemini-2.5-pro | Overkill for binary/ternary classification | High | ~1-3s |

**Recommend: gpt-4o-mini or gemini-2.5-flash**, whichever provider is already configured. The existing `get_llm_provider()` factory can be extended with a `get_fast_llm_provider()` variant.

### Short-Circuit Cases (No LLM Needed)

Several cases can be decided deterministically without any LLM call:

| Condition | Verdict | Rationale |
|-----------|---------|-----------|
| Section deleted entirely in new content | DROP | Nothing to anchor to |
| Anchor text appears verbatim in new section | KEEP | Exact match, no ambiguity |
| Section unchanged (byte-equal) | KEEP | No change, no risk |
| Anchor text is empty (self-closing marker) | KEEP (conservative) | Can't evaluate without anchor text |

These short-circuits reduce LLM calls significantly. In practice, many comments will survive unchanged sections.

### Complexity: Medium

The prompt itself is simple. The structured output model integrates with the existing `_generate_structured_with_retry()` pattern.

---

## Feature 3: Parallel Evaluation with asyncio.gather

### What It Is

Fan out per-comment LLM evaluations to run concurrently, then collect all verdicts before reassembly.

### Architecture Pattern: Map-Reduce

```
Extract comments (map)
    |
    v
[Comment 1] [Comment 2] [Comment 3] ... [Comment N]
    |            |            |               |
    v            v            v               v
[Evaluate]  [Evaluate]  [Evaluate]  ...  [Evaluate]
    |            |            |               |
    v            v            v               v
[Verdict 1] [Verdict 2] [Verdict 3] ... [Verdict N]
    |            |            |               |
    +------------+------------+---------------+
                        |
                        v
                  Reassemble document
```

### Implementation Strategy

```python
async def evaluate_comments_parallel(
    comments: list[CommentContext],
    llm: AugmentedLLM,
    max_concurrency: int = 10,
) -> list[CommentVerdict]:
    semaphore = asyncio.Semaphore(max_concurrency)

    async def evaluate_one(ctx: CommentContext) -> CommentVerdict:
        # Short-circuit deterministic cases first
        if ctx.new_section is None:
            return CommentVerdict(comment_ref=ctx.ref, decision="DROP", confidence=1.0, ...)
        if ctx.anchor_text in ctx.new_section:
            return CommentVerdict(comment_ref=ctx.ref, decision="KEEP", confidence=1.0, ...)

        async with semaphore:
            return await _llm_evaluate_comment(llm, ctx)

    return await asyncio.gather(*[evaluate_one(c) for c in comments])
```

### Concurrency Considerations

| Concern | Mitigation |
|---------|------------|
| API rate limits | Semaphore (max 10 concurrent by default); configurable |
| Token budget | Per-comment calls are tiny (~200-500 tokens each); 50 comments = ~15K-25K tokens total |
| Latency | With 10 parallel slots, 50 comments complete in ~5 batches of ~200ms = ~1s total |
| Error handling | Individual failures should default to KEEP (conservative); log warning |
| Cost | 50 comments at gpt-4o-mini pricing: ~$0.001-0.003 total |

### Comparison with Current Approach

| Metric | Current (full-page 3-phase) | Proposed (per-comment parallel) |
|--------|---------------------------|-------------------------------|
| LLM calls | 3 (merge + reflect + critic) | N (one per ambiguous comment) + 0-1 for final assembly |
| Input tokens per call | 10K-100K+ (full page x2 per call) | 200-800 per comment |
| Total tokens | 30K-300K+ | 5K-25K for 50 comments |
| Latency | 15-60s (serial chain) | 1-5s (parallel, small models) |
| Cost | $0.10-1.00+ per update | $0.001-0.01 per update |
| Comment intelligence | None (preserve all or nothing) | Per-comment semantic evaluation |

### Complexity: Low-Medium

The asyncio.gather pattern is straightforward. The semaphore handles rate limiting. Error handling follows the existing retry pattern.

---

## Feature 4: Document Reassembly from Verdicts

### What It Is

After all per-comment verdicts are collected, apply them to produce the final merged document.

### Strategy: Start from New Content, Re-inject Surviving Comments

This is the critical insight: **do NOT ask the LLM to produce merged XML**. Instead:

1. Start with the new content (already in storage format from `convert_markdown_to_storage()`)
2. For each KEEP verdict: find the anchor text in the new content, wrap it with the original `<ac:inline-comment-marker>` tag
3. For each RELOCATE verdict: find the new anchor text, wrap it with the original marker (preserving the `ac:ref` UUID)
4. For each DROP verdict: do nothing (comment is omitted from new content)

### Anchor Text Matching for Re-injection

This is the trickiest part. The anchor text from the original may not appear verbatim in the new content due to:

- Minor reformatting (whitespace, entity encoding)
- Semantic tags changing (`<strong>` vs `<b>`)
- Text rewriting (paraphrasing)

**Matching hierarchy:**

1. **Exact match**: `anchor_text in new_section_text` -- use directly
2. **Normalized match**: Strip XML tags from both, normalize whitespace, compare plain text
3. **LLM-provided anchor** (for RELOCATE verdicts): The evaluation prompt already asked for `relocated_anchor` -- use that
4. **Fuzzy match** (fallback): Use difflib.SequenceMatcher or similar to find the closest substring in the new section with ratio > 0.8

### XML Manipulation

Use `lxml.etree` for surgical insertion of comment markers into the storage format tree. Do NOT use regex for XML manipulation -- it is too fragile for nested structures.

```python
# Pseudocode for re-injection
for verdict in verdicts:
    if verdict.decision == "KEEP":
        # Find the text node containing anchor_text in the new content tree
        # Wrap it with the original <ac:inline-comment-marker> element
        wrap_text_with_marker(new_tree, verdict.anchor_text, verdict.marker_xml)
    elif verdict.decision == "RELOCATE":
        wrap_text_with_marker(new_tree, verdict.relocated_anchor, verdict.marker_xml)
    # DROP: do nothing
```

### Edge Cases

| Edge Case | Handling |
|-----------|----------|
| Anchor text appears multiple times in new content | Use section context to disambiguate; if still ambiguous, attach to first occurrence |
| Anchor text spans multiple XML elements | Preserve the span -- use the original marker's structure as template |
| Overlapping comments (two markers on same text) | Process in document order; inner markers first |
| RELOCATE anchor not found in new content | Fallback to DROP with warning log |
| All comments dropped | Valid outcome -- proceed with clean new content |

### Complexity: High

This is the hardest feature. XML tree manipulation with text node splitting is inherently fiddly. Thorough test coverage with edge cases is essential.

---

## Feature 5: Integration with Existing Pipeline

### What It Is

Replace the current 3-phase LLM pipeline with the per-comment evaluation when the goal is comment preservation.

### Proposed Architecture Change

```
BEFORE:
  new_content + original_content
       |
       v
  [MERGE LLM] --> [REFLECT LLM] --> [CRITIC LLM]
       |                                    |
       v                                    v
  merged_content                    final_content

AFTER:
  new_content + original_content
       |
       v
  [Extract comments + context]
       |
       v
  [Short-circuit deterministic cases]
       |
       v
  [Parallel per-comment LLM evaluation] (small/fast model)
       |
       v
  [Collect verdicts]
       |
       v
  [Reassemble: new_content + surviving comments]
       |
       v
  final_content
```

### What This Replaces

The entire `_process_content_with_llm()` function (merge + reflect + critic chain). The new pipeline:

- Does NOT use the LLM for content merging (new content is authoritative)
- Does NOT need reflection or critic phases (per-comment evaluation is self-contained)
- Uses the LLM ONLY for the semantic question: "is this comment still relevant?"

### Backward Compatibility

The `update_confluence_page()` function already has the right branching:

- Empty page -> bypass LLM (unchanged)
- No inline comments -> bypass LLM (unchanged)
- Has inline comments -> **replace** `_process_content_with_llm()` with new per-comment pipeline

### Complexity: Medium

The integration point is clean. The main risk is regressions in edge cases that the current 3-phase pipeline handles implicitly.

---

## Anti-Features

Features to explicitly NOT build.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| LLM-generated merged XML | LLMs are unreliable at preserving XML structure byte-for-byte; current 3-phase approach exists precisely because of this fragility | Use deterministic XML manipulation for reassembly; LLM only for classification |
| Full-page context per comment | Defeats the purpose of per-comment evaluation; expensive, slow | Section-level context is sufficient |
| Comment thread resolution | Confluence comments have thread replies via API, not inline markers | Out of scope -- inline markers are the only concern |
| Automatic comment creation | Adding new comments based on content changes | Out of scope -- only evaluate existing comments |
| Comment content modification | Rewriting comment text to match new content | Never modify comment content -- only decide KEEP/DROP/RELOCATE for the marker |

---

## Feature Dependencies

```
Feature 1 (Context Extraction) --> Feature 2 (Evaluation Prompt)
Feature 1 (Context Extraction) --> Feature 4 (Reassembly)
Feature 2 (Evaluation Prompt)  --> Feature 3 (Parallel Evaluation)
Feature 3 (Parallel Evaluation) --> Feature 4 (Reassembly)
Feature 4 (Reassembly)         --> Feature 5 (Pipeline Integration)
```

All features are required for the complete solution. Features 1 and 2 can be developed and tested independently. Feature 4 (Reassembly) is the highest-risk component.

---

## MVP Recommendation

### Phase 1: Deterministic Short-Circuits + Section Extraction

Build Features 1 and the short-circuit logic from Feature 2 first. This alone provides significant value:

1. Parse original content into sections
2. Match sections between original and new content
3. For each comment: if section deleted -> DROP; if anchor text exact match -> KEEP; if section unchanged -> KEEP
4. Only comments in changed sections with non-matching anchor text need LLM evaluation
5. For MVP, default these ambiguous cases to KEEP (conservative)

**Value**: Eliminates the expensive 3-phase LLM pipeline for pages where most comments are in unchanged sections. Zero LLM cost for the common case.

### Phase 2: LLM-Based Evaluation for Ambiguous Cases

Add Feature 2 (prompt) and Feature 3 (parallel evaluation) to handle the comments that couldn't be resolved deterministically.

### Phase 3: Intelligent Reassembly

Add Feature 4 (reassembly) to handle RELOCATE verdicts and fuzzy anchor matching.

### Defer

- **RELOCATE verdict support**: Start with KEEP/DROP only. RELOCATE requires the hardest part of reassembly (finding new anchor positions). Add it after KEEP/DROP is solid.

---

## Prior Art in Document Merge with Annotation Preservation

### Google Docs Suggestion Resolution

Google Docs tracks suggestions (similar to comments) with anchor ranges. When content is edited, the system uses operational transforms (OT) to adjust anchor positions. The key insight: anchors are defined by character offsets, and OT maintains a mapping of "old offset -> new offset" through each edit operation. This is deterministic, not LLM-based.

**Applicable insight**: For comments where the anchor text survives verbatim, offset-based relocation is more reliable than LLM evaluation. Use LLM only when text actually changed.

### Microsoft Word Track Changes

Word's revision tracking maintains anchor associations through a similar offset adjustment system. When text is deleted, comments on that text are flagged as "orphaned" and shown in a sidebar rather than silently dropped.

**Applicable insight**: Consider a "orphaned comments" report rather than silent DROP. Log which comments were dropped and why, so users can review.

### diff3 and Operational Transform

Traditional 3-way merge tools (diff3, Git's merge strategy) handle this at the text level. They identify corresponding regions between base, ours, and theirs using longest common subsequence algorithms. Annotations/comments are not natively supported, but the region-mapping approach is relevant.

**Applicable insight**: Use difflib (Python stdlib) to build a correspondence map between original and new sections before involving the LLM. This can identify exact-match regions, modified regions, and deleted regions programmatically.

### CKEditor Track Changes

CKEditor 5 (the editor Confluence uses internally) has a track changes feature that maintains suggestion markers through content edits. Their approach uses a tree-walking algorithm that adjusts marker positions after each edit operation.

**Applicable insight**: Confluence's own editor handles this during live editing. The challenge is specifically in the "offline" merge case (markdown -> storage format), where we don't have the sequence of edit operations, only the before and after states.

### Confidence: MEDIUM

Prior art observations are from training data, not verified against current documentation. The core patterns (OT for offset adjustment, LCS for region correspondence, conservative defaults for ambiguous cases) are well-established computer science concepts regardless.

---

## Sources

- Codebase analysis: `agent.py`, `llm_prompts.py`, `converter.py`, `llm.py`, `models.py`, `test_agent.py`
- Prior art patterns: Based on established document merge and OT literature (training data; MEDIUM confidence)
- Model pricing/latency estimates: Based on known model characteristics as of early 2025 (may be outdated; LOW confidence on specific numbers)
