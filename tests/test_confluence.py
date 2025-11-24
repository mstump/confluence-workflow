from typing import Generator
from unittest.mock import MagicMock, patch

import pytest
from atlassian.errors import ApiError

from confluence_agent.confluence import ConfluenceClient


@pytest.fixture
def mock_confluence_api() -> Generator[MagicMock, None, None]:
    """Fixture to create a mock Confluence API object."""
    with patch("confluence_agent.confluence.Confluence") as mock_confluence_class:
        instance = MagicMock()
        mock_confluence_class.return_value = instance
        yield instance


def test_confluence_client_initialization(mock_confluence_api: MagicMock) -> None:
    """Tests that the Confluence client initializes the API wrapper correctly."""
    ConfluenceClient(
        url="https://test.atlassian.net",
        username="user",
        api_token="token",
    )
    # The mock_confluence_api fixture now correctly patches where Confluence is instantiated
    # and we can assert it was called as expected.
    from confluence_agent.confluence import Confluence

    Confluence.assert_called_once_with(
        url="https://test.atlassian.net",
        username="user",
        password="token",
        cloud=True,
    )


def test_get_page_id_from_url() -> None:
    """Tests the extraction of the page ID from various Confluence URL formats."""
    urls = {
        "https://domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Page+Title": "12345",
        "https://domain.atlassian.net/wiki/spaces/SPACE/pages/edit-v2/67890?draftShareId=abc": "67890",
        "https://domain.atlassian.net/wiki/pages/viewpage.action?pageId=54321": "54321",
    }
    for url, expected_id in urls.items():
        assert ConfluenceClient._get_page_id_from_url(url) == expected_id


def test_get_page_id_from_url_invalid() -> None:
    """Tests that an invalid Confluence URL raises a ValueError."""
    with pytest.raises(ValueError, match="Could not extract page ID from URL"):
        ConfluenceClient._get_page_id_from_url("https://invalid.url")


def test_get_page_content_and_version_success(mock_confluence_api: MagicMock) -> None:
    """Tests successfully fetching page content and version."""
    mock_confluence_api.get_page_by_id.return_value = {
        "body": {"storage": {"value": "<p>Content</p>"}},
        "version": {"number": 2},
        "title": "Test Page",
    }
    client = ConfluenceClient("https://test.atlassian.net", "user", "token")
    # We now need to assign the mock to the client's confluence attribute
    client.confluence = mock_confluence_api
    content, version, title = client.get_page_content_and_version("12345")

    mock_confluence_api.get_page_by_id.assert_called_once_with(
        "12345", expand="body.storage,version"
    )
    assert content == "<p>Content</p>"
    assert version == 2
    assert title == "Test Page"


def test_get_page_content_and_version_failure(mock_confluence_api: MagicMock) -> None:
    """Tests failure in fetching page content."""
    mock_confluence_api.get_page_by_id.side_effect = ApiError("API Error")
    client = ConfluenceClient("https://test.atlassian.net", "user", "token")
    client.confluence = mock_confluence_api

    with pytest.raises(ApiError, match="API Error"):
        client.get_page_content_and_version("12345")


def test_update_page_content_success(mock_confluence_api: MagicMock) -> None:
    """Tests successfully updating a page's content."""
    client = ConfluenceClient("https://test.atlassian.net", "user", "token")
    client.confluence = mock_confluence_api
    client.update_page_content("12345", "Test Page", "<p>New Content</p>", 3)

    mock_confluence_api.update_page.assert_called_once_with(
        page_id="12345",
        title="Test Page",
        body="<p>New Content</p>",
        parent_id=None,
        type="page",
        representation="storage",
        minor_edit=True,
    )


def test_update_page_content_failure(mock_confluence_api: MagicMock) -> None:
    """Tests failure in updating a page's content."""
    mock_confluence_api.update_page.side_effect = ApiError("API Error")
    client = ConfluenceClient("https://test.atlassian.net", "user", "token")
    client.confluence = mock_confluence_api

    with pytest.raises(ApiError, match="API Error"):
        client.update_page_content("12345", "Test Page", "<p>New Content</p>", 3)
