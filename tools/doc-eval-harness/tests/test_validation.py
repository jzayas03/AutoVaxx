from __future__ import annotations

import json

import pytest
from conftest import SyntheticCase

from autovaxx_doc_harness.claims import validate_claims
from autovaxx_doc_harness.diffing import build_validated_patch
from autovaxx_doc_harness.errors import ClaimValidationError, EditValidationError, ProvenanceError
from autovaxx_doc_harness.manifest import load_manifest
from autovaxx_doc_harness.models import (
    BudgetPolicy,
    Claim,
    DraftEnvelope,
    ExtractionEnvelope,
)
from autovaxx_doc_harness.provenance import build_verified_finding_index


def test_multibyte_finding_offsets_are_byte_precise(synthetic_case: SyntheticCase) -> None:
    catalog = load_manifest(
        synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy()
    )
    payload = json.loads(synthetic_case.extraction_json)
    accent_byte = synthetic_case.evidence.index("á".encode())
    payload["findings"][0]["start_offset"] = accent_byte + 1
    payload["findings"][0]["exact_quote"] = "x"
    envelope = ExtractionEnvelope.model_validate(payload)

    with pytest.raises(ProvenanceError, match="UTF-8"):
        build_verified_finding_index(envelope, catalog)


def test_editable_target_cannot_be_used_as_evidence(synthetic_case: SyntheticCase) -> None:
    catalog = load_manifest(
        synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy()
    )
    payload = json.loads(synthetic_case.extraction_json)
    quote = b"Texto anterior."
    start = synthetic_case.target.index(quote)
    finding = payload["findings"][0]
    finding.update(
        {
            "source_id": "Target1",
            "source_sha256": catalog.target.sha256,
            "start_offset": start,
            "end_offset": start + len(quote),
            "exact_quote": quote.decode(),
        }
    )

    with pytest.raises(ProvenanceError, match="evidence-only"):
        build_verified_finding_index(ExtractionEnvelope.model_validate(payload), catalog)


def test_claim_offsets_use_utf8_bytes(synthetic_case: SyntheticCase) -> None:
    replacement = "está"
    valid_claim = Claim(
        claim_id="Claim1",
        claim_text=replacement,
        start_offset=0,
        end_offset=len(replacement.encode()),
        supporting_finding_ids=["Finding1"],
    )
    catalog = load_manifest(
        synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy()
    )
    findings = build_verified_finding_index(
        ExtractionEnvelope.model_validate_json(synthetic_case.extraction_json), catalog
    )
    validate_claims(replacement, [valid_claim], findings)

    invalid_claim = valid_claim.model_copy(update={"end_offset": len(replacement)})
    with pytest.raises(ClaimValidationError):
        validate_claims(replacement, [invalid_claim], findings)


def test_unclaimed_non_whitespace_is_rejected(synthetic_case: SyntheticCase) -> None:
    catalog = load_manifest(
        synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy()
    )
    findings = build_verified_finding_index(
        ExtractionEnvelope.model_validate_json(synthetic_case.extraction_json), catalog
    )
    claim = Claim(
        claim_id="Claim1",
        claim_text="Supported",
        start_offset=0,
        end_offset=len(b"Supported"),
        supporting_finding_ids=["Finding1"],
    )

    with pytest.raises(ClaimValidationError, match="non-whitespace"):
        validate_claims("Supported UNSUPPORTED", [claim], findings)


def test_unknown_finding_reference_is_rejected(synthetic_case: SyntheticCase) -> None:
    catalog = load_manifest(
        synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy()
    )
    findings = build_verified_finding_index(
        ExtractionEnvelope.model_validate_json(synthetic_case.extraction_json), catalog
    )
    claim = Claim(
        claim_id="Claim1",
        claim_text="Supported",
        start_offset=0,
        end_offset=len(b"Supported"),
        supporting_finding_ids=["UnknownFinding"],
    )

    with pytest.raises(ClaimValidationError, match="unverified"):
        validate_claims("Supported", [claim], findings)


def test_edit_budget_counts_removed_plus_inserted_bytes(
    synthetic_case: SyntheticCase,
) -> None:
    catalog = load_manifest(
        synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy()
    )
    findings = build_verified_finding_index(
        ExtractionEnvelope.model_validate_json(synthetic_case.extraction_json), catalog
    )
    proposal = DraftEnvelope.model_validate_json(synthetic_case.draft_json).proposals[0]
    changed_bytes = (
        proposal.replacement_end
        - proposal.replacement_start
        + len(proposal.replacement_text.encode())
    )

    with pytest.raises(EditValidationError, match="removed-plus-inserted"):
        build_validated_patch(
            proposal,
            catalog,
            findings,
            BudgetPolicy(max_changed_bytes=changed_bytes - 1),
        )


def test_edit_budget_enforces_removal_fraction(synthetic_case: SyntheticCase) -> None:
    catalog = load_manifest(
        synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy()
    )
    findings = build_verified_finding_index(
        ExtractionEnvelope.model_validate_json(synthetic_case.extraction_json), catalog
    )
    proposal = DraftEnvelope.model_validate_json(synthetic_case.draft_json).proposals[0]

    with pytest.raises(EditValidationError, match="removable file fraction"):
        build_validated_patch(
            proposal,
            catalog,
            findings,
            BudgetPolicy(max_removed_fraction=0.01),
        )


def test_whole_file_replacement_is_prohibited(synthetic_case: SyntheticCase) -> None:
    catalog = load_manifest(
        synthetic_case.manifest_path, synthetic_case.source_root, BudgetPolicy()
    )
    findings = build_verified_finding_index(
        ExtractionEnvelope.model_validate_json(synthetic_case.extraction_json), catalog
    )
    proposal = DraftEnvelope.model_validate_json(synthetic_case.draft_json).proposals[0]
    whole_file = proposal.model_copy(
        update={"replacement_start": 0, "replacement_end": len(synthetic_case.target)}
    )

    with pytest.raises(EditValidationError, match="whole-file"):
        build_validated_patch(whole_file, catalog, findings, BudgetPolicy())
