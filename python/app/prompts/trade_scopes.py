"""Prompt templates for trade scope extraction."""

TRADE_SCOPES_PROMPT = """You are an expert construction estimator preparing bid packages. Analyze the following document and extract detailed scope information for each trade.

**Document Content:**
{document_text}

**Trades to analyze:**
{trades_list}

**Your task:**
For each trade listed, extract:
1. **Inclusions**: Work items that ARE included in this trade's scope
2. **Exclusions**: Work items that are NOT included or are by others
3. **Required sheets**: Drawing sheets this trade needs to reference
4. **Spec sections**: Specification sections applicable to this trade
5. **RFI needed**: Items requiring clarification before bidding
6. **Assumptions**: Assumptions you made due to unclear information

**Guidelines:**
- Be specific about scope boundaries between trades
- Note coordination requirements between trades
- Flag any scope gaps or overlaps you identify
- Use CSI division numbers where applicable (e.g., "03 30 00 - Cast-in-Place Concrete")
- If a trade has no applicable scope in this document, indicate that clearly
- Don't invent scope items not supported by the document

**Output format:** Return a JSON object:
{{
    "project_id": "{project_id}",
    "trades": [
        {{
            "trade": "trade name",
            "csi_division": "XX XX XX or null",
            "inclusions": ["list of included work items"],
            "exclusions": ["list of excluded items"],
            "required_sheets": ["A101", "S201", etc.],
            "spec_sections": ["section numbers"],
            "rfi_needed": ["questions requiring clarification"],
            "assumptions": ["assumptions made"]
        }}
    ],
    "general_notes": ["notes applicable to all trades"],
    "confidence": 0.0 to 1.0
}}

Analyze the document for the specified trades:"""


# Standard trade list for construction projects
STANDARD_TRADES = [
    "General Conditions",
    "Sitework & Excavation",
    "Concrete",
    "Masonry",
    "Structural Steel",
    "Rough Carpentry",
    "Finish Carpentry & Millwork",
    "Waterproofing & Roofing",
    "Doors, Frames & Hardware",
    "Glass & Glazing",
    "Drywall & Framing",
    "Painting",
    "Flooring",
    "Ceiling Systems",
    "Mechanical (HVAC)",
    "Plumbing",
    "Electrical",
    "Fire Protection",
    "Fire Alarm",
    "Elevators & Conveyance",
]


def build_trade_scopes_prompt(
    document_text: str,
    trades: list[str] | None = None,
    project_id: str | None = None,
) -> str:
    """
    Build the trade scopes extraction prompt.

    Args:
        document_text: The document content to analyze
        trades: List of trades to extract (defaults to STANDARD_TRADES)
        project_id: Optional project identifier

    Returns:
        Formatted prompt string
    """
    if trades is None:
        trades = STANDARD_TRADES

    trades_list = "\n".join(f"- {trade}" for trade in trades)

    return TRADE_SCOPES_PROMPT.format(
        document_text=document_text[:50000],
        trades_list=trades_list,
        project_id=project_id or "unknown",
    )


# Specialized prompts for specific trade deep-dives

ELECTRICAL_SCOPE_PROMPT = """Analyze this document for ELECTRICAL scope only.

{document_text}

Extract detailed electrical scope:
1. **Power Distribution**
   - Service entrance size and voltage
   - Switchgear and distribution equipment
   - Panel locations and sizes
   - Emergency/standby power systems

2. **Branch Circuiting**
   - Receptacle types and quantities
   - Dedicated circuits required
   - Special voltage requirements

3. **Lighting**
   - Fixture types and quantities
   - Lighting controls
   - Emergency/exit lighting

4. **Low Voltage Systems**
   - Fire alarm (if not separate contract)
   - Data/communications rough-in
   - Security systems
   - Audio/visual systems

5. **Site Electrical**
   - Site lighting
   - Underground utilities

Return detailed JSON scope for electrical trade."""


MECHANICAL_SCOPE_PROMPT = """Analyze this document for MECHANICAL (HVAC) scope only.

{document_text}

Extract detailed mechanical scope:
1. **Heating Systems**
   - Boilers/furnaces
   - Distribution (hot water, steam, forced air)
   - Terminal units

2. **Cooling Systems**
   - Chillers/condensing units
   - Air handling units
   - Ductwork scope
   - Terminal units (VAV, FCU, etc.)

3. **Ventilation**
   - Outside air requirements
   - Exhaust systems
   - Make-up air units

4. **Controls**
   - Building automation system
   - Thermostats/sensors
   - Integration requirements

5. **Specialties**
   - Kitchen exhaust
   - Lab exhaust
   - Clean room requirements

Return detailed JSON scope for mechanical trade."""
