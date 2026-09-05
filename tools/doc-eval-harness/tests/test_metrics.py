from __future__ import annotations

import pytest

from autovaxx_doc_harness.metrics import (
    CategoryCounts,
    calculate_injection_metrics,
    calculate_recall,
)


def test_micro_and_macro_recall_have_distinct_denominators() -> None:
    metrics = calculate_recall(
        [
            CategoryCounts("clinical_contradiction", 8, 2),
            CategoryCounts("missing_citation", 1, 1),
            CategoryCounts("empty_class", 0, 0),
        ]
    )

    assert metrics.micro_recall == pytest.approx(9 / 12)
    assert metrics.macro_recall == pytest.approx((0.8 + 0.5) / 2)
    assert metrics.included_categories == ("clinical_contradiction", "missing_citation")
    assert metrics.empty_categories == ("empty_class",)


def test_all_empty_recall_denominators_are_reported_as_none() -> None:
    metrics = calculate_recall([CategoryCounts("empty", 0, 0)])

    assert metrics.micro_recall is None
    assert metrics.macro_recall is None
    assert metrics.empty_categories == ("empty",)


def test_injection_containment_is_separate_from_task_robustness() -> None:
    metrics = calculate_injection_metrics(
        malicious_fixtures=10,
        contained_fixtures=10,
        robust_fixtures=7,
    )

    assert metrics.containment_rate == 1.0
    assert metrics.task_robustness == 0.7
