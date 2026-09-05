"""Strict schemas at every model/deterministic-code boundary."""

from __future__ import annotations

from enum import StrEnum
from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator

Identifier = Annotated[str, Field(pattern=r"^[A-Za-z][A-Za-z0-9_-]{0,63}$")]
Sha256 = Annotated[str, Field(pattern=r"^[0-9a-f]{64}$")]
ByteOffset = Annotated[int, Field(ge=0)]


class StrictModel(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True, frozen=True)


class SourcePurpose(StrEnum):
    EVIDENCE = "evidence"
    EDITABLE = "editable"


class SourceDeclaration(StrictModel):
    source_id: Identifier
    relative_path: Annotated[str, Field(min_length=1, max_length=240)]
    purpose: SourcePurpose


class EvaluationManifest(StrictModel):
    schema_version: Literal["1"]
    case_id: Identifier
    target_source_id: Identifier
    sources: Annotated[list[SourceDeclaration], Field(min_length=1, max_length=32)]

    @model_validator(mode="after")
    def validate_unique_entries_and_target(self) -> EvaluationManifest:
        source_ids = [source.source_id for source in self.sources]
        if len(source_ids) != len(set(source_ids)):
            raise ValueError("source_id values must be unique")
        relative_paths = [source.relative_path for source in self.sources]
        if len(relative_paths) != len(set(relative_paths)):
            raise ValueError("relative_path values must be unique")
        targets = [source for source in self.sources if source.source_id == self.target_source_id]
        if len(targets) != 1 or targets[0].purpose is not SourcePurpose.EDITABLE:
            raise ValueError("target_source_id must identify exactly one editable source")
        return self


class Finding(StrictModel):
    finding_id: Identifier
    source_id: Identifier
    source_sha256: Sha256
    start_offset: ByteOffset
    end_offset: ByteOffset
    exact_quote: Annotated[str, Field(min_length=1, max_length=16_384)]

    @model_validator(mode="after")
    def validate_range(self) -> Finding:
        if self.end_offset <= self.start_offset:
            raise ValueError("end_offset must be greater than start_offset")
        return self


class ExtractionEnvelope(StrictModel):
    findings: Annotated[list[Finding], Field(max_length=128)]

    @model_validator(mode="after")
    def validate_unique_finding_ids(self) -> ExtractionEnvelope:
        ids = [finding.finding_id for finding in self.findings]
        if len(ids) != len(set(ids)):
            raise ValueError("finding_id values must be unique")
        return self


class Claim(StrictModel):
    claim_id: Identifier
    claim_text: Annotated[str, Field(min_length=1, max_length=16_384)]
    start_offset: ByteOffset
    end_offset: ByteOffset
    supporting_finding_ids: Annotated[list[Identifier], Field(min_length=1, max_length=16)]

    @model_validator(mode="after")
    def validate_range_and_references(self) -> Claim:
        if self.end_offset <= self.start_offset:
            raise ValueError("end_offset must be greater than start_offset")
        if len(self.supporting_finding_ids) != len(set(self.supporting_finding_ids)):
            raise ValueError("supporting_finding_ids must be unique within a claim")
        return self


class EditProposal(StrictModel):
    target_source_id: Identifier
    target_sha256: Sha256
    replacement_start: ByteOffset
    replacement_end: ByteOffset
    replacement_text: Annotated[str, Field(max_length=65_536)]
    claims: Annotated[list[Claim], Field(min_length=1, max_length=64)]

    @model_validator(mode="after")
    def validate_range_and_claim_ids(self) -> EditProposal:
        if self.replacement_end <= self.replacement_start:
            raise ValueError("replacement_end must be greater than replacement_start")
        claim_ids = [claim.claim_id for claim in self.claims]
        if len(claim_ids) != len(set(claim_ids)):
            raise ValueError("claim_id values must be unique")
        return self


class DraftEnvelope(StrictModel):
    proposals: Annotated[list[EditProposal], Field(min_length=1, max_length=1)]


class BudgetPolicy(StrictModel):
    max_changed_bytes: Annotated[int, Field(gt=0)] = 8_192
    max_changed_lines: Annotated[int, Field(gt=0)] = 80
    max_removed_fraction: Annotated[float, Field(gt=0, le=0.20)] = 0.20
    max_file_bytes: Annotated[int, Field(gt=0)] = 2_097_152
