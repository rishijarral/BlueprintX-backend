"""Prompt templates for room extraction from blueprints."""

ROOMS_EXTRACTION_PROMPT = """You are an expert architectural analyst extracting room information from floor plans and blueprints.

**Document Content (OCR extracted):**
{document_text}

**Page Information:**
- Current page: {page_number}
- Document ID: {document_id}
- Project ID: {project_id}

**Your Task:**
Extract ALL rooms and spaces identified on this page. For each room, capture:
1. **Room Name**: The room's name (e.g., "Conference Room", "Kitchen")
2. **Room Number**: Room identifier if shown (e.g., "101", "A-201")
3. **Room Type**: Classification (office, restroom, corridor, storage, etc.)
4. **Floor**: Floor level (Ground, 1st, 2nd, Basement, etc.)
5. **Area**: Square footage if noted
6. **Ceiling Height**: If specified
7. **Finishes**: Floor, wall, ceiling, and base finishes from finish schedules
8. **Fixtures**: Plumbing fixtures, built-ins, equipment noted

**Room Types to Look For:**
- Office spaces: Private offices, open office, conference rooms
- Support: Restrooms, kitchens, break rooms, copy rooms
- Circulation: Corridors, lobbies, elevator lobbies, stairs
- Utility: Mechanical, electrical, IT/server, janitor
- Storage: General storage, file rooms, warehouses
- Special: Labs, clean rooms, assembly spaces

**Finish Schedule Key (if applicable):**
Look for room finish schedules with codes like:
- Floor: VCT, CPT, CT, EP, POL-CON (vinyl, carpet, ceramic tile, epoxy, polished concrete)
- Walls: PT, WC, CT, FRP (paint, wallcovering, ceramic tile, fiberglass)
- Ceiling: ACT, GYP, EXP (acoustic tile, gypsum, exposed)
- Base: RB, WB, CT (rubber base, wood base, ceramic tile)

**Guidelines:**
- Extract rooms from floor plans, room schedules, and finish schedules
- Include all spaces even if minimal info (just name/number)
- Correlate finish codes with the finish schedule legend if present
- Note door and window counts if visible
- Extract dimensions or areas when shown
- For multi-floor documents, note the floor level

**Output Format:** Return a JSON object:
{{
    "rooms": [
        {{
            "room_name": "room name",
            "room_number": "101" or null,
            "room_type": "classification" or null,
            "floor": "floor level" or null,
            "area_sqft": 150.0 or null,
            "ceiling_height": 9.0 or null,
            "perimeter_ft": 50.0 or null,
            "finishes": {{
                "floor": "VCT" or null,
                "walls": "PT-1" or null,
                "ceiling": "ACT" or null,
                "base": "RB-4" or null,
                "paint_color": "SW 7015" or null
            }},
            "fixtures": ["toilet", "sink"] or [],
            "notes": "additional notes" or null,
            "source_page": {page_number},
            "confidence": 0.0 to 1.0
        }}
    ],
    "finish_legend": {{
        "VCT": "Vinyl Composition Tile",
        "CPT": "Carpet"
    }},
    "extraction_notes": ["notes about extraction"],
    "confidence": 0.0 to 1.0
}}

Extract all rooms from this page:"""


ROOMS_AGGREGATION_PROMPT = """You are an expert architectural analyst consolidating room data from multiple pages.

**Extracted Rooms from All Pages:**
{rooms_json}

**Finish Legend (consolidated):**
{finish_legend}

**Your Task:**
1. Merge duplicate room entries (same room on multiple pages)
2. Consolidate finish information from different sources
3. Calculate totals (area per floor, per type)
4. Organize rooms by floor and type

**Guidelines:**
- Combine information from floor plans and finish schedules
- Use the most complete entry when duplicates exist
- Expand finish codes to full descriptions where possible
- Flag any conflicting information
- Calculate total area by floor if individual areas are available

**Output Format:** Return a JSON object:
{{
    "rooms": [
        {{
            "room_name": "consolidated name",
            "room_number": "number",
            "room_type": "type",
            "floor": "floor",
            "area_sqft": area or null,
            "ceiling_height": height or null,
            "perimeter_ft": perimeter or null,
            "finishes": {{
                "floor": "full description",
                "walls": "full description",
                "ceiling": "full description",
                "base": "full description",
                "paint_color": "color code/name" or null
            }},
            "fixtures": ["list"],
            "notes": "combined notes",
            "source_pages": [1, 3],
            "confidence": 0.0 to 1.0
        }}
    ],
    "summary": {{
        "total_rooms": number,
        "total_area_sqft": total or null,
        "floors": ["Ground", "2nd"],
        "room_types": {{"office": 5, "restroom": 2}}
    }},
    "extraction_notes": ["consolidation notes"],
    "confidence": 0.0 to 1.0
}}

Consolidate the room data:"""


# Common room type mappings
ROOM_TYPE_MAPPINGS = {
    "office": ["office", "ofc", "private office", "open office"],
    "conference": ["conference", "conf", "meeting", "boardroom"],
    "restroom": ["restroom", "toilet", "bathroom", "wc", "lavatory"],
    "kitchen": ["kitchen", "kitchenette", "break room", "pantry"],
    "storage": ["storage", "stor", "closet", "janitor", "jc"],
    "corridor": ["corridor", "corr", "hall", "hallway", "passage"],
    "lobby": ["lobby", "reception", "vestibule", "entry"],
    "mechanical": ["mechanical", "mech", "hvac", "boiler", "equipment"],
    "electrical": ["electrical", "elec", "transformer", "switchgear"],
    "data": ["data", "server", "it", "telecom", "mdf", "idf"],
    "stair": ["stair", "stairwell", "egress"],
    "elevator": ["elevator", "elev", "lift"],
}


def build_rooms_prompt(
    document_text: str,
    page_number: int,
    document_id: str | None = None,
    project_id: str | None = None,
) -> str:
    """
    Build the rooms extraction prompt for a single page.

    Args:
        document_text: The OCR extracted text from the page
        page_number: The page number
        document_id: Optional document identifier
        project_id: Optional project identifier

    Returns:
        Formatted prompt string
    """
    return ROOMS_EXTRACTION_PROMPT.format(
        document_text=document_text[:30000],
        page_number=page_number,
        document_id=document_id or "unknown",
        project_id=project_id or "unknown",
    )


def build_rooms_aggregation_prompt(
    rooms_json: str,
    finish_legend: str | None = None,
) -> str:
    """
    Build the rooms aggregation prompt.

    Args:
        rooms_json: JSON string of all extracted rooms
        finish_legend: Optional consolidated finish legend

    Returns:
        Formatted prompt string
    """
    return ROOMS_AGGREGATION_PROMPT.format(
        rooms_json=rooms_json[:50000],
        finish_legend=finish_legend or "{}",
    )


def normalize_room_type(room_name: str, room_type: str | None = None) -> str | None:
    """
    Normalize a room type to a standard category.

    Args:
        room_name: The room name
        room_type: Optional existing room type

    Returns:
        Normalized room type or None
    """
    search_text = f"{room_name} {room_type or ''}".lower()

    for category, keywords in ROOM_TYPE_MAPPINGS.items():
        for keyword in keywords:
            if keyword in search_text:
                return category

    return room_type
