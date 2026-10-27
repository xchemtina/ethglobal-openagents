use chimiaclaw_posyn::{
    decode_portable, import_bundle, parse_manifest, verify_portable, Claims, VerifiedBundle,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const ROOT: &str = "blake3:a48f37a331a0949452b6b3535ff7da81cc28ab89e9b2860c13256369404411cd";
static NEXT: AtomicU64 = AtomicU64::new(0);

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/synthetic/bundle")
}

struct CopyBundle(PathBuf);
impl CopyBundle {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "posyn-bridge-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("objects")).unwrap();
        fs::copy(fixture().join("manifest.json"), path.join("manifest.json")).unwrap();
        for entry in fs::read_dir(fixture().join("objects")).unwrap() {
            let entry = entry.unwrap();
            fs::copy(entry.path(), path.join("objects").join(entry.file_name())).unwrap();
        }
        Self(path)
    }
    fn first_object(&self) -> PathBuf {
        fs::read_dir(self.0.join("objects"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
    }
}
impl Drop for CopyBundle {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn portable() -> VerifiedBundle {
    import_bundle(&fixture(), ROOT).unwrap()
}
fn manifest_json(bundle: &VerifiedBundle) -> serde_json::Value {
    serde_json::from_slice(&hex::decode(&bundle.manifest_hex).unwrap()).unwrap()
}
fn replace_manifest(bundle: &mut VerifiedBundle, value: &serde_json::Value) {
    bundle.manifest_hex = hex::encode(serde_json::to_vec(value).unwrap());
}

#[test]
fn native_fixture_roundtrips_exact_bytes_and_refusals() {
    let bundle = portable();
    assert_eq!(
        hex::decode(&bundle.manifest_hex).unwrap(),
        fs::read(fixture().join("manifest.json")).unwrap()
    );
    let manifest = parse_manifest(&bundle).unwrap();
    for object in &bundle.objects {
        assert_eq!(
            hex::decode(&object.bytes_hex).unwrap(),
            fs::read(fixture().join("objects").join(&object.digest[7..])).unwrap()
        );
    }
    let result = manifest
        .payload
        .artifacts
        .iter()
        .find(|a| a.role == chimiaclaw_posyn::Role::Result)
        .unwrap();
    let object = bundle
        .objects
        .iter()
        .find(|o| o.digest == result.digest)
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&hex::decode(&object.bytes_hex).unwrap()).unwrap();
    assert_eq!(
        value["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["status"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["accepted", "unsupported", "rejected", "missing"]
    );
    let verification = verify_portable(&bundle, ROOT).unwrap();
    assert_eq!(verification.total_bytes, 1426);
    assert_eq!(verification.artifact_count, 4);
    assert_eq!(verification.claims, Claims::default());
    assert!(verification.expected_hash_matched);
    assert_eq!(
        decode_portable(&serde_json::to_vec(&bundle).unwrap(), ROOT).unwrap(),
        bundle
    );
}

#[test]
fn separately_held_root_is_mandatory() {
    for root in [
        "",
        "sha256:a48f37a331a0949452b6b3535ff7da81cc28ab89e9b2860c13256369404411cd",
        "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    ] {
        assert!(import_bundle(&fixture(), root).is_err());
        assert!(verify_portable(&portable(), root).is_err());
    }
}

#[test]
fn altered_object_bytes_rejected_on_disk_and_transport() {
    let local = CopyBundle::new();
    let object = local.first_object();
    let mut bytes = fs::read(&object).unwrap();
    bytes[0] ^= 1;
    fs::write(object, bytes).unwrap();
    assert!(import_bundle(&local.0, ROOT).is_err());
    let mut bundle = portable();
    bundle.objects[0].bytes_hex.replace_range(0..2, "00");
    assert!(verify_portable(&bundle, ROOT).is_err());
}

#[test]
fn object_inventory_is_exact_and_ordered() {
    let mut bundle = portable();
    bundle.objects.pop();
    assert!(verify_portable(&bundle, ROOT).is_err());
    let mut bundle = portable();
    bundle.objects.push(bundle.objects[0].clone());
    assert!(verify_portable(&bundle, ROOT).is_err());
    let mut bundle = portable();
    bundle.objects.swap(0, 1);
    assert!(verify_portable(&bundle, ROOT).is_err());
    let local = CopyBundle::new();
    fs::write(local.0.join("objects").join("unexpected"), b"extra").unwrap();
    assert!(import_bundle(&local.0, ROOT).is_err());
    fs::remove_file(local.0.join("objects").join("unexpected")).unwrap();
    fs::remove_file(local.first_object()).unwrap();
    assert!(import_bundle(&local.0, ROOT).is_err());
}

#[test]
fn root_inventory_cannot_hide_auxiliary_files() {
    let local = CopyBundle::new();
    fs::write(local.0.join("README"), b"extra").unwrap();
    assert!(import_bundle(&local.0, ROOT).is_err());
}

#[test]
fn malformed_schema_claims_names_sizes_and_roles_rejected() {
    for pointer in [
        "/payload/schema",
        "/payload/profile",
        "/payload/claims/execution",
        "/payload/claims/reproducibility",
        "/payload/claims/physical_synthesis",
    ] {
        let mut bundle = portable();
        let mut manifest = manifest_json(&bundle);
        *manifest.pointer_mut(pointer).unwrap() = "verified".into();
        replace_manifest(&mut bundle, &manifest);
        assert!(verify_portable(&bundle, ROOT).is_err(), "{pointer}");
    }
    for name in [
        "../secret",
        "absolute/path",
        "",
        "évidence",
        ".hidden",
        "a\\b",
    ] {
        let mut bundle = portable();
        let mut manifest = manifest_json(&bundle);
        manifest["payload"]["artifacts"][0]["name"] = name.into();
        replace_manifest(&mut bundle, &manifest);
        assert!(verify_portable(&bundle, ROOT).is_err(), "{name}");
    }
    for value in [
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!(1_048_577_u64),
        serde_json::json!(0),
    ] {
        let mut bundle = portable();
        let mut manifest = manifest_json(&bundle);
        manifest["payload"]["artifacts"][0]["size_bytes"] = value;
        replace_manifest(&mut bundle, &manifest);
        assert!(verify_portable(&bundle, ROOT).is_err());
    }
    let mut bundle = portable();
    let mut manifest = manifest_json(&bundle);
    manifest["payload"]["artifacts"][0]["role"] = "evidence".into();
    replace_manifest(&mut bundle, &manifest);
    assert!(verify_portable(&bundle, ROOT).is_err());
}

#[test]
fn typed_json_rejects_unknown_and_duplicate_members() {
    let mut bundle = portable();
    let mut manifest = manifest_json(&bundle);
    manifest["payload"]["claims"]["trust"] = true.into();
    replace_manifest(&mut bundle, &manifest);
    assert!(verify_portable(&bundle, ROOT).is_err());
    let mut bundle = portable();
    let raw = String::from_utf8(hex::decode(&bundle.manifest_hex).unwrap()).unwrap();
    bundle.manifest_hex = hex::encode(raw.replacen(
        "\"schema\":",
        "\"schema\":\"posyn.bundle.v1\",\"schema\":",
        1,
    ));
    assert!(verify_portable(&bundle, ROOT).is_err());
    let bundle = portable();
    let raw = serde_json::to_string(&bundle).unwrap();
    let duplicate = raw.replacen(
        "\"schema\":",
        "\"schema\":\"openagents.posyn-bundle.v1\",\"schema\":",
        1,
    );
    assert!(decode_portable(duplicate.as_bytes(), ROOT).is_err());
    let unknown = raw.replacen('{', "{\"extra\":true,", 1);
    assert!(decode_portable(unknown.as_bytes(), ROOT).is_err());
    let duplicate_object = raw.replacen("\"digest\":", "\"digest\":\"ignored\",\"digest\":", 1);
    assert!(decode_portable(duplicate_object.as_bytes(), ROOT).is_err());
}

#[test]
fn resealed_replacement_cannot_pass_original_expected_root() {
    let mut bundle = portable();
    let mut manifest = manifest_json(&bundle);
    // A perfectly coherent replacement of a filename changes the commitment.
    manifest["payload"]["artifacts"][0]["name"] = "replacement.json".into();
    let mut hash = blake3::Hasher::new();
    hash.update(b"ChimiaDAO-PoSyn\0posyn.bundle.v1\0");
    hash.update(&serde_json::to_vec(&manifest["payload"]).unwrap());
    let replacement = format!("blake3:{}", hash.finalize().to_hex());
    manifest["attestation_hash"] = replacement.clone().into();
    replace_manifest(&mut bundle, &manifest);
    assert!(verify_portable(&bundle, &replacement).is_ok());
    assert!(verify_portable(&bundle, ROOT).is_err());
}

#[test]
fn bounded_portable_input_and_noncanonical_hex_rejected() {
    assert!(decode_portable(&vec![b' '; chimiaclaw_posyn::MAX_PORTABLE_BYTES + 1], ROOT).is_err());
    let mut bundle = portable();
    bundle.manifest_hex = "aa".repeat(chimiaclaw_posyn::MAX_MANIFEST_BYTES + 1);
    assert!(verify_portable(&bundle, ROOT).is_err());
    let mut bundle = portable();
    bundle.objects[0].bytes_hex = "AA".into();
    assert!(verify_portable(&bundle, ROOT).is_err());
    let mut bundle = portable();
    bundle.objects[0].bytes_hex = "a".into();
    assert!(verify_portable(&bundle, ROOT).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_bundle_objects_and_manifest_rejected() {
    use std::os::unix::fs::symlink;
    let local = CopyBundle::new();
    let manifest = local.0.join("manifest.json");
    fs::remove_file(&manifest).unwrap();
    symlink(fixture().join("manifest.json"), &manifest).unwrap();
    assert!(import_bundle(&local.0, ROOT).is_err());
    let local = CopyBundle::new();
    let object = local.first_object();
    let source = fixture().join("objects").join(object.file_name().unwrap());
    fs::remove_file(&object).unwrap();
    symlink(source, object).unwrap();
    assert!(import_bundle(&local.0, ROOT).is_err());
    let local = CopyBundle::new();
    fs::remove_dir_all(local.0.join("objects")).unwrap();
    symlink(fixture().join("objects"), local.0.join("objects")).unwrap();
    assert!(import_bundle(&local.0, ROOT).is_err());
    let outer = local.0.join("alias");
    symlink(fixture(), &outer).unwrap();
    assert!(import_bundle(&outer, ROOT).is_err());
}
