"""PostgreSQL pgvector implementation of VectorStore."""

from __future__ import annotations

import uuid
from typing import Any

from pgvector.sqlalchemy import Vector  # type: ignore[import-untyped]
from sqlalchemy import Column, DateTime, Index, Integer, String, Text, delete, func, select
from sqlalchemy.dialects.postgresql import JSONB
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import DeclarativeBase

from app.gemini.embeddings import GeminiEmbeddings
from app.logging import get_logger
from app.vectorstore.base import Document, SearchResult, VectorStore

logger = get_logger(__name__)


class Base(DeclarativeBase):
    """SQLAlchemy declarative base."""

    pass


class DocumentEmbedding(Base):
    """SQLAlchemy model for document embeddings."""

    __tablename__ = "document_embeddings"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    content = Column(Text, nullable=False)
    embedding = Column(Vector(768), nullable=False)  # Gemini text-embedding-004 = 768 dims
    metadata_ = Column("metadata", JSONB, nullable=False, default=dict)

    # Denormalized fields for efficient filtering
    project_id = Column(String(36), index=True, nullable=True)
    document_id = Column(String(36), index=True, nullable=True)
    page_number = Column(Integer, nullable=True)
    chunk_index = Column(Integer, nullable=True)
    source = Column(String(255), nullable=True)

    created_at = Column(DateTime, server_default=func.now(), nullable=False)

    __table_args__ = (
        # IVFFlat index for approximate nearest neighbor search
        # Lists = sqrt(n) where n = expected number of rows
        Index(
            "ix_document_embeddings_embedding",
            embedding,
            postgresql_using="ivfflat",
            postgresql_with={"lists": 100},
            postgresql_ops={"embedding": "vector_cosine_ops"},
        ),
        # Composite index for filtered searches
        Index("ix_document_embeddings_project_document", project_id, document_id),
    )


class PgVectorStore(VectorStore):
    """
    PostgreSQL pgvector implementation.

    Features:
    - Cosine similarity search with IVFFlat index
    - Metadata filtering on denormalized columns
    - Batch inserts for performance
    - Automatic embedding generation if not provided
    """

    def __init__(
        self,
        session: AsyncSession,
        embeddings: GeminiEmbeddings,
        collection_name: str = "document_embeddings",
        embedding_dimensions: int = 768,
    ) -> None:
        self.session = session
        self.embeddings = embeddings
        self.collection_name = collection_name
        self.embedding_dimensions = embedding_dimensions

    async def initialize(self) -> None:
        """
        Initialize pgvector extension and create tables.

        This should be called once at application startup.
        """
        logger.info("Initializing pgvector store")

        # Enable pgvector extension
        await self.session.execute(
            "CREATE EXTENSION IF NOT EXISTS vector"  # type: ignore
        )

        # Create tables (in production, use Alembic migrations)
        # For now, we assume the table exists or will be created by migrations

        logger.info("pgvector store initialized")

    async def add_documents(
        self,
        documents: list[Document],
        batch_size: int = 100,
    ) -> list[str]:
        """Add documents with embeddings to pgvector."""
        if not documents:
            return []

        logger.info("Adding documents to pgvector", count=len(documents))

        ids: list[str] = []

        # Process in batches
        for i in range(0, len(documents), batch_size):
            batch = documents[i : i + batch_size]

            # Generate embeddings for documents without them
            texts_to_embed = []
            embed_indices = []

            for j, doc in enumerate(batch):
                if doc.embedding is None:
                    texts_to_embed.append(doc.content)
                    embed_indices.append(j)

            if texts_to_embed:
                new_embeddings = await self.embeddings.embed_texts(texts_to_embed)
                for idx, embedding in zip(embed_indices, new_embeddings):
                    batch[idx].embedding = embedding

            # Insert batch
            for doc in batch:
                doc_id = doc.id or str(uuid.uuid4())

                db_doc = DocumentEmbedding(
                    id=doc_id,
                    content=doc.content,
                    embedding=doc.embedding,
                    metadata_=doc.metadata,
                    project_id=doc.project_id or doc.metadata.get("project_id"),
                    document_id=doc.document_id or doc.metadata.get("document_id"),
                    page_number=doc.page_number or doc.metadata.get("page_number"),
                    chunk_index=doc.chunk_index or doc.metadata.get("chunk_index"),
                    source=doc.source or doc.metadata.get("source"),
                )

                self.session.add(db_doc)
                ids.append(doc_id)

            await self.session.flush()

        logger.info("Documents added", count=len(ids))
        return ids

    async def similarity_search(
        self,
        query_embedding: list[float],
        k: int = 5,
        filter_metadata: dict[str, Any] | None = None,
    ) -> list[SearchResult]:
        """Search for similar documents using cosine distance."""
        logger.debug("Similarity search", k=k, has_filter=filter_metadata is not None)

        # Build query with cosine distance
        # pgvector uses <=> for cosine distance (1 - cosine_similarity)
        distance_expr = DocumentEmbedding.embedding.cosine_distance(query_embedding)

        query = select(
            DocumentEmbedding,
            distance_expr.label("distance"),
        )

        # Apply metadata filters
        if filter_metadata:
            if "project_id" in filter_metadata:
                query = query.where(
                    DocumentEmbedding.project_id == filter_metadata["project_id"]
                )
            if "document_id" in filter_metadata:
                query = query.where(
                    DocumentEmbedding.document_id == filter_metadata["document_id"]
                )

        # Order by distance (ascending = most similar first)
        query = query.order_by(distance_expr).limit(k)

        result = await self.session.execute(query)
        rows = result.all()

        search_results = []
        for row in rows:
            # SQLAlchemy ORM model - use Any to bypass static type checking
            db_doc: Any = row[0]
            distance: float = row[1]

            # Convert distance to similarity score (cosine_similarity = 1 - distance)
            score = 1.0 - distance

            doc = Document(
                id=db_doc.id,
                content=db_doc.content,
                embedding=list(db_doc.embedding) if db_doc.embedding else None,
                metadata=db_doc.metadata_ or {},
                project_id=db_doc.project_id,
                document_id=db_doc.document_id,
                page_number=db_doc.page_number,
                chunk_index=db_doc.chunk_index,
                source=db_doc.source,
            )

            search_results.append(
                SearchResult(
                    document=doc,
                    score=score,
                    distance=distance,
                )
            )

        logger.debug("Search completed", results=len(search_results))
        return search_results

    async def delete(
        self,
        ids: list[str] | None = None,
        filter_metadata: dict[str, Any] | None = None,
    ) -> int:
        """Delete documents by ID or metadata filter."""
        if not ids and not filter_metadata:
            logger.warning("Delete called with no IDs or filters")
            return 0

        query = delete(DocumentEmbedding)

        if ids:
            query = query.where(DocumentEmbedding.id.in_(ids))

        if filter_metadata:
            if "project_id" in filter_metadata:
                query = query.where(
                    DocumentEmbedding.project_id == filter_metadata["project_id"]
                )
            if "document_id" in filter_metadata:
                query = query.where(
                    DocumentEmbedding.document_id == filter_metadata["document_id"]
                )

        result = await self.session.execute(query)
        # rowcount is available on CursorResult but type system doesn't know
        count: int = getattr(result, "rowcount", 0)

        logger.info("Documents deleted", count=count)
        return count

    async def get_by_id(self, doc_id: str) -> Document | None:
        """Retrieve a document by ID."""
        query = select(DocumentEmbedding).where(DocumentEmbedding.id == doc_id)
        result = await self.session.execute(query)
        db_doc = result.scalar_one_or_none()

        if not db_doc:
            return None

        # SQLAlchemy ORM model attributes are typed as Column but are actually
        # runtime values - using Any to bypass static type checking
        row: Any = db_doc
        return Document(
            id=row.id,
            content=row.content,
            embedding=list(row.embedding) if row.embedding else None,
            metadata=row.metadata_ or {},
            project_id=row.project_id,
            document_id=row.document_id,
            page_number=row.page_number,
            chunk_index=row.chunk_index,
            source=row.source,
        )

    async def count(
        self,
        filter_metadata: dict[str, Any] | None = None,
    ) -> int:
        """Count documents with optional filter."""
        query = select(func.count(DocumentEmbedding.id))

        if filter_metadata:
            if "project_id" in filter_metadata:
                query = query.where(
                    DocumentEmbedding.project_id == filter_metadata["project_id"]
                )
            if "document_id" in filter_metadata:
                query = query.where(
                    DocumentEmbedding.document_id == filter_metadata["document_id"]
                )

        result = await self.session.execute(query)
        return result.scalar() or 0

    async def search_with_text(
        self,
        query_text: str,
        k: int = 5,
        filter_metadata: dict[str, Any] | None = None,
    ) -> list[SearchResult]:
        """
        Convenience method: embed query text and search.

        Args:
            query_text: Natural language query
            k: Number of results
            filter_metadata: Optional filters

        Returns:
            List of similar documents
        """
        query_embedding = await self.embeddings.embed_query(query_text)
        return await self.similarity_search(query_embedding, k, filter_metadata)
