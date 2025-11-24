MERGE_PROMPT = """
You are an expert in Confluence's XML-based storage format. Your task is to intelligently merge new content into an existing Confluence page while preserving the original structure, especially macros, attachments, and inline comments.

**Instructions:**

1.  You will be given the original content of a Confluence page in its storage format.
2.  You will also be given new content, also in storage format, that needs to be merged.
3.  Analyze both documents to understand the structure and identify corresponding sections.
4.  Update the original content with the new content. If a section exists in both, replace the original with the new. If a section is new, add it in a logical place.
5.  **Crucially, you must preserve any Confluence-specific elements from the original document**, such as:
    -   `<ac:structured-macro>` (macros)
    -   `<ri:attachment>` (attachments)
    -   `<ac:inline-comment-marker>` (inline comments)
6.  Do not add any explanatory text, preamble, or markdown formatting in your output.
7.  The final output must be only the complete, merged content in valid Confluence storage format.

**Original Content (Storage Format):**
```xml
{original_content}
```

**New Content to Merge (Storage Format):**
```xml
{new_content_storage}
```

**Merged Content (Storage Format only):**
"""
