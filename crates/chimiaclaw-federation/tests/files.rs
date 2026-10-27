#![cfg(unix)]
use chimiaclaw_federation::{files, IdentityKind};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::PathBuf;

struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        let mut nonce = [0u8; 16];
        getrandom::getrandom(&mut nonce).unwrap();
        let path = std::env::temp_dir().join(format!("federation-test-{}", hex::encode(nonce)));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn fresh_os_keys_are_distinct_private_and_never_overwritten() {
    let temp = Temporary::new();
    let first = temp.0.join("first.key");
    let second = temp.0.join("second.key");
    let a = files::initialize("alice".into(), IdentityKind::Human, &first).unwrap();
    let b = files::initialize("alice".into(), IdentityKind::Human, &second).unwrap();
    assert_ne!(a.identity().public_key, b.identity().public_key);
    assert_eq!(
        fs::metadata(&first).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(files::load_key(&first).unwrap().identity(), a.identity());
    let before = fs::read(&first).unwrap();
    assert!(files::initialize("mallory".into(), IdentityKind::Human, &first).is_err());
    assert_eq!(fs::read(&first).unwrap(), before);
}

#[test]
fn symlink_and_readable_secret_files_are_rejected() {
    let temp = Temporary::new();
    let key = temp.0.join("secret");
    let link = temp.0.join("link");
    files::initialize("alice".into(), IdentityKind::Human, &key).unwrap();
    symlink(&key, &link).unwrap();
    assert!(files::load_key(&link).is_err());
    fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(files::load_key(&key).is_err());
    assert!(files::write_new(&link, b"replace").is_err());
}

#[test]
fn output_packages_are_private_and_input_reads_bounded() {
    let temp = Temporary::new();
    let file = temp.0.join("evidence.json");
    files::write_new(&file, b"package").unwrap();
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(files::read(&file, 3, false).is_err());
    assert_eq!(files::read(&file, 10, false).unwrap(), b"package");
}
