"""Synthetic-only AutoVaxx documentation evaluation harness."""

from .models import BudgetPolicy, EvaluationManifest
from .state_machine import RunResult, StateMachineRunner, TerminalState

__all__ = [
    "BudgetPolicy",
    "EvaluationManifest",
    "RunResult",
    "StateMachineRunner",
    "TerminalState",
]
