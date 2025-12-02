import pytest
from typer.testing import CliRunner
from unittest.mock import patch, MagicMock, ANY
from pathlib import Path

from confluence_agent.cli import app
from confluence_agent.confluence import ConfluenceClient

runner = CliRunner()


@pytest.fixture
def mock_settings():
    with patch("confluence_agent.cli.Settings") as mock:
        mock_instance = MagicMock()
        mock.return_value = mock_instance
        mock_instance.plantuml_jar_path = "plantuml.jar"
        yield mock_instance


@pytest.fixture
def mock_confluence_client():
    with patch("confluence_agent.cli.ConfluenceClient", spec=ConfluenceClient) as mock:
        mock_instance = MagicMock(spec=ConfluenceClient)
        mock.return_value = mock_instance
        # This now correctly reflects the actual function's return signature
        mock_instance.get_page_content_and_version.return_value = (
            "content",
            1,
            "Test Page",
        )
        yield mock_instance


@pytest.fixture
def mock_converter():
    with patch(
        "confluence_agent.cli.convert_markdown_to_storage", new_callable=MagicMock
    ) as mock:
        mock.return_value = ("storage", [])
        yield mock


@pytest.fixture
def mock_get_page_id():
    with patch("confluence_agent.cli.ConfluenceClient._get_page_id_from_url") as mock:
        mock.return_value = "12345"
        yield mock


def test_upload_command_with_attachments(
    mock_settings,
    mock_confluence_client,
    mock_converter,
    mock_get_page_id,
):
    # Arrange
    attachments = [("diagram.png", b"imagedata")]
    mock_converter.return_value = ("storage", attachments)

    # Act
    runner.invoke(
        app,
        [
            "upload",
            "tests/fixtures/test.md",
            "http://example.com/display/SPACE/Page+Title",
        ],
    )

    # Assert
    mock_confluence_client.upload_attachments.assert_called_once()
    args, _ = mock_confluence_client.upload_attachments.call_args
    assert args[0] == "12345"
    uploaded_attachments = args[1]
    assert len(uploaded_attachments) == 1
    filepath, content = uploaded_attachments[0]
    assert Path(filepath).name == "diagram.png"
    assert content == b"imagedata"


def test_upload_command_with_plantuml_path(
    mock_settings,
    mock_confluence_client,
    mock_converter,
):
    result = runner.invoke(
        app,
        [
            "upload",
            "tests/fixtures/test.md",
            "http://example.com/display/SPACE/Page+Title",
        ],
    )
    assert result.exit_code == 0
    mock_converter.assert_called_with(
        "# Test Markdown\n\nThis is a test file.\n",
        mock_settings,
        ANY,
    )


def test_upload_command_handles_unpacking_correctly(
    mock_settings,
    mock_confluence_client,
    mock_converter,
):
    """
    This test is designed to fail with the original bug and pass with the fix.
    """
    result = runner.invoke(
        app,
        [
            "upload",
            "tests/fixtures/test.md",
            "http://example.com/display/SPACE/Page+Title",
        ],
    )
    assert result.exit_code == 0
    assert "Success" in result.stdout


@patch("confluence_agent.cli.update_confluence_page")
def test_update_command_success(mock_update_confluence_page):
    """
    Tests that the update command runs successfully.
    """
    mock_update_confluence_page.return_value = "Success"
    result = runner.invoke(
        app,
        [
            "update",
            "tests/fixtures/test.md",
            "http://example.com/display/SPACE/Page+Title",
        ],
    )
    assert result.exit_code == 0
    assert "Success" in result.stdout
