# Olympus DFT inventory (synced)

Generated: `2026-07-12T18:17:08.663697+00:00`

## Summary counts

| Batch | Count |
|-------|------:|
| Gallery PBE/def2-tzvp (cube-backed) | 6 |
| B3LYP precursors def2-svp | 3 |
| Overnight Ge scalar | 5 |
| Overnight-full result files | 13 |
| Overnight-full unique converged | 7 |

## Gallery (PBE/def2-tzvp, cubes)

| Molecule | E (Ha) | gap (eV) | wall (s) | artifact |
|----------|-------:|---------:|---------:|----------|
| MOLADT.WATER.001 | -76.376421 | 6.964 | 0.17 | `art_3d5c1283b1a8f79f` |
| MOLADT.METHANOL.001 | -115.626291 | 6.025 | 0.68 | `art_563825a02d8ea8a3` |
| MOLADT.RDKIT.GHVNFZFCNZKVNT-UHFFFAOYSA-N | -543.084109 | 5.389 | 523.12 | `art_5d1b8812735b2611` |
| MOLADT.BENZENE.001 | -232.018795 | 5.129 | 8.12 | `art_87a648cd3b5f6490` |
| MOLADT.RDKIT.WWZKQHOCKIZLMA-UHFFFAOYSA-N | -464.544963 | 5.257 | 305.76 | `art_b4002fedd3e69f20` |
| MOLADT.RDKIT.DNIAPMSPPWPWGF-VKHMYHEASA-N | -269.351584 | 6.226 | 7.17 | `art_c1d9cf319fc537e2` |

## Overnight Ge scalar (PBE/def2-svp)

| Path | E (Ha) | gap | artifact |
|------|-------:|----:|----------|
| `demo/overnight-science-out/dft/adamantylgermane/chem_dft_result.art_bb3490ecb173f082.json` | -2467.519022 | 6.336 | `art_bb3490ecb173f082` |
| `demo/overnight-science-out/dft/cyclopropylgermane/chem_dft_result.art_69972a470fe7966c.json` | -2195.248871 | 6.954 | `art_69972a470fe7966c` |
| `demo/overnight-science-out/dft/germane/chem_dft_result.art_72799a3871d01929.json` | -2078.792573 | 8.944 | `art_72799a3871d01929` |
| `demo/overnight-science-out/dft/germatrane/chem_dft_result.art_d1a3d12e5978be50.json` | -2810.512862 | 5.469 | `art_d1a3d12e5978be50` |
| `demo/overnight-science-out/dft/methylgermane/chem_dft_result.art_691b9179ea649f38.json` | -2118.028742 | 7.942 | `art_691b9179ea649f38` |

## Overnight-full unique organics / Ge

| Path | func/basis | E (Ha) | artifact |
|------|------------|-------:|----------|
| `demo/overnight-full-out/dft/cyclopropylgermane/chem_dft_result.art_06259b13c154631b.json` | pbe/def2-svp | -2195.248871 | `art_06259b13c154631b` |
| `demo/overnight-full-out/dft/germane/chem_dft_result.art_4c9525e1e15d5173.json` | pbe/def2-svp | -2078.792573 | `art_4c9525e1e15d5173` |
| `demo/overnight-full-out/dft/methylgermane/chem_dft_result.art_2f4a17d98f12ee76.json` | pbe/def2-svp | -2118.028742 | `art_2f4a17d98f12ee76` |
| `demo/overnight-full-out/dft/propylene-glycol-dibutyrate/chem_dft_result.art_dc304c0ee27ff5e5.json` | pbe/def2-svp | -730.713639 | `art_dc304c0ee27ff5e5` |
| `demo/overnight-full-out/dft/propylene-glycol-dihexanoate/chem_dft_result.art_c186f3052b8156c6.json` | pbe/def2-svp | -887.621428 | `art_c186f3052b8156c6` |
| `demo/overnight-full-out/dft/propylene-glycol-dioctanoate/chem_dft_result.art_83814605e6db0693.json` | b3lyp/def2-svp | -1045.884678 | `art_83814605e6db0693` |
| `demo/overnight-full-out/dft/propylene-glycol-dipentanoate/chem_dft_result.art_8134f4cfe538f0a0.json` | pbe/def2-svp | -809.167705 | `art_8134f4cfe538f0a0` |

## Notes

- Gallery glycol is propylene glycol (1,2-propanediol), not diethylene glycol.
- C8/C10 = caprylic/capric acids in gallery.
- Overnight-full includes propylene-glycol diesters (dibutyrate..dioctanoate).
- Empty overnight-full dirs (stannylene etc.) were planned but not executed.
- MNT Ge XYZ opts are starting geometries; not all have ChimiaClaw-signed SCF yet.
