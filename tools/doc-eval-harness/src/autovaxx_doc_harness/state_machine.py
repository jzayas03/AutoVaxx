"""Explicit synthetic documentation-agent state machine."""

from __future__ import annotations

import json
import time
from collections.abc import Callable
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Protocol, TypeVar

from pydantic import BaseModel, ValidationError

from .diffing import build_validated_patch
from .errors import (
    ClaimValidationError,
    EditValidationError,
    ManifestError,
    OutputSecurityError,
    ProvenanceError,
    ProviderDeadlineOverrun,
    ProviderDiskFull,
    ProviderInputRejected,
    ProviderOutOfMemory,
    ProviderTerminationFailed,
    ProviderTimeout,
    ProviderTransientError,
)
from .manifest import SourceCatalog, load_manifest
from .models import BudgetPolicy, DraftEnvelope, ExtractionEnvelope
from .output import SecureOutputRoot
from .provenance import build_verified_finding_index
from .provider import DocumentationProvider


class MachineState(StrEnum):
    LOAD_MANIFEST = "LOAD_MANIFEST"
    EXTRACT = "EXTRACT"
    PARSE_SCHEMA = "PARSE_SCHEMA"
    REPAIR_SCHEMA = "REPAIR_SCHEMA"
    VERIFY_PROVENANCE = "VERIFY_PROVENANCE"
    DRAFT = "DRAFT"
    VALIDATE_DOCUMENT = "VALIDATE_DOCUMENT"
    PACKAGE_REVIEW_ARTIFACT = "PACKAGE_REVIEW_ARTIFACT"
    TERMINAL = "TERMINAL"


class TerminalState(StrEnum):
    AWAITING_HUMAN_REVIEW = "AWAITING_HUMAN_REVIEW"
    TIMED_OUT = "TIMED_OUT"
    DEADLINE_OVERRUN = "DEADLINE_OVERRUN"
    TERMINATION_FAILED = "TERMINATION_FAILED"
    PROVIDER_UNAVAILABLE = "PROVIDER_UNAVAILABLE"
    RESOURCE_EXHAUSTED = "RESOURCE_EXHAUSTED"
    INPUT_REJECTED = "INPUT_REJECTED"
    SCHEMA_INVALID = "SCHEMA_INVALID"
    EVIDENCE_INVALID = "EVIDENCE_INVALID"
    PROPOSAL_INVALID = "PROPOSAL_INVALID"
    MANIFEST_INVALID = "MANIFEST_INVALID"
    OUTPUT_FAILED = "OUTPUT_FAILED"


class Clock(Protocol):
    def monotonic_ns(self) -> int: ...


class SystemClock:
    def monotonic_ns(self) -> int:
        return time.monotonic_ns()


@dataclass(frozen=True, slots=True)
class RunResult:
    terminal_state: TerminalState
    artifacts_written: int
    run_id: str | None
    run_directory: Path | None
    transport_retries: int
    structural_repairs: int
    state_trace: tuple[MachineState, ...]


@dataclass(slots=True)
class _Counters:
    transport_retries: int = 0
    structural_repairs: int = 0


class _Deadline:
    def __init__(self, clock: Clock, seconds: float):
        if seconds <= 0:
            raise ValueError("deadline_seconds must be positive")
        self.clock = clock
        self.absolute_ns = clock.monotonic_ns() + int(seconds * 1_000_000_000)

    def remaining(self) -> float:
        return max(0.0, (self.absolute_ns - self.clock.monotonic_ns()) / 1_000_000_000)

    def expired(self) -> bool:
        return self.clock.monotonic_ns() >= self.absolute_ns


Envelope = TypeVar("Envelope", bound=BaseModel)


class StateMachineRunner:
    """Run one bounded synthetic case and stop at human review."""

    def __init__(
        self,
        *,
        deadline_seconds: float,
        output_root: Path,
        policy: BudgetPolicy | None = None,
        clock: Clock | None = None,
    ):
        self.deadline_seconds = deadline_seconds
        self.output_root = output_root
        self.policy = policy or BudgetPolicy()
        self.clock = clock or SystemClock()

    def run(
        self,
        *,
        provider: DocumentationProvider,
        manifest_path: Path,
        source_root: Path,
    ) -> RunResult:
        deadline = _Deadline(self.clock, self.deadline_seconds)
        trace: list[MachineState] = [MachineState.LOAD_MANIFEST]
        counters = _Counters()
        catalog: SourceCatalog | None = None

        try:
            catalog = load_manifest(manifest_path, source_root, self.policy)
        except ManifestError:
            return self._result(TerminalState.MANIFEST_INVALID, trace, counters)

        try:
            trace.append(MachineState.EXTRACT)
            raw_extraction = self._call_with_transport_retry(
                lambda remaining: provider.extract(
                    {source.declaration.source_id: source.raw for source in catalog.evidence},
                    remaining,
                ),
                deadline,
                counters,
            )
            trace.append(MachineState.PARSE_SCHEMA)
            extraction = self._parse_with_one_repair(
                ExtractionEnvelope,
                raw_extraction,
                "EXTRACT",
                provider,
                deadline,
                trace,
                counters,
            )

            trace.append(MachineState.VERIFY_PROVENANCE)
            verified = build_verified_finding_index(extraction, catalog)

            trace.append(MachineState.DRAFT)
            raw_draft = self._call_once(
                lambda remaining: provider.draft(
                    verified,
                    catalog.target.declaration.source_id,
                    catalog.target.raw,
                    remaining,
                ),
                deadline,
            )
            trace.append(MachineState.PARSE_SCHEMA)
            draft = self._parse_with_one_repair(
                DraftEnvelope,
                raw_draft,
                "DRAFT",
                provider,
                deadline,
                trace,
                counters,
                repair_allowed=counters.structural_repairs == 0,
            )

            trace.append(MachineState.VALIDATE_DOCUMENT)
            patch = build_validated_patch(draft.proposals[0], catalog, verified, self.policy)
            trace.append(MachineState.PACKAGE_REVIEW_ARTIFACT)
            report = self._report(
                terminal=TerminalState.AWAITING_HUMAN_REVIEW,
                catalog=catalog,
                retries=counters.transport_retries,
                repairs=counters.structural_repairs,
                trace=trace,
                patch_metrics={
                    "removed_bytes": patch.removed_bytes,
                    "inserted_bytes": patch.inserted_bytes,
                    "changed_lines": patch.changed_lines,
                },
            )
            with SecureOutputRoot(self.output_root, source_root) as output:
                run_id, run_directory = output.publish(
                    {
                        "review.patch": patch.unified_diff.encode("utf-8"),
                        "report.json": report,
                    }
                )
            trace.append(MachineState.TERMINAL)
            return RunResult(
                TerminalState.AWAITING_HUMAN_REVIEW,
                2,
                run_id,
                run_directory,
                counters.transport_retries,
                counters.structural_repairs,
                tuple(trace),
            )
        except ProviderTimeout:
            return self._result(TerminalState.TIMED_OUT, trace, counters)
        except ProviderDeadlineOverrun:
            return self._result(TerminalState.DEADLINE_OVERRUN, trace, counters)
        except ProviderTerminationFailed:
            return self._result(TerminalState.TERMINATION_FAILED, trace, counters)
        except ProviderTransientError:
            return self._result(TerminalState.PROVIDER_UNAVAILABLE, trace, counters)
        except (ProviderOutOfMemory, ProviderDiskFull):
            return self._result(TerminalState.RESOURCE_EXHAUSTED, trace, counters)
        except ProviderInputRejected:
            return self._result(TerminalState.INPUT_REJECTED, trace, counters)
        except ValidationError:
            return self._diagnostic_result(
                TerminalState.SCHEMA_INVALID, catalog, source_root, trace, counters
            )
        except ProvenanceError:
            return self._diagnostic_result(
                TerminalState.EVIDENCE_INVALID, catalog, source_root, trace, counters
            )
        except (ClaimValidationError, EditValidationError):
            return self._diagnostic_result(
                TerminalState.PROPOSAL_INVALID, catalog, source_root, trace, counters
            )
        except OutputSecurityError:
            return self._result(TerminalState.OUTPUT_FAILED, trace, counters)

    def _call_with_transport_retry(
        self,
        operation: Callable[[float], str],
        deadline: _Deadline,
        counters: _Counters,
    ) -> str:
        while True:
            try:
                return self._call_once(operation, deadline)
            except ProviderTransientError:
                if counters.transport_retries >= 1 or deadline.expired():
                    raise
                counters.transport_retries += 1

    def _call_once(self, operation: Callable[[float], str], deadline: _Deadline) -> str:
        remaining = deadline.remaining()
        if remaining <= 0:
            raise ProviderTimeout("absolute deadline expired before provider call")
        response = operation(remaining)
        if deadline.expired():
            raise ProviderDeadlineOverrun("provider returned after the absolute deadline")
        return response

    def _parse_with_one_repair(
        self,
        schema: type[Envelope],
        raw: str,
        stage: str,
        provider: DocumentationProvider,
        deadline: _Deadline,
        trace: list[MachineState],
        counters: _Counters,
        *,
        repair_allowed: bool = True,
    ) -> Envelope:
        try:
            return schema.model_validate_json(raw)
        except ValidationError:
            if not repair_allowed:
                raise
            trace.append(MachineState.REPAIR_SCHEMA)
            counters.structural_repairs += 1
            repaired = self._call_once(
                lambda remaining: provider.repair(stage, raw, remaining), deadline
            )
            return schema.model_validate_json(repaired)

    def _diagnostic_result(
        self,
        terminal: TerminalState,
        catalog: SourceCatalog,
        source_root: Path,
        trace: list[MachineState],
        counters: _Counters,
    ) -> RunResult:
        report = self._report(
            terminal,
            catalog,
            counters.transport_retries,
            counters.structural_repairs,
            trace,
        )
        try:
            with SecureOutputRoot(self.output_root, source_root) as output:
                run_id, run_directory = output.publish({"report.json": report})
        except OutputSecurityError:
            return self._result(TerminalState.OUTPUT_FAILED, trace, counters)
        trace.append(MachineState.TERMINAL)
        return RunResult(
            terminal,
            1,
            run_id,
            run_directory,
            counters.transport_retries,
            counters.structural_repairs,
            tuple(trace),
        )

    @staticmethod
    def _report(
        terminal: TerminalState,
        catalog: SourceCatalog,
        retries: int,
        repairs: int,
        trace: list[MachineState],
        patch_metrics: dict[str, int] | None = None,
    ) -> bytes:
        payload: dict[str, object] = {
            "schema_version": "1",
            "case_id": catalog.manifest.case_id,
            "terminal_state": terminal,
            "transport_retries": retries,
            "structural_repairs": repairs,
            "state_trace": [state.value for state in trace],
            "source_digests": {
                source.source_id: catalog.get(source.source_id).sha256
                for source in catalog.manifest.sources
            },
        }
        if patch_metrics is not None:
            payload["patch_metrics"] = patch_metrics
        return (json.dumps(payload, sort_keys=True, separators=(",", ":")) + "\n").encode()

    @staticmethod
    def _result(
        terminal: TerminalState,
        trace: list[MachineState],
        counters: _Counters,
    ) -> RunResult:
        trace.append(MachineState.TERMINAL)
        return RunResult(
            terminal,
            0,
            None,
            None,
            counters.transport_retries,
            counters.structural_repairs,
            tuple(trace),
        )
