"""Text chunking strategies for document embedding."""

from enum import Enum

from pydantic import BaseModel

from app.config import Settings, get_settings
from app.logging import get_logger

logger = get_logger(__name__)


class ChunkingStrategy(str, Enum):
    """Available chunking strategies."""

    FIXED_SIZE = "fixed_size"
    SEMANTIC = "semantic"  # Future: use LLM to find semantic boundaries
    SENTENCE = "sentence"
    PARAGRAPH = "paragraph"


class TextChunk(BaseModel):
    """A chunk of text with metadata."""

    content: str
    chunk_index: int
    start_char: int
    end_char: int
    metadata: dict = {}

    # Optional source tracking
    page_number: int | None = None
    source_type: str | None = None  # "ocr", "extracted", etc.


class Chunker:
    """
    Text chunker for preparing documents for embedding.

    Supports multiple strategies optimized for different document types.
    Construction/blueprint documents often have specific structures
    (sheets, sections, notes) that benefit from semantic chunking.
    """

    def __init__(self, settings: Settings | None = None) -> None:
        self.settings = settings or get_settings()
        self.chunk_size = self.settings.chunk_size
        self.chunk_overlap = self.settings.chunk_overlap

    def chunk_text(
        self,
        text: str,
        strategy: ChunkingStrategy = ChunkingStrategy.FIXED_SIZE,
        page_number: int | None = None,
        metadata: dict | None = None,
    ) -> list[TextChunk]:
        """
        Split text into chunks using the specified strategy.

        Args:
            text: Text to chunk
            strategy: Chunking strategy to use
            page_number: Optional page number for tracking
            metadata: Optional metadata to include in each chunk

        Returns:
            List of TextChunk objects
        """
        if not text or not text.strip():
            return []

        base_metadata = metadata or {}

        if strategy == ChunkingStrategy.FIXED_SIZE:
            return self._chunk_fixed_size(text, page_number, base_metadata)
        elif strategy == ChunkingStrategy.SENTENCE:
            return self._chunk_by_sentences(text, page_number, base_metadata)
        elif strategy == ChunkingStrategy.PARAGRAPH:
            return self._chunk_by_paragraphs(text, page_number, base_metadata)
        else:
            # Default to fixed size
            return self._chunk_fixed_size(text, page_number, base_metadata)

    def _chunk_fixed_size(
        self,
        text: str,
        page_number: int | None,
        metadata: dict,
    ) -> list[TextChunk]:
        """
        Split text into fixed-size chunks with overlap.

        This is the most reliable strategy for arbitrary text.
        """
        chunks = []
        text_length = len(text)

        if text_length <= self.chunk_size:
            # Text fits in one chunk
            return [
                TextChunk(
                    content=text,
                    chunk_index=0,
                    start_char=0,
                    end_char=text_length,
                    page_number=page_number,
                    metadata=metadata,
                )
            ]

        start = 0
        chunk_index = 0

        while start < text_length:
            # Calculate end position
            end = min(start + self.chunk_size, text_length)

            # Try to break at word boundary (look back for space)
            if end < text_length:
                # Look for last space within chunk
                last_space = text.rfind(" ", start, end)
                if last_space > start + self.chunk_size // 2:
                    end = last_space + 1  # Include the space

            chunk_text = text[start:end].strip()

            if chunk_text:
                chunks.append(
                    TextChunk(
                        content=chunk_text,
                        chunk_index=chunk_index,
                        start_char=start,
                        end_char=end,
                        page_number=page_number,
                        metadata=metadata,
                    )
                )
                chunk_index += 1

            # Move start with overlap
            start = end - self.chunk_overlap
            if start >= text_length:
                break

        logger.debug(
            "Fixed-size chunking complete",
            total_chunks=len(chunks),
            text_length=text_length,
        )

        return chunks

    def _chunk_by_sentences(
        self,
        text: str,
        page_number: int | None,
        metadata: dict,
    ) -> list[TextChunk]:
        """
        Split text by sentences, combining until chunk size is reached.

        Better for narrative text; preserves sentence boundaries.
        """
        import re

        # Simple sentence splitting (handles common cases)
        sentence_pattern = r"(?<=[.!?])\s+"
        sentences = re.split(sentence_pattern, text)

        chunks = []
        current_chunk = []
        current_length = 0
        chunk_index = 0
        start_char = 0

        for sentence in sentences:
            sentence = sentence.strip()
            if not sentence:
                continue

            sentence_length = len(sentence)

            # Check if adding this sentence exceeds chunk size
            if current_length + sentence_length > self.chunk_size and current_chunk:
                # Save current chunk
                chunk_text = " ".join(current_chunk)
                chunks.append(
                    TextChunk(
                        content=chunk_text,
                        chunk_index=chunk_index,
                        start_char=start_char,
                        end_char=start_char + len(chunk_text),
                        page_number=page_number,
                        metadata=metadata,
                    )
                )
                chunk_index += 1

                # Start new chunk with overlap (include last sentence)
                if self.chunk_overlap > 0 and current_chunk:
                    overlap_text = current_chunk[-1]
                    current_chunk = [overlap_text, sentence]
                    current_length = len(overlap_text) + sentence_length + 1
                else:
                    current_chunk = [sentence]
                    current_length = sentence_length

                start_char += len(chunk_text) + 1
            else:
                current_chunk.append(sentence)
                current_length += sentence_length + 1

        # Add remaining chunk
        if current_chunk:
            chunk_text = " ".join(current_chunk)
            chunks.append(
                TextChunk(
                    content=chunk_text,
                    chunk_index=chunk_index,
                    start_char=start_char,
                    end_char=start_char + len(chunk_text),
                    page_number=page_number,
                    metadata=metadata,
                )
            )

        logger.debug(
            "Sentence chunking complete",
            total_chunks=len(chunks),
            sentences=len(sentences),
        )

        return chunks

    def _chunk_by_paragraphs(
        self,
        text: str,
        page_number: int | None,
        metadata: dict,
    ) -> list[TextChunk]:
        """
        Split text by paragraphs (double newlines).

        Best for structured documents with clear paragraph breaks.
        """
        import re

        # Split on double newlines
        paragraphs = re.split(r"\n\s*\n", text)

        chunks = []
        current_chunk = []
        current_length = 0
        chunk_index = 0
        start_char = 0

        for para in paragraphs:
            para = para.strip()
            if not para:
                continue

            para_length = len(para)

            # If single paragraph exceeds chunk size, use fixed-size chunking
            if para_length > self.chunk_size:
                # Save current chunk first
                if current_chunk:
                    chunk_text = "\n\n".join(current_chunk)
                    chunks.append(
                        TextChunk(
                            content=chunk_text,
                            chunk_index=chunk_index,
                            start_char=start_char,
                            end_char=start_char + len(chunk_text),
                            page_number=page_number,
                            metadata=metadata,
                        )
                    )
                    chunk_index += 1
                    start_char += len(chunk_text) + 2
                    current_chunk = []
                    current_length = 0

                # Chunk the large paragraph
                para_chunks = self._chunk_fixed_size(para, page_number, metadata)
                for pc in para_chunks:
                    pc.chunk_index = chunk_index
                    chunks.append(pc)
                    chunk_index += 1

                continue

            if current_length + para_length > self.chunk_size and current_chunk:
                chunk_text = "\n\n".join(current_chunk)
                chunks.append(
                    TextChunk(
                        content=chunk_text,
                        chunk_index=chunk_index,
                        start_char=start_char,
                        end_char=start_char + len(chunk_text),
                        page_number=page_number,
                        metadata=metadata,
                    )
                )
                chunk_index += 1
                start_char += len(chunk_text) + 2
                current_chunk = [para]
                current_length = para_length
            else:
                current_chunk.append(para)
                current_length += para_length + 2

        if current_chunk:
            chunk_text = "\n\n".join(current_chunk)
            chunks.append(
                TextChunk(
                    content=chunk_text,
                    chunk_index=chunk_index,
                    start_char=start_char,
                    end_char=start_char + len(chunk_text),
                    page_number=page_number,
                    metadata=metadata,
                )
            )

        logger.debug(
            "Paragraph chunking complete",
            total_chunks=len(chunks),
            paragraphs=len(paragraphs),
        )

        return chunks

    def chunk_pages(
        self,
        page_texts: list[tuple[int, str]],
        strategy: ChunkingStrategy = ChunkingStrategy.FIXED_SIZE,
        document_id: str | None = None,
    ) -> list[TextChunk]:
        """
        Chunk multiple pages of text.

        Args:
            page_texts: List of (page_number, text) tuples
            strategy: Chunking strategy
            document_id: Optional document ID for metadata

        Returns:
            All chunks from all pages
        """
        all_chunks = []
        global_index = 0

        for page_num, text in page_texts:
            metadata = {}
            if document_id:
                metadata["document_id"] = document_id

            page_chunks = self.chunk_text(
                text,
                strategy=strategy,
                page_number=page_num,
                metadata=metadata,
            )

            # Update global indices
            for chunk in page_chunks:
                chunk.chunk_index = global_index
                all_chunks.append(chunk)
                global_index += 1

        logger.info(
            "Multi-page chunking complete",
            pages=len(page_texts),
            total_chunks=len(all_chunks),
        )

        return all_chunks
