class _GoogleFakePart:
    def __init__(self, text: str | None = None) -> None:
        self.text = text


class _GoogleFakeContent:
    def __init__(self, parts: list[_GoogleFakePart] | None = None) -> None:
        self.parts = parts or []


class _GoogleFakeCandidate:
    def __init__(self, content: _GoogleFakeContent | None = None) -> None:
        self.content = content


class _GoogleFakeResponse:
    def __init__(self, candidates: list[_GoogleFakeCandidate] | None = None) -> None:
        self.candidates = candidates or []


class _OpenAIFakeMessage:
    def __init__(self, content) -> None:
        self.content = content


class _OpenAIFakeChoice:
    def __init__(self, message: _OpenAIFakeMessage) -> None:
        self.message = message


class _OpenAIFakeCompletion:
    def __init__(self, choices: list[_OpenAIFakeChoice] | None = None) -> None:
        self.choices = choices or []


def test_extract_structured_json_text_google_concatenates_parts() -> None:
    """
    Gemini can return JSON split across multiple content parts.
    We must concatenate part.text entries, in order, before parsing JSON.
    """
    from confluence_agent.structured_output import extract_structured_json_text

    response = _GoogleFakeResponse(
        candidates=[
            _GoogleFakeCandidate(
                content=_GoogleFakeContent(
                    parts=[
                        _GoogleFakePart(text='{"content":"hello '),
                        _GoogleFakePart(text='world"}'),
                    ]
                )
            )
        ]
    )

    assert extract_structured_json_text(response) == '{"content":"hello world"}'


def test_extract_structured_json_text_google_keeps_internal_whitespace() -> None:
    from confluence_agent.structured_output import extract_structured_json_text

    response = _GoogleFakeResponse(
        candidates=[
            _GoogleFakeCandidate(
                content=_GoogleFakeContent(
                    parts=[
                        _GoogleFakePart(text="  "),
                        _GoogleFakePart(text='{"k":'),
                        _GoogleFakePart(text=' "v"}  '),
                    ]
                )
            )
        ]
    )

    # We strip only the outside whitespace; interior whitespace is preserved.
    assert extract_structured_json_text(response) == '{"k": "v"}'


def test_extract_structured_json_text_openai_content_str() -> None:
    from confluence_agent.structured_output import extract_structured_json_text

    completion = _OpenAIFakeCompletion(
        choices=[_OpenAIFakeChoice(message=_OpenAIFakeMessage(content='{"a":1}'))]
    )
    assert extract_structured_json_text(completion) == '{"a":1}'


def test_extract_structured_json_text_openai_content_blocks_list() -> None:
    """
    Some OpenAI clients/models can return content as blocks (list) rather than a string.
    We should concatenate all text-like blocks.
    """
    from confluence_agent.structured_output import extract_structured_json_text

    completion = _OpenAIFakeCompletion(
        choices=[
            _OpenAIFakeChoice(
                message=_OpenAIFakeMessage(
                    content=[
                        {"type": "text", "text": '{"x":"hel'},
                        {"type": "text", "text": 'lo"}'},
                    ]
                )
            )
        ]
    )
    assert extract_structured_json_text(completion) == '{"x":"hello"}'


def test_extract_structured_json_text_no_candidates_or_choices() -> None:
    from confluence_agent.structured_output import extract_structured_json_text

    assert extract_structured_json_text(_GoogleFakeResponse(candidates=[])) is None
    assert extract_structured_json_text(_OpenAIFakeCompletion(choices=[])) is None
