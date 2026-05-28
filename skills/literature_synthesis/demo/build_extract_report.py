"""Build a PDF report from the most recent `extract-corpus` test run.

Reads `~/.chimia_kb/literature/corpus-extract.json` plus each paper's
`synthesis.json` + `synthesis-meta.json`, renders a thumbnail of every
Docling-tagged figure straight out of the original PDF, generates summary
plots with matplotlib, and assembles a multi-page PDF with reportlab.

Run:
    .venv/bin/python demo/build_extract_report.py
        [--corpus-manifest PATH] [--output PATH]
"""

from __future__ import annotations

import argparse
import io
import json
import textwrap
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import fitz  # PyMuPDF
import matplotlib

matplotlib.use("Agg")  # non-interactive backend; required for headless runs
import matplotlib.pyplot as plt
from reportlab.lib import colors
from reportlab.lib.pagesizes import LETTER
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import (
    Image as RLImage,
    KeepTogether,
    PageBreak,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)


DEFAULT_CORPUS_MANIFEST = Path("~/.chimia_kb/literature/corpus-extract.json").expanduser()
DEFAULT_OUTPUT = Path("~/.chimia_kb/literature/extract-test-report.pdf").expanduser()
DEFAULT_CACHE_ROOT = Path("~/.chimia_kb/docling").expanduser()


# ---------------------------------------------------------------------------- io


def load_corpus_manifest(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_paper_artifacts(output_dir: Path) -> Tuple[Optional[dict], Optional[dict]]:
    synth = output_dir / "synthesis.json"
    meta = output_dir / "synthesis-meta.json"
    s = json.loads(synth.read_text(encoding="utf-8")) if synth.exists() else None
    m = json.loads(meta.read_text(encoding="utf-8")) if meta.exists() else None
    return s, m


def load_docling_document(cache_root: Path, content_hash: str) -> Optional[dict]:
    p = cache_root / content_hash[:2] / content_hash / "document.json"
    if not p.exists():
        return None
    return json.loads(p.read_text(encoding="utf-8"))


# ---------------------------------------------------------------------------- figure extraction

# Heuristic thresholds. Tuned against the 5-paper test corpus where every real
# scientific figure had a caption beginning with one of these tokens and every
# rejected logo/footer/icon either lacked a caption or was tiny.
FIGURE_CAPTION_KEYWORDS = (
    "fig.",
    "fig ",
    "figure",
    "scheme",
    "table",
    "chart",
    "plate",
    "graph",
    "abstract:",  # graphical abstracts (ACS/Wiley)
)
MIN_FIGURE_AREA_PT2 = 8_000.0  # below ~90x90 pt → almost always junk
MIN_FIGURE_DIM_PT = 60.0       # reject anything thinner than this on either axis
MAX_ASPECT_RATIO = 8.0         # reject hairline rules / wide banners
FALLBACK_PAGE_FRACTION = 0.30  # uncaptioned figures kept iff ≥30% of page area
SINGLE_PICTURE_PAGE_FRACTION = 0.15  # poster-style 1-picture docs are kept lower
FIGURE_PADDING_PT = 4.0        # padding around the clipped rect
CLIP_DPI_SCALE = 3.0           # 216 DPI render — sharp axis labels & subscripts


def _caption_text(pic: dict, text_by_ref: Dict[str, dict]) -> str:
    """Concatenate all caption texts referenced by a Docling picture."""
    out: List[str] = []
    for cap in pic.get("captions", []) or []:
        ref = cap.get("$ref")
        if not ref:
            continue
        tx = text_by_ref.get(ref) or {}
        text = (tx.get("text") or "").strip()
        if text:
            out.append(text)
    return " ".join(out)


def _caption_bbox(
    pic: dict, text_by_ref: Dict[str, dict], page_no: int
) -> Optional[Dict[str, float]]:
    """Union of caption bboxes on the same page as the figure."""
    union: Optional[Dict[str, float]] = None
    for cap in pic.get("captions", []) or []:
        ref = cap.get("$ref")
        if not ref:
            continue
        tx = text_by_ref.get(ref) or {}
        for prov in tx.get("prov", []) or []:
            if prov.get("page_no") != page_no:
                continue
            bb = prov.get("bbox") or {}
            if not bb:
                continue
            if union is None:
                union = dict(bb)
            else:
                union["l"] = min(union["l"], bb["l"])
                union["r"] = max(union["r"], bb["r"])
                union["t"] = max(union["t"], bb["t"])
                union["b"] = min(union["b"], bb["b"])
    return union


def _bbox_union(a: Dict[str, float], b: Optional[Dict[str, float]]) -> Dict[str, float]:
    if b is None:
        return dict(a)
    return {
        "l": min(a["l"], b["l"]),
        "r": max(a["r"], b["r"]),
        "t": max(a["t"], b["t"]),
        "b": min(a["b"], b["b"]),
    }


def _bbox_area(bb: Dict[str, float]) -> float:
    return max(0.0, bb["r"] - bb["l"]) * max(0.0, bb["t"] - bb["b"])


def _page_size(docling_doc: dict, page_no: int) -> Tuple[float, float]:
    pages = docling_doc.get("pages") or {}
    entry = pages.get(str(page_no)) or pages.get(page_no) or {}
    size = entry.get("size") or {}
    return float(size.get("width") or 612.0), float(size.get("height") or 792.0)


def _is_scientific_figure(
    pic: dict,
    text_by_ref: Dict[str, dict],
    page_w: float,
    page_h: float,
    total_pictures: int,
) -> bool:
    prov_list = pic.get("prov") or []
    if not prov_list:
        return False
    prov = prov_list[0]
    page_no = prov.get("page_no", 1)
    bbox = prov.get("bbox") or {}
    if not bbox:
        return False

    w = bbox.get("r", 0) - bbox.get("l", 0)
    h = bbox.get("t", 0) - bbox.get("b", 0)
    if w < MIN_FIGURE_DIM_PT or h < MIN_FIGURE_DIM_PT:
        return False
    area = w * h
    if area < MIN_FIGURE_AREA_PT2:
        return False
    aspect = max(w, h) / max(1e-3, min(w, h))
    if aspect > MAX_ASPECT_RATIO:
        return False

    page_area = max(1.0, page_w * page_h)
    area_frac = area / page_area

    caption = _caption_text(pic, text_by_ref).lower().lstrip("*•·- ")
    has_keyword = any(caption.startswith(k) for k in FIGURE_CAPTION_KEYWORDS)

    # Hard reject: page-1 elements with no caption that don't dominate the page
    # are almost always journal headers, logos, or graphical-abstract chrome.
    if page_no == 1 and not has_keyword and area_frac < 0.25:
        return False

    if has_keyword:
        return True
    if area_frac >= FALLBACK_PAGE_FRACTION:
        return True
    if total_pictures == 1 and area_frac >= SINGLE_PICTURE_PAGE_FRACTION:
        return True
    return False


def render_figure_clip(
    pdf_path: Path,
    page_no: int,
    bbox_bottom_up: Dict[str, float],
    page_w: float,
    page_h: float,
) -> Optional[bytes]:
    """Clip a figure (already filtered) out of the original PDF.

    Adds ``FIGURE_PADDING_PT`` of padding, clamps to the page rect, and renders
    at ``CLIP_DPI_SCALE``× (≈216 DPI by default) so axis ticks and subscripts
    stay legible.
    """
    try:
        doc = fitz.open(str(pdf_path))
        page = doc.load_page(page_no - 1)
        ph = page.rect.height
        # Trust the explicit Docling page height when available; fall back to
        # PyMuPDF if it differs by more than 1pt (rare).
        ph_eff = page_h if abs(page_h - ph) < 1.0 else ph
        x0 = bbox_bottom_up["l"] - FIGURE_PADDING_PT
        x1 = bbox_bottom_up["r"] + FIGURE_PADDING_PT
        y0 = ph_eff - bbox_bottom_up["t"] - FIGURE_PADDING_PT
        y1 = ph_eff - bbox_bottom_up["b"] + FIGURE_PADDING_PT
        rect = fitz.Rect(x0, y0, x1, y1) & page.rect  # clamp to page
        if rect.width < MIN_FIGURE_DIM_PT or rect.height < MIN_FIGURE_DIM_PT:
            doc.close()
            return None
        pix = page.get_pixmap(
            matrix=fitz.Matrix(CLIP_DPI_SCALE, CLIP_DPI_SCALE),
            clip=rect,
            alpha=False,
            colorspace=fitz.csRGB,
        )
        png = pix.tobytes("png")
        doc.close()
        return png
    except Exception:
        return None


def select_figures_for_paper(
    primary_path: Path,
    docling_doc: Optional[dict],
    max_figures: int = 2,
) -> List[Tuple[bytes, str]]:
    """Filter Docling pictures down to scientific figures and re-render them.

    Returns ``[(png_bytes, caption_text), ...]``. No page-thumbnail fallback —
    we'd rather show nothing than show a journal-banner thumbnail.
    """
    if not docling_doc:
        return []
    text_by_ref = {t["self_ref"]: t for t in docling_doc.get("texts", [])}
    pictures = docling_doc.get("pictures", []) or []
    total = len(pictures)

    out: List[Tuple[bytes, str]] = []
    for pic in pictures:
        prov_list = pic.get("prov") or []
        if not prov_list:
            continue
        page_no = prov_list[0].get("page_no", 1)
        page_w, page_h = _page_size(docling_doc, page_no)

        if not _is_scientific_figure(pic, text_by_ref, page_w, page_h, total):
            continue

        fig_bbox = prov_list[0]["bbox"]
        cap_bbox = _caption_bbox(pic, text_by_ref, page_no)
        clip_bbox = _bbox_union(fig_bbox, cap_bbox)
        png = render_figure_clip(
            primary_path, page_no, clip_bbox, page_w, page_h
        )
        if not png:
            continue
        caption = _caption_text(pic, text_by_ref)
        out.append((png, caption))
        if len(out) >= max_figures:
            break
    return out


# ---------------------------------------------------------------------------- plots


def plot_per_paper_counts(papers: List[dict], titles: List[str], out_path: Path) -> None:
    n = len(papers)
    if n == 0:
        return
    claims = [p.get("claim_count", 0) for p in papers]
    mols = [p.get("molecule_count", 0) for p in papers]
    rxns = [p.get("reaction_count", 0) for p in papers]
    x = list(range(n))
    fig, ax = plt.subplots(figsize=(8.0, 3.4))
    bottoms = [0] * n
    for vals, label, color in (
        (claims, "claims", "#1f77b4"),
        (mols, "molecules", "#2ca02c"),
        (rxns, "reactions", "#d62728"),
    ):
        ax.bar(x, vals, bottom=bottoms, label=label, color=color)
        bottoms = [b + v for b, v in zip(bottoms, vals)]
    ax.set_xticks(x)
    ax.set_xticklabels([t[:20] for t in titles], rotation=30, ha="right", fontsize=8)
    ax.set_ylabel("count")
    ax.set_title("Extracted items per paper")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_durations(papers: List[dict], titles: List[str], out_path: Path) -> None:
    n = len(papers)
    if n == 0:
        return
    durs = [p.get("duration_seconds", 0.0) for p in papers]
    fig, ax = plt.subplots(figsize=(8.0, 3.0))
    ax.bar(range(n), durs, color="#9467bd")
    ax.set_xticks(range(n))
    ax.set_xticklabels([t[:20] for t in titles], rotation=30, ha="right", fontsize=8)
    ax.set_ylabel("seconds")
    ax.set_title("Extraction duration per paper (MLX local, Qwen3.6-35B-A3B-4bit)")
    fig.tight_layout()
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_status_pie(totals: Dict[str, int], out_path: Path) -> None:
    labels, sizes, colors_ = [], [], []
    palette = {"ok": "#2ca02c", "cached": "#1f77b4", "error": "#d62728"}
    for key in ("ok", "cached", "error"):
        v = int(totals.get(key, 0))
        if v > 0:
            labels.append(f"{key} ({v})")
            sizes.append(v)
            colors_.append(palette[key])
    if not sizes:
        return
    fig, ax = plt.subplots(figsize=(4.0, 4.0))
    ax.pie(sizes, labels=labels, autopct="%1.0f%%", colors=colors_, startangle=90)
    ax.set_title("Per-paper status")
    fig.tight_layout()
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


# ---------------------------------------------------------------------------- report assembly


def _truncate(text: str, n: int) -> str:
    text = text.strip()
    if len(text) <= n:
        return text
    return text[: n - 1] + "…"


def _short_title(meta: Optional[dict], paper_entry: dict) -> str:
    if meta and meta.get("title"):
        t = str(meta["title"]).strip()
        if t:
            return t
    pth = paper_entry.get("primary_path", "")
    return Path(pth).stem if pth else paper_entry.get("content_hash", "")[:12]


def _hill_formula(molecule: Dict[str, Any]) -> str:
    """Hill notation (C first, H second, then alphabetical).

    Operates on the JSON form of a MolADT ``Molecule`` so the report can be
    built without importing the Pydantic models.
    """
    counts: Dict[str, int] = {}
    for atom in molecule.get("atoms", []) or []:
        sym = str(atom.get("symbol", "")).strip()
        if not sym:
            continue
        counts[sym] = counts.get(sym, 0) + 1
    if not counts:
        return "-"
    ordered: List[str] = []
    if "C" in counts:
        ordered.append("C")
        if "H" in counts:
            ordered.append("H")
    for sym in sorted(counts):
        if sym in ("C", "H") and "C" in counts:
            continue
        ordered.append(sym)
    parts: List[str] = []
    for sym in ordered:
        n = counts[sym]
        parts.append(sym if n == 1 else f"{sym}{n}")
    return "".join(parts)


def _format_stoich_side(entries: List[Dict[str, Any]]) -> str:
    if not entries:
        return "∅"
    pieces: List[str] = []
    for entry in entries:
        coeff = entry.get("coefficient", 1.0)
        formula = _hill_formula(entry.get("molecule") or {})
        coeff_str = (
            "" if abs(coeff - 1.0) < 1e-9 else f"{int(coeff) if coeff == int(coeff) else coeff} "
        )
        pieces.append(f"{coeff_str}{formula}")
    return " + ".join(pieces)


def _format_reaction_equation(rxn: Dict[str, Any]) -> str:
    lhs = _format_stoich_side(rxn.get("reactants") or [])
    rhs = _format_stoich_side(rxn.get("products") or [])
    return f"{lhs} → {rhs}"


def _format_conditions(conditions: List[Dict[str, Any]]) -> str:
    parts: List[str] = []
    for cond in conditions:
        kind = cond.get("kind")
        if kind == "temperature":
            parts.append(f"T = {cond.get('kelvin')} K")
        elif kind == "pressure":
            parts.append(f"P = {cond.get('bar')} bar")
        else:
            parts.append(f"{kind}={cond}")
    return ", ".join(parts)


def build_report(
    corpus_manifest: Dict[str, Any],
    output_pdf: Path,
    cache_root: Path,
    max_figures_per_paper: int = 4,
) -> None:
    styles = getSampleStyleSheet()
    body = ParagraphStyle(
        "Body", parent=styles["BodyText"], fontSize=9, leading=12, spaceAfter=4
    )
    small = ParagraphStyle(
        "Small", parent=styles["BodyText"], fontSize=8, leading=10, textColor=colors.grey
    )
    h1 = ParagraphStyle(
        "H1",
        parent=styles["Heading1"],
        fontSize=18,
        leading=22,
        spaceAfter=8,
    )
    h2 = ParagraphStyle("H2", parent=styles["Heading2"], fontSize=12, leading=16)
    h3 = ParagraphStyle("H3", parent=styles["Heading3"], fontSize=10, leading=14)
    quote = ParagraphStyle(
        "Quote",
        parent=styles["BodyText"],
        fontSize=8,
        leading=11,
        leftIndent=14,
        textColor=colors.darkslategray,
        fontName="Helvetica-Oblique",
    )

    output_pdf.parent.mkdir(parents=True, exist_ok=True)
    doc = SimpleDocTemplate(
        str(output_pdf),
        pagesize=LETTER,
        leftMargin=0.7 * inch,
        rightMargin=0.7 * inch,
        topMargin=0.6 * inch,
        bottomMargin=0.6 * inch,
        title="ChimiaCLAW Literature Extraction — Test Report",
        author="literature_synthesis",
    )

    story: list = []
    papers = corpus_manifest.get("papers", [])
    totals = corpus_manifest.get("totals", {})

    # -------- cover --------
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    story.append(Paragraph("ChimiaCLAW Literature Extraction", h1))
    story.append(Paragraph("Test report — local MLX run", h2))
    story.append(Spacer(1, 0.15 * inch))
    cover_rows = [
        ["Runtime", str(corpus_manifest.get("runtime", "?"))],
        ["Model", str(corpus_manifest.get("model_id", "?"))],
        ["Query", str(corpus_manifest.get("query", "?"))],
        ["Sector", str(corpus_manifest.get("sector", "?"))],
        ["Papers (total)", str(totals.get("papers_total", len(papers)))],
        ["OK / Cached / Error", f"{totals.get('ok',0)} / {totals.get('cached',0)} / {totals.get('error',0)}"],
        ["Elapsed (s)", f"{corpus_manifest.get('elapsed_seconds', 0):.2f}"],
        ["Generated", now],
    ]
    t = Table(cover_rows, colWidths=[1.6 * inch, 5.0 * inch])
    t.setStyle(
        TableStyle(
            [
                ("FONTNAME", (0, 0), (0, -1), "Helvetica-Bold"),
                ("FONTSIZE", (0, 0), (-1, -1), 9),
                ("BACKGROUND", (0, 0), (0, -1), colors.lightgrey),
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("LEFTPADDING", (0, 0), (-1, -1), 6),
                ("RIGHTPADDING", (0, 0), (-1, -1), 6),
                ("TOPPADDING", (0, 0), (-1, -1), 3),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 3),
                ("GRID", (0, 0), (-1, -1), 0.25, colors.lightgrey),
            ]
        )
    )
    story.append(t)
    story.append(Spacer(1, 0.2 * inch))

    # -------- summary plots page --------
    tmp_dir = Path(output_pdf.parent) / ".report_plots"
    tmp_dir.mkdir(exist_ok=True)
    titles_for_plot = [_short_title(None, p) for p in papers]

    p1 = tmp_dir / "counts.png"
    p2 = tmp_dir / "durations.png"
    p3 = tmp_dir / "status.png"
    plot_per_paper_counts(papers, titles_for_plot, p1)
    plot_durations(papers, titles_for_plot, p2)
    plot_status_pie(totals, p3)

    if p1.exists():
        story.append(Paragraph("Summary plots", h2))
        story.append(RLImage(str(p1), width=6.8 * inch, height=2.8 * inch))
        story.append(Spacer(1, 0.1 * inch))
    if p2.exists():
        story.append(RLImage(str(p2), width=6.8 * inch, height=2.6 * inch))
        story.append(Spacer(1, 0.1 * inch))
    if p3.exists():
        story.append(RLImage(str(p3), width=3.4 * inch, height=3.4 * inch))

    story.append(PageBreak())

    # -------- per-paper detail --------
    for idx, paper_entry in enumerate(papers, start=1):
        content_hash = paper_entry["content_hash"]
        out_dir = Path(paper_entry["output_dir"])
        synth, meta = load_paper_artifacts(out_dir)
        title = _short_title(meta, paper_entry)
        primary_path = Path(paper_entry.get("primary_path", ""))

        story.append(Paragraph(f"{idx}. {title}", h2))
        info_rows = [
            ["Hash", content_hash[:32] + "…"],
            ["Source", _truncate(str(primary_path), 90)],
            ["Status", paper_entry.get("status", "?")],
            ["Duration (s)", f"{paper_entry.get('duration_seconds', 0):.2f}"],
            [
                "Counts",
                f"claims={paper_entry.get('claim_count',0)}, "
                f"molecules={paper_entry.get('molecule_count',0)}, "
                f"reactions={paper_entry.get('reaction_count',0)}",
            ],
            ["Prompt hash", _truncate(paper_entry.get("prompt_hash", "") or "", 60)],
        ]
        t = Table(info_rows, colWidths=[1.1 * inch, 5.5 * inch])
        t.setStyle(
            TableStyle(
                [
                    ("FONTNAME", (0, 0), (0, -1), "Helvetica-Bold"),
                    ("FONTSIZE", (0, 0), (-1, -1), 8),
                    ("VALIGN", (0, 0), (-1, -1), "TOP"),
                    ("LEFTPADDING", (0, 0), (-1, -1), 4),
                    ("RIGHTPADDING", (0, 0), (-1, -1), 4),
                    ("TOPPADDING", (0, 0), (-1, -1), 2),
                    ("BOTTOMPADDING", (0, 0), (-1, -1), 2),
                    ("BACKGROUND", (0, 0), (0, -1), colors.whitesmoke),
                    ("GRID", (0, 0), (-1, -1), 0.25, colors.lightgrey),
                ]
            )
        )
        story.append(t)
        story.append(Spacer(1, 0.08 * inch))

        # summary
        if synth:
            summary = str(synth.get("summary", "")).strip()
            if summary:
                story.append(Paragraph("<b>Summary</b>", h3))
                story.append(Paragraph(summary, body))

        # figures
        docling_doc = load_docling_document(cache_root, content_hash)
        figures = (
            select_figures_for_paper(
                primary_path, docling_doc, max_figures=max_figures_per_paper
            )
            if primary_path.exists()
            else []
        )
        for png_bytes, caption in figures:
            try:
                # Preserve native aspect ratio: constrain by max width, let
                # height scale proportionally. This keeps scientific figures
                # readable instead of stretching them to a fixed rectangle.
                buf = io.BytesIO(png_bytes)
                img = RLImage(buf)
                native_w, native_h = img.imageWidth, img.imageHeight
                max_w = 5.5 * inch
                max_h = 3.6 * inch
                scale = min(max_w / native_w, max_h / native_h, 1.0)
                img.drawWidth = native_w * scale
                img.drawHeight = native_h * scale
                img.hAlign = "LEFT"
                story.append(Spacer(1, 0.05 * inch))
                story.append(img)
                if caption:
                    story.append(Paragraph(_truncate(caption, 380), small))
            except Exception:
                continue

        # claims (verbatim with evidence span)
        if synth:
            claims = synth.get("extracted_claims", [])
            if claims:
                story.append(Spacer(1, 0.08 * inch))
                story.append(Paragraph(f"<b>Extracted claims ({len(claims)})</b>", h3))
                for c_idx, claim in enumerate(claims, start=1):
                    text = str(claim.get("claim", "")).strip()
                    evidence = str(claim.get("evidence_span", "")).strip()
                    src_idx = claim.get("source_citation_index", "?")
                    story.append(
                        Paragraph(f"<b>{c_idx}.</b> {text} <i>[cite {src_idx}]</i>", body)
                    )
                    if evidence:
                        story.append(Paragraph(f"“{_truncate(evidence, 500)}”", quote))

            molecules = synth.get("molecule_candidates", [])
            if molecules:
                story.append(Spacer(1, 0.05 * inch))
                story.append(
                    Paragraph(f"<b>Molecule candidates ({len(molecules)})</b>", h3)
                )
                rows = [["name", "formula", "atoms", "σ bonds", "systems", "role"]]
                for m in molecules:
                    mol = m.get("molecule") or {}
                    rows.append(
                        [
                            _truncate(str(m.get("name", "")), 24),
                            _truncate(_hill_formula(mol), 22),
                            str(len(mol.get("atoms", []) or [])),
                            str(len(mol.get("local_bonds", []) or [])),
                            str(len(mol.get("systems", []) or [])),
                            str(m.get("role", "")),
                        ]
                    )
                mt = Table(
                    rows,
                    colWidths=[
                        1.4 * inch,
                        1.2 * inch,
                        0.5 * inch,
                        0.6 * inch,
                        0.7 * inch,
                        0.9 * inch,
                    ],
                )
                mt.setStyle(
                    TableStyle(
                        [
                            ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
                            ("FONTSIZE", (0, 0), (-1, -1), 8),
                            ("BACKGROUND", (0, 0), (-1, 0), colors.lightgrey),
                            ("GRID", (0, 0), (-1, -1), 0.25, colors.grey),
                            ("VALIGN", (0, 0), (-1, -1), "TOP"),
                            ("LEFTPADDING", (0, 0), (-1, -1), 3),
                            ("RIGHTPADDING", (0, 0), (-1, -1), 3),
                        ]
                    )
                )
                story.append(mt)

            reactions = synth.get("reaction_candidates", [])
            if reactions:
                story.append(Spacer(1, 0.05 * inch))
                story.append(
                    Paragraph(f"<b>Reaction candidates ({len(reactions)})</b>", h3)
                )
                for r in reactions:
                    rxn = r.get("reaction") or {}
                    equation = _format_reaction_equation(rxn)
                    cond_str = _format_conditions(rxn.get("conditions") or []) or "(none)"
                    conf = r.get("confidence", "?")
                    rate = rxn.get("rate", 0.0)
                    story.append(
                        Paragraph(
                            f"<b>equation:</b> {equation}<br/>"
                            f"<b>conditions:</b> {cond_str}<br/>"
                            f"<b>rate:</b> {rate}<br/>"
                            f"<b>confidence:</b> {conf}",
                            body,
                        )
                    )

        story.append(PageBreak())

    doc.build(story)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--corpus-manifest",
        type=Path,
        default=DEFAULT_CORPUS_MANIFEST,
        help="Path to corpus-extract.json",
    )
    parser.add_argument(
        "--cache-root",
        type=Path,
        default=DEFAULT_CACHE_ROOT,
        help="Docling cache root",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=DEFAULT_OUTPUT,
        help="Output PDF path",
    )
    parser.add_argument(
        "--max-figures-per-paper",
        type=int,
        default=4,
        help="Cap on scientific figures embedded per paper (post-filter)",
    )
    args = parser.parse_args()

    manifest = load_corpus_manifest(args.corpus_manifest)
    build_report(
        manifest,
        args.output,
        args.cache_root,
        max_figures_per_paper=args.max_figures_per_paper,
    )
    print(
        f"wrote {args.output} "
        f"({args.output.stat().st_size / 1024:.1f} KiB, "
        f"{len(manifest.get('papers', []))} papers)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
