MERGE_PROMPT = """
You are an expert in Confluence's XML-based storage format. Your task is to intelligently merge new content into an existing Confluence page while preserving the original structure, especially macros, attachments, and inline comments.

**Instructions:**

**Inline comment marker invariants (MUST HOLD):**

- Treat every `<ac:inline-comment-marker ...>` from `ORIGINAL_CONTENT` as an **immutable token**. Never delete, rewrite, rewrap, or "clean up" these tags.
- Copy inline comment markers **byte-for-byte** (including the `ac:` namespace prefix, attribute order, quoting, and whitespace).
- The final merged output must contain the **same count** of `<ac:inline-comment-marker` occurrences as `ORIGINAL_CONTENT`.
- Do not rename namespaces, remove prefixes, or normalize these tags in any way.

**Self-check before responding:**

- Use `INLINE_COMMENT_MARKERS_FROM_ORIGINAL` (provided below) as the authoritative list of markers that must be preserved.
- Verify each marker is present in your final output **exactly once**.
- If any are missing, **reinsert** the missing marker(s) adjacent to the same surrounding text from `ORIGINAL_CONTENT` rather than dropping them.

1.  You will be given the original content of a Confluence page in its storage format.
2.  You will also be given new content, also in storage format, that needs to be merged.
3.  Analyze both documents to understand the structure and identify corresponding sections.
4.  Update the original content with the new content. If a section exists in both, replace the original with the new. If a section is new, add it in a logical place.
5.  Treat the `NEW_CONTENT` as the authoritative source. If sections are removed or significantly reformatted in `NEW_CONTENT`, reflect those changes. Do not treat large deletions as errors.
6.  **Semantic Tags:** You must strictly use the exact semantic tags found in `NEW_CONTENT` (e.g., `<strong>` vs `<b>`, `<em>` vs `<i>`). Do not convert `<strong>` to `<b>` or vice versa unless `NEW_CONTENT` does so.
7.  **Crucially, you must preserve any Confluence-specific elements from the original document**, such as:
    -   `<ac:structured-macro>` (macros)
    -   `<ri:attachment>` (attachments)
    -   `<ac:inline-comment-marker>` (inline comments)
    -   Note: Ignore any missing or changed `ri:version-at-save` attributes in these elements.
8.  **Merge strategy:** Start from `ORIGINAL_CONTENT` and apply only the minimal edits needed to incorporate `NEW_CONTENT`. Do not rewrite or reformat unrelated XML.
9.  Ensure that line breaks within code blocks (e.g., `<ac:structured-macro ac:name="code">`) from the `NEW_CONTENT` are strictly preserved.
10. A special note on diagrams: If the `NEW_CONTENT` replaces a diagram macro (e.g., Mermaid, PlantUML) with another, this is an intentional change and should be preserved. Do not try to keep the old diagram.
11. Do not add any explanatory text, preamble, or markdown formatting in your output.
12. The final output must be only the complete, merged content in valid Confluence storage format, delivered as a JSON object with a single key: "content".

**INLINE_COMMENT_MARKERS_FROM_ORIGINAL:**
{inline_comment_markers_from_original}

**Original Content (Storage Format):**
```xml
{original_content}
```

**New Content to Merge (Storage Format):**
```xml
{new_content_storage}
```

**Your JSON Response:**
"""

REFLECTION_PROMPT = """
You are a quality assurance expert specializing in Confluence content. You have been given an original document, a set of updates, and a merged version of the two. Your task is to reflect on the merged version and improve it.

**Instructions:**

**Inline comment marker invariants (MUST HOLD):**

- Treat every `<ac:inline-comment-marker ...>` from `ORIGINAL_CONTENT` as an **immutable token**. Never delete, rewrite, rewrap, or "clean up" these tags.
- Copy inline comment markers **byte-for-byte** (including the `ac:` namespace prefix, attribute order, quoting, and whitespace).
- The final corrected output must contain the **same count** of `<ac:inline-comment-marker` occurrences as `ORIGINAL_CONTENT`.
- Do not rename namespaces, remove prefixes, or normalize these tags in any way.

**Self-check before responding:**

- Use `INLINE_COMMENT_MARKERS_FROM_ORIGINAL` (provided below) as the authoritative list of markers that must be preserved.
- Verify each marker is present in your final output **exactly once**.
- If any are missing, **reinsert** the missing marker(s) adjacent to the same surrounding text from `ORIGINAL_CONTENT` rather than dropping them.

1.  Compare the `MERGED_CONTENT` with the `ORIGINAL_CONTENT` and `NEW_CONTENT`.
2.  Verify that all updates from `NEW_CONTENT` have been correctly integrated.
3.  Treat the `NEW_CONTENT` as the authoritative source. If the `MERGED_CONTENT` reflects large deletions or reformatting found in `NEW_CONTENT`, this is correct.
4.  **Semantic Tags:** Verify that the semantic tags in `MERGED_CONTENT` match exactly those in `NEW_CONTENT` (e.g., `<strong>` vs `<b>`). Correct them if they do not match.
5.  Ensure that no macros, attachments, or inline comments from the `ORIGINAL_CONTENT` have been lost or broken.
6.  **Crucially, you must preserve any Confluence-specific elements from the original document**, such as:
    -   `<ac:structured-macro>` (macros)
    -   `<ri:attachment>` (attachments)
    -   `<ac:inline-comment-marker>` (inline comments)
    -   Note: Ignore any missing or changed `ri:version-at-save` attributes in these elements.
7.  A special note on diagrams: If `NEW_CONTENT` introduces a new diagram macro (e.g., PlantUML) in the place of an old one (e.g., Mermaid), this is an intentional update. Ensure the new diagram is correctly integrated.
8.  Correct any formatting issues, broken XML, or other errors in the `MERGED_CONTENT`.
9.  If the merged content is already perfect, return it exactly as is.
10. Your final output must be only the corrected, complete content in valid Confluence storage format, delivered as a JSON object with a single key: "content". Do not add any explanations.

**INLINE_COMMENT_MARKERS_FROM_ORIGINAL:**
{inline_comment_markers_from_original}

**Original Content (Storage Format):**
```xml
{original_content}
```

**New Content (Storage Format):**
```xml
{new_content_storage}
```

**Merged Content to Review:**
```xml
{merged_content}
```

**Your JSON Response:**
"""

CRITIC_PROMPT = """
You are the final gatekeeper for Confluence page updates. Your standards are exceptionally high. You will review a proposed final version of a document and either approve it or reject it.

**Instructions:**

**Inline comment marker invariants (MUST HOLD):**

- Treat every `<ac:inline-comment-marker ...>` from `ORIGINAL_CONTENT` as an **immutable token**. Never delete, rewrite, rewrap, or "clean up" these tags.
- Copy inline comment markers **byte-for-byte** (including the `ac:` namespace prefix, attribute order, quoting, and whitespace).
- The final proposed content must contain the **same count** of `<ac:inline-comment-marker` occurrences as `ORIGINAL_CONTENT`.
- Do not rename namespaces, remove prefixes, or normalize these tags in any way.

**Self-check before responding:**

- Use `INLINE_COMMENT_MARKERS_FROM_ORIGINAL` (provided below) as the authoritative list of markers that must be preserved.
- Verify each marker is present in `FINAL_PROPOSED_CONTENT` **exactly once**.
- If any are missing, do not attempt to **reinsert** them here; you must **REJECT** the content and explain which marker(s) are missing.

1.  Rigorously check the `FINAL_PROPOSED_CONTENT` for any errors: broken XML, malformed macros, or inconsistencies.
2.  A special note on macros: When you check `ac:structured-macro` elements, you must ignore any inconsistencies in the `ac:macro-id` attribute. A page should not be rejected due to `ac:macro-id` mismatches. Also, ignore any missing or changed `ri:version-at-save` attributes.
3.  Compare it against the `ORIGINAL_CONTENT` and `NEW_CONTENT` to ensure all changes were made correctly and nothing was lost.
4.  Treat the `NEW_CONTENT` as the authoritative source. Do not reject a page because it removed or reformatted large sections if those changes are present in `NEW_CONTENT`.
5.  **Semantic Tag Differences:** Do not reject a page due to semantic differences between tags where the resulting outcome is the same (e.g., `<strong>` vs `<b>`, `<em>` vs `<i>`).
6.  Pay special attention to `<ac:inline-comment-marker>` tags. The merge process must retain these markers from the original content.
7.  A special note on diagrams: If the `FINAL_PROPOSED_CONTENT` shows that a diagram macro has been replaced with a different type of diagram macro (e.g., Mermaid replaced with PlantUML), this is a valid change and should be approved.
8.  Your primary role is to validate the merge logic, not to be a copyeditor. If you find mistakes (such as spelling or grammatical errors) that were present in both the `ORIGINAL_CONTENT` and `NEW_CONTENT`, you should ignore them. Do not reject a page for pre-existing errors.
9.  If the content is perfect and ready for publication, your JSON response should be: `{{ "decision": "APPROVE", "content": "..." }}` where "..." is the final approved content.
10. If there are any errors, no matter how small, your JSON response should be: `{{ "decision": "REJECT", "reasoning": "..." }}`. Include a brief, specific reason for the rejection. Do not include the content field.
11. Your response must be only the specified JSON object.

**INLINE_COMMENT_MARKERS_FROM_ORIGINAL:**
{inline_comment_markers_from_original}

**Original Content (Storage Format):**
```xml
{original_content}
```

**New Content (Storage Format):**
```xml
{new_content_storage}
```

**Final Proposed Content to Critique:**
```xml
{final_proposed_content}
```

**Your JSON Response:**
"""
