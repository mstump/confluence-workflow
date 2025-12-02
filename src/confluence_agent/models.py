from pydantic import BaseModel, Field
from typing import Literal, Optional


class ConfluenceContent(BaseModel):
    """Data model for Confluence page content."""

    content: str = Field(..., description="The Confluence content in storage format.")


class CriticResponse(BaseModel):
    """Data model for the critic's response."""

    decision: str
    reasoning: Optional[str] = None
    content: Optional[str] = None
