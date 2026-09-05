from __future__ import annotations

import json
import os

import pytest
from conftest import SyntheticCase

from autovaxx_doc_harness.errors import ManifestError
from autovaxx_doc_harness.manifest import load_manifest
from autovaxx_doc_harness.models import BudgetPolicy


@pytest.mark.parametrize(
    "bad_path",
    [
        "/etc/passwd",
        "../evidence.md",
        "./evidence.md",
        "folder/./evidence.md",
        "folder//evidence.md",
        "folder\\evidence.md",
        "C:/evidence.md",
        "evidence.md\nInjected",
        "evidence.md\u2028Injected",
    ],
)
def test_manifest_rejects_unsafe_paths(synthetic_case: SyntheticCase, bad_path: str) -> None:
    payload = json.loads(synthetic_case.manifest_path.read_text())
    payload["sources"][0]["relative_path"] = bad_path
    synthetic_case.manifest_path.write_text(json.dumps(payload))

    with pytest.raises(ManifestError):
        load_manifest(synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy())


def test_manifest_rejects_duplicate_source_ids(synthetic_case: SyntheticCase) -> None:
    payload = json.loads(synthetic_case.manifest_path.read_text())
    payload["sources"][1]["source_id"] = "Evidence1"
    synthetic_case.manifest_path.write_text(json.dumps(payload))

    with pytest.raises(ManifestError):
        load_manifest(synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy())


def test_manifest_rejects_python_as_editable(synthetic_case: SyntheticCase) -> None:
    python_file = synthetic_case.source_root / "target.py"
    python_file.write_text("print('synthetic')\n")
    payload = json.loads(synthetic_case.manifest_path.read_text())
    payload["sources"][1]["relative_path"] = "target.py"
    synthetic_case.manifest_path.write_text(json.dumps(payload))

    with pytest.raises(ManifestError):
        load_manifest(synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy())


def test_manifest_rejects_symlink_source(synthetic_case: SyntheticCase) -> None:
    link = synthetic_case.source_root / "linked.md"
    os.symlink(synthetic_case.source_root / "evidence.md", link)
    payload = json.loads(synthetic_case.manifest_path.read_text())
    payload["sources"][0]["relative_path"] = "linked.md"
    synthetic_case.manifest_path.write_text(json.dumps(payload))

    with pytest.raises(ManifestError):
        load_manifest(synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy())
