"""Descriptor-relative, fail-closed review artifact publication."""

from __future__ import annotations

import os
import stat
import uuid
from collections.abc import Mapping
from contextlib import suppress
from pathlib import Path

from .errors import ManifestError, OutputSecurityError
from .manifest import _reject_symlink_components

_ALLOWED_ARTIFACTS = frozenset({"review.patch", "report.json"})
_MAX_ARTIFACT_BYTES = 4_194_304


class SecureOutputRoot:
    """Publish complete artifact bundles below an external, trusted directory."""

    def __init__(self, output_root: Path, source_root: Path):
        try:
            _reject_symlink_components(output_root)
            _reject_symlink_components(source_root)
        except ManifestError as exc:
            raise OutputSecurityError("trusted output paths cannot contain symlinks") from exc
        resolved_output = output_root.resolve(strict=True)
        resolved_source = source_root.resolve(strict=True)
        if (
            resolved_output == resolved_source
            or resolved_output.is_relative_to(resolved_source)
            or resolved_source.is_relative_to(resolved_output)
        ):
            raise OutputSecurityError("output root must be isolated from the source repository")
        self.path = resolved_output
        self._root_fd = self._open_root(resolved_output)
        try:
            self._runs_fd = self._open_or_create_runs(self._root_fd)
        except OutputSecurityError:
            os.close(self._root_fd)
            self._root_fd = -1
            raise

    def close(self) -> None:
        if getattr(self, "_runs_fd", -1) >= 0:
            os.close(self._runs_fd)
            self._runs_fd = -1
        if getattr(self, "_root_fd", -1) >= 0:
            os.close(self._root_fd)
            self._root_fd = -1

    def __enter__(self) -> SecureOutputRoot:
        return self

    def __exit__(self, exc_type: object, exc: object, traceback: object) -> None:
        self.close()

    def publish(self, artifacts: Mapping[str, bytes]) -> tuple[str, Path]:
        if not artifacts or set(artifacts) - _ALLOWED_ARTIFACTS:
            raise OutputSecurityError("artifact bundle contains an unapproved filename")
        if any(len(content) > _MAX_ARTIFACT_BYTES for content in artifacts.values()):
            raise OutputSecurityError("artifact exceeds the publication byte limit")

        run_id = str(uuid.uuid4())
        try:
            os.mkdir(run_id, mode=0o700, dir_fd=self._runs_fd)
        except OSError as exc:
            raise OutputSecurityError("exclusive run directory creation failed") from exc
        try:
            run_fd = os.open(
                run_id,
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                dir_fd=self._runs_fd,
            )
        except OSError as exc:
            with suppress(OSError):
                os.rmdir(run_id, dir_fd=self._runs_fd)
            raise OutputSecurityError("exclusive run directory creation failed") from exc

        temporary_names: list[str] = []
        published_names: list[str] = []
        try:
            for name, content in artifacts.items():
                temporary = f".{name}.{uuid.uuid4().hex}.tmp"
                temporary_names.append(temporary)
                file_fd = os.open(
                    temporary,
                    os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                    0o600,
                    dir_fd=run_fd,
                )
                try:
                    _write_all(file_fd, content)
                    os.fsync(file_fd)
                finally:
                    os.close(file_fd)

            for temporary, final_name in zip(temporary_names, artifacts, strict=True):
                os.link(
                    temporary,
                    final_name,
                    src_dir_fd=run_fd,
                    dst_dir_fd=run_fd,
                    follow_symlinks=False,
                )
                published_names.append(final_name)
            for temporary in temporary_names:
                os.unlink(temporary, dir_fd=run_fd)
            temporary_names.clear()
            os.fsync(run_fd)
            os.fsync(self._runs_fd)
        except OSError as exc:
            for name in (*temporary_names, *published_names):
                with suppress(OSError):
                    os.unlink(name, dir_fd=run_fd)
            os.close(run_fd)
            with suppress(OSError):
                os.rmdir(run_id, dir_fd=self._runs_fd)
            raise OutputSecurityError("atomic artifact bundle publication failed") from exc
        os.close(run_fd)
        return run_id, self.path / "runs" / run_id

    @staticmethod
    def _open_root(path: Path) -> int:
        flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC
        flags |= getattr(os, "O_NOFOLLOW_ANY", os.O_NOFOLLOW)
        try:
            root_fd = os.open(path, flags)
        except OSError as exc:
            raise OutputSecurityError("output root must be a non-symlink directory") from exc
        root_stat = os.fstat(root_fd)
        if not stat.S_ISDIR(root_stat.st_mode):
            os.close(root_fd)
            raise OutputSecurityError("output root must be a directory")
        if root_stat.st_uid != os.geteuid() or stat.S_IMODE(root_stat.st_mode) != 0o700:
            os.close(root_fd)
            raise OutputSecurityError("output root must be owner-controlled with mode 0700")
        return root_fd

    @staticmethod
    def _open_or_create_runs(root_fd: int) -> int:
        try:
            os.mkdir("runs", mode=0o700, dir_fd=root_fd)
            os.fsync(root_fd)
        except FileExistsError:
            pass
        except OSError as exc:
            raise OutputSecurityError("runs directory could not be created safely") from exc
        try:
            runs_fd = os.open(
                "runs",
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                dir_fd=root_fd,
            )
        except OSError as exc:
            raise OutputSecurityError("runs directory is unsafe or unavailable") from exc
        runs_stat = os.fstat(runs_fd)
        if runs_stat.st_uid != os.geteuid() or stat.S_IMODE(runs_stat.st_mode) != 0o700:
            os.close(runs_fd)
            raise OutputSecurityError("runs directory must be owner-controlled with mode 0700")
        return runs_fd


def _write_all(file_fd: int, content: bytes) -> None:
    view = memoryview(content)
    while view:
        written = os.write(file_fd, view)
        if written <= 0:
            raise OSError("artifact write made no progress")
        view = view[written:]
