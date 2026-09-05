from __future__ import annotations

import json
from pathlib import Path

from conftest import FakeClock, ScriptedProvider, SyntheticCase

from autovaxx_doc_harness.errors import (
    ProviderDiskFull,
    ProviderInputRejected,
    ProviderOutOfMemory,
    ProviderTerminationFailed,
    ProviderTimeout,
    ProviderTransientError,
)
from autovaxx_doc_harness.state_machine import StateMachineRunner, TerminalState


def _runner(case: SyntheticCase, *, clock: FakeClock | None = None, seconds: float = 10.0):
    return StateMachineRunner(
        deadline_seconds=seconds,
        output_root=case.output_root,
        clock=clock,
    )


def _run(runner: StateMachineRunner, provider: ScriptedProvider, case: SyntheticCase):
    return runner.run(
        provider=provider,
        manifest_path=case.manifest_path,
        source_root=case.source_root,
    )


def _assert_no_artifacts(output_root: Path) -> None:
    runs = output_root / "runs"
    assert not (runs.exists() and any(runs.iterdir()))


def test_success_stops_at_human_review_and_writes_inert_bundle(
    synthetic_case: SyntheticCase,
) -> None:
    provider = ScriptedProvider(
        extracts=[synthetic_case.extraction_json], drafts=[synthetic_case.draft_json]
    )

    result = _run(_runner(synthetic_case), provider, synthetic_case)

    assert result.terminal_state is TerminalState.AWAITING_HUMAN_REVIEW
    assert result.artifacts_written == 2
    assert result.run_directory is not None
    assert {path.name for path in result.run_directory.iterdir()} == {
        "report.json",
        "review.patch",
    }
    patch = (result.run_directory / "review.patch").read_text()
    assert "--- a/Target1" in patch
    assert "+++ b/Target1" in patch
    assert str(synthetic_case.source_root) not in patch
    assert synthetic_case.target == (synthetic_case.source_root / "target.md").read_bytes()


def test_remaining_deadline_shrinks_across_single_transport_retry(
    synthetic_case: SyntheticCase,
) -> None:
    clock = FakeClock()

    def first_attempt(_: float) -> str:
        clock.advance(2.0)
        raise ProviderTransientError("retry")

    provider = ScriptedProvider(
        extracts=[first_attempt, synthetic_case.extraction_json],
        drafts=[synthetic_case.draft_json],
    )
    result = _run(_runner(synthetic_case, clock=clock), provider, synthetic_case)

    extraction_budgets = [budget for stage, budget in provider.timeouts if stage == "extract"]
    assert result.terminal_state is TerminalState.AWAITING_HUMAN_REVIEW
    assert result.transport_retries == 1
    assert len(extraction_budgets) == 2
    assert extraction_budgets[1] < extraction_budgets[0]


def test_second_transport_error_is_terminal_and_writes_nothing(
    synthetic_case: SyntheticCase,
) -> None:
    provider = ScriptedProvider(
        extracts=[ProviderTransientError("one"), ProviderTransientError("two")]
    )

    result = _run(_runner(synthetic_case), provider, synthetic_case)

    assert result.terminal_state is TerminalState.PROVIDER_UNAVAILABLE
    assert result.artifacts_written == 0
    assert result.transport_retries == 1
    _assert_no_artifacts(synthetic_case.output_root)


def test_provider_timeout_writes_no_artifacts(synthetic_case: SyntheticCase) -> None:
    provider = ScriptedProvider(extracts=[ProviderTimeout("cancelled")])

    result = _run(_runner(synthetic_case), provider, synthetic_case)

    assert result.terminal_state is TerminalState.TIMED_OUT
    assert result.artifacts_written == 0
    _assert_no_artifacts(synthetic_case.output_root)


def test_return_after_deadline_is_deadline_overrun_and_writes_nothing(
    synthetic_case: SyntheticCase,
) -> None:
    clock = FakeClock()

    def ignores_deadline(_: float) -> str:
        clock.advance(11.0)
        return synthetic_case.extraction_json

    provider = ScriptedProvider(extracts=[ignores_deadline])

    result = _run(_runner(synthetic_case, clock=clock), provider, synthetic_case)

    assert result.terminal_state is TerminalState.DEADLINE_OVERRUN
    assert result.artifacts_written == 0
    _assert_no_artifacts(synthetic_case.output_root)


def test_termination_failure_writes_no_artifacts(synthetic_case: SyntheticCase) -> None:
    provider = ScriptedProvider(extracts=[ProviderTerminationFailed("worker remained alive")])

    result = _run(_runner(synthetic_case), provider, synthetic_case)

    assert result.terminal_state is TerminalState.TERMINATION_FAILED
    _assert_no_artifacts(synthetic_case.output_root)


def test_oom_is_not_retried_and_writes_no_artifacts(synthetic_case: SyntheticCase) -> None:
    provider = ScriptedProvider(extracts=[ProviderOutOfMemory("synthetic OOM")])

    result = _run(_runner(synthetic_case), provider, synthetic_case)

    assert result.terminal_state is TerminalState.RESOURCE_EXHAUSTED
    assert len(provider.timeouts) == 1
    _assert_no_artifacts(synthetic_case.output_root)


def test_disk_full_is_not_retried_and_writes_no_artifacts(
    synthetic_case: SyntheticCase,
) -> None:
    provider = ScriptedProvider(extracts=[ProviderDiskFull("synthetic disk full")])

    result = _run(_runner(synthetic_case), provider, synthetic_case)

    assert result.terminal_state is TerminalState.RESOURCE_EXHAUSTED
    assert len(provider.timeouts) == 1
    _assert_no_artifacts(synthetic_case.output_root)


def test_provider_input_rejection_is_not_retried_and_writes_no_artifacts(
    synthetic_case: SyntheticCase,
) -> None:
    provider = ScriptedProvider(extracts=[ProviderInputRejected("synthetic oversized input")])

    result = _run(_runner(synthetic_case), provider, synthetic_case)

    assert result.terminal_state is TerminalState.INPUT_REJECTED
    assert len(provider.timeouts) == 1
    _assert_no_artifacts(synthetic_case.output_root)


def test_one_malformed_envelope_can_be_repaired(synthetic_case: SyntheticCase) -> None:
    provider = ScriptedProvider(
        extracts=["not-json"],
        repairs=[synthetic_case.extraction_json],
        drafts=[synthetic_case.draft_json],
    )

    result = _run(_runner(synthetic_case), provider, synthetic_case)

    assert result.terminal_state is TerminalState.AWAITING_HUMAN_REVIEW
    assert result.structural_repairs == 1


def test_malformed_repair_is_terminal_with_diagnostics_only(
    synthetic_case: SyntheticCase,
) -> None:
    provider = ScriptedProvider(extracts=["{"], repairs=["]"])

    result = _run(_runner(synthetic_case), provider, synthetic_case)

    assert result.terminal_state is TerminalState.SCHEMA_INVALID
    assert result.artifacts_written == 1
    assert result.structural_repairs == 1
    assert result.run_directory is not None
    assert [path.name for path in result.run_directory.iterdir()] == ["report.json"]
    report = json.loads((result.run_directory / "report.json").read_text())
    assert report["terminal_state"] == "SCHEMA_INVALID"
    assert "provider_output" not in report
