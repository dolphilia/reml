use reml_runtime::io::FsAdapter;
use reml_runtime::runtime::bridge::RuntimeBridgeRegistry;
use serde_json::json;

#[test]
fn stage_records_are_accessible_after_fs_operations() {
    let registry = RuntimeBridgeRegistry::global();
    registry.clear();

    FsAdapter::global()
        .ensure_read_capability()
        .expect("io.fs.read capability should be registered");
    FsAdapter::global()
        .ensure_write_capability()
        .expect("io.fs.write capability should be registered");

    let records = registry.stage_records();
    assert!(
        records
            .iter()
            .any(|record| record.capability == "io.fs.read"),
        "record for io.fs.read should exist"
    );
    assert!(
        records
            .iter()
            .any(|record| record.capability == "io.fs.write"),
        "record for io.fs.write should exist"
    );

    let payload = json!({
      "records": records,
    });
    let pretty = serde_json::to_string_pretty(&payload).unwrap();
    println!("{pretty}");

    if let Ok(path) = std::env::var("BRIDGE_STAGE_RECORDS_PATH") {
        std::fs::write(path, &pretty).expect("failed to write bridge stage records snapshot");
    }
}
