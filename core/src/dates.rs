//! Relative-date resolution over chrono, tuned for honesty: vague
//! expressions ("soon", "next week") resolve to `None`, never to an
//! invented concrete day. Concrete expressions (weekday names, "the 30th",
//! "tomorrow", ISO strings) resolve to a real date.

use chrono::{Duration, NaiveDate};

#[derive(Debug, Clone, PartialEq)]
pub struct DateResolution {
    pub date: Option<NaiveDate>,
    /// true when the text contained a *vague* temporal reference
    pub vague_marker_found: bool,
}

/// Resolve the first date-like expression in `text` relative to `today`.
pub fn parse_date_expression(text: &str, today: NaiveDate) -> DateResolution {
    let lower = text.to_lowercase();

    // explicit ISO date: scan for a 4-digit-year YYYY-MM-DD substring anywhere
    // in the text (handles glued words like "by2026-09-01" too, because we
    // don't split on alphanumeric).
    if let Some(d) = find_iso_date(text) {
        return res(Some(d), false);
    }

    if contains_word(&lower, &["today", "tonight"]) {
        return res(Some(today), false);
    }
    if contains_word(&lower, &["tomorrow"]) {
        return res(Some(today + Duration::days(1)), false);
    }

    // "in N days/weeks"
    if let Some(n) = capture_in_n(&lower, "day") {
        return res(Some(today + Duration::days(n)), false);
    }
    if let Some(n) = capture_in_n(&lower, "week") {
        return res(None, true).also_date_if_none(today + Duration::days(7 * n));
    }

    // "the Nth" / "on the Nth" — day of current or next month
    if let Some(day) = capture_day_of_month(&lower) {
        let this_month = today.with_day(day).filter(|d| *d >= today);
        let resolved = this_month.or_else(|| next_month_with_day(today, day));
        return res(resolved, false);
    }

    // weekday names: nearest upcoming occurrence (strictly after today),
    // with "next <weekday>" meaning the one after that
    const WEEKDAYS: [(&str, chrono::Weekday); 7] = [
        ("monday", chrono::Weekday::Mon),
        ("tuesday", chrono::Weekday::Tue),
        ("wednesday", chrono::Weekday::Wed),
        ("thursday", chrono::Weekday::Thu),
        ("friday", chrono::Weekday::Fri),
        ("saturday", chrono::Weekday::Sat),
        ("sunday", chrono::Weekday::Sun),
    ];
    for (name, target) in WEEKDAYS {
        if contains_word(&lower, &[name]) {
            let days_ahead = days_until(target, today);
            let base = today + Duration::days(days_ahead);
            let d = if has_next_qualifier(&lower, name) {
                base + Duration::days(7)
            } else {
                base
            };
            return res(Some(d), false);
        }
    }

    if contains_word(&lower, &["this weekend", "weekend"]) {
        // Saturday of the current week
        let sat = days_until(chrono::Weekday::Sat, today);
        return res(Some(today + Duration::days(sat)), false);
    }

    // deliberately vague: no concrete resolution
    const VAGUE: [&str; 9] = [
        "soon",
        "next week",
        "this week",
        "next month",
        "this month",
        "sometime",
        "eventually",
        "asap",
        "when i get a chance",
    ];
    let vague = contains_word(&lower, &VAGUE);
    res(None, vague)
}

fn res(date: Option<NaiveDate>, vague: bool) -> DateResolution {
    DateResolution {
        date,
        vague_marker_found: vague,
    }
}

trait AlsoDate {
    fn also_date_if_none(self, d: NaiveDate) -> DateResolution;
}
impl AlsoDate for DateResolution {
    fn also_date_if_none(mut self, d: NaiveDate) -> DateResolution {
        if self.date.is_none() && !self.vague_marker_found {
            self.date = Some(d);
        } else if self.date.is_none() {
            self.vague_marker_found = true;
        }
        self
    }
}

fn contains_word(hay: &str, words: &[&str]) -> bool {
    words.iter().any(|w| {
        hay.split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
            .any(|tok| tok == *w || (w.contains(' ') && hay.contains(w)))
    })
}

fn has_next_qualifier(hay: &str, weekday: &str) -> bool {
    // "next friday" but not "friday"
    if let Some(pos) = hay.find(weekday) {
        let before = &hay[..pos];
        let last_word = before
            .split(|c: char| !c.is_ascii_alphanumeric())
            .rfind(|t| !t.is_empty())
            .unwrap_or("");
        return last_word == "next";
    }
    false
}

fn capture_in_n(text: &str, unit: &str) -> Option<i64> {
    let pat = format!(r"in\s+(\d{{1,3}})\s+{unit}s?\b");
    let re_text = pat;
    simple_number_after_in(text, unit).or_else(|| regex_lite_fallback(&re_text, text))
}

/// tiny helper avoiding an external regex dependency
fn regex_lite_fallback(_pat: &str, _text: &str) -> Option<i64> {
    None
}

fn simple_number_after_in(text: &str, unit: &str) -> Option<i64> {
    let toks: Vec<&str> = text.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    for i in 0..toks.len().saturating_sub(2) {
        if toks[i] == "in" {
            if let Ok(n) = toks[i + 1].parse::<i64>() {
                let u = toks[i + 2].trim_end_matches('s');
                if u == unit {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn capture_day_of_month(text: &str) -> Option<u32> {
    let toks: Vec<&str> = text.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    for i in 0..toks.len() {
        if toks[i] == "the" && i + 1 < toks.len() {
            let raw = toks[i + 1];
            let num: String = raw.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !num.is_empty() {
                if let Ok(d) = num.parse::<u32>() {
                    if (1..=31).contains(&d) {
                        return Some(d);
                    }
                }
            }
            // ordinal suffixes: 30th, 1st, 2nd, 3rd handled by take_while digits
        }
    }
    None
}

fn days_until(target: chrono::Weekday, from: NaiveDate) -> i64 {
    let cur = from.weekday().num_days_from_monday() as i64;
    let tgt = target.num_days_from_monday() as i64;
    let mut delta = (tgt - cur).rem_euclid(7);
    if delta == 0 {
        delta = 7; // strictly future occurrence
    }
    delta
}

fn next_month_with_day(today: NaiveDate, day: u32) -> Option<NaiveDate> {
    let (y, m) = (today.year(), today.month());
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, day)
}

/// Scan for a 4-digit-year ISO date `YYYY-MM-DD` anywhere in the text,
/// including inside glued alphanumeric runs (`"by2026-09-01"`). Returns the
/// first hit; rejects 2- or 3-digit years so `"26-09-01"` doesn't silently
/// become year 26 AD.
fn find_iso_date(text: &str) -> Option<NaiveDate> {
    let b = text.as_bytes();
    let mut i = 0;
    while i + 10 <= b.len() {
        if b[i].is_ascii_digit()
            && b[i + 1].is_ascii_digit()
            && b[i + 2].is_ascii_digit()
            && b[i + 3].is_ascii_digit()
            && b[i + 4] == b'-'
            && b[i + 5].is_ascii_digit()
            && b[i + 6].is_ascii_digit()
            && b[i + 7] == b'-'
            && b[i + 8].is_ascii_digit()
            && b[i + 9].is_ascii_digit()
        {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(&text[i..i + 10], "%Y-%m-%d") {
                return Some(d);
            }
        }
        i += 1;
    }
    None
}

use chrono::Datelike;

#[cfg(test)]
mod tests {
    use super::*;
    fn wed() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 26).unwrap() // Wednesday
    }

    #[test]
    fn iso_direct() {
        assert_eq!(
            parse_date_expression("pay by 2026-09-01 ok", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap())
        );
    }

    #[test]
    fn weekdays() {
        assert_eq!(
            parse_date_expression("this friday", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 8, 28).unwrap())
        );
        assert_eq!(
            parse_date_expression("on sunday", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 8, 30).unwrap())
        );
        assert_eq!(
            parse_date_expression("next friday", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 9, 4).unwrap())
        );
    }

    #[test]
    fn day_of_month() {
        assert_eq!(
            parse_date_expression("by the 30th", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 8, 30).unwrap())
        );
        assert_eq!(
            parse_date_expression("before the 5th", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 9, 5).unwrap())
        );
    }

    #[test]
    fn relative_words() {
        assert_eq!(
            parse_date_expression("due tomorrow morning", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 8, 27).unwrap())
        );
        assert_eq!(
            parse_date_expression("in 3 days", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 8, 29).unwrap())
        );
        assert_eq!(
            parse_date_expression("this weekend", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 8, 29).unwrap())
        );
    }

    #[test]
    fn vague_stays_null() {
        for s in ["soon", "next week", "sometime this week", "asap"] {
            let r = parse_date_expression(s, wed());
            assert_eq!(r.date, None, "{s}");
            assert!(r.vague_marker_found, "{s}");
        }
    }

    #[test]
    fn none_present() {
        let r = parse_date_expression("no time info here at all", wed());
        assert_eq!(r.date, None);
        assert!(!r.vague_marker_found);
    }

    #[test]
    fn iso_without_spaces_is_found() {
        // Common when copy-pasted without spacing — "pay by2026-09-01".
        assert_eq!(
            parse_date_expression("pay by2026-09-01", wed()).date,
            Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap())
        );
    }

    #[test]
    fn two_digit_year_is_rejected() {
        // The 2-digit-year footgun ("26-09-01" -> year 0026) is closed: the
        // scanner requires exactly 4 digits before the first dash.
        for s in ["26-09-01", "2026-09-1", "2026-9-01"] {
            assert_eq!(
                parse_date_expression(s, wed()).date,
                None,
                "{s} must not be accepted as a date"
            );
        }
    }

    #[test]
    fn tomorrow_from_saturday_rolls_to_sunday() {
        let sat = NaiveDate::from_ymd_opt(2026, 8, 29).unwrap();
        assert_eq!(
            parse_date_expression("tomorrow", sat).date,
            Some(NaiveDate::from_ymd_opt(2026, 8, 30).unwrap())
        );
    }

    #[test]
    fn weekend_from_sunday_returns_next_saturday() {
        // "this weekend" from Sunday should NOT be today (Sunday) — must
        // look forward to next Saturday. The implementation uses
        // days_until(Sat, today), which is 6 from Sunday.
        let sun = NaiveDate::from_ymd_opt(2026, 8, 30).unwrap();
        assert_eq!(
            parse_date_expression("this weekend", sun).date,
            Some(NaiveDate::from_ymd_opt(2026, 9, 5).unwrap())
        );
    }

    #[test]
    fn weekday_when_today_is_same_weekday_skips_a_week() {
        // If today IS Thursday and you say "thursday", the parser must NOT
        // return today (days_until returns 7 in that case). This is the
        // strict "future occurrence" contract.
        let thu = NaiveDate::from_ymd_opt(2026, 8, 27).unwrap();
        assert_eq!(
            parse_date_expression("thursday", thu).date,
            Some(NaiveDate::from_ymd_opt(2026, 9, 3).unwrap())
        );
    }

    #[test]
    fn the_nth_in_past_rolls_to_next_month() {
        // "the 25th" on the 26th should NOT be today-1 (yesterday); should be
        // next month.
        let r = parse_date_expression("by the 25th", wed());
        assert_eq!(r.date, Some(NaiveDate::from_ymd_opt(2026, 9, 25).unwrap()));
    }

    #[test]
    fn the_nth_impossible_day_rolls_to_next_valid_month() {
        // Document the chosen contract: "by the 30th" in February rolls
        // forward to March 30 (the next valid occurrence). This is
        // intentional — most users mean "the next 30th" — but it is a
        // real footgun if anyone changes the resolution semantics.
        let feb = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
        assert_eq!(
            parse_date_expression("by the 30th", feb).date,
            Some(NaiveDate::from_ymd_opt(2026, 3, 30).unwrap())
        );
    }

    #[test]
    fn the_nth_year_boundary_rolls_into_next_year() {
        // December "by the 5th" should roll into next January.
        let dec = NaiveDate::from_ymd_opt(2026, 12, 20).unwrap();
        assert_eq!(
            parse_date_expression("by the 5th", dec).date,
            Some(NaiveDate::from_ymd_opt(2027, 1, 5).unwrap())
        );
    }

    #[test]
    fn in_zero_days_is_today() {
        // Edge: "in 0 days" — current impl uses Duration::days(0), so this
        // is "today". Pin that contract.
        assert_eq!(parse_date_expression("in 0 days", wed()).date, Some(wed()));
    }

    #[test]
    fn invalid_iso_format_is_rejected() {
        // Out-of-range month/day are rejected by chrono.
        for s in ["2026-13-01", "2026-02-30", "2026-00-15"] {
            let r = parse_date_expression(s, wed());
            assert_eq!(r.date, None, "garbage ISO {s} must not be parsed as a date");
        }
        // Truncated / wrong-shape dates are also rejected by the strict
        // 4-digit-year + strict shape scan.
        assert_eq!(parse_date_expression("26-09-01", wed()).date, None);
        assert_eq!(parse_date_expression("2026-9-1", wed()).date, None);
    }
}
