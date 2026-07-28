import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from risk_policy import canonical_category, postprocess


class RiskPolicyTests(unittest.TestCase):
    def test_evidence_overrides_wrong_free_form_code(self):
        row = {
            "category_code": "UNILATERAL_CHANGE",
            "risk_type": "其他",
            "source_quote": "仅限本市类似项目业绩，外地案例不予认可。",
        }
        self.assertEqual(canonical_category(row), "REGIONAL_PERFORMANCE")

    def test_deduplicates_aliases_and_recomputes_critical(self):
        base = {
            "document_id": "D1",
            "source_quote": "供应商须在本市注册，外地企业不接受投标。",
            "severity": "high",
            "no_risk": False,
            "confidence": 0.8,
        }
        rows = [
            {**base, "category_code": "LOCAL_REGISTRATION", "is_critical": False},
            {**base, "category_code": "REGIONAL_RESTRICTION", "confidence": 0.9},
        ]
        output, stats = postprocess(rows)
        self.assertEqual(len(output), 1)
        self.assertTrue(output[0]["is_critical"])
        self.assertEqual(stats["deduplicated"], 1)

    def test_rejects_empty_evidence(self):
        output, stats = postprocess([{
            "document_id": "D1",
            "category_code": "OTHER",
            "source_quote": "",
            "severity": "info",
            "no_risk": False,
        }])
        self.assertEqual(output, [])
        self.assertEqual(stats["rejected_evidence"], 1)

    def test_date_and_time_aliases_are_canonical(self):
        self.assertEqual(
            canonical_category({
                "category_code": "DATE_CONFLICT",
                "source_quote": "投标截止时间为[日期]9时，同时规定[日期]17时后提交的文件一律拒收。",
            }),
            "CONFLICTING_DATES",
        )
        self.assertEqual(
            canonical_category({
                "category_code": "TIME_LIMIT",
                "source_quote": "供应商须在获取本条款后10日内递交投标文件，该期限不作顺延。",
            }),
            "SHORT_DEADLINE",
        )


if __name__ == "__main__":
    unittest.main()
