"""Byte-precise provenance verification."""

from __future__ import annotations

import hashlib

from .errors import ProvenanceError
from .manifest import SourceCatalog
from .models import ExtractionEnvelope, Finding, SourcePurpose


def build_verified_finding_index(
    envelope: ExtractionEnvelope,
    catalog: SourceCatalog,
) -> dict[str, Finding]:
    """Return only findings whose file hash, byte range, and exact quote all match."""
    verified: dict[str, Finding] = {}
    for finding in envelope.findings:
        source = catalog.get(finding.source_id)
        if source.declaration.purpose is not SourcePurpose.EVIDENCE:
            raise ProvenanceError("findings may cite evidence-only sources")
        actual_hash = hashlib.sha256(source.raw).hexdigest()
        if actual_hash != source.sha256 or finding.source_sha256 != actual_hash:
            raise ProvenanceError("finding source hash does not match the complete source file")
        if finding.end_offset > len(source.raw):
            raise ProvenanceError("finding byte range is outside the source file")
        try:
            quote = source.raw[finding.start_offset : finding.end_offset].decode(
                "utf-8", errors="strict"
            )
        except UnicodeDecodeError as exc:
            raise ProvenanceError("finding offsets do not align to UTF-8 boundaries") from exc
        if quote != finding.exact_quote:
            raise ProvenanceError("finding quote does not match its exact source byte slice")
        verified[finding.finding_id] = finding
    return verified
