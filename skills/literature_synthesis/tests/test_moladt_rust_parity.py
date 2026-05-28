"""Cross-language parity check: Python ``AtomicSymbol`` mirrors Rust's.

The Rust crate ``chimiaclaw-moladt`` (``crates/chimiaclaw-moladt/src/lib.rs``)
is the **single source of truth** for atomic-symbol coverage across every
language binding. This test parses the Rust source directly, extracts the
``MOLADT_ELEMENT_MANIFEST`` rows, and asserts that the Python
``AtomicSymbol`` enum exposes exactly the same set of element values.

Failure means the Python and Rust enums have drifted and an artifact written
on one side may not decode cleanly on the other. To repair, add the new
variants to ``skills/literature_synthesis/src/literature_synthesis/moladt.py``
in atomic-number order so the order matches the Rust enum too.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

from literature_synthesis.moladt import AtomicSymbol


# Walk up from this test file (skills/literature_synthesis/tests/...) to the
# repository root so we can reach the Rust crate that lives at the same level
# as ``skills/``.
_REPO_ROOT = Path(__file__).resolve().parents[3]
_RUST_LIB = _REPO_ROOT / "crates" / "chimiaclaw-moladt" / "src" / "lib.rs"

# Lines in the manifest look like:
#     (AtomicSymbol::Cu, 29, 63.546,   "Cu"),
# Match the textual symbol in the trailing string literal.
_MANIFEST_ROW = re.compile(
    r"\(\s*AtomicSymbol::([A-Za-z]+)\s*,\s*\d+\s*,\s*[-+0-9eE.]+\s*,\s*\"([A-Za-z]+)\"\s*\)"
)


@pytest.fixture(scope="module")
def rust_symbols() -> tuple[list[str], list[int]]:
    """Parse the Rust manifest and return ``(symbols_in_z_order, atomic_numbers)``."""
    if not _RUST_LIB.exists():
        pytest.skip(
            f"Rust source not found at {_RUST_LIB}; this parity test only runs "
            "inside a checkout of the OpenAgents monorepo."
        )
    source = _RUST_LIB.read_text(encoding="utf-8")

    # Restrict to the manifest block so we don't pick up the enum match arms
    # (which would double-count).
    block_re = re.compile(
        r"pub const MOLADT_ELEMENT_MANIFEST.*?=\s*&\[(?P<body>.*?)\];",
        re.DOTALL,
    )
    block = block_re.search(source)
    assert block, "could not locate MOLADT_ELEMENT_MANIFEST in Rust source"

    symbols: list[str] = []
    atomic_numbers: list[int] = []
    for row in re.finditer(
        r"\(\s*AtomicSymbol::([A-Za-z]+)\s*,\s*(\d+)\s*,\s*[-+0-9eE.]+\s*,\s*\"([A-Za-z]+)\"\s*\)",
        block.group("body"),
    ):
        rust_variant, z_str, text_symbol = row.group(1), row.group(2), row.group(3)
        # The Rust enum constructor name and the wire-string must agree; if
        # the Rust source ever drifts, this assertion catches it.
        assert rust_variant == text_symbol, (
            f"Rust manifest row mismatch: variant={rust_variant!r} "
            f"text={text_symbol!r}"
        )
        symbols.append(text_symbol)
        atomic_numbers.append(int(z_str))
    assert symbols, "no element rows extracted from manifest"
    return symbols, atomic_numbers


def test_python_atomic_symbol_set_matches_rust(rust_symbols: tuple[list[str], list[int]]) -> None:
    rust_set = set(rust_symbols[0])
    python_set = {member.value for member in AtomicSymbol}
    missing_in_python = rust_set - python_set
    extra_in_python = python_set - rust_set
    assert not missing_in_python, (
        f"Python AtomicSymbol is missing elements present in the Rust manifest: "
        f"{sorted(missing_in_python)}"
    )
    assert not extra_in_python, (
        f"Python AtomicSymbol has elements not in the Rust manifest "
        f"(Rust is the source of truth): {sorted(extra_in_python)}"
    )


def test_python_atomic_symbol_count_matches_rust(rust_symbols: tuple[list[str], list[int]]) -> None:
    assert len(rust_symbols[0]) == len(list(AtomicSymbol)), (
        f"element count mismatch: Rust={len(rust_symbols[0])} Python={len(list(AtomicSymbol))}"
    )


def test_python_atomic_symbol_declaration_order_matches_rust(
    rust_symbols: tuple[list[str], list[int]],
) -> None:
    """Both sides must be in atomic-number order so a Rust artifact with a sorted
    BTreeSet of atoms round-trips through Python with the same canonical order."""
    python_order = [member.value for member in AtomicSymbol]
    assert python_order == rust_symbols[0], (
        "AtomicSymbol declaration order has drifted between Python and Rust.\n"
        f"Rust   : {rust_symbols[0][:10]} ...\n"
        f"Python : {python_order[:10]} ..."
    )


def test_known_extended_elements_round_trip() -> None:
    """Spot-check the elements that were added during the consolidation."""
    for raw in ("Cu", "Sn", "Ge", "Au", "Ru", "Pt", "La", "U", "Si", "Ti"):
        assert AtomicSymbol(raw).value == raw
