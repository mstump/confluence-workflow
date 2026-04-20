---
name: confluence-upload
description: Upload a Markdown file directly to a Confluence page, overwriting existing content without LLM merge. Use when the user wants a fast overwrite with no comment preservation.
argument-hint: "<markdown_path> <confluence_page_url>"
disable-model-invocation: true
allowed-tools: Bash(confluence-agent *)
---

Upload a Markdown file to Confluence (direct overwrite, no LLM merge).

Run the following command and show me the output:

```bash
confluence-agent upload "$0" "$1" --output json
```

Report the result to the user:

- On success: show the page URL
- On failure: show the error message and suggest checking credentials (CONFLUENCE_API_TOKEN, CONFLUENCE_URL)
