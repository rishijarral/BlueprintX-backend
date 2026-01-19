"""Gemini API client using google-genai SDK with retries, timeouts, and safe logging."""

import json
from typing import Any, Type, TypeVar

from google import genai
from google.genai import types
from google.genai.errors import APIError, ClientError, ServerError
from pydantic import BaseModel
from tenacity import (
    retry,
    retry_if_exception_type,
    stop_after_attempt,
    wait_exponential,
)

from app.config import Settings
from app.errors import LLMError
from app.gemini.schemas import GenerationConfig, GeminiResponse, VisionInput
from app.logging import get_logger

logger = get_logger(__name__)

T = TypeVar("T", bound=BaseModel)

# Transient errors that should be retried
RETRYABLE_EXCEPTIONS = (
    ServerError,  # 5xx errors
)


class GeminiClient:
    """
    Gemini API client using google-genai SDK with:
    - Retry with exponential backoff
    - Configurable timeouts
    - Safe logging (no tokens, truncated content)
    - Structured JSON output support
    """

    def __init__(self, settings: Settings) -> None:
        self.settings = settings
        self._client = self._create_client()

    def _create_client(self) -> genai.Client:
        """Create and configure the Gemini client."""
        client = genai.Client(api_key=self.settings.gemini_api_key)
        logger.info(
            "Gemini client configured",
            model_text=self.settings.gemini_model_text,
            model_vision=self.settings.gemini_model_vision,
        )
        return client

    def _build_config(
        self,
        config: GenerationConfig | None = None,
    ) -> types.GenerateContentConfig:
        """Build generation config for the API."""
        cfg = config or GenerationConfig()

        gen_config = types.GenerateContentConfig(
            temperature=cfg.temperature,
            top_p=cfg.top_p,
            top_k=cfg.top_k,
            max_output_tokens=cfg.max_output_tokens,
        )

        if cfg.response_mime_type:
            gen_config.response_mime_type = cfg.response_mime_type

        return gen_config

    def _log_request(self, prompt: str, model: str, has_image: bool = False) -> None:
        """Log request safely (truncate content)."""
        truncated = prompt[:200] + "..." if len(prompt) > 200 else prompt
        logger.info(
            "Gemini request",
            model=model,
            prompt_length=len(prompt),
            prompt_preview=truncated,
            has_image=has_image,
        )

    def _log_response(self, response: GeminiResponse) -> None:
        """Log response safely."""
        truncated = (
            response.text[:200] + "..." if len(response.text) > 200 else response.text
        )
        logger.info(
            "Gemini response",
            model=response.model,
            response_length=len(response.text),
            response_preview=truncated,
            finish_reason=response.finish_reason,
            usage=response.usage,
        )

    @retry(
        retry=retry_if_exception_type(RETRYABLE_EXCEPTIONS),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=2, max=60),
        reraise=True,
    )
    async def generate(
        self,
        prompt: str,
        model: str | None = None,
        config: GenerationConfig | None = None,
    ) -> GeminiResponse:
        """
        Generate text from a prompt.

        Args:
            prompt: The input prompt
            model: Model name (defaults to settings.gemini_model_text)
            config: Generation configuration

        Returns:
            GeminiResponse with generated text

        Raises:
            LLMError: If generation fails after retries
        """
        model_name = model or self.settings.gemini_model_text
        self._log_request(prompt, model_name)

        try:
            response = await self._client.aio.models.generate_content(
                model=model_name,
                contents=prompt,
                config=self._build_config(config),
            )

            # Extract text from response
            text = response.text or ""

            # Extract usage metadata
            usage = None
            if response.usage_metadata:
                usage = {
                    "prompt_tokens": response.usage_metadata.prompt_token_count or 0,
                    "completion_tokens": response.usage_metadata.candidates_token_count or 0,
                    "total_tokens": response.usage_metadata.total_token_count or 0,
                }

            # Extract finish reason
            finish_reason = None
            if response.candidates and response.candidates[0].finish_reason:
                finish_reason = response.candidates[0].finish_reason.name

            result = GeminiResponse(
                text=text,
                model=model_name,
                finish_reason=finish_reason,
                usage=usage,
            )

            self._log_response(result)
            return result

        except RETRYABLE_EXCEPTIONS:
            # Will be retried by tenacity
            raise
        except (APIError, ClientError) as e:
            logger.error("Gemini API error", error=str(e), model=model_name)
            raise LLMError(f"Text generation failed: {str(e)}") from e
        except Exception as e:
            logger.error("Gemini generation failed", error=str(e), model=model_name)
            raise LLMError(f"Text generation failed: {str(e)}") from e

    @retry(
        retry=retry_if_exception_type(RETRYABLE_EXCEPTIONS),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=2, max=60),
        reraise=True,
    )
    async def generate_vision(
        self,
        image_input: VisionInput,
        model: str | None = None,
        config: GenerationConfig | None = None,
    ) -> GeminiResponse:
        """
        Generate text from an image and prompt (vision).

        Args:
            image_input: Image bytes, mime type, and prompt
            model: Model name (defaults to settings.gemini_model_vision)
            config: Generation configuration

        Returns:
            GeminiResponse with generated text

        Raises:
            LLMError: If generation fails after retries
        """
        model_name = model or self.settings.gemini_model_vision
        self._log_request(image_input.prompt, model_name, has_image=True)

        try:
            # Create content with image and text parts
            image_part = types.Part.from_bytes(
                data=image_input.image_bytes,
                mime_type=image_input.mime_type,
            )

            response = await self._client.aio.models.generate_content(
                model=model_name,
                contents=[image_input.prompt, image_part],  # type: ignore[arg-type]
                config=self._build_config(config),
            )

            text = response.text or ""

            usage = None
            if response.usage_metadata:
                usage = {
                    "prompt_tokens": response.usage_metadata.prompt_token_count or 0,
                    "completion_tokens": response.usage_metadata.candidates_token_count or 0,
                    "total_tokens": response.usage_metadata.total_token_count or 0,
                }

            finish_reason = None
            if response.candidates and response.candidates[0].finish_reason:
                finish_reason = response.candidates[0].finish_reason.name

            result = GeminiResponse(
                text=text,
                model=model_name,
                finish_reason=finish_reason,
                usage=usage,
            )

            self._log_response(result)
            return result

        except RETRYABLE_EXCEPTIONS:
            raise
        except (APIError, ClientError) as e:
            logger.error("Gemini vision API error", error=str(e), model=model_name)
            raise LLMError(f"Vision generation failed: {str(e)}") from e
        except Exception as e:
            logger.error("Gemini vision failed", error=str(e), model=model_name)
            raise LLMError(f"Vision generation failed: {str(e)}") from e

    async def generate_structured(
        self,
        prompt: str,
        output_schema: Type[T],
        model: str | None = None,
        config: GenerationConfig | None = None,
    ) -> T:
        """
        Generate structured JSON output validated against a Pydantic model.

        Args:
            prompt: The input prompt (should ask for JSON output)
            output_schema: Pydantic model class for validation
            model: Model name
            config: Generation configuration (response_mime_type will be set to JSON)

        Returns:
            Validated Pydantic model instance

        Raises:
            LLMError: If generation or parsing fails
        """
        # Ensure JSON output
        json_config = config or GenerationConfig()
        json_config.response_mime_type = "application/json"

        try:
            response = await self.generate(prompt, model, json_config)

            # Parse JSON response
            try:
                data = json.loads(response.text)
            except json.JSONDecodeError as e:
                logger.warning(
                    "Failed to parse JSON response, attempting fix",
                    error=str(e),
                    response_preview=response.text[:200],
                )
                # Retry with fix prompt
                data = await self._fix_json(response.text, output_schema)

            # Validate with Pydantic
            return output_schema.model_validate(data)

        except LLMError:
            raise
        except Exception as e:
            logger.error(
                "Structured generation failed",
                error=str(e),
                schema=output_schema.__name__,
            )
            raise LLMError(f"Structured generation failed: {str(e)}") from e

    async def _fix_json(
        self,
        broken_json: str,
        output_schema: Type[T],
    ) -> dict[str, Any]:
        """Attempt to fix malformed JSON with a follow-up prompt."""
        fix_prompt = f"""The following JSON is malformed. Fix it to be valid JSON matching this schema:

Schema: {output_schema.model_json_schema()}

Malformed JSON:
{broken_json[:2000]}

Return ONLY the corrected JSON, no explanation."""

        config = GenerationConfig(
            temperature=0.1,
            response_mime_type="application/json",
        )

        response = await self.generate(fix_prompt, self.settings.gemini_model_fast, config)

        try:
            return json.loads(response.text)
        except json.JSONDecodeError as e:
            raise LLMError(f"Failed to fix JSON: {str(e)}") from e

    async def generate_vision_structured(
        self,
        image_input: VisionInput,
        output_schema: Type[T],
        model: str | None = None,
        config: GenerationConfig | None = None,
    ) -> T:
        """
        Generate structured JSON from an image.

        Args:
            image_input: Image and prompt
            output_schema: Pydantic model for validation
            model: Model name
            config: Generation config

        Returns:
            Validated Pydantic model instance
        """
        json_config = config or GenerationConfig()
        json_config.response_mime_type = "application/json"

        try:
            response = await self.generate_vision(image_input, model, json_config)

            try:
                data = json.loads(response.text)
            except json.JSONDecodeError:
                data = await self._fix_json(response.text, output_schema)

            return output_schema.model_validate(data)

        except LLMError:
            raise
        except Exception as e:
            raise LLMError(f"Structured vision generation failed: {str(e)}") from e
