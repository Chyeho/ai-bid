from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent
GOLD = ROOT / "data" / "annotations.jsonl"
EVALUATOR = ROOT / "evaluate.py"


def load_gold(splits: set[str]) -> list[dict]:
    rows = []
    with GOLD.open("r", encoding="utf-8") as f:
        for line in f:
            row = json.loads(line)
            if row["split"] in splits:
                rows.append(row)
    return rows


def run_evaluator(predictions: list[dict]) -> dict:
    with tempfile.TemporaryDirectory() as temp_dir:
        pred_path = Path(temp_dir) / "predictions.jsonl"
        with pred_path.open("w", encoding="utf-8") as f:
            for row in predictions:
                f.write(json.dumps(row, ensure_ascii=False) + "\n")
        result = subprocess.run(
            [
                sys.executable,
                str(EVALUATOR),
                "--gold",
                str(GOLD),
                "--pred",
                str(pred_path),
                "--include-splits",
                "dev,test",
            ],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        return json.loads(result.stdout)


class EvaluateTests(unittest.TestCase):
    def test_perfect_predictions_pass_gate(self) -> None:
        predictions = []
        for row in load_gold({"dev", "test"}):
            predictions.append({
                "document_id": row["document_id"],
                "risk_id": "P-" + row["finding_id"],
                "severity": row["severity"],
                "is_critical": row["is_critical"],
                "risk_type": row["risk_type"],
                "category_code": row["category_code"].split("_", 1)[-1],
                "source_quote": row["source_quote"],
                "page_number": row["page_number"],
            })
        result = run_evaluator(predictions)
        self.assertEqual(result["overall"]["f1"], 1.0)
        self.assertEqual(result["critical"]["recall"], 1.0)
        self.assertTrue(result["release_gate"]["passed"])

    def test_critical_detection_without_flag_fails_critical_gate(self) -> None:
        predictions = []
        for row in load_gold({"dev", "test"}):
            predictions.append({
                "document_id": row["document_id"],
                "risk_id": "P-" + row["finding_id"],
                "severity": row["severity"],
                "is_critical": False,
                "risk_type": row["risk_type"],
                "source_quote": row["source_quote"],
                "page_number": row["page_number"],
            })
        result = run_evaluator(predictions)
        self.assertEqual(result["critical"]["detection_recall"], 1.0)
        self.assertEqual(result["critical"]["recall"], 0.0)
        self.assertFalse(result["release_gate"]["passed"])

    def test_empty_predictions_fail_gate(self) -> None:
        result = run_evaluator([])
        self.assertEqual(result["overall"]["recall"], 0.0)
        self.assertEqual(result["critical"]["recall"], 0.0)
        self.assertFalse(result["release_gate"]["passed"])

    def test_prediction_on_original_page_is_out_of_scope(self) -> None:
        row = load_gold({"dev", "test"})[0]
        prediction = {
            "document_id": row["document_id"],
            "risk_id": "P-OUTSIDE",
            "severity": row["severity"],
            "risk_type": row["risk_type"],
            "source_quote": "原文第一页的另一项普通描述，与注入条款无关。",
            "page_number": 1,
        }
        result = run_evaluator([prediction])
        self.assertEqual(len(result["out_of_scope_predictions"]), 1)
        self.assertEqual(result["overall"]["fp"], 0)


if __name__ == "__main__":
    unittest.main()
