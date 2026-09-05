"""Strict manifest parsing and descriptor-relative source reads."""

from __future__ import annotations

import hashlib
import os
import stat
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

from pydantic import ValidationError

from .errors import ManifestError
from .models import BudgetPolicy, EvaluationManifest, SourceDeclaration, SourcePurpose

_CONTROL_OR_SEPARATOR = frozenset(chr(value) for value in range(0x20)) | {
    chr(0x7F),
    "\u2028",
    "\u2029",
}
_EVIDENCE_EXTENSIONS = frozenset({".md", ".txt", ".py"})
_EDITABLE_EXTENSIONS = frozenset({".md", ".txt"})


@dataclass(frozen=True, slots=True)
class LoadedSource:
    declaration: SourceDeclaration
    raw: bytes
    sha256: str
    device: int
    inode: int


class SourceCatalog:
    """Verified source files indexed only by manifest-owned opaque identifiers."""

    def __init__(self, root: Path, manifest: EvaluationManifest, sources: list[LoadedSource]):
        self.root = root
        self.manifest = manifest
        self._sources = {source.declaration.source_id: source for source in sources}

    def get(self, source_id: str) -> LoadedSource:
        try:
            return self._sources[source_id]
        except KeyError as exc:
            raise ManifestError("source_id is not present in the verified manifest") from exc

    @property
    def evidence(self) -> tuple[LoadedSource, ...]:
        return tuple(
            source
            for source in self._sources.values()
            if source.declaration.purpose is SourcePurpose.EVIDENCE
        )

    @property
    def target(self) -> LoadedSource:
        return self.get(self.manifest.target_source_id)


def load_manifest(
    manifest_path: Path,
    source_root: Path,
    policy: BudgetPolicy,
) -> SourceCatalog:
    """Load a strict manifest and all declared sources without following symlinks."""
    _reject_symlink_components(manifest_path)
    _reject_symlink_components(source_root)
    try:
        manifest_bytes = manifest_path.read_bytes()
        manifest = EvaluationManifest.model_validate_json(manifest_bytes)
    except (OSError, UnicodeError, ValidationError) as exc:
        raise ManifestError("manifest is missing, unreadable, or schema-invalid") from exc

    source_root = source_root.resolve(strict=True)
    root_fd = _open_directory(source_root)
    loaded: list[LoadedSource] = []
    seen_files: set[tuple[int, int]] = set()
    try:
        for declaration in manifest.sources:
            parts = _validate_declaration(declaration)
            source = _read_relative_file(root_fd, declaration, parts, policy.max_file_bytes)
            identity = (source.device, source.inode)
            if identity in seen_files:
                raise ManifestError("multiple manifest entries resolve to the same source file")
            seen_files.add(identity)
            loaded.append(source)
    finally:
        os.close(root_fd)
    return SourceCatalog(source_root, manifest, loaded)


def _validate_declaration(declaration: SourceDeclaration) -> tuple[str, ...]:
    for value in (declaration.source_id, declaration.relative_path):
        if any(character in _CONTROL_OR_SEPARATOR for character in value):
            raise ManifestError("manifest identifiers and paths cannot contain control characters")
        if "\\" in value:
            raise ManifestError("backslashes are forbidden in manifest identifiers and paths")

    raw_parts = declaration.relative_path.split("/")
    if any(part in {"", ".", ".."} for part in raw_parts):
        raise ManifestError("source paths must contain only ordinary relative components")
    candidate = PurePosixPath(declaration.relative_path)
    if candidate.is_absolute():
        raise ManifestError("absolute source paths are forbidden")
    parts = candidate.parts
    if not parts:
        raise ManifestError("source paths must contain only ordinary relative components")
    if ":" in parts[0]:
        raise ManifestError("drive-like absolute path indicators are forbidden")

    suffix = PurePosixPath(parts[-1]).suffix.lower()
    allowed = (
        _EDITABLE_EXTENSIONS
        if declaration.purpose is SourcePurpose.EDITABLE
        else _EVIDENCE_EXTENSIONS
    )
    if suffix not in allowed:
        raise ManifestError("source extension is not allowed for its declared purpose")
    return parts


def _open_directory(path: Path) -> int:
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC
    flags |= getattr(os, "O_NOFOLLOW_ANY", os.O_NOFOLLOW)
    try:
        return os.open(path, flags)
    except OSError as exc:
        raise ManifestError("source root must be an existing non-symlink directory") from exc


def _read_relative_file(
    root_fd: int,
    declaration: SourceDeclaration,
    parts: tuple[str, ...],
    max_file_bytes: int,
) -> LoadedSource:
    directory_fd = os.dup(root_fd)
    try:
        for component in parts[:-1]:
            next_fd = os.open(
                component,
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                dir_fd=directory_fd,
            )
            os.close(directory_fd)
            directory_fd = next_fd
        file_fd = os.open(
            parts[-1],
            os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC,
            dir_fd=directory_fd,
        )
        try:
            file_stat = os.fstat(file_fd)
            if not stat.S_ISREG(file_stat.st_mode):
                raise ManifestError("declared sources must be regular files")
            if file_stat.st_size > max_file_bytes:
                raise ManifestError("declared source exceeds the configured byte limit")
            raw = _read_bounded(file_fd, max_file_bytes)
        finally:
            os.close(file_fd)
    except OSError as exc:
        raise ManifestError(
            "declared source could not be opened without following symlinks"
        ) from exc
    finally:
        os.close(directory_fd)

    try:
        raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise ManifestError("declared sources must be valid UTF-8") from exc
    return LoadedSource(
        declaration=declaration,
        raw=raw,
        sha256=hashlib.sha256(raw).hexdigest(),
        device=file_stat.st_dev,
        inode=file_stat.st_ino,
    )


def _read_bounded(file_fd: int, maximum: int) -> bytes:
    chunks: list[bytes] = []
    total = 0
    while True:
        chunk = os.read(file_fd, min(65_536, maximum + 1 - total))
        if not chunk:
            return b"".join(chunks)
        chunks.append(chunk)
        total += len(chunk)
        if total > maximum:
            raise ManifestError("declared source grew beyond the configured byte limit")


def _reject_symlink_components(path: Path) -> None:
    candidate = path.absolute()
    current = Path(candidate.anchor)
    for component in candidate.parts[1:]:
        current /= component
        try:
            if stat.S_ISLNK(current.lstat().st_mode):
                raise ManifestError("symlinks are forbidden in trusted paths")
        except FileNotFoundError as exc:
            raise ManifestError("trusted path does not exist") from exc
