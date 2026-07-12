# Ge→Sn batch results (Olympus)

Updated: `2026-07-12T20:08:35.378412+00:00`

**7 / 10 converged** · 1 unconverged · 2 missing

| Molecule | status | E (Ha) | gap (eV) | wall (s) |
|----------|--------|-------:|---------:|---------:|
| NC3Sn_H | converged | -2066.416353 | 3.288842549426863 | 241.16790914535522 |
| NC3Sn_Cl | converged | -2525.525087 | 3.979444617459434 | 240.22392296791077 |
| C3_NC3Sn_H | converged | -2065.504249 | 3.2595195436151685 | 230.9944589138031 |
| C3_NC3Sn_Cl | converged | -2524.628703 | 3.589342279825486 | 228.32601308822632 |
| Ad_SnH3 | converged | -2047.533908 | 3.5568291481755274 | 243.0799059867859 |
| Ad_SnCl3 | converged | -3425.198750 | 5.2107243149279725 | 328.8185589313507 |
| Ad_SnMe3 | converged | -2169.969544 | 4.610478377000384 | 557.5479769706726 |
| beta_OH_NC3Sn_H | unconverged | -2409.906837 | 0.014524308298318698 | 1597.9701550006866 |
| beta_SH_NC3Sn_H | pending | — | — | — |
| Ad_stannatrane | pending | — | — | — |

## Notes

- Collect anytime: `python3 tools/ge_sn_batch/collect_results.py`
- Do not wait on this script; it never blocks on SCF.
