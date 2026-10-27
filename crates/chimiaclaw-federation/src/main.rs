use chimiaclaw_federation::{
    create_delegation, create_evidence, create_review, decode_envelope, decode_policy, files,
    verify_package, Audience, Error, IdentityKind, Result, ReviewDecision, MAX_PACKAGE_BYTES,
    MAX_POLICY_BYTES,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "Offline signed evidence exchange (packages are NOT encrypted).
Commands:
  init --id ID --kind human|ai --key NEW_FILE
  import-posyn --bundle DIR --expect ROOT --key FILE --policy FILE --out NEW_FILE [--delegation FILE] [--audience private|synthetic]
  delegate --key FILE --policy FILE --delegate-id AI_ID --expires-at UNIX --out NEW_FILE
  review --package FILE --key FILE --policy FILE --decision reviewed|corrections_required --out NEW_FILE
  verify --package FILE --policy FILE [--evidence FILE]
Key initialization prints only the public identity. Verify/review use locally pinned
policy keys, expected root, exact scope and revocations; no live registry is queried.
Review verification requires --evidence. All files written are create-only mode0600.
";

struct Args(BTreeMap<String, String>);
impl Args {
    fn parse(values: impl Iterator<Item = String>) -> Result<Self> {
        let mut values = values;
        let mut out = BTreeMap::new();
        while let Some(key) = values.next() {
            if !key.starts_with("--") {
                return Err(Error("invalid_arguments"));
            }
            let value = values.next().ok_or(Error("missing_argument_value"))?;
            if value.starts_with("--") || out.insert(key, value).is_some() {
                return Err(Error("invalid_arguments"));
            }
        }
        Ok(Self(out))
    }
    fn take(&mut self, key: &str) -> Result<String> {
        self.0.remove(key).ok_or(Error("missing_required_argument"))
    }
    fn optional(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }
    fn done(&self) -> Result<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(Error("unknown_argument"))
        }
    }
}
fn envelope(path: &str) -> Result<chimiaclaw_federation::Envelope> {
    decode_envelope(&files::read(Path::new(path), MAX_PACKAGE_BYTES, false)?)
}
fn encode(value: &impl serde::Serialize) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|_| Error("output_encoding_failed"))
}

fn run(command: &str, mut args: Args) -> Result<Vec<u8>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error("invalid_clock"))?
        .as_secs();
    if command == "init" {
        let id = args.take("--id")?;
        let kind = match args.take("--kind")?.as_str() {
            "human" => IdentityKind::Human,
            "ai" => IdentityKind::Ai,
            _ => return Err(Error("invalid_identity_kind")),
        };
        let key = args.take("--key")?;
        args.done()?;
        return encode(files::initialize(id, kind, Path::new(&key))?.identity());
    }
    let policy_bytes = files::read(Path::new(&args.take("--policy")?), MAX_POLICY_BYTES, false)?;
    let policy = decode_policy(&policy_bytes)?;
    policy.validate(now)?;
    if command == "verify" {
        let package = envelope(&args.take("--package")?)?;
        let evidence = args
            .optional("--evidence")
            .map(|p| envelope(&p))
            .transpose()?;
        args.done()?;
        return encode(&verify_package(
            &package,
            &policy_bytes,
            now,
            evidence.as_ref(),
        )?);
    }
    let key = files::load_key(Path::new(&args.take("--key")?))?;
    let output = args.take("--out")?;
    let mut review_evidence = None;
    let package = match command {
        "import-posyn" => {
            let bundle = args.take("--bundle")?;
            let expected = args.take("--expect")?;
            if expected != policy.expected_posyn_root {
                return Err(Error("expected_root_mismatch"));
            }
            let delegation = args
                .optional("--delegation")
                .map(|p| envelope(&p))
                .transpose()?;
            let audience = match args.optional("--audience").as_deref().unwrap_or("private") {
                "private" => Audience::Private,
                "synthetic" => Audience::Synthetic,
                _ => return Err(Error("invalid_audience")),
            };
            args.done()?;
            let bundle = chimiaclaw_posyn::import_bundle(Path::new(&bundle), &expected)
                .map_err(|_| Error("invalid_posyn_bundle"))?;
            create_evidence(&key, &policy, bundle, delegation, audience, now)?
        }
        "delegate" => {
            let delegate_id = args.take("--delegate-id")?;
            let expires = args
                .take("--expires-at")?
                .parse()
                .map_err(|_| Error("invalid_expiry"))?;
            args.done()?;
            create_delegation(&key, &policy, delegate_id, expires, now)?
        }
        "review" => {
            let evidence = envelope(&args.take("--package")?)?;
            let decision = match args.take("--decision")?.as_str() {
                "reviewed" => ReviewDecision::Reviewed,
                "corrections_required" => ReviewDecision::CorrectionsRequired,
                _ => return Err(Error("invalid_review_decision")),
            };
            args.done()?;
            let package = create_review(&key, &policy_bytes, &evidence, decision, now)?;
            review_evidence = Some(evidence);
            package
        }
        _ => return Err(Error("unknown_command")),
    };
    let receipt = verify_package(&package, &policy_bytes, now, review_evidence.as_ref())?;
    let package_bytes = encode(&package)?;
    if package_bytes.len() > MAX_PACKAGE_BYTES {
        return Err(Error("output_too_large"));
    }
    files::write_new(Path::new(&output), &package_bytes)?;
    encode(&receipt)
}

fn main() {
    let mut raw = std::env::args().skip(1);
    let Some(command) = raw.next() else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    if command == "--help" || command == "help" {
        print!("{USAGE}");
        return;
    }
    match Args::parse(raw).and_then(|args| run(&command, args)) {
        Ok(bytes) => println!("{}", String::from_utf8_lossy(&bytes)),
        Err(error) => {
            eprintln!("{{\"error\":\"{}\"}}", error.0);
            std::process::exit(1);
        }
    }
}
