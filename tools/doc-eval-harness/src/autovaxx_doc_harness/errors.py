"""Typed failures used to make state-machine routing explicit."""


class HarnessError(Exception):
    """Base class for expected, safely reportable harness failures."""


class ManifestError(HarnessError):
    """The trusted manifest or one of its declared files is invalid."""


class ProvenanceError(HarnessError):
    """A finding does not close over the exact declared source bytes."""


class ClaimValidationError(HarnessError):
    """A drafted claim is structurally invalid or lacks verified references."""


class EditValidationError(HarnessError):
    """An edit violates target, encoding, or budget policy."""


class OutputSecurityError(HarnessError):
    """The output location or artifact publication operation is unsafe."""


class ProviderError(HarnessError):
    """Base class for local provider failures."""


class ProviderInputRejected(ProviderError):
    """The provider input exceeds a deterministic local safety bound."""


class ProviderTransientError(ProviderError):
    """A retryable local transport failure."""


class ProviderTimeout(ProviderError):
    """The provider honored cancellation at the request timeout."""


class ProviderDeadlineOverrun(ProviderError):
    """The provider continued after its remaining deadline."""


class ProviderTerminationFailed(ProviderError):
    """An owned worker could not be terminated after cancellation."""


class ProviderOutOfMemory(ProviderError):
    """The local provider exhausted memory."""


class ProviderDiskFull(ProviderError):
    """The local provider or harness exhausted its allocated disk."""
