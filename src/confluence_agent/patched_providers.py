from __future__ import annotations

from typing import Any, Callable, Type, TypeVar, cast

from pydantic import BaseModel
from mcp_agent.workflows.llm.augmented_llm import RequestParams
from mcp_agent.workflows.llm.augmented_llm_google import GoogleAugmentedLLM
from mcp_agent.workflows.llm.augmented_llm_openai import OpenAIAugmentedLLM

from confluence_agent.structured_output import (
    extract_structured_json_text,
    safe_preview,
)

ModelT = TypeVar("ModelT")


class ChunkAwareGoogleAugmentedLLM(GoogleAugmentedLLM):
    async def generate_structured(
        self,
        message: Any,
        response_model: Type[ModelT],
        request_params: RequestParams | None = None,
    ) -> ModelT:
        """
        Same as mcp-agent's GoogleAugmentedLLM.generate_structured, but concatenates
        all text parts before JSON parsing to handle chunked responses.
        """
        import json

        # Import provider-specific types/tasks from mcp-agent implementation
        from google.genai import types
        from mcp_agent.workflows.llm.augmented_llm_google import (
            GoogleCompletionTasks,
            RequestCompletionRequest,
        )
        from mcp_agent.workflows.llm.multipart_converter_google import GoogleConverter
        from mcp_agent.tracing.telemetry import get_tracer
        from mcp_agent.tracing.semconv import GEN_AI_AGENT_NAME, GEN_AI_REQUEST_MODEL

        tracer = get_tracer(self.context)
        with tracer.start_as_current_span(
            f"{self.__class__.__name__}.{self.name}.generate_structured"
        ) as span:
            if self.context.tracing_enabled:
                span.set_attribute(GEN_AI_AGENT_NAME, self.agent.name)

            params: RequestParams = self.get_request_params(request_params)
            model = await self.select_model(params) or (
                params.model or "gemini-3-flash-preview"
            )
            if self.context.tracing_enabled:
                span.set_attribute(GEN_AI_REQUEST_MODEL, model)

            messages = GoogleConverter.convert_mixed_messages_to_google(message)

            model_cls = cast(type[BaseModel], response_model)

            try:
                schema: Any = model_cls.model_json_schema()
            except Exception:
                schema = None

            config = types.GenerateContentConfig(
                max_output_tokens=params.maxTokens,
                temperature=params.temperature,
                stop_sequences=params.stopSequences or [],
                system_instruction=self.instruction or params.systemPrompt,
            )
            config.response_mime_type = "application/json"
            config.response_schema = schema if schema is not None else response_model

            conversation: list[types.Content] = []
            if params.use_history:
                conversation.extend(self.history.get())
            if isinstance(messages, list):
                conversation.extend(messages)
            else:
                conversation.append(messages)

            if self.executor is None:
                raise RuntimeError("LLM executor is not configured.")
            if self.context.config is None or self.context.config.google is None:
                raise RuntimeError("Google provider configuration is not available.")

            api_response = await self.executor.execute(
                GoogleCompletionTasks.request_completion_task,
                RequestCompletionRequest(
                    config=self.context.config.google,
                    payload={
                        "model": model,
                        "contents": conversation,
                        "config": config,
                    },
                ),
            )

            if isinstance(api_response, BaseException):
                raise api_response

            # Trace/log the raw provider response before deserializing.
            if self.logger:
                self.logger.debug(
                    "Gemini generate_structured api_response", data=api_response
                )
            if self.context.tracing_enabled:
                preview = safe_preview(api_response, limit=8000)
                span.set_attribute("gemini.api_response_preview", preview)

            text = extract_structured_json_text(api_response)
            if not text:
                raise ValueError("No structured response returned by Gemini")

            data = json.loads(text)
            return cast(ModelT, model_cls.model_validate(data))


class ChunkAwareOpenAIAugmentedLLM(OpenAIAugmentedLLM):
    async def generate_structured(
        self,
        message: Any,
        response_model: Type[ModelT],
        request_params: RequestParams | None = None,
    ) -> ModelT:
        """
        Same as mcp-agent's OpenAIAugmentedLLM.generate_structured, but supports
        content being returned as chunked blocks (list) as well as a string.
        """
        import json

        # Import types/helpers from mcp-agent implementation
        from openai.types.chat import (
            ChatCompletion,
            ChatCompletionMessageParam,
            ChatCompletionSystemMessageParam,
        )
        from mcp_agent.workflows.llm.augmented_llm_openai import (
            OpenAICompletionTasks,
            RequestCompletionRequest,
        )
        from mcp_agent.workflows.llm.multipart_converter_openai import OpenAIConverter
        from mcp_agent.tracing.telemetry import get_tracer
        from mcp_agent.tracing.semconv import GEN_AI_AGENT_NAME, GEN_AI_REQUEST_MODEL
        from mcp_agent.workflows.llm.augmented_llm import AugmentedLLM

        tracer = get_tracer(self.context)
        with tracer.start_as_current_span(
            f"{self.__class__.__name__}.{self.name}.generate_structured"
        ) as span:
            if self.context.tracing_enabled:
                span.set_attribute(GEN_AI_AGENT_NAME, self.agent.name)
                self._annotate_span_for_generation_message(span, message)

            params: RequestParams = self.get_request_params(request_params)
            default_model = "gpt-4o"
            if self.default_request_params and self.default_request_params.model:
                default_model = self.default_request_params.model
            model = await self.select_model(params) or default_model
            if self.context.tracing_enabled:
                AugmentedLLM.annotate_span_with_request_params(span, params)
                span.set_attribute(GEN_AI_REQUEST_MODEL, model)
                span.set_attribute("response_model", response_model.__name__)

            messages: list[ChatCompletionMessageParam] = []
            system_prompt = self.instruction or params.systemPrompt
            if system_prompt is not None:
                if not isinstance(system_prompt, str):
                    system_prompt = str(system_prompt)
            if system_prompt:
                messages.append(
                    ChatCompletionSystemMessageParam(
                        role="system", content=system_prompt
                    )
                )
            if params.use_history:
                messages.extend(self.history.get())
            messages.extend(OpenAIConverter.convert_mixed_messages_to_openai(message))

            model_cls = cast(type[BaseModel], response_model)
            schema = model_cls.model_json_schema()

            def _ensure_no_additional_props_and_require_all(
                node: dict[str, Any],
            ) -> None:
                if not isinstance(node, dict):
                    return
                node_type = node.get("type")
                if node_type == "object":
                    if "additionalProperties" not in node:
                        node["additionalProperties"] = False
                    props = node.get("properties")
                    if isinstance(props, dict):
                        node["required"] = list(props.keys())

                for key in ("properties", "$defs", "definitions"):
                    sub = node.get(key)
                    if isinstance(sub, dict):
                        for v in sub.values():
                            _ensure_no_additional_props_and_require_all(v)
                if "items" in node:
                    _ensure_no_additional_props_and_require_all(node["items"])
                for key in ("oneOf", "anyOf", "allOf"):
                    subs = node.get(key)
                    if isinstance(subs, list):
                        for v in subs:
                            _ensure_no_additional_props_and_require_all(v)

            if params.strict:
                _ensure_no_additional_props_and_require_all(schema)

            response_format = {
                "type": "json_schema",
                "json_schema": {
                    "name": getattr(response_model, "__name__", "StructuredOutput"),
                    "schema": schema,
                    "strict": params.strict,
                },
            }

            payload: dict[str, Any] = {
                "model": model,
                "messages": messages,
                "response_format": response_format,
            }

            reasoning_fn = cast(Callable[[str], bool], getattr(self, "_reasoning"))
            if reasoning_fn(model):
                payload["max_completion_tokens"] = params.maxTokens
                payload["reasoning_effort"] = self._reasoning_effort
            else:
                payload["max_tokens"] = params.maxTokens

            if self.context.config is None or self.context.config.openai is None:
                raise RuntimeError("OpenAI provider configuration is not available.")

            user = params.user or getattr(self.context.config.openai, "user", None)
            if user:
                payload["user"] = user
            if params.stopSequences is not None:
                payload["stop"] = params.stopSequences
            if params.metadata:
                payload.update(params.metadata)

            if self.executor is None:
                raise RuntimeError("LLM executor is not configured.")

            completion_any = await self.executor.execute(
                OpenAICompletionTasks.request_completion_task,
                RequestCompletionRequest(
                    config=self.context.config.openai, payload=payload
                ),
            )

            if isinstance(completion_any, BaseException):
                raise completion_any

            completion = cast(ChatCompletion, completion_any)

            text = extract_structured_json_text(completion)
            if not text:
                raise ValueError("No structured content returned by model")

            try:
                data = json.loads(text)
                return cast(ModelT, model_cls.model_validate(data))
            except Exception:
                return cast(ModelT, model_cls.model_validate_json(text))
