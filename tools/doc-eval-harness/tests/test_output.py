from __future__ import annotations

import os
import stat
import uuid
from pathlib import Path

import pytest
from conftest import SyntheticCase

from autovaxx_doc_harness import output as output_module
from autovaxx_doc_harness.errors import ManifestError, OutputSecurityError
from autovaxx_doc_harness.output import SecureOutputRoot


def test_output_root_inside_source_is_rejected(synthetic_case: SyntheticCase) -> None:
    unsafe = synthetic_case.source_root / "output"
    unsafe.mkdir()

    with pytest.raises(OutputSecurityError):
        SecureOutputRoot(unsafe, synthetic_case.source_root)


def test_output_symlink_is_rejected(synthetic_case: SyntheticCase, tmp_path: Path) -> None:
    link = tmp_path / "output-link"
    os.symlink(synthetic_case.output_root, link)

    with pytest.raises((ManifestError, OutputSecurityError)):
        SecureOutputRoot(link, synthetic_case.source_root)


@pytest.mark.parametrize("mode", [0o500, 0o755])
def test_non_0700_output_root_is_rejected(synthetic_case: SyntheticCase, mode: int) -> None:
    synthetic_case.output_root.chmod(mode)

    with pytest.raises(OutputSecurityError, match="owner-controlled"):
        SecureOutputRoot(synthetic_case.output_root, synthetic_case.source_root)


def test_non_0700_runs_directory_is_rejected(synthetic_case: SyntheticCase) -> None:
    runs = synthetic_case.output_root / "runs"
    runs.mkdir(mode=0o500)

    with pytest.raises(OutputSecurityError, match="runs directory"):
        SecureOutputRoot(synthetic_case.output_root, synthetic_case.source_root)


def test_secure_publication_uses_restricted_permissions(
    synthetic_case: SyntheticCase,
) -> None:
    with SecureOutputRoot(synthetic_case.output_root, synthetic_case.source_root) as output:
        run_id, run_directory = output.publish({"report.json": b"{}\n"})

    assert run_directory.name == run_id
    assert stat.S_IMODE(run_directory.stat().st_mode) == 0o700
    assert stat.S_IMODE((run_directory / "report.json").stat().st_mode) == 0o600
    assert not any(path.name.endswith(".tmp") for path in run_directory.iterdir())


def test_existing_run_id_is_never_overwritten(
    synthetic_case: SyntheticCase, monkeypatch: pytest.MonkeyPatch
) -> None:
    fixed_id = uuid.UUID("00000000-0000-4000-8000-000000000001")
    with SecureOutputRoot(synthetic_case.output_root, synthetic_case.source_root) as output:
        occupied = synthetic_case.output_root / "runs" / str(fixed_id)
        occupied.mkdir(mode=0o700)
        marker = occupied / "marker"
        marker.write_text("preserve")
        monkeypatch.setattr(output_module.uuid, "uuid4", lambda: fixed_id)

        with pytest.raises(OutputSecurityError, match="exclusive run"):
            output.publish({"report.json": b"{}\n"})

    assert marker.read_text() == "preserve"
