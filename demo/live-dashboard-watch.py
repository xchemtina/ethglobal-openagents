#!/usr/bin/env python3
"""Generate a live ChimiaClaw dashboard model from overnight pipeline outputs.

The browser cannot list local directories, so this script scans the operator-run
pipeline output tree and writes a single JSON file that `world-map.html` can
poll. It is intentionally dependency-free and tolerant of artifacts being
written while the scan is running.
"""

from __future__ import annotations

import argparse
import copy
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


EXPECTED_DFT_MOLECULES = 6
EXPECTED_UNISWAP_QUOTES = 3
EXPECTED_ENS_AGENTS = 3


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def decode_inline_payload(artifact: dict[str, Any]) -> dict[str, Any] | None:
    location = artifact.get("payload", {}).get("location", {})
    inline = location.get("Inline") if isinstance(location, dict) else None
    bytes_hex = inline.get("bytes_hex") if isinstance(inline, dict) else None
    if not bytes_hex:
        return None
    raw = bytes.fromhex(bytes_hex).decode("utf-8")
    payload = json.loads(raw)
    return payload if isinstance(payload, dict) else None


def read_artifact(path: Path) -> dict[str, Any] | None:
    try:
        artifact = load_json(path)
        if not isinstance(artifact, dict) or not str(artifact.get("id", "")).startswith("art_"):
            return None
        return {
            "path": path,
            "artifact": artifact,
            "payload": decode_inline_payload(artifact),
        }
    except (OSError, ValueError, json.JSONDecodeError):
        # Pipeline files may be visible before the writer has flushed them.
        return None


def short_path(path: Path, repo_root: Path) -> str:
    try:
        return str(path.relative_to(repo_root))
    except ValueError:
        return str(path)


def artifact_records(root: Path, pattern: str) -> list[dict[str, Any]]:
    return [
        record
        for path in sorted(root.rglob(pattern))
        if path.is_file()
        for record in [read_artifact(path)]
        if record is not None
    ]


def number(value: Any, default: float = 0.0) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def as_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def rel_pipeline_dir(pipeline_dir: Path, repo_root: Path) -> str:
    return short_path(pipeline_dir, repo_root)


def scan_dft(pipeline_dir: Path, repo_root: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for record in artifact_records(pipeline_dir / "dft", "chem_dft_result.art_*.json"):
        artifact = record["artifact"]
        payload = record.get("payload") or {}
        path = record["path"]
        molecule_label = path.parent.name
        orbitals = payload.get("orbitals", {}) if isinstance(payload.get("orbitals"), dict) else {}
        dipole = payload.get("dipole", {}) if isinstance(payload.get("dipole"), dict) else {}
        convergence = payload.get("convergence", {}) if isinstance(payload.get("convergence"), dict) else {}
        timings = payload.get("timings", {}) if isinstance(payload.get("timings"), dict) else {}
        rows.append(
            {
                "label": molecule_label,
                "artifact_id": artifact.get("id"),
                "request_artifact_id": (artifact.get("parent_artifact_ids") or [None])[0],
                "path": short_path(path, repo_root),
                "molecule_id": payload.get("molecule_id"),
                "functional": payload.get("functional"),
                "basis_set": payload.get("basis_set"),
                "backend": payload.get("backend"),
                "energy_hartree": payload.get("energy_hartree"),
                "gap_ev": orbitals.get("gap_ev"),
                "dipole_debye": dipole.get("magnitude_debye"),
                "wall_seconds": timings.get("wall_seconds"),
                "converged": convergence.get("converged"),
                "created_at_unix": artifact.get("created_at_unix", 0),
            }
        )
    return sorted(rows, key=lambda row: (row.get("created_at_unix") or 0, row.get("label") or ""))

def latest_dft_rows(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    latest: dict[str, dict[str, Any]] = {}
    for row in rows:
        label = str(row.get("label") or row.get("artifact_id") or "")
        previous = latest.get(label)
        row_sort = (row.get("created_at_unix") or 0, row.get("artifact_id") or "")
        previous_sort = (
            previous.get("created_at_unix") or 0,
            previous.get("artifact_id") or "",
        ) if previous else (-1, "")
        if previous is None or row_sort >= previous_sort:
            latest[label] = row
    return sorted(latest.values(), key=lambda row: (row.get("created_at_unix") or 0, row.get("label") or ""))


def scan_uniswap(pipeline_dir: Path, repo_root: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for record in artifact_records(pipeline_dir / "uniswap", "market_uniswap_quote.art_*.json"):
        artifact = record["artifact"]
        payload = record.get("payload") or {}
        request = payload.get("request", {}) if isinstance(payload.get("request"), dict) else {}
        response = payload.get("response", {}) if isinstance(payload.get("response"), dict) else {}
        quote = response.get("quote", {}) if isinstance(response.get("quote"), dict) else payload.get("quote", {})
        output = quote.get("output", {}) if isinstance(quote.get("output"), dict) else {}
        aggregated = quote.get("aggregatedOutputs", []) if isinstance(quote.get("aggregatedOutputs"), list) else []
        output_amount = (
            payload.get("output_amount")
            or output.get("amount")
            or (aggregated[0].get("amount") if aggregated and isinstance(aggregated[0], dict) else None)
        )
        rows.append(
            {
                "artifact_id": artifact.get("id"),
                "path": short_path(record["path"], repo_root),
                "amount_in": request.get("amount"),
                "amount_out": output_amount,
                "token_in": request.get("tokenIn"),
                "token_out": request.get("tokenOut"),
                "routing_type": payload.get("routing_type") or payload.get("routing") or response.get("routing"),
                "gas_fee_usd": payload.get("gas_fee_usd") or quote.get("gasFeeUSD"),
                "quote_only": True,
                "created_at_unix": artifact.get("created_at_unix", 0),
            }
        )
    return sorted(rows, key=lambda row: row.get("created_at_unix") or 0)


def scan_zerog(pipeline_dir: Path, repo_root: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for record in artifact_records(pipeline_dir / "zerog", "*.json"):
        artifact = record["artifact"]
        payload = record.get("payload") or {}
        receipt = payload.get("receipt", {}) if isinstance(payload.get("receipt"), dict) else {}
        rows.append(
            {
                "artifact_id": artifact.get("id"),
                "path": short_path(record["path"], repo_root),
                "source_artifact_id": payload.get("source_artifact_id") or (artifact.get("parent_artifact_ids") or [None])[0],
                "storage_uri": payload.get("storage_uri") or artifact.get("output_cid"),
                "network": receipt.get("network"),
                "root_hash": (receipt.get("root_hashes") or [None])[0] if isinstance(receipt.get("root_hashes"), list) else None,
                "created_at_unix": artifact.get("created_at_unix", 0),
            }
        )
    return sorted(rows, key=lambda row: row.get("created_at_unix") or 0)


def scan_literature(pipeline_dir: Path, repo_root: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for record in artifact_records(pipeline_dir / "literature", "science_literature_*.art_*.json"):
        artifact = record["artifact"]
        payload = record.get("payload") or {}
        schema = (artifact.get("schema_tags") or ["science.literature.artifact"])[0]
        kind = "synthesis" if "synthesis" in str(schema) else "ingest"
        provenance = payload.get("model_provenance", {}) if isinstance(payload.get("model_provenance"), dict) else {}
        rows.append(
            {
                "artifact_id": artifact.get("id"),
                "kind": kind,
                "schema_tag": schema,
                "path": short_path(record["path"], repo_root),
                "query": payload.get("query"),
                "sector": payload.get("sector"),
                "summary": payload.get("summary"),
                "citation_count": len(payload.get("citations", [])) if isinstance(payload.get("citations"), list) else None,
                "source_count": len(payload.get("sources", [])) if isinstance(payload.get("sources"), list) else None,
                "claim_count": len(payload.get("extracted_claims", [])) if isinstance(payload.get("extracted_claims"), list) else None,
                "molecule_count": len(payload.get("molecule_candidates", [])) if isinstance(payload.get("molecule_candidates"), list) else None,
                "reaction_count": len(payload.get("reaction_candidates", [])) if isinstance(payload.get("reaction_candidates"), list) else None,
                "runtime": provenance.get("runtime"),
                "model_id": provenance.get("model_id"),
                "prompt_hash": provenance.get("prompt_hash"),
                "deterministic": provenance.get("deterministic"),
                "created_at_unix": artifact.get("created_at_unix", 0),
            }
        )
    return sorted(rows, key=lambda row: (row.get("created_at_unix") or 0, row.get("kind") or ""))


def scan_ens(pipeline_dir: Path, repo_root: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for record in artifact_records(pipeline_dir / "ens", "identity_ens_*.json"):
        artifact = record["artifact"]
        payload = record.get("payload") or {}
        schema = (artifact.get("schema_tags") or ["identity.ens.artifact"])[0]
        kind = str(schema).split(".")[-1]
        ens_name = payload.get("ens_name") or artifact.get("agent")
        rows.append(
            {
                "artifact_id": artifact.get("id"),
                "kind": kind,
                "schema_tag": schema,
                "agent": artifact.get("agent"),
                "ens_name": ens_name,
                "path": short_path(record["path"], repo_root),
                "chain_id": payload.get("chain_id"),
                "verified": payload.get("verified"),
                "mismatches": len(payload.get("mismatches", [])) if isinstance(payload.get("mismatches"), list) else None,
                "records": len(payload.get("records", [])) if isinstance(payload.get("records"), list) else None,
                "created_at_unix": artifact.get("created_at_unix", 0),
            }
        )
    return sorted(rows, key=lambda row: row.get("created_at_unix") or 0)


def latest_log_line(pipeline_dir: Path) -> str | None:
    log_path = pipeline_dir / "overnight-full.log"
    try:
        lines = [line.strip() for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines()]
    except OSError:
        return None
    for line in reversed(lines):
        if line:
            return line
    return None


def upsert_metric(snapshot: dict[str, Any], label: str, value: str, status: str) -> None:
    metrics = snapshot.setdefault("metrics", [])
    for metric in metrics:
        if metric.get("label") == label:
            metric["value"] = value
            metric["status"] = status
            return
    metrics.append({"label": label, "value": value, "status": status})


def upsert_evidence(model: dict[str, Any], card: dict[str, Any]) -> None:
    evidence = model.setdefault("live_sponsor_evidence", [])
    sponsor = card.get("sponsor")
    for index, existing in enumerate(evidence):
        if existing.get("sponsor") == sponsor:
            evidence[index] = card
            return
    evidence.append(card)


def append_artifact_card(model: dict[str, Any], card: dict[str, Any]) -> None:
    cards = model.setdefault("artifact_cards", [])
    if any(existing.get("id") == card.get("id") for existing in cards):
        return
    cards.append(card)


def dft_record_lines(rows: list[dict[str, Any]], limit: int = 8) -> list[dict[str, str]]:
    records = []
    for row in rows[-limit:]:
        gap = number(row.get("gap_ev"))
        dipole = number(row.get("dipole_debye"))
        wall = number(row.get("wall_seconds"))
        status = "OK" if row.get("converged") is True else "pending convergence"
        records.append(
            {
                "key": row.get("label") or "dft-result",
                "value": f"{status} · gap {gap:.3f} eV · |μ| {dipole:.3f} D · {wall:.1f}s · {row.get('artifact_id')}",
            }
        )
    return records


def quote_record_lines(rows: list[dict[str, Any]], limit: int = 6) -> list[dict[str, str]]:
    records = []
    for row in rows[-limit:]:
        amount_in = as_int(row.get("amount_in")) / 1_000_000
        amount_out = as_int(row.get("amount_out")) / 1_000_000
        gas = row.get("gas_fee_usd") or "?"
        records.append(
            {
                "key": row.get("routing_type") or "quote",
                "value": f"{amount_in:.2f} USDC → {amount_out:.2f} USDT · gas ${gas} · quote-only · {row.get('artifact_id')}",
            }
        )
    return records


def build_live_model(base_model: dict[str, Any], pipeline_dir: Path, output_path: Path) -> dict[str, Any]:
    repo_root = output_path.parent.parent.resolve()
    now = utc_now()
    all_dft_rows = scan_dft(pipeline_dir, repo_root)
    dft_rows = latest_dft_rows(all_dft_rows)
    quote_rows = scan_uniswap(pipeline_dir, repo_root)
    zerog_rows = scan_zerog(pipeline_dir, repo_root)
    ens_rows = scan_ens(pipeline_dir, repo_root)
    literature_rows = scan_literature(pipeline_dir, repo_root)
    log_line = latest_log_line(pipeline_dir)
    dft_started = len(list((pipeline_dir / "molecules").glob("*"))) if (pipeline_dir / "molecules").exists() else 0
    artifact_ids = [
        *(row["artifact_id"] for row in dft_rows if row.get("artifact_id")),
        *(row["artifact_id"] for row in quote_rows if row.get("artifact_id")),
        *(row["artifact_id"] for row in zerog_rows if row.get("artifact_id")),
        *(row["artifact_id"] for row in ens_rows if row.get("artifact_id")),
        *(row["artifact_id"] for row in literature_rows if row.get("artifact_id")),
    ]

    model = copy.deepcopy(base_model)
    model["generated_at"] = now
    model["maturity"] = "three-agent-pipeline-live-refresh"
    model["subtitle"] = (
        "Live auto-refresh projection of the Literature → Retrosynthesis → DFT dashboard. "
        "The static model remains the fallback; this live layer scans overnight DFT, Uniswap, 0G, "
        "and ENS artifacts emitted by demo/overnight-full-pipeline.sh."
    )
    model.setdefault("current_truth", []).append(
        "Live mode reads demo/world-model.live.json, which is regenerated from local signed artifacts; the browser still does not contact wallets, sponsor APIs, or private services."
    )

    snapshot = model.setdefault("submission_snapshot", {})
    snapshot["status"] = "live-refreshing"
    snapshot["summary"] = (
        "The dashboard is watching the full overnight pipeline output directory and refreshes as signed "
        "DFT results, quote-only Uniswap artifacts, 0G upload anchors, and ENS publication/verification artifacts appear."
    )
    upsert_metric(snapshot, "Live model", f"refreshed {now}", "active")
    upsert_metric(snapshot, "Live DFT batch", f"{len(dft_rows)}/{EXPECTED_DFT_MOLECULES} latest results", "real-execution" if dft_rows else "active")
    upsert_metric(snapshot, "Live Uniswap quotes", f"{len(quote_rows)}/{EXPECTED_UNISWAP_QUOTES} quote-only", "live-quote-only" if quote_rows else "operator-gated-next")
    upsert_metric(snapshot, "Live 0G anchors", str(len(zerog_rows)), "testnet-anchor" if zerog_rows else "operator-gated-next")
    upsert_metric(snapshot, "Live ENS artifacts", f"{len(ens_rows)} artifacts", "live-sepolia-verified" if ens_rows else "operator-gated-next")
    literature_synth_count = sum(1 for row in literature_rows if row.get("kind") == "synthesis")
    upsert_metric(
        snapshot,
        "Live Literature synthesis",
        f"{literature_synth_count} signed synthesis artifact(s)",
        "real-execution" if literature_synth_count else "operator-gated",
    )

    model.setdefault("activity_ticker", []).insert(
        0,
        {
            "timestamp": "LIVE",
            "agent": "Overnight full pipeline",
            "action": log_line or f"watching {rel_pipeline_dir(pipeline_dir, repo_root)}",
            "color": "oklch(0.70 0.18 148)",
        },
    )

    full_records = [
        {"key": "output dir", "value": rel_pipeline_dir(pipeline_dir, repo_root)},
        {"key": "last scan", "value": now},
        {"key": "DFT", "value": f"{len(dft_rows)}/{EXPECTED_DFT_MOLECULES} unique molecules complete; {len(all_dft_rows)} result artifacts; {dft_started} molecule dirs started"},
        {"key": "Uniswap", "value": f"{len(quote_rows)}/{EXPECTED_UNISWAP_QUOTES} quote-only artifacts"},
        {"key": "0G", "value": f"{len(zerog_rows)} upload anchor artifacts"},
        {"key": "ENS", "value": f"{len(ens_rows)} publication/resolution/verification artifacts"},
    ]
    if log_line:
        full_records.append({"key": "latest log", "value": log_line})
    upsert_evidence(
        model,
        {
            "sponsor": "Overnight Full Pipeline",
            "status": "real-execution" if dft_rows else "active",
            "summary": "Auto-refreshing local projection of demo/overnight-full-pipeline.sh outputs.",
            "records": full_records,
            "artifacts": [{"kind": "artifact", "id": art_id} for art_id in artifact_ids[-10:]],
            "caveat": "This card is generated locally from files already written by the operator-run pipeline; it never invokes external services itself.",
        },
    )

    if dft_rows:
        upsert_evidence(
            model,
            {
                "sponsor": "Live Pipeline DFT",
                "status": "real-execution",
                "summary": "New overnight full-pipeline PySCF PBE/def2-svp scalar DFT results are appearing as signed chem.dft.result artifacts.",
                "records": dft_record_lines(dft_rows),
                "artifacts": [{"kind": "dft-result", "id": row["artifact_id"]} for row in dft_rows if row.get("artifact_id")],
                "caveat": "These full-pipeline DFT jobs request scalar energy/gap/dipole values unless cube generation is explicitly enabled; do not add them to the WebGPU orbital gallery without cube-backed reruns.",
            },
        )
        append_artifact_card(
            model,
            {
                "id": "ART.LIVE.OVERNIGHT_FULL.DFT",
                "label": "Live overnight full-pipeline DFT results",
                "schema_tag": "chem.dft.result",
                "lineage": [row["artifact_id"] for row in dft_rows if row.get("artifact_id")],
                "status": "real-execution",
            },
        )

    if quote_rows:
        upsert_evidence(
            model,
            {
                "sponsor": "Uniswap Settlement",
                "status": "live-quote-only",
                "summary": "Live Uniswap Trade API quote artifacts exist for service settlement pricing; no swap execution is claimed.",
                "records": quote_record_lines(quote_rows),
                "artifacts": [{"kind": "quote", "id": row["artifact_id"]} for row in quote_rows if row.get("artifact_id")],
                "caveat": "Quote-only boundary: ChimiaClaw signs market.uniswap.quote artifacts and does not call /swap without explicit operator confirmation.",
            },
        )
        append_artifact_card(
            model,
            {
                "id": "ART.LIVE.UNISWAP.QUOTE_BATCH",
                "label": "Live Uniswap quote-only settlement artifacts",
                "schema_tag": "market.uniswap.quote",
                "lineage": [row["artifact_id"] for row in quote_rows if row.get("artifact_id")],
                "status": "live-quote-only",
            },
        )

    if zerog_rows:
        upsert_evidence(
            model,
            {
                "sponsor": "0G Storage",
                "status": "testnet-anchor-batch",
                "summary": f"{len(zerog_rows)} full-pipeline DFT result artifact(s) have been anchored through 0G Storage.",
                "network": zerog_rows[-1].get("network") or "0G",
                "storage_uri": zerog_rows[-1].get("storage_uri"),
                "records": [
                    {"key": row.get("source_artifact_id") or "source", "value": f"{row.get('storage_uri')} · {row.get('artifact_id')}"}
                    for row in zerog_rows[-8:]
                ],
                "artifacts": [{"kind": "anchor", "id": row["artifact_id"]} for row in zerog_rows if row.get("artifact_id")],
                "caveat": "Generated from local signed storage.zerog.upload artifacts written by the pipeline.",
            },
        )

    if literature_rows:
        synth_rows = [row for row in literature_rows if row.get("kind") == "synthesis"]
        ingest_rows = [row for row in literature_rows if row.get("kind") == "ingest"]
        latest_synth = synth_rows[-1] if synth_rows else None
        records = []
        if latest_synth:
            records.extend(
                [
                    {"key": "query", "value": str(latest_synth.get("query") or "-")},
                    {"key": "runtime", "value": f"{latest_synth.get('runtime') or '?'}:{latest_synth.get('model_id') or '?'}"},
                    {
                        "key": "counts",
                        "value": (
                            f"{latest_synth.get('citation_count') or 0} citations · "
                            f"{latest_synth.get('claim_count') or 0} claims · "
                            f"{latest_synth.get('molecule_count') or 0} molecules · "
                            f"{latest_synth.get('reaction_count') or 0} reactions"
                        ),
                    },
                    {"key": "prompt_hash", "value": str(latest_synth.get("prompt_hash") or "-")},
                    {"key": "deterministic", "value": str(latest_synth.get("deterministic"))},
                ]
            )
        for row in literature_rows[-6:]:
            records.append(
                {
                    "key": f"{row.get('kind')}:{row.get('artifact_id')}",
                    "value": str(row.get("summary") or row.get("query") or row.get("path")),
                }
            )
        upsert_evidence(
            model,
            {
                "sponsor": "Literature",
                "status": "real-execution" if synth_rows else "operator-gated",
                "summary": (
                    f"Live signed Literature artifacts: {len(ingest_rows)} ingest manifest(s), "
                    f"{len(synth_rows)} synthesis result(s)."
                ),
                "records": records,
                "artifacts": [
                    {"kind": row["kind"], "id": row["artifact_id"]}
                    for row in literature_rows
                    if row.get("artifact_id")
                ],
                "caveat": "Generated locally from signed science.literature.* artifacts; the browser does not call any LLM or paper API.",
            },
        )
        if synth_rows:
            append_artifact_card(
                model,
                {
                    "id": "ART.LIVE.LITERATURE.SYNTHESIS",
                    "label": "Live signed Literature synthesis artifacts",
                    "schema_tag": "science.literature.synthesis",
                    "lineage": [row["artifact_id"] for row in synth_rows if row.get("artifact_id")],
                    "status": "real-execution",
                },
            )

    if ens_rows:
        publications = [row for row in ens_rows if row.get("kind") == "publication"]
        verifications = [row for row in ens_rows if row.get("kind") == "verification"]
        upsert_evidence(
            model,
            {
                "sponsor": "ENS",
                "status": "live-sepolia-agent-records",
                "summary": f"Full-pipeline ENS publication/resolution/verification artifacts are live for service capability records ({len(publications)} publication(s), {len(verifications)} verification(s)).",
                "network": "Sepolia chain 11155111",
                "records": [
                    {
                        "key": f"{row.get('kind')}:{row.get('agent')}",
                        "value": (
                            f"{row.get('records')} record(s) · {row.get('artifact_id')}"
                            if row.get("kind") == "publication"
                            else f"verified={row.get('verified')} mismatches={row.get('mismatches')} · {row.get('artifact_id')}"
                        ),
                    }
                    for row in ens_rows[-8:]
                ],
                "artifacts": [{"kind": row["kind"], "id": row["artifact_id"]} for row in ens_rows if row.get("artifact_id")],
                "caveat": "Generated from local signed identity.ens.* artifacts; private key material remains outside dashboard files.",
            },
        )

    if dft_rows:
        last = dft_rows[-1]
        model.setdefault("science_transactions", []).append(
            {
                "id": "TX.LIVE.OVERNIGHT_FULL.DFT",
                "status": "real-execution",
                "service_kind": "dft",
                "provider_ens": "dft.service.chimiaclaw.eth",
                "provider_agent": "AGENT.DFT",
                "requester_agent": "operator.chimiaclaw.eth",
                "target_lab": "AGENT.DFT",
                "offer_id": "OFFER.LIVE.OVERNIGHT_FULL.DFT",
                "request_id": f"LIVE.DFT.BATCH.{len(dft_rows)}",
                "quote_id": quote_rows[-1]["artifact_id"] if quote_rows else "market.uniswap.quote:pending",
                "settlement_id": "SETTLE.LIVE.UNISWAP.QUOTE_ONLY",
                "result_id": last["artifact_id"],
                "price_usdc_micros": 0,
                "estimated_latency_seconds": int(number(last.get("wall_seconds"), 0)),
                "summary": f"Live overnight full-pipeline scalar DFT batch has {len(dft_rows)} signed result artifact(s) so far.",
                "summary_chem": "; ".join(
                    f"{row['label']} gap {number(row.get('gap_ev')):.3f} eV"
                    for row in dft_rows[-3:]
                ),
                "artifact_flow": [
                    "chem.molecule.adt",
                    "chem.dft.request",
                    "chem.dft.result",
                    "market.uniswap.quote",
                    "storage.zerog.upload",
                    "identity.ens.publication",
                ],
                "sponsor_bindings": [
                    {"sponsor": "Uniswap", "live_status": "live-quote-only", "attachment": f"{len(quote_rows)} quote artifact(s)"},
                    {"sponsor": "0G", "live_status": "testnet-anchor-batch" if zerog_rows else "operator-gated-next", "attachment": f"{len(zerog_rows)} anchor artifact(s)"},
                    {"sponsor": "ENS", "live_status": "live-sepolia-agent-records" if ens_rows else "operator-gated-next", "attachment": f"{len(ens_rows)} ENS artifact(s)"},
                ],
                "payer_agent": "operator.chimiaclaw.eth",
                "payee_agent": "dft.service.chimiaclaw.eth",
                "asset": "USDC quote-only",
                "settlement_method": "UniswapPreparedTransfer quote-only",
                "acceptance_id": "ACCEPT.LIVE.OVERNIGHT_FULL.DFT",
                "escrow_id": "ESCROW.LIVE.UNISWAP.QUOTE_ONLY",
                "acknowledgement_id": "ACK.LIVE.OVERNIGHT_FULL.DFT",
                "release_id": last["artifact_id"],
                "release_status": "verified-signed-result",
                "transaction_ref": last["path"],
                "refund_policy": {"refund_to_agent": "operator.chimiaclaw.eth"},
            }
        )

    model["overnight_full_pipeline"] = {
        "literature_artifact_ids": [row["artifact_id"] for row in literature_rows if row.get("artifact_id")],
        "literature_artifacts": literature_rows,
        "schema_tag": "chimiaclaw.live_dashboard.scan.v1",
        "source_dir": rel_pipeline_dir(pipeline_dir, repo_root),
        "generated_at": now,
        "log_path": short_path(pipeline_dir / "overnight-full.log", repo_root),
        "latest_log_line": log_line,
        "artifact_ids": artifact_ids,
        "dft_result_artifact_ids": [row["artifact_id"] for row in dft_rows if row.get("artifact_id")],
        "dft_all_result_artifact_ids": [row["artifact_id"] for row in all_dft_rows if row.get("artifact_id")],
        "uniswap_quote_artifact_ids": [row["artifact_id"] for row in quote_rows if row.get("artifact_id")],
        "zerog_anchor_artifact_ids": [row["artifact_id"] for row in zerog_rows if row.get("artifact_id")],
        "ens_artifact_ids": [row["artifact_id"] for row in ens_rows if row.get("artifact_id")],
        "counts": {
            "dft_results": len(dft_rows),
            "dft_result_artifacts": len(all_dft_rows),
            "dft_expected": EXPECTED_DFT_MOLECULES,
            "uniswap_quotes": len(quote_rows),
            "uniswap_expected": EXPECTED_UNISWAP_QUOTES,
            "zerog_anchors": len(zerog_rows),
            "ens_artifacts": len(ens_rows),
            "ens_expected_agents": EXPECTED_ENS_AGENTS,
            "literature_artifacts": len(literature_rows),
            "literature_syntheses": literature_synth_count,
        },
        "dft_results": dft_rows,
        "dft_result_artifacts": all_dft_rows,
        "uniswap_quotes": quote_rows,
        "zerog_anchors": zerog_rows,
        "ens_artifacts": ens_rows,
    }
    return model


def write_json_atomic(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = path.with_suffix(path.suffix + ".tmp")
    with tmp_path.open("w", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, sort_keys=False)
        handle.write("\n")
    os.replace(tmp_path, path)


def run_once(args: argparse.Namespace) -> None:
    base = load_json(args.base_world_model)
    model = build_live_model(base, args.pipeline_dir.resolve(), args.output.resolve())
    write_json_atomic(args.output, model)
    counts = model["overnight_full_pipeline"]["counts"]
    print(
        "live dashboard refreshed: "
        f"DFT {counts['dft_results']}/{counts['dft_expected']}, "
        f"Uniswap {counts['uniswap_quotes']}/{counts['uniswap_expected']}, "
        f"0G {counts['zerog_anchors']}, ENS artifacts {counts['ens_artifacts']}, "
        f"Literature syntheses {counts['literature_syntheses']} -> {args.output}"
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-world-model", type=Path, default=Path("demo/world-model.json"))
    parser.add_argument("--pipeline-dir", type=Path, default=Path("demo/overnight-full-out"))
    parser.add_argument("--output", type=Path, default=Path("demo/world-model.live.json"))
    parser.add_argument("--interval-seconds", type=float, default=5.0)
    parser.add_argument("--once", action="store_true", help="write once and exit")
    parser.add_argument("--watch", action="store_true", help="keep refreshing until interrupted")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if not args.base_world_model.exists():
        print(f"base world model not found: {args.base_world_model}", file=sys.stderr)
        return 2
    if args.once or not args.watch:
        run_once(args)
        return 0
    while True:
        run_once(args)
        time.sleep(max(1.0, args.interval_seconds))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
