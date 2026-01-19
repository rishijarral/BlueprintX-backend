"""Vector store module with pluggable backends."""

from app.vectorstore.base import Document, VectorStore
from app.vectorstore.pgvector import PgVectorStore

__all__ = [
    "Document",
    "VectorStore",
    "PgVectorStore",
]
