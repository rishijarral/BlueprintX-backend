"""LangGraph pipeline for RAG-based Q&A."""

from typing import Any, TypedDict

from langgraph.graph import END, StateGraph

from app.gemini.client import GeminiClient
from app.gemini.embeddings import GeminiEmbeddings
from app.gemini.schemas import GenerationConfig, QnAResponse
from app.logging import get_logger
from app.prompts.qna import build_qna_prompt
from app.vectorstore.base import SearchResult, VectorStore

logger = get_logger(__name__)


class QnAState(TypedDict):
    """State for Q&A pipeline."""

    # Input
    project_id: str
    document_id: str | None  # Optional: filter to specific document
    question: str
    document_text: str | None  # If provided, skip retrieval

    # Retrieval state
    query_embedding: list[float] | None
    retrieved_chunks: list[SearchResult]
    context: str | None

    # Output
    result: QnAResponse | None
    status: str
    error: str | None


class QnAPipeline:
    """
    RAG-based Q&A pipeline using LangGraph.

    Pipeline stages:
    1. embed_query: Generate embedding for the question
    2. retrieve: Find relevant document chunks
    3. generate_answer: Use Gemini to answer based on context
    """

    def __init__(
        self,
        gemini_client: GeminiClient,
        embeddings: GeminiEmbeddings,
        vector_store: VectorStore,
        top_k: int = 5,
    ) -> None:
        self.gemini = gemini_client
        self.embeddings = embeddings
        self.vector_store = vector_store
        self.top_k = top_k
        self.graph = self._build_graph()

    def _build_graph(self) -> StateGraph:
        """Build the Q&A graph."""
        graph = StateGraph(QnAState)

        # Add nodes
        graph.add_node("check_input", self._check_input)
        graph.add_node("embed_query", self._embed_query)
        graph.add_node("retrieve", self._retrieve)
        graph.add_node("generate_answer", self._generate_answer)
        graph.add_node("handle_error", self._handle_error)

        # Set entry point
        graph.set_entry_point("check_input")

        # Add conditional edges
        graph.add_conditional_edges(
            "check_input",
            self._route_input,
            {
                "use_provided_text": "generate_answer",
                "use_retrieval": "embed_query",
                "error": "handle_error",
            },
        )

        graph.add_conditional_edges(
            "embed_query",
            self._check_embedding,
            {
                "success": "retrieve",
                "error": "handle_error",
            },
        )

        graph.add_conditional_edges(
            "retrieve",
            self._check_retrieval,
            {
                "success": "generate_answer",
                "no_results": "generate_answer",  # Still try to answer
                "error": "handle_error",
            },
        )

        graph.add_edge("generate_answer", END)
        graph.add_edge("handle_error", END)

        return graph.compile()

    async def _check_input(self, state: QnAState) -> dict[str, Any]:
        """Validate input."""
        logger.info(
            "Q&A request",
            project_id=state["project_id"],
            question_length=len(state["question"]),
            has_document_text=state.get("document_text") is not None,
        )

        if not state["question"].strip():
            return {
                "status": "failed",
                "error": "Question cannot be empty",
            }

        return {"status": "processing"}

    def _route_input(self, state: QnAState) -> str:
        """Determine whether to use provided text or retrieval."""
        if state.get("status") == "failed":
            return "error"
        if state.get("document_text"):
            return "use_provided_text"
        return "use_retrieval"

    async def _embed_query(self, state: QnAState) -> dict[str, Any]:
        """Generate embedding for the question."""
        logger.debug("Embedding query", question=state["question"][:100])

        try:
            embedding = await self.embeddings.embed_query(state["question"])
            return {"query_embedding": embedding}

        except Exception as e:
            logger.error("Query embedding failed", error=str(e))
            return {
                "status": "failed",
                "error": f"Failed to embed query: {str(e)}",
            }

    def _check_embedding(self, state: QnAState) -> str:
        """Check if embedding succeeded."""
        if state.get("status") == "failed":
            return "error"
        if not state.get("query_embedding"):
            return "error"
        return "success"

    async def _retrieve(self, state: QnAState) -> dict[str, Any]:
        """Retrieve relevant document chunks."""
        logger.info(
            "Retrieving chunks",
            project_id=state["project_id"],
            document_id=state.get("document_id"),
            top_k=self.top_k,
        )

        try:
            # Build filter
            filter_metadata = {"project_id": state["project_id"]}
            if state.get("document_id"):
                filter_metadata["document_id"] = state["document_id"]

            # Search
            results = await self.vector_store.similarity_search(
                query_embedding=state["query_embedding"],
                k=self.top_k,
                filter_metadata=filter_metadata,
            )

            logger.info(
                "Chunks retrieved",
                count=len(results),
                top_score=results[0].score if results else None,
            )

            # Build context string
            context_parts = []
            for i, result in enumerate(results):
                source_info = []
                if result.document.page_number:
                    source_info.append(f"Page {result.document.page_number}")
                if result.document.chunk_index is not None:
                    source_info.append(f"Chunk {result.document.chunk_index}")

                source_str = ", ".join(source_info) if source_info else f"Source {i+1}"
                context_parts.append(f"[{source_str}]\n{result.document.content}")

            context = "\n\n---\n\n".join(context_parts) if context_parts else None

            return {
                "retrieved_chunks": results,
                "context": context,
            }

        except Exception as e:
            logger.error("Retrieval failed", error=str(e))
            return {
                "status": "failed",
                "error": f"Retrieval failed: {str(e)}",
            }

    def _check_retrieval(self, state: QnAState) -> str:
        """Check retrieval results."""
        if state.get("status") == "failed":
            return "error"
        if not state.get("retrieved_chunks"):
            return "no_results"
        return "success"

    async def _generate_answer(self, state: QnAState) -> dict[str, Any]:
        """Generate answer using Gemini."""
        logger.info("Generating answer", project_id=state["project_id"])

        try:
            # Determine context source
            if state.get("document_text"):
                context = state["document_text"]
            elif state.get("context"):
                context = state["context"]
            else:
                # No context available
                context = "[No relevant document content found]"

            # Build prompt
            prompt = build_qna_prompt(
                question=state["question"],
                context_chunks=[context],
            )

            config = GenerationConfig(
                temperature=0.3,
                max_output_tokens=4096,
            )

            result = await self.gemini.generate_structured(
                prompt,
                QnAResponse,
                config=config,
            )

            logger.info(
                "Answer generated",
                project_id=state["project_id"],
                confidence=result.confidence,
                citations=len(result.citations),
            )

            return {
                "result": result,
                "status": "completed",
            }

        except Exception as e:
            logger.error("Answer generation failed", error=str(e))
            return {
                "status": "failed",
                "error": f"Answer generation failed: {str(e)}",
            }

    async def _handle_error(self, state: QnAState) -> dict[str, Any]:
        """Handle pipeline errors."""
        logger.error(
            "Q&A error",
            project_id=state["project_id"],
            error=state.get("error"),
        )
        return {"status": "failed"}

    async def run(
        self,
        project_id: str,
        question: str,
        document_id: str | None = None,
        document_text: str | None = None,
    ) -> dict[str, Any]:
        """
        Run the Q&A pipeline.

        Args:
            project_id: Project to search within
            question: The question to answer
            document_id: Optional document to filter to
            document_text: If provided, use this instead of retrieval

        Returns:
            Pipeline result with answer
        """
        state: QnAState = {
            "project_id": project_id,
            "document_id": document_id,
            "question": question,
            "document_text": document_text,
            "query_embedding": None,
            "retrieved_chunks": [],
            "context": None,
            "result": None,
            "status": "pending",
            "error": None,
        }

        return await self.graph.ainvoke(state)


def create_qna_graph(
    gemini_client: GeminiClient,
    embeddings: GeminiEmbeddings,
    vector_store: VectorStore,
    top_k: int = 5,
) -> QnAPipeline:
    """Factory function to create a Q&A pipeline."""
    return QnAPipeline(gemini_client, embeddings, vector_store, top_k)
