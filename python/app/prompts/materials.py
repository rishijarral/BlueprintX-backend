"""Prompt templates for material takeoff extraction from blueprints."""

MATERIALS_EXTRACTION_PROMPT = """You are an expert construction estimator performing a material takeoff from blueprints and construction documents.

**Document Content (OCR extracted):**
{document_text}

**Page Information:**
- Current page: {page_number}
- Document ID: {document_id}
- Project ID: {project_id}

**Your Task:**
Extract ALL materials mentioned on this page. For each material, capture:
1. **Name**: Material name (e.g., "2x4 SPF Stud", "Type X Gypsum Board")
2. **Description**: Additional details about the material
3. **Quantity**: Numerical quantity if mentioned (with unit)
4. **Unit**: Unit of measurement (SF, LF, EA, CY, etc.)
5. **Location**: Where in the building (e.g., "Ground Floor", "Exterior Walls")
6. **Room**: Specific room or area if mentioned
7. **Specification**: Spec section or product specification
8. **Trade Category**: Which trade uses this (Framing, Drywall, Electrical, etc.)
9. **CSI Division**: CSI MasterFormat division if identifiable

**Material Categories to Look For:**
- Structural: Concrete, rebar, steel beams, wood framing
- Exterior: Roofing, siding, windows, doors, waterproofing
- Interior: Drywall, insulation, flooring, ceiling tiles
- MEP: Pipes, conduit, ductwork, fixtures
- Finishes: Paint, trim, hardware

**Guidelines:**
- Extract materials from schedules, notes, details, and callouts
- Include dimensions and sizes when mentioned (e.g., "3/4 inch plywood")
- Note any brand names or product codes
- If quantity cannot be determined, leave it null
- Assign confidence based on how clearly the material is defined
- Do NOT guess quantities - only extract what's explicitly stated
- Group similar materials (e.g., different sizes of same type)

**Output Format:** Return a JSON object:
{{
    "materials": [
        {{
            "name": "material name",
            "description": "additional details or null",
            "quantity": 100 or null,
            "unit": "SF" or null,
            "location": "location in building or null",
            "room": "room name or null",
            "specification": "spec reference or null",
            "trade_category": "trade name or null",
            "csi_division": "XX XX XX" or null,
            "source_page": {page_number},
            "confidence": 0.0 to 1.0
        }}
    ],
    "extraction_notes": ["notes about the extraction process"],
    "confidence": 0.0 to 1.0
}}

Extract all materials from this page:"""


MATERIALS_AGGREGATION_PROMPT = """You are an expert construction estimator consolidating material takeoffs from multiple pages.

**Extracted Materials from All Pages:**
{materials_json}

**Your Task:**
1. Consolidate duplicate materials (same material mentioned on multiple pages)
2. Aggregate quantities where applicable
3. Resolve any conflicts in specifications or descriptions
4. Organize materials by trade category

**Guidelines:**
- Combine quantities for identical materials
- Use the highest confidence specification when conflicts exist
- Keep materials separate if specifications differ (e.g., different thicknesses)
- Note all source pages for each material
- Provide a confidence score for the consolidated entry

**Output Format:** Return a JSON object:
{{
    "materials": [
        {{
            "name": "consolidated material name",
            "description": "best description",
            "quantity": total quantity or null,
            "unit": "unit",
            "location": "primary location",
            "room": "primary room or null",
            "specification": "consolidated spec",
            "trade_category": "trade",
            "csi_division": "XX XX XX or null",
            "source_pages": [1, 3, 5],
            "confidence": 0.0 to 1.0
        }}
    ],
    "total_count": number,
    "extraction_notes": ["notes about consolidation"],
    "confidence": 0.0 to 1.0
}}

Consolidate the materials:"""


def build_materials_prompt(
    document_text: str,
    page_number: int,
    document_id: str | None = None,
    project_id: str | None = None,
) -> str:
    """
    Build the materials extraction prompt for a single page.

    Args:
        document_text: The OCR extracted text from the page
        page_number: The page number
        document_id: Optional document identifier
        project_id: Optional project identifier

    Returns:
        Formatted prompt string
    """
    return MATERIALS_EXTRACTION_PROMPT.format(
        document_text=document_text[:30000],  # Limit to prevent token overflow
        page_number=page_number,
        document_id=document_id or "unknown",
        project_id=project_id or "unknown",
    )


def build_materials_aggregation_prompt(materials_json: str) -> str:
    """
    Build the materials aggregation prompt.

    Args:
        materials_json: JSON string of all extracted materials

    Returns:
        Formatted prompt string
    """
    return MATERIALS_AGGREGATION_PROMPT.format(
        materials_json=materials_json[:50000],  # Limit for large takeoffs
    )
