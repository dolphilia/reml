//! Config/Data 章で利用する差分マージユーティリティ。
//! `PersistentMap::merge_with_change_set` を公開し、監査ログへ
//! `ChangeSet` を取り込む手続きを補助する。

use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::Value;

use crate::collections::{
    audit_bridge::{AuditBridgeError, ChangeSet},
    persistent::btree::PersistentMap,
};

/// Config マージの結果。新しい `PersistentMap` と `ChangeSet` を保持する。
pub struct ConfigMergeOutcome<K, V> {
    pub merged: PersistentMap<K, V>,
    pub change_set: ChangeSet,
}

impl<K, V> ConfigMergeOutcome<K, V> {
    /// `ChangeSet` を JSON へ変換する。
    pub fn change_set_json(&self) -> Value {
        self.change_set.to_value()
    }

    /// `ChangeSet` を指定したパスへ書き出す。
    pub fn write_change_set(&self, path: impl AsRef<Path>) -> io::Result<()> {
        write_change_set_to_path(&self.change_set, path)
    }
}

/// `PersistentMap::merge_with_change_set` をラップし、Config/Data 用の
/// 結果構造へまとめる。
pub fn merge_maps_with_audit<K, V, F>(
    base: &PersistentMap<K, V>,
    delta: &PersistentMap<K, V>,
    resolver: F,
) -> Result<ConfigMergeOutcome<K, V>, AuditBridgeError>
where
    K: Ord + Clone + Serialize,
    V: Clone + Serialize,
    F: FnMut(&K, &V, &V) -> V,
{
    let (merged, change_set) = base.merge_with_change_set(delta, resolver)?;
    Ok(ConfigMergeOutcome { merged, change_set })
}

/// `ChangeSet` を JSON として保存し、保存先パスを返す。
pub fn write_change_set_to_path(change_set: &ChangeSet, path: impl AsRef<Path>) -> io::Result<()> {
    let value = change_set.to_value();
    // replaced to_value?
    let body = serde_json::to_string_pretty(&value)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    if let Some(parent) = path.as_ref().parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, body)
}

/// `ChangeSet` をテンポラリファイルへ保存し、パスを返す。
pub fn write_change_set_to_temp_dir(change_set: &ChangeSet) -> io::Result<PathBuf> {
    let mut path = std::env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    path.push(format!("reml-config-change-set-{timestamp}.json"));
    write_change_set_to_path(change_set, &path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_map() -> PersistentMap<String, i32> {
        let mut map = PersistentMap::new();
        map = map.insert("alpha".into(), 1);
        map = map.insert("beta".into(), 2);
        map
    }

    #[test]
    fn merge_with_audit_records_changes() {
        let base = sample_map();
        let mut delta = PersistentMap::new();
        delta = delta.insert("beta".into(), 3);
        delta = delta.insert("gamma".into(), 4);
        let outcome =
            merge_maps_with_audit(&base, &delta, |_, _left, right| right.clone()).expect("merge");
        let summary = outcome.change_set.summary();
        assert_eq!(summary.added, 1);
        assert_eq!(summary.updated, 1);
        assert_eq!(summary.removed, 0);
        assert_eq!(outcome.merged.len(), 3);
    }

    #[test]
    fn change_set_is_written_to_disk() {
        let base = sample_map();
        let mut delta = PersistentMap::new();
        delta = delta.insert("beta".into(), 5);
        let outcome =
            merge_maps_with_audit(&base, &delta, |_, _, right| right.clone()).expect("merge");
        let path = write_change_set_to_temp_dir(&outcome.change_set).expect("write");
        let body = fs::read_to_string(&path).expect("read change_set");
        assert!(
            body.contains("collections.diff.map"),
            "expected JSON to include diff metadata: {body}"
        );
        let _ = fs::remove_file(path);
    }
}
