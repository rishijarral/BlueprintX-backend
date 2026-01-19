"""Shared FastAPI dependencies."""

from typing import Annotated, AsyncGenerator

from fastapi import Depends
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from app.config import Settings, get_settings
from app.gemini.client import GeminiClient
from app.gemini.embeddings import GeminiEmbeddings
from app.jobs.store import JobStore, MemoryJobStore
from app.vectorstore.base import VectorStore
from app.vectorstore.pgvector import PgVectorStore

# Database engine (lazy initialized)
_engine = None
_session_factory: async_sessionmaker[AsyncSession] | None = None


def get_engine(settings: Settings = Depends(get_settings)):
    """Get or create the database engine."""
    global _engine
    if _engine is None:
        _engine = create_async_engine(
            settings.database_url,
            echo=settings.env == "dev",
            pool_pre_ping=True,
            pool_size=5,
            max_overflow=10,
        )
    return _engine


def get_session_factory(
    settings: Settings = Depends(get_settings),
) -> async_sessionmaker[AsyncSession]:
    """Get or create the session factory."""
    global _session_factory
    if _session_factory is None:
        engine = get_engine(settings)
        _session_factory = async_sessionmaker(
            engine,
            class_=AsyncSession,
            expire_on_commit=False,
        )
    return _session_factory


async def get_db_session(
    settings: Settings = Depends(get_settings),
) -> AsyncGenerator[AsyncSession, None]:
    """Get a database session."""
    factory = get_session_factory(settings)
    async with factory() as session:
        try:
            yield session
            await session.commit()
        except Exception:
            await session.rollback()
            raise


# Singletons for clients (lazy initialized)
_gemini_client: GeminiClient | None = None
_gemini_embeddings: GeminiEmbeddings | None = None
_job_store: JobStore | None = None
_vector_store: VectorStore | None = None


def get_gemini_client(settings: Settings = Depends(get_settings)) -> GeminiClient:
    """Get the Gemini client singleton."""
    global _gemini_client
    if _gemini_client is None:
        _gemini_client = GeminiClient(settings)
    return _gemini_client


def get_gemini_embeddings(
    settings: Settings = Depends(get_settings),
) -> GeminiEmbeddings:
    """Get the Gemini embeddings client singleton."""
    global _gemini_embeddings
    if _gemini_embeddings is None:
        _gemini_embeddings = GeminiEmbeddings(settings)
    return _gemini_embeddings


def get_job_store(settings: Settings = Depends(get_settings)) -> JobStore:
    """Get the job store singleton."""
    global _job_store
    if _job_store is None:
        if settings.job_store_type == "memory":
            _job_store = MemoryJobStore()
        else:
            # Future: Redis, etc.
            _job_store = MemoryJobStore()
    return _job_store


async def get_vector_store(
    settings: Settings = Depends(get_settings),
    session: AsyncSession = Depends(get_db_session),
    embeddings: GeminiEmbeddings = Depends(get_gemini_embeddings),
) -> VectorStore:
    """Get the vector store instance."""
    # Note: We create a new instance per request since it needs the session
    # The actual connection pooling is handled by SQLAlchemy
    if settings.vector_store_type == "pgvector":
        store = PgVectorStore(
            session=session,
            embeddings=embeddings,
            collection_name=settings.pgvector_collection_name,
            embedding_dimensions=settings.pgvector_embedding_dimensions,
        )
        return store
    else:
        # Future: Pinecone, Qdrant, etc.
        raise ValueError(f"Unsupported vector store type: {settings.vector_store_type}")


# Type aliases for cleaner dependency injection
DbSession = Annotated[AsyncSession, Depends(get_db_session)]
GeminiClientDep = Annotated[GeminiClient, Depends(get_gemini_client)]
GeminiEmbeddingsDep = Annotated[GeminiEmbeddings, Depends(get_gemini_embeddings)]
JobStoreDep = Annotated[JobStore, Depends(get_job_store)]
VectorStoreDep = Annotated[VectorStore, Depends(get_vector_store)]
