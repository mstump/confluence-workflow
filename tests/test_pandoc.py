import subprocess
from unittest.mock import patch, MagicMock

import pytest

from confluence_agent.pandoc import markdown_to_confluence_storage


@patch("subprocess.run")
def test_markdown_to_confluence_storage_success(mock_subprocess_run):
    """Tests successful conversion of a markdown file to Confluence storage format."""
    mock_process = MagicMock()
    mock_process.returncode = 0
    mock_process.stdout = "<p>Hello, World!</p>"
    mock_process.stderr = ""
    mock_subprocess_run.return_value = mock_process

    markdown_path = "test.md"
    result = markdown_to_confluence_storage(markdown_path)

    expected_command = [
        "pandoc",
        "--from=markdown",
        "--to=jira",
        markdown_path,
    ]
    mock_subprocess_run.assert_called_once_with(
        expected_command, capture_output=True, text=True, check=False
    )
    assert result == "<p>Hello, World!</p>"


@patch("subprocess.run")
def test_markdown_to_confluence_storage_pandoc_not_found(mock_subprocess_run):
    """Tests that a FileNotFoundError is raised if pandoc is not installed."""
    mock_subprocess_run.side_effect = FileNotFoundError("pandoc not found")

    with pytest.raises(FileNotFoundError, match="pandoc not found"):
        markdown_to_confluence_storage("test.md")


@patch("subprocess.run")
def test_markdown_to_confluence_storage_conversion_error(mock_subprocess_run):
    """Tests that a CalledProcessError is raised on a pandoc conversion error."""
    mock_process = MagicMock()
    mock_process.returncode = 1
    mock_process.stdout = ""
    mock_process.stderr = "pandoc: Error converting file"
    mock_subprocess_run.return_value = mock_process

    with pytest.raises(subprocess.CalledProcessError) as excinfo:
        markdown_to_confluence_storage("test.md")

    assert (
        "Command '['pandoc', '--from=markdown', '--to=jira', 'test.md']' returned non-zero exit status 1."
        in str(excinfo.value)
    )
