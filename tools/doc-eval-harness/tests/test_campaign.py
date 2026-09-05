from __future__ import annotations

import json
from pathlib import Path

import pytest

from autovaxx_doc_harness.campaign import _load_cases


def test_committed_campaign_has_independent_minimum_corpus() -> None:
    cases_root = Path(__file__).parents[1] / "evaluation" / "cases"

    cases = _load_cases(cases_root)

    assert len(cases) >= 25
    assert sum(truth.malicious for _, truth in cases) >= 5
    assert len({truth.category for _, truth in cases}) >= 10


def test_campaign_rejects_renamed_duplicate_evidence(tmp_path: Path) -> None:
    fixtures_root = tmp_path / "cases"
    fixtures_root.mkdir()
    for directory_name, case_id in (("first", "CaseOne"), ("second", "CaseTwo")):
        _write_case(fixtures_root, directory_name, case_id, "A fictional fact.")

    with pytest.raises(ValueError, match="independent evidence corpora"):
        _load_cases(fixtures_root)


def test_campaign_rejects_duplicate_case_ids(tmp_path: Path) -> None:
    fixtures_root = tmp_path / "cases"
    fixtures_root.mkdir()
    _write_case(fixtures_root, "first", "RepeatedCase", "The first fictional fact.")
    _write_case(fixtures_root, "second", "RepeatedCase", "The second fictional fact.")

    with pytest.raises(ValueError, match="case_id values must be unique"):
        _load_cases(fixtures_root)


def test_campaign_rejects_target_as_ground_truth_evidence(tmp_path: Path) -> None:
    fixtures_root = tmp_path / "cases"
    fixtures_root.mkdir()
    _write_case(
        fixtures_root,
        "case",
        "TargetFinding",
        "A fictional fact.",
        expected_source_id="Target1",
        expected_quote="Human review required.",
    )

    with pytest.raises(ValueError, match="must reference evidence sources"):
        _load_cases(fixtures_root)


def test_campaign_rejects_duplicate_expected_findings(tmp_path: Path) -> None:
    fixtures_root = tmp_path / "cases"
    fixtures_root.mkdir()
    case_root = _write_case(fixtures_root, "case", "DuplicateTruth", "A fictional fact.")
    truth_path = case_root / "ground_truth.json"
    truth = json.loads(truth_path.read_text(encoding="utf-8"))
    truth["expected_findings"].append(dict(truth["expected_findings"][0]))
    truth_path.write_text(json.dumps(truth), encoding="utf-8")

    with pytest.raises(ValueError, match="expected findings must be unique"):
        _load_cases(fixtures_root)


def _write_case(
    fixtures_root: Path,
    directory_name: str,
    case_id: str,
    evidence_quote: str,
    *,
    expected_source_id: str = "Evidence1",
    expected_quote: str | None = None,
) -> Path:
    case_root = fixtures_root / directory_name
    case_root.mkdir()
    (case_root / "evidence.md").write_text(f"{evidence_quote}\n", encoding="utf-8")
    (case_root / "target.md").write_text(
        "# Synthetic target\n\nContext long enough for the edit budget.\n\n"
        "[[PROPOSED_SUMMARY]]\n\nHuman review required.\n",
        encoding="utf-8",
    )
    manifest = {
        "schema_version": "1",
        "case_id": case_id,
        "target_source_id": "Target1",
        "sources": [
            {
                "source_id": "Evidence1",
                "relative_path": "evidence.md",
                "purpose": "evidence",
            },
            {
                "source_id": "Target1",
                "relative_path": "target.md",
                "purpose": "editable",
            },
        ],
    }
    truth = {
        "schema_version": "1",
        "category": "duplicate_check",
        "malicious": False,
        "expected_findings": [
            {
                "source_id": expected_source_id,
                "exact_quote": expected_quote or evidence_quote,
            }
        ],
    }
    (case_root / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
    (case_root / "ground_truth.json").write_text(json.dumps(truth), encoding="utf-8")
    return case_root
