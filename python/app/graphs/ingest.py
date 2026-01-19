"""LangGraph pipeline for document ingestion: PDF -> OCR -> Chunk -> Embed -> Store."""

from typing import Any, TypedDict

from langgraph.graph import END, StateGraph

from app.documents.chunker import Chunker, ChunkingStrategy, TextChunk
from app.documents.pdf import PageImage, PDFMetadata, PDFProcessor
from app.gemini.client import GeminiClient
from app.gemini.embeddings import GeminiEmbeddings
from app.gemini.schemas import VisionInput, VisionOCRResult
from app.logging import get_logger
from app.prompts.vision_ocr import build_vision_ocr_prompt
from app.vectorstore.base import Document, VectorStore

logger = get_logger(__name__)


class IngestState(TypedDict):
    """State for the document ingestion pipeline."""

    # Input
    job_id: str
    project_id: str
    document_id: str
    file_path: str | None
    file_bytes: bytes | None

    # Processing state
    pdf_metadata: PDFMetadata | None
    page_images: list[PageImage]
    ocr_results: list[VisionOCRResult]
    chunks: list[TextChunk]
    embeddings_stored: int

    # Output
    status: str  # pending, processing, completed, failed
    error: str | None
    progress: float  # 0.0 to 1.0


class IngestPipeline:
    """
    Document ingestion pipeline using LangGraph.

    Pipeline stages:
    1. extract_pages: Convert PDF to images
    2. ocr_pages: Use Gemini Vision to extract text from each page
    3. chunk_text: Split extracted text into chunks
    4. embed_and_store: Generate embeddings and store in vector DB
    """

    def __init__(
        self,
        gemini_client: GeminiClient,
        embeddings: GeminiEmbeddings,
        vector_store: VectorStore,
        pdf_processor: PDFProcessor | None = None,
        chunker: Chunker | None = None,
    ) -> None:
        self.gemini = gemini_client
        self.embeddings = embeddings
        self.vector_store = vector_store
        self.pdf_processor = pdf_processor or PDFProcessor()
        self.chunker = chunker or Chunker()
        self.graph = self._build_graph()

    def _build_graph(self) -> StateGraph:
        """Build the LangGraph state machine."""
        graph = StateGraph(IngestState)

        # Add nodes
        graph.add_node("extract_pages", self._extract_pages)
        graph.add_node("ocr_pages", self._ocr_pages)
        graph.add_node("chunk_text", self._chunk_text)
        graph.add_node("embed_and_store", self._embed_and_store)
        graph.add_node("handle_error", self._handle_error)

        # Add edges
        graph.set_entry_point("extract_pages")

        graph.add_conditional_edges(
            "extract_pages",
            self._check_extraction,
            {
                "success": "ocr_pages",
                "error": "handle_error",
            },
        )

        graph.add_conditional_edges(
            "ocr_pages",
            self._check_ocr,
            {
                "success": "chunk_text",
                "error": "handle_error",
            },
        )

        graph.add_edge("chunk_text", "embed_and_store")

        graph.add_conditional_edges(
            "embed_and_store",
            self._check_embedding,
            {
                "success": END,
                "error": "handle_error",
            },
        )

        graph.add_edge("handle_error", END)

        return graph.compile()

    async def _extract_pages(self, state: IngestState) -> dict[str, Any]:
        """Extract pages from PDF as images."""
        logger.info(
            "Extracting pages from PDF",
            job_id=state["job_id"],
            document_id=state["document_id"],
        )

        try:
            if state["file_path"]:
                metadata, page_images = await self.pdf_processor.process_file(
                    state["file_path"]
                )
            elif state["file_bytes"]:
                metadata, page_images = await self.pdf_processor.process_bytes(
                    state["file_bytes"]
                )
            else:
                return {
                    "status": "failed",
                    "error": "No file path or bytes provided",
                }

            logger.info(
                "Pages extracted",
                job_id=state["job_id"],
                page_count=len(page_images),
            )

            return {
                "pdf_metadata": metadata,
                "page_images": page_images,
                "status": "processing",
                "progress": 0.2,
            }

        except Exception as e:
            logger.error("Page extraction failed", error=str(e))
            return {
                "status": "failed",
                "error": f"Page extraction failed: {str(e)}",
            }

    def _check_extraction(self, state: IngestState) -> str:
        """Check if extraction succeeded."""
        if state.get("status") == "failed":
            return "error"
        if not state.get("page_images"):
            return "error"
        return "success"

    async def _ocr_pages(self, state: IngestState) -> dict[str, Any]:
        """Run Gemini Vision OCR on each page."""
        logger.info(
            "Running OCR on pages",
            job_id=state["job_id"],
            page_count=len(state["page_images"]),
        )

        ocr_results = []
        total_pages = len(state["page_images"])

        for i, page_image in enumerate(state["page_images"]):
            try:
                prompt = build_vision_ocr_prompt(page_image.page_number)

                vision_input = VisionInput(
                    image_bytes=page_image.image_bytes,
                    mime_type=page_image.mime_type,
                    prompt=prompt,
                )

                result = await self.gemini.generate_vision_structured(
                    vision_input,
                    VisionOCRResult,
                )

                ocr_results.append(result)

                logger.debug(
                    "Page OCR complete",
                    page=page_image.page_number,
                    text_length=len(result.text_content),
                )

            except Exception as e:
                logger.error(
                    "OCR failed for page",
                    page=page_image.page_number,
                    error=str(e),
                )
                # Continue with other pages, store error result
                ocr_results.append(
                    VisionOCRResult(
                        page_number=page_image.page_number,
                        text_content=f"[OCR ERROR: {str(e)}]",
                    )
                )

            # Update progress
            progress = 0.2 + (0.5 * (i + 1) / total_pages)

        logger.info(
            "OCR complete",
            job_id=state["job_id"],
            pages_processed=len(ocr_results),
        )

        return {
            "ocr_results": ocr_results,
            "progress": 0.7,
        }

    def _check_ocr(self, state: IngestState) -> str:
        """Check if OCR succeeded."""
        if state.get("status") == "failed":
            return "error"
        if not state.get("ocr_results"):
            return "error"
        return "success"

    async def _chunk_text(self, state: IngestState) -> dict[str, Any]:
        """Chunk the extracted text."""
        logger.info(
            "Chunking extracted text",
            job_id=state["job_id"],
            pages=len(state["ocr_results"]),
        )

        # Prepare page texts for chunking
        page_texts = []
        for ocr_result in state["ocr_results"]:
            # Combine all text content from OCR result
            full_text = ocr_result.text_content

            # Add annotations, notes, etc. if present
            if ocr_result.notes:
                full_text += "\n\nNOTES:\n" + "\n".join(ocr_result.notes)
            if ocr_result.annotations:
                full_text += "\n\nANNOTATIONS:\n" + "\n".join(ocr_result.annotations)

            page_texts.append((ocr_result.page_number, full_text))

        # Chunk all pages
        chunks = self.chunker.chunk_pages(
            page_texts,
            strategy=ChunkingStrategy.FIXED_SIZE,
            document_id=state["document_id"],
        )

        logger.info(
            "Chunking complete",
            job_id=state["job_id"],
            total_chunks=len(chunks),
        )

        return {
            "chunks": chunks,
            "progress": 0.8,
        }

    async def _embed_and_store(self, state: IngestState) -> dict[str, Any]:
        """Generate embeddings and store in vector database."""
        logger.info(
            "Embedding and storing chunks",
            job_id=state["job_id"],
            chunk_count=len(state["chunks"]),
        )

        try:
            # Convert chunks to documents
            documents = []
            for chunk in state["chunks"]:
                doc = Document(
                    content=chunk.content,
                    project_id=state["project_id"],
                    document_id=state["document_id"],
                    page_number=chunk.page_number,
                    chunk_index=chunk.chunk_index,
                    metadata={
                        "job_id": state["job_id"],
                        "start_char": chunk.start_char,
                        "end_char": chunk.end_char,
                        **chunk.metadata,
                    },
                )
                documents.append(doc)

            # Store documents (embeddings generated automatically)
            ids = await self.vector_store.add_documents(documents)

            logger.info(
                "Documents stored",
                job_id=state["job_id"],
                count=len(ids),
            )

            return {
                "embeddings_stored": len(ids),
                "status": "completed",
                "progress": 1.0,
            }

        except Exception as e:
            logger.error("Embedding/storage failed", error=str(e))
            return {
                "status": "failed",
                "error": f"Embedding/storage failed: {str(e)}",
            }

    def _check_embedding(self, state: IngestState) -> str:
        """Check if embedding succeeded."""
        if state.get("status") == "failed":
            return "error"
        return "success"

    async def _handle_error(self, state: IngestState) -> dict[str, Any]:
        """Handle pipeline errors."""
        logger.error(
            "Pipeline error",
            job_id=state["job_id"],
            error=state.get("error"),
        )
        return {
            "status": "failed",
        }

    async def run(
        self,
        job_id: str,
        project_id: str,
        document_id: str,
        file_path: str | None = None,
        file_bytes: bytes | None = None,
    ) -> IngestState:
        """
        Run the ingestion pipeline.

        Args:
            job_id: Unique job identifier
            project_id: Project this document belongs to
            document_id: Document identifier
            file_path: Path to PDF file (or file_bytes)
            file_bytes: PDF content as bytes (or file_path)

        Returns:
            Final pipeline state
        """
        initial_state: IngestState = {
            "job_id": job_id,
            "project_id": project_id,
            "document_id": document_id,
            "file_path": file_path,
            "file_bytes": file_bytes,
            "pdf_metadata": None,
            "page_images": [],
            "ocr_results": [],
            "chunks": [],
            "embeddings_stored": 0,
            "status": "pending",
            "error": None,
            "progress": 0.0,
        }

        logger.info(
            "Starting ingestion pipeline",
            job_id=job_id,
            document_id=document_id,
        )

        result = await self.graph.ainvoke(initial_state)
        return result


def create_ingest_graph(
    gemini_client: GeminiClient,
    embeddings: GeminiEmbeddings,
    vector_store: VectorStore,
) -> IngestPipeline:
    """Factory function to create an ingestion pipeline."""
    return IngestPipeline(gemini_client, embeddings, vector_store)
