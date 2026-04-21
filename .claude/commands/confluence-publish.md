Publish the current markdown file to Confluence using the confluence-workflow CLI.

**Arguments:** `$ARGUMENTS` (optional: a Confluence page URL)

## Steps

1. Identify the markdown file to publish:
   - If the user has a file open in the editor, use that file.
   - Otherwise, ask the user which markdown file to publish.

2. Identify the target Confluence page URL:
   - If a URL was provided as `$ARGUMENTS`, use it.
   - Otherwise, check if the file has a Confluence URL in its YAML frontmatter (e.g., `confluence_url:`).
   - If neither, ask the user for the Confluence page URL.

3. Confirm with the user before running: show the markdown file path and the target URL.

4. Run the update command using the intelligent LLM merge workflow:

   ```bash
   confluence-workflow update '<markdown_path>' '<page_url>'
   ```

   If the user wants a direct upload (no LLM merge), use `upload` instead of `update`.

5. Report whether the publish succeeded or failed. On failure, show the error and suggest fixes.
