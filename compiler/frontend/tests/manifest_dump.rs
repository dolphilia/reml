use reml_runtime::config::Manifest;
use std::fs;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/manifest")
}

#[test]
fn manifest_dump_matches_golden() {
    let dir = fixtures_dir();
    let manifest_path = dir.join("sample.reml.toml");
    let expected_path = dir.join("sample.dump.json");
    let raw = fs::read_to_string(&manifest_path).expect("manifest fixture");
    let manifest = Manifest::parse_toml(&raw)
        .expect("manifest parse")
        .with_manifest_path(&manifest_path);
    let actual = serde_json::to_string_pretty(&manifest).expect("serialize manifest") + "\n";
    let expected = fs::read_to_string(&expected_path).expect("golden json");
    assert_eq!(
        actual, expected,
        "manifest dump does not match the golden JSON"
    );
}
