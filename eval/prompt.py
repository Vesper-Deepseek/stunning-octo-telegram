"""Stage 1 first-draft extraction prompt (Loose Ends).

Requirements implemented (spec Section 7):
- Few-shot examples cover: ambiguous direction, vague/missing dates,
  multiple commitments per message, pure noise rejection.
- Explicitly permits/encourages "no commitment found" (empty array).
- Explicitly instructs low-confidence marking over fabrication;
  prefer null/"unclear" over a plausible guess.
- Fixed JSON output shape only (array of the Section 5 schema objects).
"""

TODAY_LINE = "Today's date is Wednesday 2026-08-26."
PROMPT_VERSION = "v2_direction_rules"

SYSTEM_PROMPT = """You extract personal commitments from messy real-world text messages. A commitment is something one person owes another: an action owed by the writer (user_owes) or owed to the writer (owed_to_user).

Rules:
1. Output ONLY a JSON array. Each element is one object exactly like this:
   {"commitment_found": true, "party": string-or-null, "party_confidence": "high"|"low", "description": string, "direction": "user_owes"|"owed_to_user"|"unclear", "expected_date": ISO-date-string-or-null, "date_confidence": "high"|"low", "overall_confidence": "high"|"low"}
2. If the text contains no commitment at all, output []. Saying nothing was found is always acceptable and preferred over inventing one.
3. expected_date must be an absolute ISO date (YYYY-MM-DD) resolved relative to today's date, given below. If no date is stated, use null — never invent one. Relative ranges like "soon", "next week", "this week", "sometime" are NOT dates: use null and mark date_confidence "low".
4. DIRECTION — ask: WHO performs the owed action?
   - The WRITER does it ("I owe Dave $15", "I'll pay Marcus back", "I must return her book") -> "user_owes"
   - Someone ELSE does it for the writer ("Dave owes me $15", "she'll send me the photos", "Rosa promised to return my keys") -> "owed_to_user"
   - Cannot tell -> "unclear". Never guess between them.
5. NEVER output the same commitment twice, and never output both directions of one obligation. One action = one object.
6. Mark party_confidence "low" when the party is unnamed ("him", "she") or ambiguous; use null when there is no name. Pronouns are NOT names.
7. Never fabricate plausible-sounding details. Prefer null / "unclear" / [] over any guess.
8. One object per distinct commitment; a message can contain several.

{today}"""

FEW_SHOTS = [
    # clear, dated, user_owes
    {
        "user": "hey don't forget I'm bringing the cake to Sam's place on saturday for game night",
        "assistant": '[{"commitment_found": true, "party": "Sam", "party_confidence": "high", "description": "bring cake to Sam\'s place for game night", "direction": "user_owes", "expected_date": "2026-08-29", "date_confidence": "high", "overall_confidence": "high"}]',
    },
    # clear, owed_to_user
    {
        "user": "Rosa said she'd drop off my keys at the cafe tomorrow afternoon",
        "assistant": '[{"commitment_found": true, "party": "Rosa", "party_confidence": "high", "description": "Rosa to return keys at the cafe", "direction": "owed_to_user", "expected_date": "2026-08-27", "date_confidence": "high", "overall_confidence": "high"}]',
    },
    # pure noise -> empty array
    {
        "user": "hahah no way, that's hilarious 😂 anyway what are you up to this weekend",
        "assistant": "[]",
    },
    # vague date -> null + low confidence
    {
        "user": "I owe Nina twenty bucks from the taxi, gotta pay her back soon",
        "assistant": '[{"commitment_found": true, "party": "Nina", "party_confidence": "high", "description": "repay Nina 20 from taxi fare", "direction": "user_owes", "expected_date": null, "date_confidence": "low", "overall_confidence": "high"}]',
    },
    # ambiguous direction
    {
        "user": "told him I'd get it back to him whenever, no rush",
        "assistant": '[{"commitment_found": true, "party": null, "party_confidence": "low", "description": "return item to unnamed person", "direction": "unclear", "expected_date": null, "date_confidence": "low", "overall_confidence": "low"}]',
    },
    # multiple commitments, mixed directions, one dated one not
    {
        "user": "so friday I pay Pete back for lunch, and he's supposed to send me those podcast notes he promised",
        "assistant": '[{"commitment_found": true, "party": "Pete", "party_confidence": "high", "description": "repay Pete for lunch", "direction": "user_owes", "expected_date": "2026-08-28", "date_confidence": "high", "overall_confidence": "high"}, {"commitment_found": true, "party": "Pete", "party_confidence": "high", "description": "Pete to send podcast notes", "direction": "owed_to_user", "expected_date": null, "date_confidence": "low", "overall_confidence": "low"}]',
    },
    # direction contrast: "owes me" -> owed_to_user
    {
        "user": "he knows what he owes me. twenty since march",
        "assistant": '[{"commitment_found": true, "party": null, "party_confidence": "low", "description": "unnamed person owes user 20 since March", "direction": "owed_to_user", "expected_date": null, "date_confidence": "low", "overall_confidence": "low"}]',
    },
    # direction contrast: writer owes
    {
        "user": "I owe my landlord for last month and I keep forgetting, will sort it out this week",
        "assistant": '[{"commitment_found": true, "party": "landlord", "party_confidence": "high", "description": "pay landlord last month\'s rent", "direction": "user_owes", "expected_date": null, "date_confidence": "low", "overall_confidence": "high"}]',
    },
]


def build_messages(input_text: str):
    system = SYSTEM_PROMPT.replace("{today}", TODAY_LINE)
    msgs = [{"role": "system", "content": system}]
    for shot in FEW_SHOTS:
        msgs.append({"role": "user", "content": shot["user"]})
        msgs.append({"role": "assistant", "content": shot["assistant"]})
    msgs.append({"role": "user", "content": input_text})
    return msgs
