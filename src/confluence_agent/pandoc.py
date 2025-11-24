import subprocess


def markdown_to_confluence_storage(markdown_file_path: str) -> str:
    """
    Converts a markdown file to Confluence storage format (Jira wiki format) using pandoc.

    Args:
        markdown_file_path: The path to the markdown file.

    Returns:
        The Confluence storage format as a string.

    Raises:
        FileNotFoundError: If pandoc is not installed or not in the system's PATH.
        subprocess.CalledProcessError: If pandoc fails to convert the file.
    """
    command = [
        "pandoc",
        "--from=markdown",
        "--to=jira",  # Confluence storage format is the same as Jira's classic wiki format
        markdown_file_path,
    ]
    try:
        result = subprocess.run(command, capture_output=True, text=True, check=False)
        if result.returncode != 0:
            raise subprocess.CalledProcessError(
                result.returncode, command, output=result.stdout, stderr=result.stderr
            )
        return result.stdout
    except FileNotFoundError as e:
        raise FileNotFoundError(
            "pandoc not found. Please ensure pandoc is installed and in your PATH."
        ) from e
