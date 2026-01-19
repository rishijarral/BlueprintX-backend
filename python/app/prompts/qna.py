"""Prompt templates for document Q&A with RAG."""

QNA_PROMPT = """You are an expert construction document analyst answering questions about project documents.

**Question:** {question}

**Relevant Document Context:**
{context}

**Guidelines:**
1. Answer based ONLY on the provided context
2. If the context doesn't contain enough information, say so clearly
3. Cite specific sources when possible (page numbers, sheet numbers, spec sections)
4. If you're uncertain, indicate your confidence level
5. Suggest follow-up questions that might help clarify the answer
6. For numerical answers, show your calculation/reasoning
7. If multiple interpretations are possible, present them

**Output format:** Return a JSON object:
{{
    "answer": "your detailed answer",
    "citations": ["list of source references"],
    "confidence": 0.0 to 1.0,
    "followups": ["suggested follow-up questions"]
}}

Provide your answer:"""


def build_qna_prompt(
    question: str,
    context_chunks: list[str],
    project_context: str | None = None,
) -> str:
    """
    Build the Q&A prompt with retrieved context.

    Args:
        question: The user's question
        context_chunks: Retrieved relevant document chunks
        project_context: Optional project background

    Returns:
        Formatted prompt string
    """
    # Format context chunks with separators
    formatted_context = "\n\n---\n\n".join(
        f"[Source {i+1}]\n{chunk}" for i, chunk in enumerate(context_chunks)
    )

    if project_context:
        formatted_context = f"**Project Background:** {project_context}\n\n{formatted_context}"

    return QNA_PROMPT.format(
        question=question,
        context=formatted_context[:40000],  # Truncate to avoid token limits
    )


# Specialized Q&A prompts for specific question types

QUANTITY_TAKEOFF_PROMPT = """You are performing a quantity takeoff based on document information.

**Question:** {question}

**Document Context:**
{context}

For quantity questions:
1. Identify the item being counted/measured
2. Find all instances in the context
3. Show your calculation methodology
4. Note any assumptions made
5. Flag items that might be duplicated or missing
6. Provide unit of measure

Return JSON:
{{
    "item": "what is being counted",
    "quantity": number or null if cannot determine,
    "unit": "each, SF, LF, CY, etc.",
    "calculation": "how you arrived at the quantity",
    "assumptions": ["assumptions made"],
    "confidence": 0.0 to 1.0,
    "warnings": ["potential issues with this count"]
}}"""


SPECIFICATION_LOOKUP_PROMPT = """You are looking up specification requirements.

**Question:** {question}

**Specification Context:**
{context}

For specification questions:
1. Identify the relevant spec section
2. Quote the specific requirement
3. Note any related requirements
4. Flag any conflicts or ambiguities
5. Reference applicable standards (ASTM, UL, etc.)

Return JSON:
{{
    "spec_section": "section number and title",
    "requirement": "the specific requirement text",
    "related_requirements": ["other relevant requirements"],
    "applicable_standards": ["ASTM, UL, etc."],
    "notes": ["clarifications or warnings"],
    "confidence": 0.0 to 1.0
}}"""


COORDINATION_CHECK_PROMPT = """You are checking for coordination issues between disciplines.

**Question:** {question}

**Multi-discipline Context:**
{context}

For coordination questions:
1. Identify elements from each discipline involved
2. Check for spatial conflicts
3. Check for specification conflicts
4. Note sequencing dependencies
5. Identify potential RFI items

Return JSON:
{{
    "disciplines_involved": ["architectural", "structural", "mep"],
    "potential_conflicts": [
        {{
            "description": "conflict description",
            "location": "where it occurs",
            "severity": "high|medium|low"
        }}
    ],
    "sequencing_issues": ["any sequencing concerns"],
    "recommended_rfis": ["questions to resolve"],
    "confidence": 0.0 to 1.0
}}"""


CHANGE_DETECTION_PROMPT = """You are comparing two versions of a document to identify changes.

**Question:** What changed between these document versions?

**Original Version:**
{original_text}

**Revised Version:**
{revised_text}

Identify:
1. Added items (new in revised)
2. Removed items (in original but not revised)
3. Modified items (changed between versions)
4. Impact assessment for each change

Return JSON:
{{
    "additions": [
        {{"item": "", "description": "", "impact": "high|medium|low"}}
    ],
    "removals": [
        {{"item": "", "description": "", "impact": "high|medium|low"}}
    ],
    "modifications": [
        {{"item": "", "original": "", "revised": "", "impact": "high|medium|low"}}
    ],
    "summary": "brief summary of changes",
    "total_changes": number,
    "recommendation": "action recommendation"
}}"""
