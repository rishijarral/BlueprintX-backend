"""Abstract base class for vector stores."""

from abc import ABC, abstractmethod
from typing import Any

from pydantic import BaseModel, Field


class Document(BaseModel):
    """Document with content and metadata for vector storage."""

    id: str | None = None
    content: str
    embedding: list[float] | None = None
    metadata: dict[str, Any] = Field(default_factory=dict)

    # Common metadata fields
    project_id: str | None = None
    document_id: str | None = None
    page_number: int | None = None
    chunk_index: int | None = None
    source: str | None = None


class SearchResult(BaseModel):
    """Result from similarity search."""

    document: Document
    score: float
    distance: float | None = None


class VectorStore(ABC):
    """
    Abstract vector store interface.

    This abstraction allows swapping between different vector store
    backends (pgvector, Pinecone, Qdrant, etc.) without changing
    application code.
    """

    @abstractmethod
    async def initialize(self) -> None:
        """
        Initialize the vector store (create tables, indexes, etc.).
        Called once at startup.
        """
        pass

    @abstractmethod
    async def add_documents(
        self,
        documents: list[Document],
        batch_size: int = 100,
    ) -> list[str]:
        """
        Add documents with embeddings to the store.

        Args:
            documents: List of documents with embeddings
            batch_size: Number of documents to insert per batch

        Returns:
            List of document IDs
        """
        pass

    @abstractmethod
    async def similarity_search(
        self,
        query_embedding: list[float],
        k: int = 5,
        filter_metadata: dict[str, Any] | None = None,
    ) -> list[SearchResult]:
        """
        Search for similar documents.

        Args:
            query_embedding: Query vector
            k: Number of results to return
            filter_metadata: Optional metadata filters (e.g., project_id)

        Returns:
            List of SearchResult sorted by relevance
        """
        pass

    @abstractmethod
    async def delete(
        self,
        ids: list[str] | None = None,
        filter_metadata: dict[str, Any] | None = None,
    ) -> int:
        """
        Delete documents by ID or metadata filter.

        Args:
            ids: List of document IDs to delete
            filter_metadata: Metadata filter for bulk delete

        Returns:
            Number of documents deleted
        """
        pass

    @abstractmethod
    async def get_by_id(self, doc_id: str) -> Document | None:
        """
        Retrieve a document by ID.

        Args:
            doc_id: Document ID

        Returns:
            Document if found, None otherwise
        """
        pass

    @abstractmethod
    async def count(
        self,
        filter_metadata: dict[str, Any] | None = None,
    ) -> int:
        """
        Count documents, optionally filtered by metadata.

        Args:
            filter_metadata: Optional metadata filter

        Returns:
            Number of matching documents
        """
        pass

    async def add_texts(
        self,
        texts: list[str],
        metadatas: list[dict[str, Any]] | None = None,
        embeddings: list[list[float]] | None = None,
    ) -> list[str]:
        """
        Convenience method to add texts as documents.

        Args:
            texts: List of text contents
            metadatas: Optional list of metadata dicts
            embeddings: Optional pre-computed embeddings

        Returns:
            List of document IDs
        """
        if metadatas is None:
            metadatas = [{} for _ in texts]

        documents = []
        for i, text in enumerate(texts):
            doc = Document(
                content=text,
                metadata=metadatas[i] if i < len(metadatas) else {},
                embedding=embeddings[i] if embeddings and i < len(embeddings) else None,
            )
            documents.append(doc)

        return await self.add_documents(documents)
