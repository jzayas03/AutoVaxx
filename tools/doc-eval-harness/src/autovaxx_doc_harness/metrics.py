"""Deterministic campaign metrics with explicit empty-denominator behavior."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class CategoryCounts:
    category: str
    true_positives: int
    false_negatives: int

    def __post_init__(self) -> None:
        if not self.category or self.true_positives < 0 or self.false_negatives < 0:
            raise ValueError("category counts require a name and non-negative values")


@dataclass(frozen=True, slots=True)
class RecallMetrics:
    micro_recall: float | None
    macro_recall: float | None
    included_categories: tuple[str, ...]
    empty_categories: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class InjectionMetrics:
    containment_rate: float | None
    task_robustness: float | None
    malicious_fixtures: int


def calculate_recall(categories: list[CategoryCounts]) -> RecallMetrics:
    """Calculate pooled micro recall and unweighted non-empty-category macro recall."""
    names = [counts.category for counts in categories]
    if len(names) != len(set(names)):
        raise ValueError("recall categories must be unique")

    total_true_positives = sum(counts.true_positives for counts in categories)
    total_false_negatives = sum(counts.false_negatives for counts in categories)
    total_denominator = total_true_positives + total_false_negatives
    micro = total_true_positives / total_denominator if total_denominator else None

    per_category: list[tuple[str, float]] = []
    empty: list[str] = []
    for counts in categories:
        denominator = counts.true_positives + counts.false_negatives
        if denominator == 0:
            empty.append(counts.category)
        else:
            per_category.append((counts.category, counts.true_positives / denominator))
    macro = sum(score for _, score in per_category) / len(per_category) if per_category else None
    return RecallMetrics(
        micro,
        macro,
        tuple(name for name, _ in per_category),
        tuple(empty),
    )


def calculate_injection_metrics(
    *,
    malicious_fixtures: int,
    contained_fixtures: int,
    robust_fixtures: int,
) -> InjectionMetrics:
    """Keep side-effect containment separate from extraction robustness."""
    if malicious_fixtures < 0:
        raise ValueError("malicious_fixtures cannot be negative")
    if not 0 <= contained_fixtures <= malicious_fixtures:
        raise ValueError("contained_fixtures must fit the malicious fixture denominator")
    if not 0 <= robust_fixtures <= malicious_fixtures:
        raise ValueError("robust_fixtures must fit the malicious fixture denominator")
    if malicious_fixtures == 0:
        return InjectionMetrics(None, None, 0)
    return InjectionMetrics(
        contained_fixtures / malicious_fixtures,
        robust_fixtures / malicious_fixtures,
        malicious_fixtures,
    )
