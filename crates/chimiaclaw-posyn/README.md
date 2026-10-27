# PoSyn evidence import

This crate imports the existing `posyn.bundle.v1` / `artifact-integrity.v1`
format into a portable, byte-preserving OpenAgents container. It is an independent
implementation of the wire profile, with conservative limits for the first
offline collaboration pilot. It does not copy or execute the PoSyn chemistry
pipeline.

```rust,ignore
let bundle = chimiaclaw_posyn::import_bundle(path, separately_held_root)?;
let json = serde_json::to_vec(&bundle)?;
// Recipient uses its own trusted commitment, not a digest beside the package.
let received = chimiaclaw_posyn::decode_portable(&json, recipient_expected_root)?;
let receipt = chimiaclaw_posyn::verify_portable(&received, recipient_expected_root)?;
```

The `VerifiedBundle` transport contains the exact manifest bytes and every unique
object as lowercase hex. Its name is not a trust marker after deserialization:
receivers must verify it again. The enclosing federation artifact adds signing,
issuer identity, scope and local trust policy. The PoSyn root, OpenAgents content
hash and any PoX SHA-256 identity remain separate values.

The parser rejects duplicate or unknown manifest/container fields, unsupported
profiles or promoted claims, unsafe names, unexpected inventories, symlinked
files/directories, sizes/digests that disagree, and replacement bundles that do
not match the separately held root. It never interprets object bytes, launches a
process, follows embedded links, extracts archives or changes source files.
Result records including accepted, unsupported, rejected and missing identifiers
are transported without conversion. This does not validate the semantics of those
records or establish chemical identity.

Limits are 128 artifact records, 1 MiB per object, 8 MiB aggregate referenced
bytes, 256 KiB manifest bytes, and 18 MiB portable JSON. Larger valid upstream
bundles are deliberately outside this pilot profile. Import directories and
their parents must be operator-owned and stable during import; this is not a
filesystem sandbox against an actor who can replace ancestor directories.

The immutable scientific claims remain:

- `execution: not_verified`
- `reproducibility: not_checked`
- `physical_synthesis: not_attested`

`fixtures/synthetic/` is wholly hand-authored synthetic evidence. Its four files
were sealed and independently verified with the actual PoSyn CLI on 27 October
2026. It contains no ORD record, private laboratory data or authentic computation.
The expected root is:

```text
blake3:a48f37a331a0949452b6b3535ff7da81cc28ab89e9b2860c13256369404411cd
```

Run `cargo test -p chimiaclaw-posyn --locked`. To cross-check with an installed
PoSyn CLI:

```sh
posyn verify crates/chimiaclaw-posyn/fixtures/synthetic/bundle \
  --expect blake3:a48f37a331a0949452b6b3535ff7da81cc28ab89e9b2860c13256369404411cd
```

An embedded fixture root is appropriate only for this public synthetic fixture.
For real exchanges, obtain the expected commitment through the agreed trusted
channel and pin it in the recipient's policy before accepting the package.
