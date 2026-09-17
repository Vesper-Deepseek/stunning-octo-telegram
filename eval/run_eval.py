#!/usr/bin/env python3
"""Loose Ends Stage 1 evaluation harness.

Runs the draft extraction prompt against a local llama.cpp server
(Qwen2.5-1.5B-Instruct Q4_K_M, greedy decoding) with NO grammar
constraints, then scores:
  - parse robustness (valid JSON array without repair?)
  - commitment-level TP / FP / FN (greedy match on content-token F1 + direction)
  - field-level accuracy on matched pairs: party, direction, date
  - fabrication metrics (invented dates/parties), noise rejection

Usage: python3 run_eval.py --server http://127.0.0.1:8012 [--limit N]
Outputs raw responses to eval/outputs/stage1/ and a report to eval/reports/.
"""

import argparse
import json
import re
import statistics
import sys
import time
from pathlib import Path

import requests

import importlib

from direction_rules import assign_direction, description_tokens

HERE = Path(__file__).parent
OUT_DIR = HERE / "outputs" / "stage1"
REPORT_DIR = HERE / "reports"

STOPWORDS = {
    "a", "an", "the", "to", "of", "for", "and", "or", "in", "on", "at", "by",
    "with", "from", "is", "are", "was", "were", "be", "been", "being", "it",
    "its", "this", "that", "i", "me", "my", "we", "us", "our", "you", "your",
    "he", "she", "him", "her", "they", "them", "their", "as", "so", "if",
    "will", "would", "shall", "should", "can", "could", "do", "does", "did",
    "have", "has", "had", "get", "got", "back", "still", "yet", "not", "no",
}

ISO_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def norm_tokens(s):
    return [
        t for t in re.findall(r"[a-z0-9]+", s.lower())
        if t not in STOPWORDS and len(t) > 1
    ]


def token_f1(a, b):
    ta, tb = set(norm_tokens(a)), set(norm_tokens(b))
    if not ta or not tb:
        return 0.0
    overlap = len(ta & tb)
    if overlap == 0:
        return 0.0
    p = overlap / len(tb)
    r = overlap / len(ta)
    return 2 * p * r / (p + r)


def norm_party(p):
    if p is None:
        return None
    s = re.sub(r"[^a-z0-9 ]", "", str(p).lower()).strip()
    for w in ("my ", "the ", "a ", "'s"):
        s = s.replace(w, " ").replace("'", "").strip()
    return re.sub(r"\s+", " ", s).strip() or None


def parties_match(pred, gold):
    np, ng = norm_party(pred), norm_party(gold)
    if np is None and ng is None:
        return True
    if np is None or ng is None:
        return False
    return np == ng or np in ng or ng in np


def extract_json_array(raw):
    """Return (list_or_None, parse_ok, repair_note). No grammar here on purpose."""
    m = re.search(r"\[.*\]", raw, re.DOTALL)
    if not m:
        return None, False, "no_array_found"
    try:
        val = json.loads(m.group(0))
    except json.JSONDecodeError as e:
        return None, False, f"json_decode_error:{e}"
    if isinstance(val, list):
        return [v for v in val if isinstance(v, dict)], True, "clean"
    return None, False, "not_a_list"


def call_model(server, messages, max_tokens=600, timeout=300):
    t0 = time.time()
    r = requests.post(
        f"{server}/v1/chat/completions",
        json={
            "messages": messages,
            "temperature": 0.0,
            "max_tokens": max_tokens,
            "stream": False,
        },
        timeout=timeout,
    )
    r.raise_for_status()
    dt = time.time() - t0
    return r.json()["choices"][0]["message"]["content"], dt


def score_example(exp_ex, preds, allow_unclear=False):
    """Greedy matching. Returns dict with tp pairs, fp count, fn list.

    allow_unclear=True additionally pairs a prediction whose direction is
    'unclear' against a gold with a concrete direction (review-usability
    metric: the entry surfaces correctly but needs human confirmation).
    """
    used = set()
    pairs = []
    fps = []
    for pred in preds:
        best_i, best_s = None, 0.0
        for i, gold in enumerate(exp_ex):
            if i in used:
                continue
            if pred.get("direction") != gold["direction"]:
                if not (
                    allow_unclear
                    and pred.get("direction") == "unclear"
                ):
                    continue
            s = token_f1(str(pred.get("description") or ""), gold["description"])
            if s > best_s:
                best_i, best_s = i, s
        if best_i is not None and best_s >= 0.5:
            used.add(best_i)
            pairs.append((pred, exp_ex[best_i]))
        else:
            fps.append(pred)
    fns = [exp_ex[i] for i in range(len(exp_ex)) if i not in used]
    return pairs, fps, fns


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--server", default="http://127.0.0.1:8012")
    ap.add_argument("--limit", type=int, default=None)
    ap.add_argument("--tag", default="stage1_baseline",
                    help="output/report subdirectory tag, e.g. stage1_prompt_v2")
    ap.add_argument("--hybrid", action="store_true",
                    help="override model direction with symbolic direction rules")
    ap.add_argument("--prompt-module", default="prompt",
                    help="prompt module: 'prompt' (v2) or 'prompt_v1_frozen'")
    ap.add_argument("--rescore", action="store_true",
                    help="score existing outputs instead of calling the server")
    args = ap.parse_args()

    pm = importlib.import_module(args.prompt_module)
    build_messages = pm.build_messages
    PROMPT_VERSION = pm.PROMPT_VERSION

    examples = []
    with open(HERE / "dataset.jsonl", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                examples.append(json.loads(line))
    if args.limit:
        examples = examples[: args.limit]

    OUT_DIR = HERE / "outputs" / args.tag
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    REPORT_DIR.mkdir(parents=True, exist_ok=True)

    n_parse_ok = n_clean_json = 0
    tp_pairs, all_fps, all_fns = [], [], []
    party_ok = party_err = dir_ok = dir_err = date_ok = date_err = date_fab = 0
    review_usable = 0
    conf_dist = {"overall": {}, "date": {}, "party": {}}
    latencies = []
    saved_preds = None

    for ex in examples:
        out_path = OUT_DIR / f"{ex['id']}.json"
        if args.rescore and out_path.exists():
            prev = json.loads(out_path.read_text(encoding="utf-8"))
            dt = prev.get("latency_s", 0.0)
            saved_preds = None
            if "preds" in prev:
                # final preds already stored (e.g. Rust integrated path)
                raw, note, parse_ok = "", "loaded_final_preds", True
                saved_preds = prev["preds"]
            else:
                raw, note, parse_ok = prev["raw"], "rescored", True
        else:
            msgs = build_messages(ex["input"])
            raw, dt = call_model(args.server, msgs)
            latencies.append(dt)
            preds_probe, parse_ok, note = extract_json_array(raw)

        if saved_preds is not None:
            preds = saved_preds
        else:
            preds, _po, _n2 = extract_json_array(raw)
        # treat model-side explicit empty as zero predictions
        if preds is None:
            preds = []

        # schema sanity: drop elements that lack required keys entirely broken
        req = {"description", "direction"}
        preds = [p for p in preds if req.issubset(set(p.keys()))] if parse_ok else preds

        n_parse_ok += int(parse_ok)
        n_clean_json += int(note == "clean")

        rec = {
            "id": ex["id"],
            "category": ex["category"],
            "input": ex["input"],
            "raw": raw,
            "parse_note": note,
            "preds": preds,
            "latency_s": round(dt, 2),
        }
        (OUT_DIR / f"{ex['id']}.json").write_text(
            json.dumps(rec, ensure_ascii=False, indent=2), encoding="utf-8"
        )

        noise = ex["category"] == "noise"
        if args.hybrid:
            # symbolic direction adjudication: rules override when decisive;
            # on "unclear" the model's direction is NOT trusted (measured flip
            # rate too high) — entry surfaces as unclear for human review.
            for p in preds:
                v = assign_direction(
                    ex["input"], description_tokens(str(p.get("description") or ""))
                )
                p["direction_model"] = p["direction"]
                p["direction"] = v

        # secondary metric: usable after human review (direction may be unclear)
        r_pairs, _, _ = score_example(ex["expected"], preds, allow_unclear=True)
        if not noise:
            review_usable += len(r_pairs)
        pairs, fps, fns = score_example(ex["expected"], preds)
        if noise:
            all_fps.extend(fps)  # any prediction on noise input is an FP
        else:
            all_fps.extend(fps)
            all_fns.extend(fns)
            for pred, gold in pairs:
                tp_pairs.append((ex["id"], pred, gold))

                pc = pred.get("party_confidence")
                dc = pred.get("date_confidence")
                oc = pred.get("overall_confidence")
                conf_dist["party"].setdefault(pc, 0)
                conf_dist["party"][pc] += 1
                conf_dist["date"].setdefault(dc, 0)
                conf_dist["date"][dc] += 1
                conf_dist["overall"].setdefault(oc, 0)
                conf_dist["overall"][oc] += 1

                pm = parties_match(pred.get("party"), gold["party"])
                party_ok += int(pm)
                party_err += int(not pm)

                dm = pred.get("direction") == gold["direction"]
                dir_ok += int(dm)
                dir_err += int(not dm)

                pd_, gd = pred.get("expected_date"), gold["expected_date"]
                valid_iso = pd_ is None or bool(ISO_DATE.match(str(pd_)))
                dmatch = pd_ == gd and valid_iso
                date_ok += int(dmatch)
                date_err += int(not dmatch)
                if pd_ is not None and gd is None:
                    date_fab += 1  # fabricated a concrete date where none stated

    total_expected = sum(len(e["expected"]) for e in examples)
    precision = len(tp_pairs) / (len(tp_pairs) + len(all_fps)) if tp_pairs else 0.0
    recall = len(tp_pairs) / total_expected if total_expected else 0.0
    f1 = (
        2 * precision * recall / (precision + recall)
        if precision + recall else 0.0
    )

    def pct(x, y):
        return round(100 * x / y, 1) if y else float("nan")

    report = {
        "stage": args.tag,
        "prompt_version": PROMPT_VERSION,
        "model": "Qwen2.5-1.5B-Instruct Q4_K_M (greedy, temp=0)",
        "n_examples": len(examples),
        "n_total_expected_commitments": total_expected,
        "parse_rate_pct": pct(n_parse_ok, len(examples)),
        "clean_json_rate_pct": pct(n_clean_json, len(examples)),
        "commitment_level": {
            "tp": len(tp_pairs),
            "fp": len(all_fps),
            "fn": len(all_fns),
            "precision_pct": pct(len(tp_pairs), len(tp_pairs) + len(all_fps)),
            "recall_pct": recall * 100,
            "f1_pct": f1 * 100,
        },
        "usable_after_review": {
            "count": review_usable,
            "recall_pct": round(100 * review_usable / total_expected, 1) if total_expected else 0,
        },
        "field_accuracy_on_matched": {
            "n_matched": len(tp_pairs),
            "party_correct_pct": pct(party_ok, party_ok + party_err),
            "direction_correct_pct": pct(dir_ok, dir_ok + dir_err),
            "date_correct_pct": pct(date_ok, date_ok + date_err),
            "fabricated_dates_on_null_gold": date_fab,
        },
        "noise_rejection": {
            "n_noise": sum(1 for e in examples if e["category"] == "noise"),
            "fp_on_noise": sum(
                1 for e in examples if e["category"] == "noise"
                for _ in e["expected"]
            ),
        },
        "latency_seconds": {
            "mean": round(statistics.mean(latencies), 2) if latencies else None,
            "median": round(statistics.median(latencies), 2) if latencies else None,
            "max": round(max(latencies), 2) if latencies else None,
            "rescored_no_inference": not latencies,
        },
        "self_reported_confidence_distribution": conf_dist,
    }

    fp_by_cat = {}
    for e in examples:
        cat_fps = score_example(e["expected"], json.loads(
            (OUT_DIR / f"{e['id']}.json").read_text(encoding="utf-8"))["preds"])[1]
        if cat_fps:
            fp_by_cat.setdefault(e["category"], []).append(e["id"])
    report["false_positive_ids_by_category"] = fp_by_cat

    out = REPORT_DIR / f"{args.tag}.json"
    out.write_text(json.dumps(report, indent=2), encoding="utf-8")

    print(json.dumps(report, indent=2))
    print(f"\nSaved report -> {out}")
    print(f"Raw outputs  -> {OUT_DIR}")


if __name__ == "__main__":
    sys.exit(main())
