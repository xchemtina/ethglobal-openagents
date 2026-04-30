//! Pure-Rust 2D SVG depiction for [`MoleculeAdt`].
//!
//! The renderer projects atoms onto the XY plane (it does not attempt a real
//! 2D layout — coordinates are taken as-is from the MolADT) and draws CPK-
//! coloured circles for atoms plus straight lines for bonds. The result is
//! deterministic byte-for-byte for a given input molecule, which keeps the
//! signed `chem.molecule.adt` artifact and any rendered child artifact in
//! agreement when the renderer is exercised inside the runtime.

use std::collections::BTreeSet;

use crate::{Atom, AtomicSymbol, Edge, MoleculeAdt};

/// Options that control the SVG output.
#[derive(Clone, Debug)]
pub struct SvgRenderOptions {
    /// Pixels per Angstrom in the rendered diagram.
    pub scale_pixels_per_angstrom: f64,
    /// Pixel padding around the bounding box.
    pub padding_pixels: f64,
    /// Pixel radius used to draw atoms (heavy atoms use this; H atoms use
    /// 0.6x to give them visual weight without dominating).
    pub atom_radius_pixels: f64,
    /// Pixel width of bond lines.
    pub bond_width_pixels: f64,
    /// If true, hydrogen atoms are drawn as small circles without labels.
    pub compact_hydrogens: bool,
    /// If true, write element symbols inside non-hydrogen atoms.
    pub label_atoms: bool,
}

impl Default for SvgRenderOptions {
    fn default() -> Self {
        Self {
            scale_pixels_per_angstrom: 60.0,
            padding_pixels: 24.0,
            atom_radius_pixels: 12.0,
            bond_width_pixels: 2.0,
            compact_hydrogens: true,
            label_atoms: true,
        }
    }
}

/// Render a molecule to a deterministic SVG string.
#[must_use]
pub fn render_svg(molecule: &MoleculeAdt, options: &SvgRenderOptions) -> String {
    let positions = projected_positions(molecule);
    let (min_x, max_x, min_y, max_y) = bounding_box(&positions);
    let scale = options.scale_pixels_per_angstrom;
    let pad = options.padding_pixels;
    let width = ((max_x - min_x) * scale + pad * 2.0).max(48.0);
    let height = ((max_y - min_y) * scale + pad * 2.0).max(48.0);

    let to_pixel = |x: f64, y: f64| -> (f64, f64) {
        let px = (x - min_x) * scale + pad;
        // Flip y so that positive y goes up in the SVG.
        let py = (max_y - y) * scale + pad;
        (px, py)
    };

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {:.1} {:.1}\" width=\"{:.0}\" height=\"{:.0}\">",
        width, height, width, height
    ));
    svg.push_str(&format!("<title>{}</title>", escape_xml(&molecule.name)));
    svg.push_str(&format!(
        "<desc>chimiaclaw-moladt SVG render of {} (formula {}, source_kind {})</desc>",
        escape_xml(&molecule.molecule_id),
        escape_xml(&molecule.formula()),
        escape_xml(&molecule.provenance.source_kind),
    ));
    svg.push_str("<rect x=\"0\" y=\"0\" width=\"100%\" height=\"100%\" fill=\"white\"/>");

    // Bonds first so atoms render on top of them.
    let bonds_sorted: Vec<&Edge> = {
        let mut bonds: Vec<&Edge> = molecule.local_bonds.iter().collect();
        bonds.sort();
        bonds
    };
    for edge in bonds_sorted {
        if let (Some(pa), Some(pb)) = (positions.get(&edge.a), positions.get(&edge.b)) {
            let (x1, y1) = to_pixel(pa.0, pa.1);
            let (x2, y2) = to_pixel(pb.0, pb.1);
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#444\" stroke-width=\"{:.2}\" stroke-linecap=\"round\"/>",
                x1, y1, x2, y2, options.bond_width_pixels
            ));
        }
    }

    let aromatic_atoms: BTreeSet<u32> = molecule
        .systems
        .iter()
        .filter(|system| {
            system
                .tag
                .as_deref()
                .is_some_and(|tag| tag.contains("aromatic") || tag.contains("ring"))
        })
        .flat_map(|system| {
            system
                .member_edges
                .iter()
                .flat_map(|edge| [edge.a, edge.b].into_iter())
        })
        .collect();

    let mut atom_ids: Vec<u32> = molecule.atoms.keys().copied().collect();
    atom_ids.sort_unstable();
    for atom_id in atom_ids {
        let atom = &molecule.atoms[&atom_id];
        let pos = positions[&atom_id];
        let (cx, cy) = to_pixel(pos.0, pos.1);
        let is_hydrogen = matches!(atom.attributes.symbol, AtomicSymbol::H);
        let radius = if is_hydrogen {
            options.atom_radius_pixels * 0.55
        } else {
            options.atom_radius_pixels
        };
        let (fill, stroke, label_color) = cpk_colors(&atom.attributes.symbol);
        if is_hydrogen && options.compact_hydrogens {
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                cx, cy, radius, fill, stroke
            ));
            continue;
        }
        let aromatic_marker = aromatic_atoms.contains(&atom_id);
        let aromatic_attr = if aromatic_marker {
            " stroke-dasharray=\"3,2\""
        } else {
            ""
        };
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{}/>",
            cx, cy, radius, fill, stroke, aromatic_attr
        ));
        if options.label_atoms {
            svg.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter, Helvetica, Arial, sans-serif\" font-size=\"{:.1}\" fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"central\">{}</text>",
                cx,
                cy,
                radius * 0.95,
                label_color,
                escape_xml(atom.attributes.symbol.as_str())
            ));
        }
    }

    let provenance = format!(
        "molecule_id={} | formula={} | source_kind={}",
        molecule.molecule_id,
        molecule.formula(),
        molecule.provenance.source_kind,
    );
    svg.push_str(&format!(
        "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"Inter, Helvetica, Arial, sans-serif\" font-size=\"10\" fill=\"#666\" text-anchor=\"start\">{}</text>",
        pad,
        height - pad * 0.4,
        escape_xml(&provenance)
    ));
    svg.push_str("</svg>");
    svg
}

/// Write an SVG render of `molecule` to disk.
pub fn write_svg_to(
    molecule: &MoleculeAdt,
    options: &SvgRenderOptions,
    path: impl AsRef<std::path::Path>,
) -> Result<(), crate::MolAdtError> {
    let svg = render_svg(molecule, options);
    std::fs::write(path, svg).map_err(|error| crate::MolAdtError::Io(error.to_string()))
}

fn projected_positions(molecule: &MoleculeAdt) -> std::collections::BTreeMap<u32, (f64, f64)> {
    molecule
        .atoms
        .iter()
        .map(|(id, atom)| (*id, atom_pixel_2d(atom)))
        .collect()
}

fn atom_pixel_2d(atom: &Atom) -> (f64, f64) {
    (atom.coordinate.x_angstrom, atom.coordinate.y_angstrom)
}

fn bounding_box(positions: &std::collections::BTreeMap<u32, (f64, f64)>) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in positions.values() {
        if *x < min_x {
            min_x = *x;
        }
        if *x > max_x {
            max_x = *x;
        }
        if *y < min_y {
            min_y = *y;
        }
        if *y > max_y {
            max_y = *y;
        }
    }
    if !min_x.is_finite() || !max_x.is_finite() {
        min_x = -1.0;
        max_x = 1.0;
    }
    if !min_y.is_finite() || !max_y.is_finite() {
        min_y = -1.0;
        max_y = 1.0;
    }
    if (max_x - min_x).abs() < 1.0e-6 {
        min_x -= 0.5;
        max_x += 0.5;
    }
    if (max_y - min_y).abs() < 1.0e-6 {
        min_y -= 0.5;
        max_y += 0.5;
    }
    (min_x, max_x, min_y, max_y)
}

fn cpk_colors(symbol: &AtomicSymbol) -> (&'static str, &'static str, &'static str) {
    match symbol {
        AtomicSymbol::H => ("#f7f7f7", "#999999", "#222222"),
        AtomicSymbol::B => ("#ffb5b5", "#a04040", "#222222"),
        AtomicSymbol::C => ("#404040", "#202020", "#ffffff"),
        AtomicSymbol::N => ("#3050f8", "#1d2f8f", "#ffffff"),
        AtomicSymbol::O => ("#ff0d0d", "#7c0707", "#ffffff"),
        AtomicSymbol::F => ("#90e050", "#4d7c2c", "#222222"),
        AtomicSymbol::Na => ("#ab5cf2", "#5b248d", "#ffffff"),
        AtomicSymbol::P => ("#ff8000", "#a04d00", "#ffffff"),
        AtomicSymbol::S => ("#ffff30", "#a0a020", "#222222"),
        AtomicSymbol::Cl => ("#1ff01f", "#0a7d0a", "#222222"),
        AtomicSymbol::Fe => ("#e06633", "#7e3517", "#ffffff"),
        AtomicSymbol::Br => ("#a62929", "#601515", "#ffffff"),
        AtomicSymbol::I => ("#940094", "#480048", "#ffffff"),
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library;

    #[test]
    fn renders_water_to_svg() {
        let svg = render_svg(&library::water(), &SvgRenderOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<line"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("MOLADT.WATER.001"));
        assert!(svg.contains("H2O") || svg.contains("water"));
    }

    #[test]
    fn renders_benzene_aromatic_marker() {
        let svg = render_svg(&library::benzene(), &SvgRenderOptions::default());
        assert!(svg.contains("stroke-dasharray=\"3,2\""));
        assert!(svg.contains("C6H6"));
    }

    #[test]
    fn render_is_deterministic() {
        let molecule = library::biphenyl();
        let options = SvgRenderOptions::default();
        let first = render_svg(&molecule, &options);
        let second = render_svg(&molecule, &options);
        assert_eq!(first, second);
    }
}
