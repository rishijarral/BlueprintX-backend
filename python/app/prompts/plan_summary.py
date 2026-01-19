"""Prompt templates for plan/project summary generation."""

PLAN_SUMMARY_PROMPT = """You are a senior construction estimator and plan analyst. Analyze the following construction document text and provide a comprehensive project summary.

**Document Content:**
{document_text}

{instructions}

**Your task:**
Generate a structured summary of this construction project. Be thorough but concise.

**Guidelines:**
1. Only include information that can be directly inferred from the document
2. If information is not available, use null or empty arrays - do NOT invent details
3. For building_type, use standard categories: residential, commercial, industrial, institutional, mixed_use, infrastructure, other
4. List specific materials mentioned (e.g., "concrete masonry units" not just "masonry")
5. For risks, focus on scope gaps, ambiguities, or potential conflicts
6. For assumptions, list what you had to assume due to incomplete information
7. Confidence should reflect how complete the source information is (0.0-1.0)

**Output format:** Return a JSON object with this exact structure:
{{
    "building_type": "string",
    "project_name": "string or null",
    "floors": number or null,
    "total_area_sqft": number or null,
    "key_materials": ["list of primary construction materials"],
    "major_systems": ["list of building systems: structural, mechanical, electrical, plumbing, fire_protection, etc."],
    "structural_system": "string describing primary structure or null",
    "risks": ["list of identified risks or concerns"],
    "assumptions": ["list of assumptions made during analysis"],
    "confidence": 0.0 to 1.0
}}

Analyze the document and provide your summary:"""


def build_plan_summary_prompt(
    document_text: str,
    instructions: str | None = None,
    project_context: str | None = None,
) -> str:
    """
    Build the plan summary prompt with document content.

    Args:
        document_text: The extracted/OCR'd document content
        instructions: Optional additional instructions from user
        project_context: Optional context about the project

    Returns:
        Formatted prompt string
    """
    instruction_text = ""
    if instructions:
        instruction_text = f"**Additional Instructions:** {instructions}\n"
    if project_context:
        instruction_text += f"**Project Context:** {project_context}\n"

    return PLAN_SUMMARY_PROMPT.format(
        document_text=document_text[:50000],  # Truncate to avoid token limits
        instructions=instruction_text,
    )


# Focused summary prompts for specific aspects

STRUCTURAL_SUMMARY_PROMPT = """Analyze this document focusing on STRUCTURAL aspects only:

{document_text}

Extract:
1. Foundation type and specifications
2. Structural framing system (steel, concrete, wood, etc.)
3. Floor/roof deck systems
4. Lateral force resisting system (if mentioned)
5. Special structural features or requirements
6. Structural risks or areas needing clarification

Return JSON:
{{
    "foundation_type": "string or null",
    "framing_system": "string or null",
    "floor_system": "string or null",
    "roof_system": "string or null",
    "lateral_system": "string or null",
    "special_features": [],
    "structural_risks": [],
    "confidence": 0.0 to 1.0
}}"""


MEP_SUMMARY_PROMPT = """Analyze this document focusing on MEP (Mechanical, Electrical, Plumbing) systems:

{document_text}

Extract:
1. HVAC system type and capacity
2. Electrical service size and distribution
3. Plumbing systems and fixtures
4. Fire protection systems
5. Building automation/controls
6. Energy efficiency features
7. MEP-related risks or coordination issues

Return JSON:
{{
    "hvac_system": "string or null",
    "hvac_capacity_tons": number or null,
    "electrical_service": "string or null",
    "plumbing_fixtures": [],
    "fire_protection": "string or null",
    "controls": "string or null",
    "energy_features": [],
    "mep_risks": [],
    "confidence": 0.0 to 1.0
}}"""
