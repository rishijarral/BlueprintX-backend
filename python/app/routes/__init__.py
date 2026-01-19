"""API routes for the AI service."""

from app.routes import documents, health, jobs, plan, qna, tenders

__all__ = [
    "documents",
    "health",
    "jobs",
    "plan",
    "qna",
    "tenders",
]
