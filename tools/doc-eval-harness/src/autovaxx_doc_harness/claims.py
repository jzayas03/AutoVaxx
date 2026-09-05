"""Cross-object claim-to-finding validation."""

from __future__ import annotations

from collections.abc import Mapping
from itertools import pairwise

from .errors import ClaimValidationError
from .models import Claim, Finding


def validate_claims(
    replacement_text: str,
    claims: list[Claim],
    verified_findings: Mapping[str, Finding],
) -> None:
    """Validate byte slices, references, non-overlap, and evidence closure."""
    replacement = replacement_text.encode("utf-8")
    ranges: list[tuple[int, int]] = []
    for claim in claims:
        if claim.end_offset > len(replacement):
            raise ClaimValidationError("claim byte range is outside replacement_text")
        try:
            decoded = replacement[claim.start_offset : claim.end_offset].decode(
                "utf-8", errors="strict"
            )
        except UnicodeDecodeError as exc:
            raise ClaimValidationError("claim offsets do not align to UTF-8 boundaries") from exc
        if decoded != claim.claim_text:
            raise ClaimValidationError("claim_text does not match its replacement byte slice")
        if any(reference not in verified_findings for reference in claim.supporting_finding_ids):
            raise ClaimValidationError("claim references an unverified finding")
        ranges.append((claim.start_offset, claim.end_offset))

    ordered = sorted(ranges)
    for previous, current in pairwise(ordered):
        if current[0] < previous[1]:
            raise ClaimValidationError("claim byte ranges cannot overlap")

    cursor = 0
    for start, end in ordered:
        _require_whitespace_only(replacement[cursor:start])
        cursor = end
    _require_whitespace_only(replacement[cursor:])


def _require_whitespace_only(raw: bytes) -> None:
    try:
        text = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise ClaimValidationError("unclaimed bytes do not align to UTF-8 boundaries") from exc
    if text and not text.isspace():
        raise ClaimValidationError("every non-whitespace replacement byte must belong to a claim")
