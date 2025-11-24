from pydantic import BaseModel, Field
from typing import Literal


class ConfluenceContent(BaseModel):
    """Data model for Confluence page content."""

    content: str = Field(..., description="The Confluence content in storage format.")


class CriticResponse(BaseModel):
    """Data model for the critic's response."""

    decision: Literal["APPROVE", "REJECT"] = Field(
        ..., description="The decision of the critic."
    )
    content: str | None = Field(
        None, description="The content to be published if the decision is APPROVE."
    )
