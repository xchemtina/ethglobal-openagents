"""Open-access licence handling.

The Literature lane is open-access only. Anything not on the whitelist is
treated as paywalled and silently skipped at ingest time. The verbatim
license string is captured on every `LiteratureCitation` so an audit can
reproduce the decision later.
"""

from __future__ import annotations

from typing import Final, Tuple

# Order matters only for human readability; matching is membership-based.
DEFAULT_LICENSE_WHITELIST: Final[Tuple[str, ...]] = (
    "cc-by",
    "cc-by-sa",
    "cc-by-nc",
    "cc-by-nc-sa",
    "cc-by-nd",
    "cc0",
    "publicdomain",
    "arxiv-perpetual",
    "arxiv-nonexclusive-distrib",
)


def normalize_license(raw: str | None) -> str:
    """Map a noisy licence hint to a canonical lowercase token.

    Returns the empty string when no hint is present so callers can treat
    "no licence" and "unknown licence" as the same conservative case.
    """
    if not raw:
        return ""
    text = raw.strip().lower()
    # Crossref returns license arrays with URLs; drop URL noise but keep the
    # tail token so e.g. "https://creativecommons.org/licenses/by/4.0/"
    # collapses to "cc-by".
    if "creativecommons.org/licenses/" in text:
        tail = text.split("creativecommons.org/licenses/", 1)[1].strip("/")
        head = tail.split("/", 1)[0]
        if head:
            return f"cc-{head}"
    if "creativecommons.org/publicdomain" in text:
        return "publicdomain"
    if text.startswith("http"):
        # Unknown URL-shaped licence; preserve verbatim, callers can decide.
        return text
    # Common arXiv hints.
    if "arxiv" in text and "perpetual" in text:
        return "arxiv-perpetual"
    if "arxiv" in text and "nonexclusive" in text:
        return "arxiv-nonexclusive-distrib"
    return text


def is_open_access(
    raw: str | None,
    whitelist: Tuple[str, ...] = DEFAULT_LICENSE_WHITELIST,
) -> bool:
    """Return True iff the licence hint matches any whitelist entry."""
    canonical = normalize_license(raw)
    if not canonical:
        return False
    return canonical in whitelist


__all__ = ["DEFAULT_LICENSE_WHITELIST", "normalize_license", "is_open_access"]
