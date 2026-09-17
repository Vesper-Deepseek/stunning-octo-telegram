# Stage 1 Report — Extraction Baseline (Python harness)

**Date:** 2026-08-26 · **Model:** Qwen2.5-1.5B-Instruct Q4_K_M · **Runtime:** llama.cpp `llama-server`, CPU-only (6 threads), greedy decoding (temp=0)
**Dataset:** `eval/dataset.jsonl` — 46 synthetic-but-realistic inputs / 47 gold commitments
Categories: clear_user_owes ×8 · clear_owed_to_user ×8 · ambiguous_direction ×6 · vague_or_missing_date ×8 · multi_commitment ×7 · noise ×9

## Headline numbers

| configuration | P | R | F1 | R after review¹ | party² | direction² | date² | fabricated dates³ | valid JSON |
|---|---|---|---|---|---|---|---|---|---|
| v1 prompt (frozen, raw model)   | 32.0 | 34.0 | **33.0** | 34.0% | 100.0 | 100.0 | 37.5 | 2 | 100% |
| v2 prompt (direction rules text)| 37.8 | 36.2 | **37.0** | 36.2% | 100.0 | 100.0 | 70.6 | 0 | 100% |
| hybrid = v2 + symbolic direction overlay | 57.8 | 55.3 | **56.5** | **68.1%** | 92.3 | 100.0 | 76.9 | 1 | 100% |

¹ Secondary metric: prediction matches a gold commitment with description-token-F1 ≥ 0.5 while its direction may be `unclear` — i.e. entries that surface correctly but need one-tap human confirmation.
² Field accuracy computed only on matched pairs (n=16/17/26 respectively).
³ Predicted an ISO date where gold is null.

Latency (CPU): mean ≈ 7–9 s, max ≈ 23 s per example. Noise rejection: **perfect in every run** (0 false positives across all 9 noise inputs, including emoji-laden and near-commitment phrasings).

## Honest findings

1. **The headline risk is confirmed.** Raw model F1 is ~33–37%. This is far below what "extracts commitments correctly" implies. Prompt iteration alone (v1→v2) moved it by ~4 points. Anyone building UI or storage on top of the assumption "the model gets it right" would be building on sand.

2. **Root cause of most failures: systematic direction inversion.** The model flips *who owes whom* on a large fraction of inputs ("I owe Dave" → `owed_to_user`). Bisecting few-shots showed no stable fix: adding any single few-shot could flip previously-correct answers, *including* few-shots labeled identically to the probe. Standalone direction probes fail too. This is a genuine capability ceiling of this 1.5B model at greedy decoding, not a harness artifact (verified against hand-rendered raw completions).

3. **Symbolic adjudication of direction works.** A deterministic first/third-person rule layer (clause splitting + actor patterns, defaulting to `unclear`) classifies direction at **34/42 correct, 0 confidently-wrong, 8 honest-unclears (19%)** on this dataset. Overriding the model's direction with these rules lifted F1 from 37.0 → 56.5 and after-review usability from 36.2% → 68.1%, while keeping matched-pair direction accuracy at 100%. Policy note: when rules say `unclear`, we do **not** fall back to the model's direction — its flip rate makes it worse than useless as a tiebreaker.

4. **Remaining weakness: commitment-level recall (~55%).** Causes: merged multiple commitments into one object (e.g. three requests collapsed into one), dropped items, and descriptions too divergent to match. Mitigations deferred to Stage 3, smallest first:
   - clause/sentence-splitting pre-pass: extract per clause instead of per message;
   - if a measured ceiling persists: flagged experiment with Qwen2.5-3B-Instruct (larger RAM footprint — must be re-justified for mid-range devices);
   - product-level: the review screen is the safety net — nothing auto-saves as fact without user confirmation (spec §1, §8).

5. **Date discipline improved with prompting.** Fabricated dates on dateless inputs: 2 → 0 → 1. Date accuracy on matched pairs rose 37.5 → 76.9. The schema's "prefer null over fabrication" instruction works at this scale.

6. **Format compliance was 100% even WITHOUT the GBNF grammar** (JSON array, correct shape). The grammar (Stage 3) remains mandatory as a *structural guarantee*, not an accuracy feature.

## Decision taken forward (explicitly recorded, not silent)

The neural path's role narrows to: party, description, date, existence-detection. **Direction is a symbolic-layer output**, cross-checked against the neural field and stored with provenance. This mirrors the Codesym split: neural candidate generation, symbolic verification of typed facts, user confirmation as the final oracle.

No fine-tuning was attempted (per spec §7). Next checkpoint re-measures everything through the actual Rust inference path (Stage 3), not just Python.

## Addendum (after Stage 2 port)

Porting the direction rules to Rust (`core/src/rules.rs`, parity-tested against a mechanically generated fixture `core/tests/direction_parity.json`) surfaced one genuine rule improvement — `X promised to bring me Y ⇒ owed_to_user` — which lifts direction classification to **35/42 confident-correct, 0 confidently-wrong, 7 honest-unclears**. Rescoring the hybrid configuration without re-running inference:

| hybrid v3 (Rust-parity rules) | P=60.0 | R=57.4 | **F1=58.7** | review-usable 68.1% | dir 100% | date 77.8% |
