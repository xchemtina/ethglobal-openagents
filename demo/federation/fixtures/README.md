# Synthetic terminal-attempt control

`terminal-no-results/` contains **zero produced scientific outputs**. The unchanged
PoSyn v1 profile still requires input, specification, provenance and result roles.
Its result role therefore contains an explicitly typed terminal failure report
named `terminal-failure.json`; the expected scientific `result.json` stays missing.
No computation or laboratory work occurred. The original fixture is unchanged.

Its separately pinned synthetic root is:

```text
blake3:f8bd1eee8d1820c91a92a82d8f54c7227992fac822076f75bd833ebc18d04790
```

It tests that a signed terminal failure preserves missing results while a successful
receipt with zero outputs is refused. The domain-separated root and every byte are
independently checked by the existing importer in `tests/attempt.rs`. No profile
limit or scientific claim was changed.
