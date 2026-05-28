"""ChimiaClaw Literature lane worker: open-access ingestion + deterministic
citation-grounded extraction. Molecule and reaction candidates are MolADT
structural objects (see :mod:`literature_synthesis.moladt`), not SMILES.
"""

from .moladt import (
    Atom,
    AtomicSymbol,
    BondingSystem,
    CatalystCondition,
    Coordinate,
    Molecule,
    PressureCondition,
    Reaction,
    SolventCondition,
    StoichiometryEntry,
    TemperatureCondition,
)
from .schema import (
    ExtractedClaim,
    LiteratureCitation,
    LiteratureIngestManifest,
    LiteratureRuntime,
    LiteratureSource,
    LiteratureSourceKind,
    LiteratureSynthesis,
    ModelProvenance,
    MoleculeCandidate,
    MoleculeRole,
    ReactionCandidate,
)

__version__ = "0.2.0"

__all__ = [
    "__version__",
    # MolADT types
    "Atom",
    "AtomicSymbol",
    "BondingSystem",
    "CatalystCondition",
    "Coordinate",
    "Molecule",
    "PressureCondition",
    "Reaction",
    "SolventCondition",
    "StoichiometryEntry",
    "TemperatureCondition",
    # Synthesis types
    "ExtractedClaim",
    "LiteratureCitation",
    "LiteratureIngestManifest",
    "LiteratureRuntime",
    "LiteratureSource",
    "LiteratureSourceKind",
    "LiteratureSynthesis",
    "ModelProvenance",
    "MoleculeCandidate",
    "MoleculeRole",
    "ReactionCandidate",
]
