"""Prompt templates for tender scope document generation."""

TENDER_SCOPE_DOC_PROMPT = """You are a senior estimator preparing a formal Scope of Work document for a subcontractor bid package.

**Trade:** {trade}
**Project Context:** {project_context}

**Extracted Scope Information:**
{scope_data}

**Your task:**
Generate a professional, comprehensive Scope of Work document that a subcontractor can use to prepare their bid.

**Document structure requirements:**
1. **Overview** - Brief description of the work and project context
2. **Inclusions** - Detailed list of all work included in this scope
3. **Exclusions** - Clear list of work NOT included (by others or not in contract)
4. **Allowances** - Any allowances to be included in the bid
5. **Alternates** - Any alternate pricing requested
6. **Submittals** - Required submittals and approval process
7. **Schedule/Lead Times** - Relevant schedule information and long-lead items
8. **Bid Instructions** - How to submit the bid, deadline, contact info
9. **RFI Questions** - Questions that need answers before bid submission

**Guidelines:**
- Use clear, professional language
- Be specific about quantities where known
- Reference drawing and spec numbers
- Note coordination requirements with other trades
- Include standard industry scope clarifications for this trade
- Format as proper Markdown with headers and bullet points

**Output format:** Return a JSON object:
{{
    "trade": "{trade}",
    "overview": "paragraph overview",
    "inclusions": ["detailed inclusion items"],
    "exclusions": ["detailed exclusion items"],
    "allowances": ["any allowances"],
    "alternates": ["any alternates requested"],
    "submittals": ["required submittals"],
    "schedule_notes": ["schedule requirements"],
    "lead_times": ["long-lead items"],
    "bid_instructions": ["how to bid"],
    "rfi_questions": ["questions needing answers"],
    "markdown": "full document in Markdown format"
}}

Generate the Scope of Work document:"""


def build_tender_scope_doc_prompt(
    trade: str,
    scope_data: dict,
    project_context: str | None = None,
    bid_due_date: str | None = None,
    gc_contact: str | None = None,
) -> str:
    """
    Build the tender scope document generation prompt.

    Args:
        trade: The trade name
        scope_data: Extracted scope information from trade_scopes
        project_context: Project name, location, type, etc.
        bid_due_date: When bids are due
        gc_contact: GC contact information

    Returns:
        Formatted prompt string
    """
    import json

    context_parts = []
    if project_context:
        context_parts.append(project_context)
    if bid_due_date:
        context_parts.append(f"Bid Due Date: {bid_due_date}")
    if gc_contact:
        context_parts.append(f"GC Contact: {gc_contact}")

    context = "\n".join(context_parts) if context_parts else "Not provided"

    return TENDER_SCOPE_DOC_PROMPT.format(
        trade=trade,
        project_context=context,
        scope_data=json.dumps(scope_data, indent=2),
    )


# Template for the generated Markdown document
SCOPE_DOC_MARKDOWN_TEMPLATE = """# Scope of Work: {trade}

## Project Information
{project_info}

---

## 1. Overview
{overview}

---

## 2. Scope of Work - Inclusions

The following work is INCLUDED in this bid package:

{inclusions}

---

## 3. Scope of Work - Exclusions

The following work is NOT INCLUDED in this bid package:

{exclusions}

---

## 4. Allowances

{allowances}

---

## 5. Alternates

{alternates}

---

## 6. Submittals Required

{submittals}

---

## 7. Schedule & Lead Times

### Schedule Requirements
{schedule_notes}

### Long-Lead Items
{lead_times}

---

## 8. Bid Instructions

{bid_instructions}

---

## 9. Questions / RFI Items

The following items require clarification. Please note any assumptions in your bid:

{rfi_questions}

---

## 10. Bid Form

| Item | Description | Amount |
|------|-------------|--------|
| Base Bid | {trade} complete per plans and specs | $ |
{alternate_rows}

**Submitted By:** _______________________
**Company:** _______________________
**Date:** _______________________
**Phone:** _______________________
**Email:** _______________________

---

*This Scope of Work is intended to clarify the work included in this bid package. 
The Contractor shall review all contract documents and include all work required 
for a complete and functional installation regardless of whether specifically 
mentioned herein.*
"""
