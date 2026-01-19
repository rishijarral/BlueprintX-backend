"""Gemini embeddings client using google-genai SDK for document vectorization."""

from google import genai
from google.genai.errors import APIError, ClientError, ServerError
from tenacity import (
    retry,
    retry_if_exception_type,
    stop_after_attempt,
    wait_exponential,
)

from app.config import Settings
from app.errors import LLMError
from app.logging import get_logger

logger = get_logger(__name__)

RETRYABLE_EXCEPTIONS = (
    ServerError,  # 5xx errors
)


class GeminiEmbeddings:
    """
    Gemini embeddings client using google-genai SDK.

    Supports both single text and batch embedding generation
    with retry logic and safe logging.
    """

    def __init__(self, settings: Settings) -> None:
        self.settings = settings
        self.model_name = settings.gemini_embedding_model
        self._client = self._create_client()

    def _create_client(self) -> genai.Client:
        """Create and configure the Gemini client."""
        client = genai.Client(api_key=self.settings.gemini_api_key)
        logger.info(
            "Gemini embeddings client configured",
            model=self.model_name,
        )
        return client

    @property
    def dimensions(self) -> int:
        """Return embedding dimensions based on model."""
        # gemini-embedding-001 produces 768-dimensional embeddings
        return self.settings.pgvector_embedding_dimensions

    @retry(
        retry=retry_if_exception_type(RETRYABLE_EXCEPTIONS),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=2, max=60),
        reraise=True,
    )
    async def embed_text(self, text: str) -> list[float]:
        """
        Generate embedding for a single text.

        Args:
            text: Input text to embed

        Returns:
            List of floats representing the embedding vector

        Raises:
            LLMError: If embedding generation fails
        """
        if not text or not text.strip():
            raise LLMError("Cannot embed empty text")

        logger.debug(
            "Generating embedding",
            text_length=len(text),
            model=self.model_name,
        )

        try:
            response = await self._client.aio.models.embed_content(
                model=self.model_name,
                contents=text,
            )

            # Extract embedding from response
            if not response.embeddings or not response.embeddings[0].values:
                raise LLMError("No embedding returned from API")

            embedding = list(response.embeddings[0].values)

            logger.debug(
                "Embedding generated",
                dimensions=len(embedding),
            )

            return embedding

        except RETRYABLE_EXCEPTIONS:
            raise
        except (APIError, ClientError) as e:
            logger.error("Embedding API error", error=str(e))
            raise LLMError(f"Embedding generation failed: {str(e)}") from e
        except Exception as e:
            logger.error("Embedding generation failed", error=str(e))
            raise LLMError(f"Embedding generation failed: {str(e)}") from e

    @retry(
        retry=retry_if_exception_type(RETRYABLE_EXCEPTIONS),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=2, max=60),
        reraise=True,
    )
    async def embed_texts(self, texts: list[str]) -> list[list[float]]:
        """
        Generate embeddings for multiple texts in batch.

        Args:
            texts: List of texts to embed

        Returns:
            List of embedding vectors

        Raises:
            LLMError: If embedding generation fails
        """
        if not texts:
            return []

        # Filter out empty texts
        valid_texts = [t for t in texts if t and t.strip()]
        if not valid_texts:
            raise LLMError("All texts are empty")

        logger.info(
            "Generating batch embeddings",
            count=len(valid_texts),
            model=self.model_name,
        )

        try:
            response = await self._client.aio.models.embed_content(
                model=self.model_name,
                contents=valid_texts,
            )

            if not response.embeddings:
                raise LLMError("No embeddings returned from API")

            embeddings = [list(emb.values) for emb in response.embeddings]

            logger.info(
                "Batch embeddings generated",
                count=len(embeddings),
                dimensions=len(embeddings[0]) if embeddings else 0,
            )

            return embeddings

        except RETRYABLE_EXCEPTIONS:
            raise
        except (APIError, ClientError) as e:
            logger.error("Batch embedding API error", error=str(e))
            raise LLMError(f"Batch embedding generation failed: {str(e)}") from e
        except Exception as e:
            logger.error("Batch embedding generation failed", error=str(e))
            raise LLMError(f"Batch embedding generation failed: {str(e)}") from e

    async def embed_query(self, query: str) -> list[float]:
        """
        Generate embedding for a query.

        Note: The google-genai SDK doesn't differentiate between document and query
        embeddings in the same way. This method is kept for API compatibility.

        Args:
            query: Query text

        Returns:
            Embedding vector for similarity search
        """
        if not query or not query.strip():
            raise LLMError("Cannot embed empty query")

        logger.debug(
            "Generating query embedding",
            query_length=len(query),
        )

        # Use the same embedding method - google-genai handles this internally
        return await self.embed_text(query)
