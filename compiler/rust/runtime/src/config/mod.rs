//! Config/Data 章で利用する差分マージユーティリティと
//! `reml.toml` マニフェスト・互換設定型を提供する。
//! `PersistentMap::merge_with_change_set` を公開し、監査ログへ
//! `ChangeSet` を取り込む手続きを補助する。
pub mod collection_diff;
pub mod compat;
pub mod manifest;
#[cfg(feature = "experimental_migration")]
pub mod migration;

pub use collection_diff::{ChangeKind, ConfigChange, SchemaDiff, SchemaDiffMetadata};
pub use compat::{
    compatibility_profile, compatibility_profile_for_stage, compatibility_violation_diagnostic,
    resolve_compat, CommentPair, CompatibilityDiagnosticBuilder, CompatibilityLayer,
    CompatibilityProfile, CompatibilityProfileError, CompatibilityViolationKind,
    ConfigCompatibility, ConfigCompatibilitySource, ConfigFormat, ConfigTriviaProfile,
    DuplicateKeyPolicy, KeyPolicy, NumberCompatibility, ResolveCompatOptions,
    ResolvedConfigCompatibility, TrailingCommaMode, CONFIG_COMPAT_DUPLICATE_KEY_CODE,
    CONFIG_COMPAT_NUMBER_CODE, CONFIG_COMPAT_TRAILING_COMMA_CODE, CONFIG_COMPAT_UNQUOTED_KEY_CODE,
};
pub use manifest::{
    declared_effects, ensure_schema_version_compatibility, load_manifest, update_dsl_signature,
    validate_manifest, CapabilityId, ConfigCompatibilityEntry, ConfigRoot, Contact, DependencySpec,
    DslEntry, DslExportRef, DslExportSignature, DslSignatureStageBounds, Manifest, ManifestBuilder,
    ManifestCapabilities, ManifestCapabilityError, ManifestLoader, ManifestParseError,
    OptimizeLevel, PackageName, ProjectKind, ProjectSection, ProjectStage, RegistrySection,
    RunCapabilityEntry, RunSection, RunTargetSection, SemanticVersion, TargetTriple,
};
#[cfg(feature = "experimental_migration")]
pub use migration::{
    MigrationDuration, MigrationPlan, MigrationRiskLevel, MigrationStep, ReorganizationStrategy,
    TypeConversionPlan, MIGRATION_EFFECT_TAG,
};

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::Value;

use crate::collections::{
    audit_bridge::{AuditBridgeError, ChangeSet},
    persistent::btree::PersistentMap,
};

static CHANGE_SET_SEQ: AtomicU64 = AtomicU64::new(0);

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
    fs::write(path, body)?;
    Ok(())
}

/// `ChangeSet` をテンポラリファイルへ保存し、パスを返す。
pub fn write_change_set_to_temp_dir(change_set: &ChangeSet) -> io::Result<PathBuf> {
    let mut path = std::env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let seq = CHANGE_SET_SEQ.fetch_add(1, Ordering::Relaxed);
    path.push(format!("reml-config-change-set-{timestamp}-{seq}.json"));
    write_change_set_to_path(change_set, &path)?;
    Ok(path)
}

/// `ChangeSet` を一時ファイルへ書き出し、CLI に読み込ませるための環境変数を設定する。
///
/// 返却値の `CollectionsChangeSetEnv` は `Drop` で `REML_COLLECTIONS_CHANGE_SET_PATH`
/// をクリアし、生成された一時ファイルの削除も試みる。CLI を起動する間この値を保持し、
/// 終了後に `drop` することでクリーンアップできる。
pub fn set_collections_change_set_env(
    change_set: &ChangeSet,
) -> io::Result<CollectionsChangeSetEnv> {
    CollectionsChangeSetEnv::new(change_set)
}

/// `REML_COLLECTIONS_CHANGE_SET_PATH` を管理するハンドル。
pub struct CollectionsChangeSetEnv {
    path: PathBuf,
}

impl CollectionsChangeSetEnv {
    fn new(change_set: &ChangeSet) -> io::Result<Self> {
        let path = write_change_set_to_temp_dir(change_set)?;
        let path_str = path.display().to_string();
        std::env::set_var("REML_COLLECTIONS_CHANGE_SET_PATH", &path_str);
        Ok(Self { path })
    }

    /// 書き出された JSON ファイルのパスを返す。
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// 内部で設定した環境変数を取り除き、パスだけを取得する。
    pub fn into_path(self) -> PathBuf {
        let path = self.path.clone();
        std::mem::forget(self);
        path
    }
}

impl Drop for CollectionsChangeSetEnv {
    fn drop(&mut self) {
        let _ = std::env::remove_var("REML_COLLECTIONS_CHANGE_SET_PATH");
        let _ = fs::remove_file(&self.path);
    }
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

    #[test]
    fn collections_change_set_env_sets_variable() {
        let base = sample_map();
        let mut delta = PersistentMap::new();
        delta = delta.insert("gamma".into(), 7);
        let outcome =
            merge_maps_with_audit(&base, &delta, |_, _left, right| right.clone()).expect("merge");
        let env_key = "REML_COLLECTIONS_CHANGE_SET_PATH";
        let guard = set_collections_change_set_env(&outcome.change_set).expect("set env");
        let env_value = std::env::var(env_key).expect("env set");
        assert_eq!(env_value, guard.path().display().to_string());
        drop(guard);
        assert!(std::env::var(env_key).is_err());
    }
}
