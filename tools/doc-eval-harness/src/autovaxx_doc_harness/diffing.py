"""Deterministic edit validation and inert unified-diff construction."""

from __future__ import annotations

import difflib
from dataclasses import dataclass

from .claims import validate_claims
from .errors import EditValidationError
from .manifest import SourceCatalog
from .models import BudgetPolicy, EditProposal, Finding, SourcePurpose


@dataclass(frozen=True, slots=True)
class ValidatedPatch:
    unified_diff: str
    removed_bytes: int
    inserted_bytes: int
    changed_lines: int


def build_validated_patch(
    proposal: EditProposal,
    catalog: SourceCatalog,
    verified_findings: dict[str, Finding],
    policy: BudgetPolicy,
) -> ValidatedPatch:
    target = catalog.get(proposal.target_source_id)
    if target.declaration.purpose is not SourcePurpose.EDITABLE:
        raise EditValidationError("proposed target is evidence-only")
    if proposal.target_source_id != catalog.manifest.target_source_id:
        raise EditValidationError("proposed target is not the manifest-selected editable file")
    if proposal.target_sha256 != target.sha256:
        raise EditValidationError("target hash changed or was not supplied correctly")
    if proposal.replacement_end > len(target.raw):
        raise EditValidationError("replacement byte range is outside the target file")

    try:
        prefix = target.raw[: proposal.replacement_start].decode("utf-8", errors="strict")
        target.raw[proposal.replacement_start : proposal.replacement_end].decode(
            "utf-8", errors="strict"
        )
        suffix = target.raw[proposal.replacement_end :].decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise EditValidationError("replacement offsets do not align to UTF-8 boundaries") from exc

    replacement_bytes = proposal.replacement_text.encode("utf-8")
    removed_bytes = proposal.replacement_end - proposal.replacement_start
    inserted_bytes = len(replacement_bytes)
    if removed_bytes + inserted_bytes > policy.max_changed_bytes:
        raise EditValidationError("edit exceeds the total removed-plus-inserted byte budget")
    if target.raw and removed_bytes == len(target.raw):
        raise EditValidationError("whole-file replacement is prohibited during the pilot")
    if target.raw and removed_bytes / len(target.raw) > policy.max_removed_fraction:
        raise EditValidationError("edit exceeds the maximum removable file fraction")

    validate_claims(proposal.replacement_text, proposal.claims, verified_findings)
    original_text = target.raw.decode("utf-8", errors="strict")
    proposed_text = prefix + proposal.replacement_text + suffix
    if not proposed_text:
        raise EditValidationError("the proposed target cannot be empty")
    if not original_text.endswith("\n") or not proposed_text.endswith("\n"):
        raise EditValidationError("pilot editable documents must remain newline-terminated")

    original_lines = original_text.splitlines(keepends=True)
    proposed_lines = proposed_text.splitlines(keepends=True)
    matcher = difflib.SequenceMatcher(a=original_lines, b=proposed_lines, autojunk=False)
    changed_lines = sum(
        (old_end - old_start) + (new_end - new_start)
        for operation, old_start, old_end, new_start, new_end in matcher.get_opcodes()
        if operation != "equal"
    )
    if changed_lines > policy.max_changed_lines:
        raise EditValidationError("edit exceeds the changed-line budget")

    header = target.declaration.source_id
    diff = "".join(
        difflib.unified_diff(
            original_lines,
            proposed_lines,
            fromfile=f"a/{header}",
            tofile=f"b/{header}",
            lineterm="\n",
        )
    )
    if not diff:
        raise EditValidationError("proposal does not change the target")
    return ValidatedPatch(diff, removed_bytes, inserted_bytes, changed_lines)
