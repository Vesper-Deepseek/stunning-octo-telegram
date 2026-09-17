//! Stage 3 evaluation CLI: runs the FULL integrated Rust extraction path
//! (neural + grammar + circuit breaker + symbolic direction + confidence
//! cross-check — exactly what the app will execute) over the Stage 1
//! dataset, emitting per-example output files compatible with the Python
//! scorer (`eval/run_eval.py --rescore`).
//!
//! Usage:
//!   cargo run --release --features neural --bin eval_rust -- \
//!     --dataset ../../eval/dataset.jsonl --out ../../eval/outputs/stage3_rust

use std::path::PathBuf;

use loose_ends_core::models::ExtractDirection;
use loose_ends_core::neural::{BreakerConfig, NeuralExtractor};

#[derive(serde::Deserialize)]
struct Example {
    id: String,
    category: String,
    input: String,
    expected: serde_json::Value,
}

fn main() -> anyhow_like::Result<()> {
    let mut dataset = PathBuf::from("../../eval/dataset.jsonl");
    let mut out_dir = PathBuf::from("../../eval/outputs/stage3_rust");
    let mut limit: Option<usize> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--dataset" => dataset = PathBuf::from(args.next().unwrap()),
            "--out" => out_dir = PathBuf::from(args.next().unwrap()),
            "--limit" => limit = Some(args.next().unwrap().parse()?),
            _ => {}
        }
    }

    std::fs::create_dir_all(&out_dir)?;
    let text = std::fs::read_to_string(&dataset)?;
    let mut examples: Vec<Example> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("bad jsonl"))
        .collect();
    if let Some(n) = limit {
        examples.truncate(n);
    }

    // Circuit breaker config mirrors production defaults.
    let extractor = NeuralExtractor::new(BreakerConfig {
        timeout: std::time::Duration::from_secs(60),
        failure_threshold: 3,
        open_cooldown: std::time::Duration::from_secs(300),
    });

    let today = chrono::NaiveDate::from_ymd_opt(2026, 8, 26).unwrap();

    for ex in &examples {
        let t0 = std::time::Instant::now();
        let (cands, path) = extractor.extract(&ex.input, today);
        let dt = t0.elapsed().as_secs_f64();

        let preds: Vec<serde_json::Value> = cands
            .iter()
            .map(|c| {
                serde_json::json!({
                    "commitment_found": true,
                    "party": c.party,
                    "description": c.description,
                    "direction": dir_str(c.direction_symbolic),
                    "expected_date": c.expected_date,
                    "date_confidence": conf_str(c.confidence.date),
                    "party_confidence": conf_str(c.confidence.party),
                    "overall_confidence": conf_str(c.confidence.overall),
                })
            })
            .collect();

        let rec = serde_json::json!({
            "id": ex.id,
            "category": ex.category,
            "input": ex.input,
            "raw": format!("<rust-path:{}>", match path {
                loose_ends_core::neural::ProvenancePath::Model => "model",
                loose_ends_core::neural::ProvenancePath::RuleFallbackTimeout => "fallback_timeout",
                loose_ends_core::neural::ProvenancePath::RuleFallbackFailure => "fallback_breaker_open",
                loose_ends_core::neural::ProvenancePath::RuleFallbackBreakerOpen => "fallback_breaker_open",
            }),
            "parse_note": "model_output",
            "provenance_path": format!("{path:?}"),
            "preds": preds,
            "latency_s": dt,
        });
        std::fs::write(
            out_dir.join(format!("{}.json", ex.id)),
            serde_json::to_string_pretty(&rec)?,
        )?;
        println!("{} [{:?}] {} preds in {:.1}s", ex.id, path, preds.len(), dt);
    }
    Ok(())
}

fn dir_str(d: ExtractDirection) -> &'static str {
    match d {
        ExtractDirection::UserOwes => "user_owes",
        ExtractDirection::OwedToUser => "owed_to_user",
        ExtractDirection::Unclear => "unclear",
    }
}

fn conf_str(c: Option<loose_ends_core::models::FieldConfidence>) -> &'static str {
    match c {
        Some(loose_ends_core::models::FieldConfidence::High) => "high",
        _ => "low",
    }
}

mod anyhow_like {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}
