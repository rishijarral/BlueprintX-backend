"""Prompt templates for project milestone extraction from blueprints."""

MILESTONES_EXTRACTION_PROMPT = """You are an expert construction scheduler analyzing blueprints to identify project milestones and phases.

**Document Content (OCR extracted):**
{document_text}

**Document Type:** {document_type}
**Project ID:** {project_id}

**Your Task:**
Analyze the document to identify construction phases and milestones. Extract:
1. **Milestone Name**: Clear, concise milestone description
2. **Description**: What work is involved
3. **Phase**: Construction phase (Pre-construction, Foundation, Structural, etc.)
4. **Phase Order**: Sequence within the project
5. **Duration**: Estimated duration in days if inferable
6. **Dependencies**: What must be completed before this milestone
7. **Trades Involved**: Which trades participate
8. **Deliverables**: What's produced/completed at this milestone

**Standard Construction Phases:**
1. Pre-Construction: Permits, mobilization, site prep
2. Foundation: Excavation, footings, foundation walls, slab
3. Structural: Steel erection, concrete structure, framing
4. Building Envelope: Roofing, exterior walls, windows, waterproofing
5. Rough-In: MEP rough-in (electrical, plumbing, HVAC)
6. Insulation & Drywall: Insulation, drywall, taping
7. Finishes: Flooring, painting, trim, ceilings
8. MEP Finish: Fixtures, equipment, final connections
9. Closeout: Punch list, final inspections, CO

**Milestones to Identify:**
- Permit approval
- Site mobilization
- Foundation pour
- Steel/structure complete (topping out)
- Building dried-in (weather tight)
- Rough-in inspections
- Drywall complete
- Finishes complete
- Substantial completion
- Final completion / CO

**Sources of Information:**
- General notes about construction sequence
- Phasing diagrams
- Specification sections on scheduling
- Drawing notes about sequencing
- References to inspections or holdpoints

**Guidelines:**
- Infer milestones from drawing sequences and trade coordination notes
- Identify dependencies from notes like "install after..." or "coordinate with..."
- Estimate durations only if explicitly stated or strongly implied
- Include inspection milestones (foundation, framing, final)
- Note any phasing requirements (occupied renovation, etc.)

**Output Format:** Return a JSON object:
{{
    "milestones": [
        {{
            "name": "milestone name",
            "description": "what work is involved",
            "phase": "construction phase",
            "phase_order": 1,
            "estimated_duration_days": 14 or null,
            "dependencies": ["previous milestone names"],
            "trades_involved": ["trade names"],
            "deliverables": ["what's completed"],
            "confidence": 0.0 to 1.0
        }}
    ],
    "phases_identified": ["Pre-Construction", "Foundation", ...],
    "extraction_notes": ["notes about extraction"],
    "confidence": 0.0 to 1.0
}}

Analyze and extract milestones:"""


MILESTONES_INFERENCE_PROMPT = """You are an expert construction scheduler creating a project milestone schedule based on project scope.

**Project Information:**
- Building Type: {building_type}
- Total Area: {total_area_sqft} SF
- Number of Floors: {floors}
- Major Systems: {major_systems}
- Key Materials: {key_materials}

**Trades Involved:**
{trades_list}

**Extracted Rooms:**
{rooms_summary}

**Your Task:**
Create a comprehensive milestone schedule for this project based on:
1. Standard construction sequencing
2. Building type and complexity
3. Trades and systems involved
4. Typical durations for buildings of this type

**Duration Guidelines (adjust for project size):**
- Permits/Pre-con: 2-4 weeks
- Foundation: 2-6 weeks (depends on size)
- Structural: 4-12 weeks
- Envelope: 4-8 weeks
- MEP Rough: 6-12 weeks
- Drywall: 3-6 weeks
- Finishes: 4-8 weeks
- Closeout: 2-4 weeks

**Output Format:** Return a JSON object:
{{
    "milestones": [
        {{
            "name": "milestone name",
            "description": "description",
            "phase": "phase name",
            "phase_order": 1,
            "estimated_duration_days": 14,
            "dependencies": ["milestone names"],
            "trades_involved": ["trades"],
            "deliverables": ["deliverables"],
            "confidence": 0.0 to 1.0
        }}
    ],
    "estimated_total_days": total project duration,
    "phases": ["ordered phase list"],
    "extraction_notes": ["notes and assumptions"],
    "confidence": 0.0 to 1.0
}}

Generate the milestone schedule:"""


# Standard construction phases with typical milestones
STANDARD_PHASES = [
    {
        "phase": "Pre-Construction",
        "order": 1,
        "milestones": [
            "Permit Approval",
            "Site Mobilization",
            "Temporary Facilities",
        ],
    },
    {
        "phase": "Foundation",
        "order": 2,
        "milestones": [
            "Excavation Complete",
            "Footings Poured",
            "Foundation Walls Complete",
            "Slab on Grade Poured",
            "Foundation Backfill",
        ],
    },
    {
        "phase": "Structural",
        "order": 3,
        "milestones": [
            "Steel Erection Start",
            "Structural Steel Complete",
            "Metal Deck Complete",
            "Concrete Topping Poured",
            "Topping Out",
        ],
    },
    {
        "phase": "Building Envelope",
        "order": 4,
        "milestones": [
            "Roofing Complete",
            "Exterior Framing Complete",
            "Windows Installed",
            "Exterior Cladding Complete",
            "Building Dried-In",
        ],
    },
    {
        "phase": "MEP Rough-In",
        "order": 5,
        "milestones": [
            "Underground Plumbing Complete",
            "Electrical Rough-In Complete",
            "Plumbing Rough-In Complete",
            "HVAC Rough-In Complete",
            "Fire Protection Rough-In Complete",
            "MEP Rough-In Inspection",
        ],
    },
    {
        "phase": "Interior Rough",
        "order": 6,
        "milestones": [
            "Interior Framing Complete",
            "Insulation Complete",
            "Drywall Hung",
            "Drywall Taped & Finished",
        ],
    },
    {
        "phase": "Finishes",
        "order": 7,
        "milestones": [
            "Painting Complete",
            "Flooring Complete",
            "Ceiling Installation Complete",
            "Millwork Installed",
            "Hardware Installed",
        ],
    },
    {
        "phase": "MEP Finish",
        "order": 8,
        "milestones": [
            "Plumbing Fixtures Installed",
            "Electrical Devices & Fixtures Installed",
            "HVAC Equipment Start-Up",
            "Fire Alarm Testing Complete",
            "Systems Commissioning",
        ],
    },
    {
        "phase": "Closeout",
        "order": 9,
        "milestones": [
            "Substantial Completion",
            "Punch List Complete",
            "Final Inspections",
            "Certificate of Occupancy",
            "Final Completion",
        ],
    },
]


def build_milestones_prompt(
    document_text: str,
    document_type: str = "construction drawings",
    project_id: str | None = None,
) -> str:
    """
    Build the milestones extraction prompt.

    Args:
        document_text: The combined OCR text from documents
        document_type: Type of document being analyzed
        project_id: Optional project identifier

    Returns:
        Formatted prompt string
    """
    return MILESTONES_EXTRACTION_PROMPT.format(
        document_text=document_text[:40000],
        document_type=document_type,
        project_id=project_id or "unknown",
    )


def build_milestones_inference_prompt(
    building_type: str,
    total_area_sqft: int | None = None,
    floors: int | None = None,
    major_systems: list[str] | None = None,
    key_materials: list[str] | None = None,
    trades: list[str] | None = None,
    rooms_summary: str | None = None,
) -> str:
    """
    Build the milestones inference prompt for generating schedule from project info.

    Args:
        building_type: Type of building
        total_area_sqft: Total building area
        floors: Number of floors
        major_systems: List of major building systems
        key_materials: List of key construction materials
        trades: List of trades involved
        rooms_summary: Summary of rooms/spaces

    Returns:
        Formatted prompt string
    """
    return MILESTONES_INFERENCE_PROMPT.format(
        building_type=building_type,
        total_area_sqft=total_area_sqft or "Unknown",
        floors=floors or "Unknown",
        major_systems=", ".join(major_systems) if major_systems else "Standard MEP",
        key_materials=", ".join(key_materials) if key_materials else "Not specified",
        trades_list="\n".join(f"- {t}" for t in trades) if trades else "Standard trades",
        rooms_summary=rooms_summary or "Not available",
    )


def get_standard_phases() -> list[dict]:
    """Get the list of standard construction phases with milestones."""
    return STANDARD_PHASES


def estimate_duration_by_building_type(
    building_type: str,
    total_area_sqft: int | None = None,
) -> dict[str, int]:
    """
    Estimate phase durations based on building type and size.

    Args:
        building_type: Type of building
        total_area_sqft: Total building area

    Returns:
        Dictionary of phase durations in days
    """
    # Base durations for a 10,000 SF building
    base_durations = {
        "Pre-Construction": 21,
        "Foundation": 28,
        "Structural": 42,
        "Building Envelope": 35,
        "MEP Rough-In": 56,
        "Interior Rough": 28,
        "Finishes": 42,
        "MEP Finish": 28,
        "Closeout": 21,
    }

    # Scale factor based on building size
    if total_area_sqft:
        scale = max(0.5, min(3.0, total_area_sqft / 10000))
    else:
        scale = 1.0

    # Building type multipliers
    type_multipliers = {
        "residential": 0.8,
        "commercial": 1.0,
        "retail": 0.9,
        "industrial": 0.7,
        "healthcare": 1.5,
        "educational": 1.2,
        "hospitality": 1.3,
        "mixed-use": 1.2,
    }

    multiplier = type_multipliers.get(building_type.lower(), 1.0)

    return {
        phase: int(days * scale * multiplier)
        for phase, days in base_durations.items()
    }
