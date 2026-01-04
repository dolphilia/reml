use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value};

use crate::capability::AdapterCapability;

const FS_EFFECT_SCOPE: &[&str] = &["effect {io.blocking}"];

/// ファイル/パス操作の Capability。
pub const FS_CAPABILITY: AdapterCapability = AdapterCapability::new(
    "adapter.fs",
    "stable",
    FS_EFFECT_SCOPE,
    "adapter.fs",
);

/// 指定パスから文字列を読み取る（`effect {io.blocking}`）。
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    fs::read_to_string(path)
}

/// パスを正規化するラッパー。
pub fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    fs::canonicalize(path)
}

/// ディレクトリを再帰的に作成する。
pub fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(path)
}

/// アダプタ共有の監査メタデータを生成する。
pub fn audit_metadata(operation: &str, status: &str) -> Map<String, Value> {
    FS_CAPABILITY.audit_metadata(operation, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, io::Write, time};

    #[test]
    fn audit_metadata_contains_expected_fields() {
        let metadata = audit_metadata("read", "success");
        assert_eq!(metadata["capability.id"], "adapter.fs");
        assert_eq!(metadata["capability.stage"], "stable");
        assert_eq!(metadata["adapter.fs.operation"], "read");
        assert_eq!(metadata["adapter.fs.status"], "success");
    }

    #[test]
    fn read_to_string_roundtrip() {
        let mut path = env::temp_dir();
        let unique = time::SystemTime::now()
            .duration_since(time::SystemTime::UNIX_EPOCH)
            .expect("time flows")
            .as_nanos();
        path.push(format!("reml_adapter_fs_{unique}.txt"));
        let mut file = fs::File::create(&path).expect("temp file");
        write!(file, "adapter").expect("write data");
        file.sync_all().expect("sync");
        let content = read_to_string(&path).expect("read back");
        assert_eq!(content, "adapter");
        fs::remove_file(&path).expect("cleanup");
    }
}
