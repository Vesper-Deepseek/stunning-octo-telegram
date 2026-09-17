//! Party detection: capitalized proper names and role words ("my landlord").

pub fn detect_party(text: &str) -> Option<String> {
    // role words after possessives
    const ROLES: &[&str] = &[
        "landlord",
        "sister",
        "brother",
        "mum",
        "mom",
        "dad",
        "boss",
        "dentist",
        "doctor",
        "vet",
        "garage",
        "library",
        "sitter",
        "plumber",
        "electrician",
        "agent",
        "professor",
        "teacher",
        "cousin",
        "neighbour",
        "neighbor",
        "flatmate",
        "roommate",
        "accountant",
        "lawyer",
    ];
    let lower = text.to_lowercase();
    for role in ROLES {
        for marker in ["my ", "the ", "our "] {
            if let Some(pos) = lower.find(&format!("{marker}{role}")) {
                let mut end = pos + marker.len() + role.len();
                let bytes = text.as_bytes();
                // Extend over a following capitalized name if present
                // ("my landlord John"). Skip at most one run of whitespace,
                // then require an uppercase alphabetic byte (a proper-noun
                // signal) before consuming any more alphabetic bytes. This
                // excludes "the landlord I'd pay" (no proper noun after).
                if end < bytes.len() && bytes[end] == b' ' {
                    let after_ws = end + 1;
                    // Require a multi-character capitalized word (reject the
                    // pronoun "I" and other single-letter "names" like "A",
                    // "K" used as initials without context).
                    if after_ws + 1 < bytes.len()
                        && bytes[after_ws].is_ascii_uppercase()
                        && bytes[after_ws + 1].is_ascii_alphabetic()
                    {
                        end = after_ws;
                        while end < bytes.len() && bytes[end].is_ascii_alphabetic() {
                            end += 1;
                        }
                    }
                }
                return Some(text[pos..end].trim().to_string());
            }
        }
    }

    // organizations / institutions worth tracking as parties
    const ORGS: &[&str] = &[
        "ups",
        "fedex",
        "dhl",
        "royal mail",
        "hmrc",
        "irs",
        "amazon",
        "ebay",
        "paypal",
        "bank",
        "gym",
        "university",
        "council",
    ];
    for org in ORGS {
        if lower.contains(org) {
            return Some(title_case(org));
        }
    }

    // capitalized tokens that are not sentence-initial and not common words
    let mut candidates: Vec<String> = Vec::new();
    let mut prev_ended_sentence = false;
    for (i, tok) in text.split_whitespace().enumerate() {
        let clean = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let is_cap = clean.chars().next().is_some_and(|c| c.is_uppercase())
            && clean.len() > 1
            && !STOP_CAPS.contains(&clean.to_lowercase().as_str());
        let sentence_initial = i == 0 || prev_ended_sentence;
        prev_ended_sentence = tok.ends_with('.') || tok.ends_with('!') || tok.ends_with('?');
        if is_cap && !sentence_initial {
            candidates.push(clean.to_string());
        }
    }

    candidates.into_iter().next()
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

const STOP_CAPS: &[&str] = &[
    "i",
    "i'll",
    "i'm",
    "i'd",
    "i've",
    "it's",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "note",
    "ok",
    "so",
    "the",
    "need",
    "must",
    "told",
    "promised",
    "still",
    "also",
    "and",
    "but",
    "when",
    "she",
    "he",
    "they",
    "this",
    "that",
    "reminder",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles() {
        assert_eq!(
            detect_party("told the landlord I'd pay"),
            Some("the landlord".into())
        );
        assert_eq!(
            detect_party("owe my sister a call"),
            Some("my sister".into())
        );
    }

    #[test]
    fn names() {
        assert_eq!(detect_party("I owe Dave 15 quid"), Some("Dave".into()));
        assert_eq!(
            detect_party("send Ingrid the signed form tomorrow"),
            Some("Ingrid".into())
        );
    }

    #[test]
    fn none_for_pronouns_only() {
        assert_eq!(detect_party("I told him I'd get it back to him"), None);
    }

    #[test]
    fn orgs_are_title_cased() {
        // Organizations should be normalized to display form regardless of
        // casing in the input.
        assert_eq!(
            detect_party("the package from amazon"),
            Some("Amazon".into())
        );
        assert_eq!(
            detect_party("paypal still has my refund"),
            Some("Paypal".into())
        );
        assert_eq!(detect_party("IRS owes me a refund"), Some("Irs".into()));
    }

    #[test]
    fn sentence_initial_capitalized_name_is_excluded() {
        // Sentence-initial capitalization isn't a reliable name signal.
        // "Dave owes me money" should NOT flag Dave because Dave starts the
        // sentence; "I owe Dave money" should flag Dave.
        assert_eq!(detect_party("Dave owes me money"), None);
        assert_eq!(detect_party("I owe Dave money"), Some("Dave".into()));
    }

    #[test]
    fn role_word_with_following_name_extends_across_space() {
        // "my landlord John" should extend to capture the full name.
        let s = "told my landlord John I'd pay by Friday";
        let p = detect_party(s).unwrap();
        assert!(p.starts_with("my landlord"), "{p}");
        assert!(p.contains("John"), "{p}");
    }

    #[test]
    fn role_word_without_following_name_stops_cleanly() {
        // No name follows -> just "my landlord", no trailing whitespace.
        assert_eq!(
            detect_party("told my landlord I'd pay"),
            Some("my landlord".into())
        );
    }

    #[test]
    fn role_word_with_following_punctuation_does_not_eat_punct() {
        // Comma immediately after the role -> no name extension.
        assert_eq!(
            detect_party("owes my landlord, John, twenty").as_deref(),
            Some("my landlord")
        );
    }

    #[test]
    fn single_letter_cap_is_excluded() {
        // "I" alone is a single capital letter and must not be a party.
        assert_eq!(detect_party("then I went home"), None);
    }

    #[test]
    fn multiple_names_returns_first_eligible() {
        // The detector returns one party; document which one wins on ties.
        let p = detect_party("I owe Dave and Marco 20 quid");
        // "Dave" comes first in the sentence; "Marco" follows. The first
        // non-sentence-initial capital token should win.
        assert_eq!(p, Some("Dave".into()));
    }

    #[test]
    fn empty_and_whitespace_only_inputs() {
        assert_eq!(detect_party(""), None);
        assert_eq!(detect_party("   "), None);
    }

    #[test]
    fn weekday_word_not_treated_as_party() {
        // "Friday" is in STOP_CAPS so a sentence like "Friday I pay back"
        // shouldn't extract "Friday" as a party.
        let s = "Friday I'll pay back Marco";
        let p = detect_party(s);
        assert_ne!(p.as_deref(), Some("Friday"));
    }
}
