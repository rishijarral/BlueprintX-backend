"""Configuration management using Pydantic Settings."""

from functools import lru_cache
from typing import Literal

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Application settings loaded from environment variables."""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
        extra="ignore",
    )

    # Environment
    env: Literal["dev", "staging", "prod"] = "dev"

    # Server
    server_host: str = "0.0.0.0"
    server_port: int = 8000

    # Security
    internal_api_token: str = Field(
        ...,
        description="Shared secret for Rust -> Python internal auth",
    )

    # Gemini API
    gemini_api_key: str = Field(..., description="Google Gemini API key")
    gemini_model_text: str = "gemini-3-flash-preview"
    gemini_model_vision: str = "gemini-3-flash-preview"
    gemini_model_fast: str = "gemini-3-flash-preview"
    gemini_embedding_model: str = "gemini-embedding-001"
    gemini_timeout_seconds: int = 300
    gemini_max_retries: int = 3

    # Database
    database_url: str = Field(
        default="postgresql+asyncpg://postgres:postgres@db:5432/blueprintx",
        description="PostgreSQL connection URL with asyncpg driver",
    )

    # Vector store
    vector_store_type: Literal["pgvector"] = "pgvector"
    pgvector_collection_name: str = "document_embeddings"
    pgvector_embedding_dimensions: int = 768

    # Document processing
    max_upload_size_mb: int = 100
    pdf_dpi: int = 600
    chunk_size: int = 2000
    chunk_overlap: int = 400

    # Redis
    redis_url: str | None = Field(
        default=None,
        description="Redis connection URL (optional - caching disabled if not set)",
    )
    redis_cache_ttl_seconds: int = 3600

    # Jobs
    job_store_type: Literal["memory", "redis"] = "memory"

    # Job Retry / Dead Letter Queue
    job_max_retries: int = Field(
        default=3,
        description="Maximum number of retry attempts before moving to DLQ",
    )
    job_retry_base_delay_seconds: int = Field(
        default=5,
        description="Base delay in seconds before first retry (exponential backoff)",
    )
    job_retry_max_delay_seconds: int = Field(
        default=300,
        description="Maximum delay in seconds between retries",
    )
    job_retry_backoff_multiplier: float = Field(
        default=2.0,
        description="Multiplier for exponential backoff between retries",
    )
    dlq_entry_ttl_days: int = Field(
        default=30,
        description="How long to keep DLQ entries (in days)",
    )

    # Logging
    log_level: Literal["DEBUG", "INFO", "WARNING", "ERROR"] = "INFO"
    log_format: Literal["json", "console"] = "json"

    @field_validator("database_url")
    @classmethod
    def validate_database_url(cls, v: str) -> str:
        """Ensure database URL uses asyncpg driver."""
        if "postgresql://" in v and "asyncpg" not in v:
            return v.replace("postgresql://", "postgresql+asyncpg://")
        return v

    @property
    def max_upload_size_bytes(self) -> int:
        """Maximum upload size in bytes."""
        return self.max_upload_size_mb * 1024 * 1024

    @property
    def is_production(self) -> bool:
        """Check if running in production."""
        return self.env == "prod"


@lru_cache
def get_settings() -> Settings:
    """Get cached settings instance."""
    return Settings()
