---
name: confluence-update
description: Update a Confluence page from a local Markdown file using the full merge pipeline (preserves inline comments). Use when the user wants to sync a .md file to Confluence.
argument-hint: "<markdown_path> <confluence_page_url>"
disable-model-invocation: true
allowed-tools: Bash(confluence-agent *)
---

Update a Confluence page from a Markdown file, preserving existing inline comments.

Run the following command and show me the output:

```bash
confluence-agent update "$0" "$1" --output json
```

Report the result to the user:

- On success: show the page URL and the number of comments kept/dropped
- On failure: show the error message and suggest checking credentials (CONFLUENCE_API_TOKEN, ANTHROPIC_API_KEY)
