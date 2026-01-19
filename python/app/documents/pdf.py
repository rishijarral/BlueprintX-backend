"""PDF processing: convert pages to images for Gemini Vision OCR."""

import io
from pathlib import Path
from typing import BinaryIO

from pdf2image import convert_from_bytes, convert_from_path
from PIL import Image
from pydantic import BaseModel

from app.config import Settings, get_settings
from app.errors import DocumentProcessingError
from app.logging import get_logger

logger = get_logger(__name__)


class PageImage(BaseModel):
    """Represents a single page converted to an image."""

    page_number: int
    image_bytes: bytes
    mime_type: str = "image/png"
    width: int
    height: int

    class Config:
        arbitrary_types_allowed = True


class PDFMetadata(BaseModel):
    """Basic PDF metadata."""

    page_count: int
    file_size_bytes: int
    source_path: str | None = None


class PDFProcessor:
    """
    PDF processor for blueprint/drawing analysis.

    Converts PDF pages to images suitable for Gemini Vision OCR.
    Optimized for technical drawings with configurable DPI.
    """

    def __init__(self, settings: Settings | None = None) -> None:
        self.settings = settings or get_settings()
        self.dpi = self.settings.pdf_dpi

    def _optimize_image(self, image: Image.Image, max_size: int = 4096) -> Image.Image:
        """
        Optimize image for Gemini Vision.

        - Resize if too large (Gemini has limits)
        - Convert to RGB if needed
        """
        # Convert to RGB if necessary
        if image.mode != "RGB":
            image = image.convert("RGB")

        # Resize if larger than max_size while maintaining aspect ratio
        width, height = image.size
        if width > max_size or height > max_size:
            ratio = min(max_size / width, max_size / height)
            new_size = (int(width * ratio), int(height * ratio))
            image = image.resize(new_size, Image.Resampling.LANCZOS)
            logger.debug(
                "Image resized",
                original_size=(width, height),
                new_size=new_size,
            )

        return image

    def _image_to_bytes(self, image: Image.Image, format: str = "PNG") -> bytes:
        """Convert PIL Image to bytes."""
        buffer = io.BytesIO()
        image.save(buffer, format=format, optimize=True)
        return buffer.getvalue()

    async def process_file(
        self,
        file_path: str | Path,
        pages: list[int] | None = None,
    ) -> tuple[PDFMetadata, list[PageImage]]:
        """
        Process a PDF file and convert pages to images.

        Args:
            file_path: Path to PDF file
            pages: Optional list of page numbers (1-indexed). If None, process all.

        Returns:
            Tuple of (metadata, list of page images)

        Raises:
            DocumentProcessingError: If processing fails
        """
        file_path = Path(file_path)

        if not file_path.exists():
            raise DocumentProcessingError(f"PDF file not found: {file_path}")

        logger.info("Processing PDF file", path=str(file_path))

        try:
            # Convert PDF to images
            images = convert_from_path(
                file_path,
                dpi=self.dpi,
                first_page=pages[0] if pages else None,
                last_page=pages[-1] if pages else None,
            )

            metadata = PDFMetadata(
                page_count=len(images),
                file_size_bytes=file_path.stat().st_size,
                source_path=str(file_path),
            )

            page_images = []
            for i, img in enumerate(images):
                page_num = pages[i] if pages else i + 1

                # Optimize for Gemini
                optimized = self._optimize_image(img)
                img_bytes = self._image_to_bytes(optimized)

                page_images.append(
                    PageImage(
                        page_number=page_num,
                        image_bytes=img_bytes,
                        mime_type="image/png",
                        width=optimized.width,
                        height=optimized.height,
                    )
                )

            logger.info(
                "PDF processed",
                pages=len(page_images),
                file_size=metadata.file_size_bytes,
            )

            return metadata, page_images

        except Exception as e:
            logger.error("PDF processing failed", error=str(e), path=str(file_path))
            raise DocumentProcessingError(f"Failed to process PDF: {str(e)}") from e

    async def process_bytes(
        self,
        pdf_bytes: bytes,
        pages: list[int] | None = None,
    ) -> tuple[PDFMetadata, list[PageImage]]:
        """
        Process PDF from bytes.

        Args:
            pdf_bytes: PDF file content as bytes
            pages: Optional list of page numbers (1-indexed)

        Returns:
            Tuple of (metadata, list of page images)
        """
        logger.info("Processing PDF from bytes", size=len(pdf_bytes))

        try:
            images = convert_from_bytes(
                pdf_bytes,
                dpi=self.dpi,
                first_page=pages[0] if pages else None,
                last_page=pages[-1] if pages else None,
            )

            metadata = PDFMetadata(
                page_count=len(images),
                file_size_bytes=len(pdf_bytes),
                source_path=None,
            )

            page_images = []
            for i, img in enumerate(images):
                page_num = pages[i] if pages else i + 1

                optimized = self._optimize_image(img)
                img_bytes = self._image_to_bytes(optimized)

                page_images.append(
                    PageImage(
                        page_number=page_num,
                        image_bytes=img_bytes,
                        mime_type="image/png",
                        width=optimized.width,
                        height=optimized.height,
                    )
                )

            logger.info("PDF bytes processed", pages=len(page_images))
            return metadata, page_images

        except Exception as e:
            logger.error("PDF bytes processing failed", error=str(e))
            raise DocumentProcessingError(f"Failed to process PDF bytes: {str(e)}") from e

    async def process_stream(
        self,
        stream: BinaryIO,
        pages: list[int] | None = None,
    ) -> tuple[PDFMetadata, list[PageImage]]:
        """
        Process PDF from a file-like stream.

        Args:
            stream: File-like object with PDF content
            pages: Optional list of page numbers

        Returns:
            Tuple of (metadata, list of page images)
        """
        pdf_bytes = stream.read()
        return await self.process_bytes(pdf_bytes, pages)

    async def get_page_count(self, file_path: str | Path) -> int:
        """
        Get the number of pages in a PDF without full processing.

        This is faster than full processing when you only need the count.
        """
        from pdf2image.pdf2image import pdfinfo_from_path

        try:
            info = pdfinfo_from_path(str(file_path))
            return info.get("Pages", 0)
        except Exception as e:
            logger.error("Failed to get page count", error=str(e))
            raise DocumentProcessingError(f"Failed to get page count: {str(e)}") from e
