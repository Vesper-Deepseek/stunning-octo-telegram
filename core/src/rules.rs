//! Rule-based fallback extractor — Rust port of the Stage-1-validated
//! Python logic (`eval/direction_rules.py`), extended into a full
//! clause-level extractor (direction + description + date + party).
//! This is the circuit-breaker's deterministic path and the direction
//! cross-check for the neural path.

use chrono::NaiveDate;

use crate::dates;
use crate::models::{Confidence, ExtractDirection, FieldConfidence};
use crate::party;

/// One candidate commitment produced by the rules.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleExtraction {
    pub description: String,
    pub direction: ExtractDirection,
    pub expected_date: Option<NaiveDate>,
    pub party_guess: Option<String>,
    pub confidence: Confidence,
    /// clause the extraction came from
    pub source_clause: String,
}

use std::sync::OnceLock;

struct Patterns {
    first_person: regex::Regex,
    user_action: regex::Regex,
    other_owes_me: regex::Regex,
    waiting_on_mine: regex::Regex,
    desire_from_me: regex::Regex,
    chase_verb_start: regex::Regex,
    promised_first: regex::Regex,
    imperative_start: regex::Regex,
    addressee_header: regex::Regex,
    third_party_action: regex::Regex,
    verb_after_first: regex::Regex,
    writer_commitment_in_prompt: regex::Regex,
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(|| Patterns {
        first_person: re(r"(?i)\bI\b|\bI'?ll\b|\bI'?m\b|\bI'?d\b|\bmy\b|\bme\b|\bmine\b|\bwe\b|\bus\b|\bour\b"),
        user_action: re(r"(?i)\b(i|i'll|i'm|i've|i'd|we|we'll)\s+(owe|owed|pay|paid|repay|send|sent|give|gave|return|bring|book|cancel|drop|get|fix|email|text|call|water|cover|reimburse|sort|handle|take|deliver|sign|finish|review|provide)|\bneed to\b|\bmust\b|\bgonna\b|\bpromised\s+\w+\s+the\b"),
        other_owes_me: re(r"(?i)\bowes? me\b|\bpay(s)? me back\b|\bsend(s|ing)? me\b|\bbring(s|ing)? me\b|\bstill (hasn'?t|has not|haven'?t)\b|\bstill waiting\b|\bstill has (my|the)\b|\bwas supposed to\b|\bwere supposed to\b|\bsupposed to (send|give|call|return|pay|have|be ready)\b"),
        waiting_on_mine: re(r"(?i)(waiting|waits|waited)\s+on\s+([^,.!?]{0,60}?)\b(me|my|mine)\b"),
        desire_from_me: re(r"(?i)^\s*(?:my |the )?[a-z' ]{2,25}\s+(wants?|needs?|asks?|asked|requested)\b"),
        chase_verb_start: re(r"(?i)^\s*(chase|nudge|remind|tell|ask|ping)\b"),
        promised_first: re(r"(?i)^\s*promised\s+([A-Za-z'. ]{2,30})"),
        imperative_start: re(r"(?i)^\s*(pay|repay|send|give|return|drop|book|cancel|get|call|email|text|water|cover|reimburse|sign|finish|bring|fix|chase|nudge|remind|tell|ask|ping|owe|owes)\b"),
        addressee_header: re(r"(?i)^\s*(?:one|two|three|four|five|\d+\s*)?things?\s+[A-Z][a-z]+\s*:"),
        third_party_action: re(r"(?i)\b(said|promised|owes?|sent|will|going to|supposed|hasn'?t)\b"),
        verb_after_first: re(r"(?i)\bi(?:'ll|'m|'d| will| am| must| need to| have to)?\b(.{0,30})"),
        writer_commitment_in_prompt: re(r"(?i)\bI\b|\bI'?ll\b|\bI'?m\b|\bI'?d\b|\bmy\b"),
    })
}

fn re(pattern: &str) -> regex::Regex {
    regex::Regex::new(pattern).expect("valid pattern")
}

const ACTION_VERB_HINTS: &[&str] = &[
    "owe", "pay", "repay", "send", "give", "return", "drop off", "book",
    "cancel", "get", "call", "email", "text", "water", "cover", "reimburse",
    "sign", "finish", "review", "bring", "fix", "chase", "remind", "tell",
    "promised", "waiting", "forgot", "forget", "refund", "settle", "square",
];

const CLAUSE_SPLIT: &str = r"[,;+.!?:]|\band also\b|\balso\b|\bplus\b";

/// Extract candidate commitments from raw text using deterministic rules only.
pub fn extract_rules(text: &str, today: NaiveDate) -> Vec<RuleExtraction> {
    let p = patterns();
    let addressed_other = p.addressee_header.is_match(text);

    let clause_re = regex::Regex::new(CLAUSE_SPLIT).unwrap();
    let clauses: Vec<&str> = clause_re
        .split(text)
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect();

    let mut out = Vec::new();
    let mut seen_descs: Vec<String> = Vec::new();

    for clause in &clauses {
        if !is_commitment_like(clause) {
            continue;
        }
        let dir = classify_clause(clause, text, &clauses);
        // noise gate: no usable signal at all
        if matches!(dir, ExtractDirection::Unclear) && !has_any_signal(clause) {
            continue;
        }

        let desc = build_description(clause);
        if desc.split_whitespace().count() < 2 {
            continue;
        }
        let norm = desc.to_lowercase();
        if seen_descs.iter().any(|d| d == &norm) {
            continue;
        }
        seen_descs.push(norm);

        let mut dr = dates::parse_date_expression(clause, today);
        if dr.date.is_none() && !dr.vague_marker_found {
            // clause itself has no temporal info; try the whole message
            dr = dates::parse_date_expression(text, today);
        }

        let party_guess = party::detect_party(clause).or_else(|| party::detect_party(text));

        let overall = match (&dir, dr.date) {
            (ExtractDirection::Unclear, _) => FieldConfidence::Low,
            (_, None) => FieldConfidence::Low,
            _ => FieldConfidence::High,
        };
        out.push(RuleExtraction {
            description: desc,
            direction: dir,
            expected_date: dr.date,
            party_guess: party_guess.clone(),
            confidence: Confidence {
                party: Some(if party_guess.is_some() { FieldConfidence::High } else { FieldConfidence::Low }),
                date: Some(match dr.date {
                    Some(_) => FieldConfidence::High,
                    None => FieldConfidence::Low,
                }),
                overall: Some(overall),
            },
            source_clause: (*clause).to_string(),
        });
    }

    if addressed_other {
        for e in &mut out {
            if !p.first_person.is_match(&e.source_clause) {
                // imperatives under an addressee header are requests to that party
                e.direction = match e.direction {
                    ExtractDirection::UserOwes => ExtractDirection::Unclear,
                    other => other,
                };
            }
        }
    }
    out
}

fn is_commitment_like(clause: &str) -> bool {
    // obligation-grammar signals only — bare action words leak noise
    // ("insane finish", "supposed to be nice")
    let p = patterns();
    p.user_action.is_match(clause)
        || p.other_owes_me.is_match(clause)
        || p.imperative_start.is_match(clause)
        || p.waiting_on_mine.is_match(clause)
        || p.promised_first.is_match(clause)
        || p.desire_from_me.is_match(clause)
        || p.chase_verb_start.is_match(clause)
}

fn has_any_signal(clause: &str) -> bool {
    is_commitment_like(clause)
}

pub fn classify_clause(clause: &str, full_text: &str, all_clauses: &[&str]) -> ExtractDirection {
    let p = patterns();
    let clause = clause.trim();
    if clause.split_whitespace().count() < 2 {
        return ExtractDirection::Unclear;
    }

    if let Some(caps) = p.waiting_on_mine.captures(clause) {
        let middle = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let has_to = middle
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w.eq_ignore_ascii_case("to"));
        if !has_to {
            return ExtractDirection::UserOwes;
        }
    }

    let other_owes = p.other_owes_me.is_match(clause);
    if other_owes {
        return ExtractDirection::OwedToUser;
    }

    // prompting verbs: usually prompting ANOTHER party who owes; unless the
    // clause carries an explicit writer commitment
    if p.chase_verb_start.is_match(clause) {
        if p.writer_commitment_in_prompt.is_match(clause)
            && (p.user_action.is_match(clause) || verb_after_first_person(clause))
        {
            return ExtractDirection::UserOwes;
        }
        return ExtractDirection::OwedToUser;
    }

    if let Some(caps) = p.promised_first.captures(clause) {
        let after = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let first_word = after
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
            .find(|w| !w.is_empty())
            .unwrap_or("");
        if !first_word.eq_ignore_ascii_case("to") && !p.third_party_action.is_match(clause) {
            return ExtractDirection::UserOwes;
        }
    }

    if p.desire_from_me.is_match(clause) && !p.third_party_action.is_match(clause) {
        return ExtractDirection::UserOwes;
    }

    if p.imperative_start.is_match(clause) {
        return ExtractDirection::UserOwes;
    }

    let has_first = p.first_person.is_match(clause);
    let user_act = p.user_action.is_match(clause);
    if user_act && !other_owes {
        return ExtractDirection::UserOwes;
    }
    if has_first && !other_owes && verb_after_first_person(clause) {
        return ExtractDirection::UserOwes;
    }

    // whole-text fallback only for single-clause texts
    if all_clauses.len() == 1 {
        let has_first_all = p.first_person.is_match(full_text);
        let user_act_all = p.user_action.is_match(full_text);
        if user_act_all && !p.other_owes_me.is_match(full_text) {
            return ExtractDirection::UserOwes;
        }
        if has_first_all && !p.other_owes_me.is_match(full_text) && verb_after_first_person(full_text) {
            return ExtractDirection::UserOwes;
        }
        if p.other_owes_me.is_match(full_text) {
            return ExtractDirection::OwedToUser;
        }
    }

    ExtractDirection::Unclear
}

fn verb_after_first_person(text: &str) -> bool {
    let p = patterns();
    if let Some(caps) = p.verb_after_first.captures(text) {
        let tail = caps.get(1).map(|m| m.as_str().to_lowercase()).unwrap_or_default();
        const VERBS: &[&str] = &[
            "owe", "pay", "repay", "send", "give", "return", "bring", "book",
            "cancel", "drop", "get", "fix", "email", "text", "call", "water",
            "cover", "reimburse", "sort", "handle", "take", "deliver", "sign",
            "finish", "review", "provide", "go", "do", "make",
        ];
        if VERBS.iter().any(|v| {
            regex::Regex::new(&format!(r"\b{}\b", v))
                .map(|rv| rv.is_match(&tail))
                .unwrap_or(false)
        }) {
            return true;
        }
    }
    false
}

/// Compact the clause into a description: strip discourse markers and
/// leading auxiliaries, keep the action phrase.
fn build_description(clause: &str) -> String {
    let mut s = clause.trim().to_string();
    let strip_prefixes = [
        "ok so ", "so ", "and so ", "note to self ", "reminder to self ",
        "need to ", "needs to ", "must ", "have to ", "having to ", "gonna ",
        "i need to ", "i must ", "i have to ", "i'll ", "i am going to ",
        "i'm going to ", "i should ", "should really ", "should ", "keep forgetting to ",
        "still owe ", "owe ", "i still owe ", "i owe ", "told ", "promised ",
        "remember to ", "don't forget to ", "dont forget to ",
        "keep forgetting to ", "i keep forgetting to ", "should really get around to ",
        "really get around to ", "get around to ",
    ];
    let lower = s.to_lowercase();
    for pre in strip_prefixes {
        if lower.starts_with(pre) {
            s = s[pre.len()..].trim().to_string();
            break;
        }
    }
    // drop trailing meta like "like agreed", "been putting it off"
    for suf in [" like agreed", " already", " lol", " tbh"] {
        if s.to_lowercase().ends_with(suf) {
            s.truncate(s.len() - suf.len());
        }
    }
    s.trim().to_string()
}


/// Port of Python `assign_direction`: pick the clause most overlapping the
/// description tokens, classify it, apply chase-continuation and
/// addressee-header rules.
pub fn normalize_tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect()
}

pub fn assign_direction(text: &str, desc_tokens: &[&str]) -> ExtractDirection {
    let p = patterns();
    let addressed_other = p.addressee_header.is_match(text);
    let clause_re = regex::Regex::new(CLAUSE_SPLIT).unwrap();
    let clauses: Vec<&str> = clause_re.split(text).map(str::trim).filter(|c| !c.is_empty()).collect();

    let dtoks: Vec<String> = desc_tokens.iter().map(|t| t.to_lowercase()).collect();
    let mut candidates: Vec<(usize, usize, &str)> = Vec::new();
    for c in &clauses {
        let w = normalize_tokens(c);
        if w.len() < 3 { continue; }
        let overlap = w.iter().filter(|x| dtoks.contains(x)).count();
        candidates.push((overlap, w.len(), c));
    }
    if candidates.is_empty() {
        for c in &clauses {
            let w = normalize_tokens(c);
            let overlap = w.iter().filter(|x| dtoks.contains(x)).count();
            candidates.push((overlap, w.len(), c));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    if candidates.is_empty() { return ExtractDirection::Unclear; }
    let best = candidates[0].2;

    let mut verdict = classify_clause(best, text, &clauses);

    // chase-continuation: prompting verb + next-clause third-party failure
    if p.chase_verb_start.is_match(best) {
        if let Some(idx) = clauses.iter().position(|c| *c == best) {
            if idx + 1 < clauses.len() && p.other_owes_me.is_match(clauses[idx + 1]) {
                verdict = ExtractDirection::OwedToUser;
            }
        }
    }

    if verdict == ExtractDirection::UserOwes && addressed_other && !p.first_person.is_match(best) {
        // imperatives under an addressee header ("three things Omar:") are requests
        return ExtractDirection::Unclear;
    }
    if verdict == ExtractDirection::Unclear && clauses.len() == 1 {
        let whole = classify_clause(text, text, &clauses);
        if whole != ExtractDirection::Unclear {
            return whole;
        }
    }
    verdict
}

#[cfg(test)]
mod parity_tests {
    //! Fixture-driven parity with eval/direction_rules.py.
    //! Fixture is GENERATED by the python implementation; regenerate rather
    //! than hand-editing expectations.

    use super::*;

    const FIXTURE: &str = include_str!("../tests/direction_parity.json");

    #[derive(serde::Deserialize)]
    struct FixtureCase {
        id: String,
        input: String,
        tokens: Vec<String>,
        expected: String,
    }

    #[test]
    fn python_parity_direction_suite() {
        let fx: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        let cases: Vec<FixtureCase> =
            serde_json::from_value(fx["cases"].clone()).unwrap();
        assert_eq!(cases.len(), 42);
        let mut ok = 0;
        let mut unc = 0;
        for c in &cases {
            let tokrefs: Vec<&str> = c.tokens.iter().map(String::as_str).collect();
            let got = assign_direction(&c.input, &tokrefs);
            let got_s = match got {
                ExtractDirection::UserOwes => "user_owes",
                ExtractDirection::OwedToUser => "owed_to_user",
                ExtractDirection::Unclear => "unclear",
            };
            match got_s == c.expected {
                true if got_s != "unclear" => ok += 1,
                true => unc += 1,
                false => panic!("mismatch {}: got {} want {}", c.id, got_s, c.expected),
            }
        }
        assert_eq!(ok, 35, "confident-correct count");
        assert_eq!(unc, 7, "honest-unclear count");
    }

    #[test]
    fn noise_produces_nothing() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        for noise in [
            "lol that meme killed me 😂😂😂",
            "weather's supposed to be nice this weekend finally",
            "did you see the match last night?? insane finish",
            "happy birthday!!! hope you have the best day",
            "just landed, taxi was a nightmare",
            "new coffee place opened near the office",
        ] {
            assert!(
                extract_rules(noise, today).is_empty(),
                "noise leaked: {noise}"
            );
        }
    }

    #[test]
    fn multi_commitment_split() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let out = extract_rules(
            "ok so: pay back Lena the 20 euros by thursday, and remind Marco to send me the receipt from the hotel",
            today,
        );
        assert_eq!(out.len(), 2, "{out:?}");
        assert!(out.iter().any(|e| e.direction == ExtractDirection::UserOwes));
        assert!(out.iter().any(|e| e.direction == ExtractDirection::OwedToUser));
    }

    #[test]
    fn vague_date_stays_null() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();
        let out = extract_rules("I need to pay back Elena soon", today);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].expected_date, None);
        assert_eq!(out[0].confidence.date, Some(FieldConfidence::Low));
    }
}
