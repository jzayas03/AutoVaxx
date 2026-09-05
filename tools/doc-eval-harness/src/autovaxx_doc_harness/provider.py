"""Provider boundary shared by deterministic fakes and the synthetic Ollama adapter."""

from __future__ import annotations

from collections.abc import Mapping
from typing import Protocol

from .models import Finding


class DocumentationProvider(Protocol):
    """Bounded interface for synthetic documentation evaluation providers."""

    def extract(self, sources: Mapping[str, bytes], timeout_seconds: float) -> str:
        """Return an ExtractionEnvelope JSON string."""
        ...

    def repair(self, stage: str, invalid_json: str, timeout_seconds: float) -> str:
        """Return one structurally repaired JSON envelope."""
        ...

    def draft(
        self,
        findings: Mapping[str, Finding],
        target_source_id: str,
        target: bytes,
        timeout_seconds: float,
    ) -> str:
        """Return a DraftEnvelope JSON string."""
        ...
