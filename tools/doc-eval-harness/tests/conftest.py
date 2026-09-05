from __future__ import annotations

import hashlib
import json
from collections import deque
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import pytest

from autovaxx_doc_harness.errors import ProviderTransientError


class FakeClock:
    def __init__(self) -> None:
        self.now_ns = 1_000_000_000

    def monotonic_ns(self) -> int:
        return self.now_ns

    def advance(self, seconds: float) -> None:
        self.now_ns += int(seconds * 1_000_000_000)


Response = str | BaseException | Callable[[float], str]


class ScriptedProvider:
    def __init__(
        self,
        *,
        extracts: list[Response],
        repairs: list[Response] | None = None,
        drafts: list[Response] | None = None,
    ) -> None:
        self.extracts = deque(extracts)
        self.repairs = deque(repairs or [])
        self.drafts = deque(drafts or [])
        self.timeouts: list[tuple[str, float]] = []

    def extract(self, sources: dict[str, bytes], timeout_seconds: float) -> str:
        assert set(sources) == {"Evidence1"}
        return self._take("extract", self.extracts, timeout_seconds)

    def repair(self, stage: str, invalid_json: str, timeout_seconds: float) -> str:
        assert stage in {"EXTRACT", "DRAFT"}
        assert invalid_json
        return self._take("repair", self.repairs, timeout_seconds)

    def draft(
        self,
        findings: dict[str, Any],
        target_source_id: str,
        target: bytes,
        timeout_seconds: float,
    ) -> str:
        assert set(findings) == {"Finding1"}
        assert target_source_id == "Target1"
        assert target
        return self._take("draft", self.drafts, timeout_seconds)

    def _take(self, stage: str, responses: deque[Response], timeout_seconds: float) -> str:
        self.timeouts.append((stage, timeout_seconds))
        if not responses:
            raise AssertionError(f"no scripted response for {stage}")
        response = responses.popleft()
        if isinstance(response, BaseException):
            raise response
        if callable(response):
            return response(timeout_seconds)
        return response


@dataclass(frozen=True)
class SyntheticCase:
    case_root: Path
    source_root: Path
    manifest_path: Path
    output_root: Path
    evidence: bytes
    target: bytes
    extraction_json: str
    draft_json: str


@pytest.fixture
def synthetic_case(tmp_path: Path) -> SyntheticCase:
    case_root = tmp_path / "case"
    source_root = case_root / "sources"
    output_root = tmp_path / "external-output"
    source_root.mkdir(parents=True)
    output_root.mkdir(mode=0o700)

    evidence = "La vacuna está disponible en la clínica.\n".encode()
    target_text = (
        "# Resumen\n\n"
        "Este archivo sintético contiene contexto suficiente para mantener pequeño el porcentaje "
        "de eliminación.\n\n"
        "Texto anterior.\n\n"
        "Fin del documento sintético.\n"
    )
    target = target_text.encode()
    (source_root / "evidence.md").write_bytes(evidence)
    (source_root / "target.md").write_bytes(target)

    manifest = {
        "schema_version": "1",
        "case_id": "Case1",
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
    manifest_path = case_root / "manifest.json"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

    quote = "La vacuna está disponible en la clínica."
    quote_bytes = quote.encode()
    finding_start = evidence.index(quote_bytes)
    finding_end = finding_start + len(quote_bytes)
    extraction = {
        "findings": [
            {
                "finding_id": "Finding1",
                "source_id": "Evidence1",
                "source_sha256": hashlib.sha256(evidence).hexdigest(),
                "start_offset": finding_start,
                "end_offset": finding_end,
                "exact_quote": quote,
            }
        ]
    }

    old = b"Texto anterior."
    replacement = quote
    replacement_bytes = replacement.encode()
    replacement_start = target.index(old)
    replacement_end = replacement_start + len(old)
    draft = {
        "proposals": [
            {
                "target_source_id": "Target1",
                "target_sha256": hashlib.sha256(target).hexdigest(),
                "replacement_start": replacement_start,
                "replacement_end": replacement_end,
                "replacement_text": replacement,
                "claims": [
                    {
                        "claim_id": "Claim1",
                        "claim_text": replacement,
                        "start_offset": 0,
                        "end_offset": len(replacement_bytes),
                        "supporting_finding_ids": ["Finding1"],
                    }
                ],
            }
        ]
    }
    return SyntheticCase(
        case_root,
        source_root,
        manifest_path,
        output_root,
        evidence,
        target,
        json.dumps(extraction),
        json.dumps(draft),
    )


@pytest.fixture
def transient_error() -> ProviderTransientError:
    return ProviderTransientError("synthetic transport failure")
