import re
from typing import Tuple

from atlassian import Confluence
from atlassian.errors import ApiError


class ConfluenceClient:
    """A client for interacting with the Confluence API."""

    def __init__(self, url: str, username: str, api_token: str):
        """
        Initializes the Confluence client.

        Args:
            url: The URL of the Confluence instance.
            username: The username for authentication.
            api_token: The API token for authentication.
        """
        self.confluence = Confluence(
            url=url,
            username=username,
            password=api_token,
            cloud=True,
        )

    @staticmethod
    def _get_page_id_from_url(url: str) -> str:
        """
        Extracts the page ID from a Confluence page URL.

        Args:
            url: The URL of the Confluence page.

        Returns:
            The extracted page ID.

        Raises:
            ValueError: If the page ID cannot be extracted from the URL.
        """
        # Regex for various Confluence URL formats
        patterns = [
            r"/pages/(\d+)",  # /pages/12345/...
            r"/pages/edit-v2/(\d+)",  # /pages/edit-v2/12345/...
            r"pageId=(\d+)",  # /viewpage.action?pageId=54321
        ]
        for pattern in patterns:
            match = re.search(pattern, url)
            if match:
                return match.group(1)
        raise ValueError(f"Could not extract page ID from URL: {url}")

    def get_page_content_and_version(self, page_id: str) -> Tuple[str, int, str]:
        """
        Retrieves the content, version, and title of a Confluence page.

        Args:
            page_id: The ID of the page to retrieve.

        Returns:
            A tuple containing the page content in storage format, the current version number, and the title.

        Raises:
            ConfluenceError: If the API call fails.
        """
        try:
            page = self.confluence.get_page_by_id(
                page_id, expand="body.storage,version"
            )
            content = page["body"]["storage"]["value"]
            version = page["version"]["number"]
            title = page["title"]
            return content, version, title
        except ApiError as e:
            raise ApiError(f"Failed to get page content for page ID {page_id}: {e}")

    def update_page_content(
        self,
        page_id: str,
        title: str,
        new_content: str,
        new_version: int,
        representation: str = "storage",
    ) -> None:
        """
        Updates the content of a Confluence page.

        Args:
            page_id: The ID of the page to update.
            title: The title of the page.
            new_content: The new content for the page.
            new_version: The new version number for the page.
            representation: The content representation format, either "storage" or "wiki". Defaults to "storage".

        Raises:
            ApiError: If the API call fails.
        """
        try:
            self.confluence.update_page(
                page_id=page_id,
                title=title,
                body=new_content,
                parent_id=None,
                type="page",
                representation=representation,
                minor_edit=True,
            )
        except ApiError as e:
            raise ApiError(f"Failed to update page content for page ID {page_id}: {e}")
