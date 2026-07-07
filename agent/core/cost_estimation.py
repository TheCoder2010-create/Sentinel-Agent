"""Conservative cost estimates for auto-approved infrastructure actions."""

import re
from dataclasses import dataclass
from typing import Any

DEFAULT_SANDBOX_RESERVATION_HOURS = 1.0

SPACE_PRICE_USD_PER_HOUR: dict[str, float] = {
    "cpu-basic": 0.0,
    "cpu-upgrade": 0.05,
    "cpu-performance": 0.50,
    "cpu-xl": 1.00,
    "t4-small": 0.60,
    "t4-medium": 0.90,
    "l4x1": 1.00,
    "l4x4": 4.00,
    "l40sx1": 2.00,
    "l40sx4": 8.00,
    "l40sx8": 16.00,
    "a10g-small": 1.00,
    "a10g-large": 2.00,
    "a10g-largex2": 4.00,
    "a10g-largex4": 8.00,
    "a100-large": 4.00,
    "a100x4": 16.00,
    "a100x8": 32.00,
    "h200": 10.00,
    "h200x2": 20.00,
    "h200x4": 40.00,
    "h200x8": 80.00,
    "inf2x6": 6.00,
}

_DURATION_RE = re.compile(r"^\s*(\d+(?:\.\d+)?)\s*([smhd]?)\s*$", re.IGNORECASE)


@dataclass(frozen=True)
class CostEstimate:
    """Estimated cost for a tool call.

    ``estimated_cost_usd=None`` means the call may be billable but we could not
    estimate it safely, so auto-approval should fall back to a human decision.
    """

    estimated_cost_usd: float | None
    billable: bool
    block_reason: str | None = None
    label: str | None = None


def parse_timeout_hours(
    value: Any, *, default_hours: float = 0.5
) -> float | None:
    if value is None or value == "":
        return default_hours
    if isinstance(value, bool):
        return None
    if isinstance(value, int | float):
        seconds = float(value)
        return seconds / 3600 if seconds > 0 else None
    if not isinstance(value, str):
        return None

    match = _DURATION_RE.match(value)
    if not match:
        return None
    amount = float(match.group(1))
    unit = match.group(2).lower() or "s"
    if amount <= 0:
        return None
    if unit == "s":
        return amount / 3600
    if unit == "m":
        return amount / 60
    if unit == "h":
        return amount
    if unit == "d":
        return amount * 24
    return None





async def estimate_sandbox_cost(
    args: dict[str, Any], *, session: Any = None
) -> CostEstimate:
    if session is not None and getattr(session, "sandbox", None):
        return CostEstimate(estimated_cost_usd=0.0, billable=False, label="existing")

    hardware = str(args.get("hardware") or "cpu-basic")
    price = SPACE_PRICE_USD_PER_HOUR.get(hardware)
    if price is None:
        return CostEstimate(
            estimated_cost_usd=None,
            billable=True,
            block_reason=f"No price is available for sandbox hardware '{hardware}'.",
            label=hardware,
        )

    return CostEstimate(
        estimated_cost_usd=round(price * DEFAULT_SANDBOX_RESERVATION_HOURS, 4),
        billable=price > 0,
        label=hardware,
    )


async def estimate_tool_cost(
    tool_name: str, args: dict[str, Any], *, session: Any = None
) -> CostEstimate:
    if tool_name == "sandbox_create":
        return await estimate_sandbox_cost(args, session=session)
    return CostEstimate(estimated_cost_usd=0.0, billable=False)
