"""Prompt templates for Gemini Vision OCR on architectural/engineering drawings."""

VISION_OCR_PROMPT = """You are an expert construction document analyst specializing in reading architectural and engineering drawings (blueprints).

Analyze this drawing page and extract all readable information. This is page {page_number} of a construction document set.

**Your task:**
1. Identify the drawing type (floor plan, elevation, section, detail, schedule, etc.)
2. Extract the sheet number and title from the title block
3. Identify the discipline (Architectural, Structural, Mechanical, Electrical, Plumbing, Civil, etc.)
4. Extract ALL visible text content including:
   - Notes and annotations
   - Dimensions and measurements
   - Material callouts and specifications
   - Room names/numbers
   - Equipment tags
   - Reference symbols and their meanings
5. List any references to other drawings (e.g., "See Detail 3/A501")
6. Identify key materials mentioned
7. Note any revision information visible

**Important guidelines:**
- If you cannot read something clearly, indicate "[unclear]" rather than guessing
- Preserve the hierarchical structure of notes (general notes, keynotes, etc.)
- Include scale information if visible
- Note any stamps, seals, or approval marks
- If this appears to be a cover sheet or index, extract the drawing list

**Output format:** Return a JSON object with the following structure:
{{
    "page_number": {page_number},
    "sheet_number": "string or null if not found",
    "sheet_title": "string or null",
    "drawing_type": "plan|elevation|section|detail|schedule|diagram|cover|other",
    "discipline": "architectural|structural|mechanical|electrical|plumbing|civil|fire_protection|other",
    "text_content": "all extracted text as a single string",
    "annotations": ["list of callouts and annotations"],
    "dimensions": ["list of key dimensions found"],
    "notes": ["list of drawing notes"],
    "materials": ["list of materials mentioned"],
    "references": ["list of references to other drawings"]
}}

Analyze the drawing now:"""


def build_vision_ocr_prompt(page_number: int, additional_context: str | None = None) -> str:
    """
    Build the vision OCR prompt for a specific page.

    Args:
        page_number: The page number being analyzed
        additional_context: Optional additional instructions

    Returns:
        Formatted prompt string
    """
    prompt = VISION_OCR_PROMPT.format(page_number=page_number)

    if additional_context:
        prompt += f"\n\n**Additional context:** {additional_context}"

    return prompt


# Specialized prompts for different drawing types

FLOOR_PLAN_OCR_PROMPT = """You are analyzing an architectural floor plan drawing.

Focus on extracting:
1. Room names, numbers, and square footages
2. Wall types and thicknesses
3. Door and window schedules/tags
4. Finish schedules
5. Furniture layouts if shown
6. Circulation paths and exits
7. Accessibility features (ramps, elevators, accessible restrooms)
8. Fire-rated assemblies and smoke compartments

Page {page_number}

Return JSON with standard fields plus:
{{
    "rooms": [{{"name": "", "number": "", "area_sqft": null}}],
    "doors": [{{"tag": "", "type": "", "size": ""}}],
    "windows": [{{"tag": "", "type": "", "size": ""}}]
}}"""


STRUCTURAL_DRAWING_OCR_PROMPT = """You are analyzing a structural engineering drawing.

Focus on extracting:
1. Structural grid lines and dimensions
2. Foundation details and specifications
3. Column sizes and locations
4. Beam sizes and spans
5. Slab thicknesses and reinforcement
6. Steel connection details
7. Concrete specifications (strength, cover, etc.)
8. Load information if shown
9. Expansion joint locations

Page {page_number}

Return JSON with standard fields plus:
{{
    "grid_lines": ["A", "B", "1", "2"],
    "columns": [{{"grid": "", "size": "", "material": ""}}],
    "beams": [{{"mark": "", "size": "", "span": ""}}],
    "concrete_specs": {{"strength_psi": null, "cover_inches": null}}
}}"""


MEP_DRAWING_OCR_PROMPT = """You are analyzing a Mechanical/Electrical/Plumbing (MEP) drawing.

Focus on extracting:
1. Equipment schedules and specifications
2. Duct/pipe sizes and routing
3. Electrical panel schedules
4. Fixture counts and types
5. Control sequences if shown
6. Circuiting information
7. Fire alarm device locations
8. Plumbing fixture unit calculations

Page {page_number}

Return JSON with standard fields plus:
{{
    "equipment": [{{"tag": "", "description": "", "specs": ""}}],
    "systems": ["HVAC", "electrical", "plumbing", "fire_alarm"],
    "panels": [{{"name": "", "voltage": "", "amperage": ""}}]
}}"""
