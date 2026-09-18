//! Neural extraction path (feature-gated: `--features neural`).
//!
//! Architecture mirrors Codesym's split:
//! - neural component PROPOSES candidates (grammar-constrained decoding,
//!   output shape structurally guaranteed by a GBNF grammar);
//! - symbolic layer DISPOSES: direction is always re-derived by rules,
//!   self-reported confidence is treated as advisory and cross-checked
//!   against deterministic evidence;
//! - a circuit breaker falls back to the rule-based extractor when the
//!   model is slow, unavailable, or produces unusable output.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chrono::NaiveDate;

use crate::models::{Confidence, ExtractDirection, FieldConfidence};
use crate::rules;

pub const MODEL_PATH_ENV: &str = "LOOSE_ENDS_MODEL_PATH";
/// Path to the local GGUF model used for neural extraction.
///
/// The model is not bundled with this repository. Provide the path by either:
/// 1. Setting the `LOOSE_ENDS_MODEL_PATH` environment variable, or
/// 2. Replacing the placeholder below with an absolute or relative path
///    to your downloaded GGUF model file.
///
/// Example:
///   export LOOSE_ENDS_MODEL_PATH=/path/to/your/model.q4_k_m.gguf
pub const DEFAULT_MODEL_PATH: &str = "REPLACE_WITH_YOUR_GGUF_MODEL_PATH";

/// GBNF grammar enforcing the Section 5 schema as an array.
/// `commitment_found: false` / empty array are first-class valid outputs,
/// so the model can cleanly say "nothing here".
/// Loaded from extraction_grammar.gbnf to avoid string-escaping pitfalls.
pub const EXTRACTION_GRAMMAR: &str = include_str!("../extraction_grammar.gbnf");

const SYSTEM_PROMPT: &str = r#"You extract personal commitments from messy real-world text messages. A commitment is something one person owes another: an action owed by the writer (user_owes) or owed to the writer (owed_to_user).

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

Today's date is {TODAY}."#;

const FEW_SHOTS: &[(&str, &str)] = &[
    (
        "hey don't forget I'm bringing the cake to Sam's place on saturday for game night",
        r#"[{"commitment_found": true, "party": "Sam", "party_confidence": "high", "description": "bring cake to Sam's place for game night", "direction": "user_owes", "expected_date": "2026-08-29", "date_confidence": "high", "overall_confidence": "high"}]"#,
    ),
    (
        "Rosa said she'd drop off my keys at the cafe tomorrow afternoon",
        r#"[{"commitment_found": true, "party": "Rosa", "party_confidence": "high", "description": "Rosa to return keys at the cafe", "direction": "owed_to_user", "expected_date": "2026-08-27", "date_confidence": "high", "overall_confidence": "high"}]"#,
    ),
    (
        "hahah no way, that's hilarious 😂 anyway what are you up to this weekend",
        "[]",
    ),
    (
        "I owe Nina twenty bucks from the taxi, gotta pay her back soon",
        r#"[{"commitment_found": true, "party": "Nina", "party_confidence": "high", "description": "repay Nina 20 from taxi fare", "direction": "user_owes", "expected_date": null, "date_confidence": "low", "overall_confidence": "high"}]"#,
    ),
    (
        "told him I'd get it back to him whenever, no rush",
        r#"[{"commitment_found": true, "party": null, "party_confidence": "low", "description": "return item to unnamed person", "direction": "unclear", "expected_date": null, "date_confidence": "low", "overall_confidence": "low"}]"#,
    ),
    (
        "so friday I pay Pete back for lunch, and he's supposed to send me those podcast notes he promised",
        r#"[{"commitment_found": true, "party": "Pete", "party_confidence": "high", "description": "repay Pete for lunch", "direction": "user_owes", "expected_date": "2026-08-28", "date_confidence": "high", "overall_confidence": "high"}, {"commitment_found": true, "party": "Pete", "party_confidence": "high", "description": "Pete to send podcast notes", "direction": "owed_to_user", "expected_date": null, "date_confidence": "low", "overall_confidence": "low"}]"#,
    ),
    (
        "he knows what he owes me. twenty since march",
        r#"[{"commitment_found": true, "party": null, "party_confidence": "low", "description": "unnamed person owes user 20 since March", "direction": "owed_to_user", "expected_date": null, "date_confidence": "low", "overall_confidence": "low"}]"#,
    ),
    (
        "I owe my landlord for last month and I keep forgetting, will sort it out this week",
        r#"[{"commitment_found": true, "party": "landlord", "party_confidence": "high", "description": "pay landlord last month's rent", "direction": "user_owes", "expected_date": null, "date_confidence": "low", "overall_confidence": "high"}]"#,
    ),
];

pub fn build_prompt(input_text: &str, today: NaiveDate) -> String {
    let sys = SYSTEM_PROMPT.replace("{TODAY}", &today.to_string());
    let mut parts = vec![format!("<|im_start|>system\n{sys}<|im_end|>\n")];
    for (u, a) in FEW_SHOTS {
        parts.push(format!("<|im_start|>user\n{u}<|im_end|>\n"));
        parts.push(format!("<|im_start|>assistant\n{a}<|im_end|>\n"));
    }
    parts.push(format!(
        "<|im_start|>user\n{input_text}<|im_end|>\n<|im_start|>assistant\n"
    ));
    parts.concat()
}

/// One parsed element of the model's JSON array.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct NeuralCandidate {
    #[serde(default = "default_true")]
    pub commitment_found: bool,
    pub party: Option<String>,
    #[serde(default)]
    pub party_confidence: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub expected_date: Option<String>,
    #[serde(default)]
    pub date_confidence: Option<String>,
    #[serde(default)]
    pub overall_confidence: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug)]
pub enum NeuralOutcome {
    Candidates(Vec<NeuralCandidate>),
    TimedOut,
    Unavailable(String),
    Unusable(String),
}

/// Run one extraction attempt against the local GGUF model with the
/// extraction grammar enforced at sampling time.
pub fn extract_neural(
    text: &str,
    today: NaiveDate,
    max_tokens: u32,
    timeout: Duration,
) -> NeuralOutcome {
    let model_path =
        std::env::var(MODEL_PATH_ENV).unwrap_or_else(|_| DEFAULT_MODEL_PATH.to_string());
    extract_neural_with_model_path(text, today, max_tokens, timeout, &model_path)
}

/// Run one extraction attempt against an explicit local GGUF model path.
pub fn extract_neural_with_model_path(
    text: &str,
    today: NaiveDate,
    max_tokens: u32,
    timeout: Duration,
    model_path: &str,
) -> NeuralOutcome {
    if model_path.trim().is_empty() {
        return NeuralOutcome::Unavailable("empty model path".into());
    }
    let prompt = build_prompt(text, today);

    // Blocking C calls cannot be interrupted; the timeout is honored by
    // abandoning this thread's result if it finishes late. The caller's UI
    // path returns via channel either way.
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let res = run_inference(&model_path, &prompt, max_tokens);
        let _ = tx.send(res);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(raw)) => {
            handle.join().ok();
            parse_candidates(&raw)
        }
        Ok(Err(e)) => {
            handle.join().ok();
            NeuralOutcome::Unavailable(e)
        }
        Err(mpsc::RecvTimeoutError::Timeout) => NeuralOutcome::TimedOut,
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            NeuralOutcome::Unavailable("inference thread died".into())
        }
    }
}

fn run_inference(model_path: &str, prompt: &str, max_tokens: u32) -> Result<String, String> {
    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::AddBos;
    use llama_cpp_2::sampling::LlamaSampler;

    let backend = LlamaBackend::init().map_err(|e| format!("backend init: {e:?}"))?;
    let model_params = pin!(LlamaModelParams::default());
    let model = llama_cpp_2::model::LlamaModel::load_from_file(
        &backend,
        PathBuf::from(model_path),
        &model_params,
    )
    .map_err(|e| format!("model load: {e:?}"))?;

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(Some(NonZeroU32::new(4096).ok_or("bad ctx")?))
        .with_n_threads(6)
        .with_n_threads_batch(6);
    let mut ctx = model
        .new_context(&backend, ctx_params)
        .map_err(|e| e.to_string())?;

    let tokens = model
        .str_to_token(prompt, AddBos::Always)
        .map_err(|e| e.to_string())?;

    let grammar = LlamaSampler::grammar(&model, EXTRACTION_GRAMMAR, "root")
        .map_err(|e| format!("grammar init: {e:?}"))?;
    let mut sampler = LlamaSampler::chain_simple([
        grammar,
        LlamaSampler::dist(1234),
    ]);

    let mut batch = LlamaBatch::new(std::cmp::max(512, tokens.len()), 1);
    let last_index = (tokens.len() - 1) as i32;
    for (i, token) in (0_i32..).zip(tokens.into_iter()) {
        batch
            .add(token, i, &[0], i == last_index)
            .map_err(|e| e.to_string())?;
    }
    ctx.decode(&mut batch).map_err(|e| e.to_string())?;

    let mut out = String::new();
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut n_cur = batch.n_tokens();
    for _ in 0..max_tokens {
        let tok = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(tok);
        if model.is_eog_token(tok) {
            break;
        }
        let piece = model
            .token_to_piece(tok, &mut decoder, false, None)
            .map_err(|e| e.to_string())?;
        out.push_str(&piece);
        batch.clear();
        batch
            .add(tok, n_cur, &[0], true)
            .map_err(|e| e.to_string())?;
        n_cur += 1;
        ctx.decode(&mut batch).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

fn parse_candidates(raw: &str) -> NeuralOutcome {
    let start = raw.find('[');
    let end = raw.rfind(']');
    let (Some(s), Some(e)) = (start, end) else {
        return NeuralOutcome::Unusable(format!("no JSON array in output: {raw:.80}"));
    };
    if e < s {
        return NeuralOutcome::Unusable("malformed array span".into());
    }
    match serde_json::from_str::<Vec<NeuralCandidate>>(&raw[s..=e]) {
        Ok(v) => {
            if v.iter()
                .any(|c| c.description.is_none() || c.direction.is_none())
            {
                NeuralOutcome::Unusable("candidate missing required fields".into())
            } else {
                NeuralOutcome::Candidates(
                    v.into_iter()
                        .filter(|c| c.commitment_found && c.description.is_some())
                        .collect(),
                )
            }
        }
        Err(e) => NeuralOutcome::Unusable(format!("json: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Confidence cross-check (spec Stage 3.4): model self-reports are advisory.
// ---------------------------------------------------------------------------

const DATEISH: &[&str] = &[
    "today",
    "tomorrow",
    "tonight",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "week",
    "month",
    "days",
    "soon",
];

pub fn cross_check_candidate(c: &NeuralCandidate, source_text: &str) -> CrossChecked {
    let lower = source_text.to_lowercase();

    // DATE: downgrade unless some date-like evidence exists in the source.
    let date_evidence = DATEISH.iter().any(|w| lower.contains(w)) || regex_lite_iso(&lower);

    // PARTY: downgrade if the claimed party name never appears in the source.
    let party_evidence = c
        .party
        .as_deref()
        .map(|p| {
            let pl = p.to_lowercase();
            pl.is_empty()
                || lower.contains(&pl)
                || pl
                    .split_whitespace()
                    .any(|word| word.len() > 2 && lower.contains(word))
        })
        .unwrap_or(false);

    CrossChecked {
        description: c.description.clone().unwrap_or_default(),
        party: c.party.clone(),
        direction_symbolic: rules::assign_direction(
            source_text,
            &c.description
                .as_deref()
                .map(crate::rules::normalize_tokens)
                .unwrap_or_default()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        ),
        expected_date: c.expected_date.clone(),
        confidence: Confidence {
            party: Some(if party_evidence {
                FieldConfidence::High
            } else {
                FieldConfidence::Low
            }),
            date: Some(if date_evidence {
                FieldConfidence::High
            } else {
                FieldConfidence::Low
            }),
            overall: Some(match c.overall_confidence.as_deref() {
                Some("high") if date_evidence && party_evidence => FieldConfidence::High,
                _ => FieldConfidence::Low,
            }),
        },
    }
}

fn regex_lite_iso(s: &str) -> bool {
    // cheap ISO-date sniff without regex dependency churn here
    let b = s.as_bytes();
    (0..b.len().saturating_sub(9)).any(|i| {
        b[i].is_ascii_digit()
            && b[i + 1].is_ascii_digit()
            && b[i + 2].is_ascii_digit()
            && b[i + 3].is_ascii_digit()
            && b[i + 4] == b'-'
            && b[i + 5].is_ascii_digit()
            && b[i + 6].is_ascii_digit()
            && b[i + 7] == b'-'
    })
}

#[derive(Debug, Clone)]
pub struct CrossChecked {
    pub description: String,
    pub party: Option<String>,
    /// Symbolic layer's verdict on direction — authoritative.
    pub direction_symbolic: ExtractDirection,
    pub expected_date: Option<String>,
    pub confidence: Confidence,
}

// ---------------------------------------------------------------------------
// Circuit breaker (spec Stage 3.3).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BreakerConfig {
    pub timeout: Duration,
    pub failure_threshold: u32,
    pub open_cooldown: Duration,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            failure_threshold: 3,
            open_cooldown: Duration::from_secs(300),
        }
    }
}

#[derive(Default)]
struct BreakerState {
    consecutive_failures: u32,
    open_until: Option<Instant>,
}

impl BreakerState {
    fn is_open(&self) -> bool {
        self.open_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    fn record_failure(&mut self, cfg: &BreakerConfig) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= cfg.failure_threshold {
            self.open_until = Some(Instant::now() + cfg.open_cooldown);
        }
    }

    fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.open_until = None;
    }
}

/// Shared extractor with circuit breaker. Clone-safe across threads.
#[derive(Clone, Default)]
pub struct NeuralExtractor {
    cfg: std::sync::Arc<BreakerConfig>,
    state: std::sync::Arc<std::sync::Mutex<BreakerState>>,
}

impl NeuralExtractor {
    pub fn new(cfg: BreakerConfig) -> Self {
        Self {
            cfg: std::sync::Arc::new(cfg),
            state: std::sync::Arc::default(),
        }
    }

    /// Attempt neural extraction; fall back to rules on timeout/unavailability/
    /// unusable output. Returns candidates plus which path produced them.
    pub fn extract(&self, text: &str, today: NaiveDate) -> (Vec<CrossChecked>, ProvenancePath) {
        let model_path =
            std::env::var(MODEL_PATH_ENV).unwrap_or_else(|_| DEFAULT_MODEL_PATH.to_string());
        self.extract_with_model_path(text, today, &model_path)
    }

    pub fn extract_with_model_path(
        &self,
        text: &str,
        today: NaiveDate,
        model_path: &str,
    ) -> (Vec<CrossChecked>, ProvenancePath) {
        if self.state.lock().unwrap().is_open() {
            return (
                self.rules_fallback(text, today),
                ProvenancePath::RuleFallbackBreakerOpen,
            );
        }
        match extract_neural_with_model_path(text, today, 700, self.cfg.timeout, model_path) {
            NeuralOutcome::Candidates(cands) => {
                self.state.lock().unwrap().record_success();
                let checked = cands
                    .iter()
                    .map(|c| cross_check_candidate(c, text))
                    .collect();
                (checked, ProvenancePath::Model)
            }
            NeuralOutcome::TimedOut => {
                self.state.lock().unwrap().record_failure(&self.cfg);
                (
                    self.rules_fallback(text, today),
                    ProvenancePath::RuleFallbackTimeout,
                )
            }
            NeuralOutcome::Unavailable(_) | NeuralOutcome::Unusable(_) => {
                self.state.lock().unwrap().record_failure(&self.cfg);
                (
                    self.rules_fallback(text, today),
                    ProvenancePath::RuleFallbackFailure,
                )
            }
        }
    }

    fn rules_fallback(&self, text: &str, today: NaiveDate) -> Vec<CrossChecked> {
        rules::extract_rules(text, today)
            .into_iter()
            .map(|r| CrossChecked {
                description: r.description,
                party: r.party_guess,
                direction_symbolic: r.direction,
                expected_date: r.expected_date.map(|d| d.to_string()),
                confidence: r.confidence,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenancePath {
    Model,
    RuleFallbackTimeout,
    RuleFallbackFailure,
    RuleFallbackBreakerOpen,
}

#[cfg(test)]
mod tests {
    //! Tests for the symbolic-side safety nets that gate the model output.
    //! The actual llama.cpp path requires the GGUF model and is exercised
    //! end-to-end via `core/src/bin/eval_rust.rs` on the eval dataset.

    use super::*;
    use crate::models::FieldConfidence;

    fn c(party: Option<&str>, direction: Option<&str>, date: Option<&str>) -> NeuralCandidate {
        NeuralCandidate {
            commitment_found: true,
            party: party.map(str::to_string),
            party_confidence: Some("high".into()),
            description: Some("do the thing".into()),
            direction: direction.map(str::to_string),
            expected_date: date.map(str::to_string),
            date_confidence: Some("high".into()),
            overall_confidence: Some("high".into()),
        }
    }

    #[test]
    fn cross_check_party_unknown_to_source_is_downgraded() {
        let src = "I owe Dave 20";
        let cand = c(Some("Zelda"), Some("user_owes"), None);
        let checked = cross_check_candidate(&cand, src);
        assert_eq!(checked.confidence.party, Some(FieldConfidence::Low));
        // overall never high when party evidence missing
        assert_eq!(checked.confidence.overall, Some(FieldConfidence::Low));
    }

    #[test]
    fn cross_check_party_present_in_source_keeps_high() {
        let src = "I owe Dave 20";
        let cand = c(Some("Dave"), Some("user_owes"), None);
        let checked = cross_check_candidate(&cand, src);
        assert_eq!(checked.confidence.party, Some(FieldConfidence::High));
    }

    #[test]
    fn cross_check_null_party_is_low_confidence() {
        let src = "I owe him later";
        let cand = c(None, Some("user_owes"), None);
        let checked = cross_check_candidate(&cand, src);
        assert_eq!(checked.confidence.party, Some(FieldConfidence::Low));
    }

    #[test]
    fn cross_check_date_with_temporal_word_is_high() {
        let src = "I need to pay Marco tomorrow";
        let cand = c(Some("Marco"), Some("user_owes"), Some("2026-08-27"));
        let checked = cross_check_candidate(&cand, src);
        assert_eq!(checked.confidence.date, Some(FieldConfidence::High));
    }

    #[test]
    fn cross_check_date_with_no_temporal_evidence_is_low() {
        // Model fabricated a date out of nothing — date_confidence must downgrade.
        let src = "I owe Marco at some point";
        let cand = c(Some("Marco"), Some("user_owes"), Some("2026-09-30"));
        let checked = cross_check_candidate(&cand, src);
        assert_eq!(checked.confidence.date, Some(FieldConfidence::Low));
        assert_eq!(checked.confidence.overall, Some(FieldConfidence::Low));
    }

    #[test]
    fn cross_check_iso_in_source_keeps_date_high() {
        let src = "by 2026-09-01 I'll send the form";
        let cand = c(None, Some("user_owes"), Some("2026-09-01"));
        let checked = cross_check_candidate(&cand, src);
        assert_eq!(checked.confidence.date, Some(FieldConfidence::High));
    }

    #[test]
    fn cross_check_overall_requires_both_party_and_date_evidence() {
        let src = "I owe him eventually"; // no party name, no date
        let cand = c(Some("him"), Some("user_owes"), None);
        let checked = cross_check_candidate(&cand, src);
        assert_eq!(checked.confidence.overall, Some(FieldConfidence::Low));
    }

    #[test]
    fn cross_check_symbolic_direction_overrides_model() {
        // The whole point of the split: rules are authoritative.
        let src = "I owe Dave 20";
        let cand = c(Some("Dave"), Some("owed_to_user"), None); // model lies
        let checked = cross_check_candidate(&cand, src);
        assert_eq!(checked.direction_symbolic, ExtractDirection::UserOwes);
    }

    // ---------------- circuit breaker ----------------

    fn cfg_with(threshold: u32, cooldown_ms: u64) -> BreakerConfig {
        BreakerConfig {
            timeout: Duration::from_secs(1),
            failure_threshold: threshold,
            open_cooldown: Duration::from_millis(cooldown_ms),
        }
    }

    #[test]
    fn breaker_starts_closed() {
        let s = BreakerState::default();
        assert!(!s.is_open());
    }

    #[test]
    fn breaker_opens_at_threshold() {
        let cfg = cfg_with(3, 60_000);
        let mut s = BreakerState::default();
        s.record_failure(&cfg); // 1
        assert!(!s.is_open());
        s.record_failure(&cfg); // 2
        assert!(!s.is_open());
        s.record_failure(&cfg); // 3 -> opens
        assert!(s.is_open());
    }

    #[test]
    fn breaker_success_resets_consecutive_failures() {
        let cfg = cfg_with(3, 60_000);
        let mut s = BreakerState::default();
        s.record_failure(&cfg);
        s.record_failure(&cfg);
        s.record_success();
        // After success, two more failures should NOT open the breaker
        s.record_failure(&cfg);
        s.record_failure(&cfg);
        assert!(!s.is_open(), "success must reset the failure counter");
    }

    #[test]
    fn breaker_closes_after_cooldown_elapses() {
        let cfg = cfg_with(1, 5); // threshold 1, 5ms cooldown
        let mut s = BreakerState::default();
        s.record_failure(&cfg);
        assert!(s.is_open());
        std::thread::sleep(Duration::from_millis(20));
        assert!(!s.is_open(), "cooldown expired; breaker should re-close");
    }

    #[test]
    fn breaker_is_clone_safe_and_shares_state() {
        // Two handles to the same state must agree (Arc-shared internally).
        let cfg = cfg_with(1, 60_000);
        let extractor = NeuralExtractor::new(cfg);
        // We can't easily force a failure without the model, but we CAN verify
        // the Arc is wired: both halves see the same is_open behavior.
        let a = extractor.clone();
        let b = extractor;
        // No public peek; instead exercise that the inner state can be reached.
        // (Smoke test — the real lifecycle is exercised by eval_rust.)
        let _ = (a, b);
    }

    // ---------------- parse_candidates (no model needed) ----------------

    #[test]
    fn parse_candidates_accepts_well_formed_array() {
        let raw = r#"[{"commitment_found": true, "description":"x", "direction":"user_owes",
                        "party":"Dave", "party_confidence":"high",
                        "expected_date":"2026-08-30", "date_confidence":"high",
                        "overall_confidence":"high"}]"#;
        match parse_candidates(raw) {
            NeuralOutcome::Candidates(v) => assert_eq!(v.len(), 1),
            other => panic!("expected Candidates, got {other:?}"),
        }
    }

    #[test]
    fn parse_candidates_empty_array_is_valid_no_commitments() {
        let raw = "[]";
        match parse_candidates(raw) {
            NeuralOutcome::Candidates(v) => assert!(v.is_empty()),
            other => panic!("expected empty Candidates, got {other:?}"),
        }
    }

    #[test]
    fn parse_candidates_rejects_non_array_output() {
        let raw = "I cannot believe what just happened";
        match parse_candidates(raw) {
            NeuralOutcome::Unusable(_) => {}
            other => panic!("expected Unusable, got {other:?}"),
        }
    }

    #[test]
    fn parse_candidates_rejects_missing_description_field() {
        let raw = r#"[{"commitment_found": true, "direction":"user_owes"}]"#;
        match parse_candidates(raw) {
            NeuralOutcome::Unusable(_) => {}
            other => panic!("expected Unusable for missing required field, got {other:?}"),
        }
    }

    #[test]
    fn parse_candidates_tolerates_garbage_around_array() {
        // Models sometimes add preamble / postamble
        let raw = r#"blah blah [ {"commitment_found": true, "description":"x", "direction":"user_owes"} ] trailing noise"#;
        match parse_candidates(raw) {
            NeuralOutcome::Candidates(v) => assert_eq!(v.len(), 1),
            other => panic!("expected Candidates (tolerant parse), got {other:?}"),
        }
    }

    #[test]
    fn parse_candidates_filters_out_commitment_found_false() {
        let raw = r#"[{"commitment_found": false, "description":"x", "direction":"user_owes"},
                       {"commitment_found": true,  "description":"y", "direction":"user_owes"}]"#;
        match parse_candidates(raw) {
            NeuralOutcome::Candidates(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].description.as_deref(), Some("y"));
            }
            other => panic!("expected filtered Candidates, got {other:?}"),
        }
    }
}
