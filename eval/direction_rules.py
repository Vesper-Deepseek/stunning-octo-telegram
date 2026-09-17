"""Deterministic direction adjudication for extracted commitments.

Symbolic counterpart to the neural extractor: given raw input text,
decide per-clause whether the writer performs the owed action
(user_owes), someone else does (owed_to_user), or signals conflict /
absence (unclear). Validated in Stage 1 before porting to the Rust
symbolic core (where it runs inside the circuit-breaker fallback and
as the cross-check on the neural path's direction field).
"""

import re

FIRST_PERSON = re.compile(
    r"\bI\b|\bI'?ll\b|\bI'?m\b|\bI'?d\b|\bmy\b|\bme\b|\bmine\b|\bwe\b|\bus\b|\bour\b",
    re.IGNORECASE,
)
USER_ACTION = re.compile(
    r"\b(i|i'll|i'm|i've|i'd|we|we'll)\s+(owe|owed|pay|paid|repay|send|sent|give|gave"
    r"|return|bring|book|cancel|drop|get|fix|email|text|call|water|cover|reimburse"
    r"|sort|handle|take|deliver|sign|finish|review|provide)"
    r"|\bneed to\b|\bmust\b|\bgonna\b|\bpromised\s+\w+\s+the\b",
    re.IGNORECASE,
)
OTHER_OWES_ME = re.compile(
    r"\bowes? me\b|\bpay(s)? me back\b|\bsend(s|ing)? me\b|\bbring(s|ing)? me\b"
    r"|\bstill (hasn'?t|has not|haven'?t)\b|\bstill waiting\b|\bstill has (my|the)\b"
    r"|\bwas supposed to\b|\bwere supposed to\b"
    r"|\bsupposed to (send|give|call|return|pay|have|be ready)\b",
    re.IGNORECASE,
)
IMPERATIVE_START = re.compile(
    r"^\s*(pay|repay|send|give|return|drop|book|cancel|get|call|email|text|water"
    r"|cover|reimburse|sign|finish|bring|fix|chase|nudge|remind|tell|ask|ping|owe|owes)\b",
    re.IGNORECASE,
)
WAITING_ON_MINE = re.compile(
    r"(waiting|waits|waited)\s+on\s+(?:(?!\bto\b)[^,.!?])*\b(me|my|mine)\b",
    re.IGNORECASE,
)
DESIRE_FROM_ME = re.compile(
    r"^\s*(?:my |the )?[a-z' ]{2,25}\s+(wants?|needs?|asks?|asked|requested)\b",
    re.IGNORECASE,
)
CHASE_VERB_START = re.compile(r"^\s*(chase|nudge|remind|tell|ask|ping)\b", re.IGNORECASE)
PROMISED_FIRST = re.compile(r"^\s*promised\s+(?!to\b)[A-Za-z'. ]{2,30}\b", re.IGNORECASE)
ADDRESSEE_HEADER = re.compile(
    r"^\s*(?:one|two|three|four|five|\d+\s*)?things?\s+[A-Z][a-z]+\s*:", re.IGNORECASE
)

CLAUSE_SPLIT = re.compile(r"[,;+.!?:]|\band also\b|\balso\b|\bplus\b")


def norm_words(s):
    return [w for w in re.findall(r"[a-z0-9']+", s.lower())]


def classify_clause(clause: str) -> str:
    clause = clause.strip()
    if len(norm_words(clause)) < 2:
        return "unclear"

    if WAITING_ON_MINE.search(clause):
        return "user_owes"

    other_owes = bool(OTHER_OWES_ME.search(clause))

    # "chase X, she still hasn't sent Y" -> Y owed by X
    if other_owes:
        return "owed_to_user"

    # prompting verbs: usually the writer prompting ANOTHER party who owes;
    # unless the clause carries an explicit writer commitment ("tell Sam I'll fix...")
    if CHASE_VERB_START.match(clause):
        if re.search(r"\bI\b|\bI'?ll\b|\bI'?m\b|\bI'?d\b|my\b", clause, re.IGNORECASE) and (
            USER_ACTION.search(clause) or _verb_after_first_person(clause)
        ):
            return "user_owes"
        return "owed_to_user"

    if PROMISED_FIRST.match(clause) and not THIRD_PARTY_ACTION(clause):
        return "user_owes"
        return "user_owes"

    if DESIRE_FROM_ME.match(clause) and not THIRD_PARTY_ACTION(clause):
        return "user_owes"

    if IMPERATIVE_START.match(clause):
        # note-to-self convention: bare imperative = writer's task, unless an
        # explicit third-party action marker is present in the same clause
        return "user_owes"

    has_first = bool(FIRST_PERSON.search(clause))
    user_act = bool(USER_ACTION.search(clause))

    if user_act and not other_owes:
        return "user_owes"
    if has_first and not other_owes and _verb_after_first_person(clause):
        return "user_owes"
    return "unclear"


def THIRD_PARTY_ACTION(clause: str) -> bool:
    return bool(
        re.search(
            r"\b(said|promised|owes?|sent|will|going to|supposed|hasn'?t)\b",
            clause,
            re.IGNORECASE,
        )
    )


def _verb_after_first_person(clause: str) -> bool:
    m = re.search(r"\bi(?:'ll|'m|'d| will| am| must| need to| have to)?\b(.{0,30})", clause, re.IGNORECASE)
    if not m:
        return False
    tail = m.group(1).lower()
    verbs = (
        "owe pay repay send give return bring book cancel drop get fix email "
        "text call water cover reimburse sort handle take deliver sign finish "
        "review provide go do make"
    ).split()
    return any(re.search(rf"\b{v}\b", tail) for v in verbs)


def assign_direction(text: str, desc_tokens) -> str:
    """Pick the clause most overlapping the extraction's description tokens."""
    addressed_other = bool(ADDRESSEE_HEADER.match(text))
    clauses = [c.strip() for c in CLAUSE_SPLIT.split(text) if c.strip()]

    dtoks = set(desc_tokens)
    candidates = []
    for i, c in enumerate(clauses):
        w = norm_words(c)
        if len(w) < 3:
            continue
        overlap = len(dtoks & set(w))
        candidates.append((overlap, len(w), i, c))
    if not candidates:
        candidates = [(len(dtoks & set(norm_words(c))), len(norm_words(c)), i, c)
                      for i, c in enumerate(clauses)]
    _, _, _, best_clause = max(candidates, key=lambda t: (t[0], t[1]))

    if best_clause is None:
        return "unclear"

    verdict = classify_clause(best_clause)

    # prompting-verb + next-clause third-party failure => other party owes
    if CHASE_VERB_START.match(best_clause):
        idx = clauses.index(best_clause)
        nxt = clauses[idx + 1] if idx + 1 < len(clauses) else ""
        if OTHER_OWES_ME.search(nxt):
            return "owed_to_user"

    if verdict == "user_owes" and addressed_other and not FIRST_PERSON.search(best_clause):
        # imperatives under an addressee header ("three things Omar:") are requests
        return "unclear"
    if verdict == "unclear" and len(clauses) == 1:
        whole = classify_clause(text)
        if whole != "unclear":
            return whole
    return verdict


def description_tokens(desc: str):
    return set(norm_words(desc))
