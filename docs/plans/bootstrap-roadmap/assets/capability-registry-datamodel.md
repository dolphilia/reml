# Capability Registry データモデル（Run ID: 20290622-capability-registry-register）

`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#2.3` で定義した「`register`/`get`/`describe` API」「CapabiltyError 列挙体」の実装結果を整理するメモ。`compiler/runtime/src/capability/registry.rs` の構造と `docs/plans/bootstrap-roadmap/assets/capability-error-matrix.csv` の更新内容を突き合わせ、今後 `verify_capability_stage` / 監査統合を拡張する際の土台とする。

## 1. データ構造
- `CapabilityRegistry` の内部は `RwLock<CapabilityEntries>` を保持し、実行時登録をスレッド安全に行う（参照: `registry.rs` L18-L82）。
- `CapabilityEntries` は 2 つのインデックスを管理する:
  1. `HashMap<CapabilityId, CapabilityEntry>`: Capability ID → `CapabilityEntry`（`CapabilityDescriptor` と `CapabilityHandle`）を保持し、`get`/`describe`/`verify_capability_stage` が参照する。
  2. `Vec<CapabilityId>`: 登録順の ID リスト。`describe_all()` がこの順序で `CapabilityDescriptor` を複製して返し、将来の CLI (`reml capability list`) やドキュメント自動生成（3.8 plan §4.3）で利用する。
- `CapabilityEntry` は登録時点の Descriptor/Handle をそのまま保持する。Descriptor を複製しているのは、将来 `CapabilityHandle` が Interior Mutability を持つ可能性を考慮し、公開メタデータを `describe()` から安全に返すため。

```
┌──────────────────┐
│ CapabilityRegistry│
│  entries: RwLock  │
└────────┬─────────┘
         │
┌────────▼──────────┐
│ CapabilityEntries  │
│  ordered_keys: Vec │
│  entries: HashMap  │
└───┬───────────────┘
    │ CapabilityId
┌───▼────────────────────┐
│ CapabilityEntry        │
│  descriptor            │
│  handle (CapabilityHandle) │
└────────────────────────┘
```

## 2. `register` / `get` / `describe` の流れ
1. `CapabilityRegistry::register(handle: CapabilityHandle)`  
   - `handle.descriptor()` を複製し `CapabilityEntry` を生成。  
   - 既存 ID が `HashMap` に存在した場合は `CapabilityError::AlreadyRegistered` を返す（`error_matrix` のステータスを更新済み）。
2. `CapabilityRegistry::get(capability: &str)`  
   - `HashMap` から `CapabilityEntry` を取得し、`CapabilityHandle` を Clone して返す。  
   - 未登録時は `CapabilityError::NotRegistered`。
3. `CapabilityRegistry::describe(capability: &str)`  
   - 登録済みの Descriptor を Clone して返す。  
   - `describe_all()` は `ordered_keys` を順にたどり Descriptor を複製する。

## 3. CapabilityError 列挙体
- `thiserror::Error` を使用し `AlreadyRegistered` / `NotRegistered` / `StageViolation` を実装。`code()` と `detail()` を従来どおり公開し、既存の診断コード参照（例: `effects.contract.stage_mismatch`）を維持した。
- `StageViolation` は既存の PoC 挙動（未登録 Capability → `StageId::Stable` とみなす）を保持しつつ、`actual_stage()` で監査側へ Stage 情報を渡す。`EffectScopeMismatch` など 3.x 節の追加バリアントは TODO。
- `impl From<CapabilityError> for RuntimeError` については `RuntimeError` 型がまだ Rust 実装に存在しないため未着手。`docs/plans/rust-migration/p2-runtime-gap-report.md#P2G-03` のフォローアップに TODO を追加する予定。

## 4. 既存 API への影響と TODO
- `verify_capability_stage()` は登録済み Descriptor があればそれを参照し、未登録の場合は従来どおり Stable として扱うフォールバックを残した（Phase 3.2 で `unknown → error` へ切り替える計画）。  
- `FsAdapter`/`WatcherAdapter`/`MetricsStageGuard` 等の PoC コードは `verify_stage_for_io()` を介して新データモデルに移行済みで、既存の Stage キャッシュ (`OnceCell<StageId>`) と互換性を保っている。
- 未対応項目:
  - `EffectScopeMismatch` / `MissingDependency` / `ContractViolation` バリアントの実装（3.2-3.3 スプリントで対応）。
  - `describe_all()` を CLI/ドキュメント生成に接続するタスク（3.8 plan §4.3）。
  - `RuntimeError` への `From` 実装。Rust Runtime に `RuntimeError` 型が導入され次第、`CapabilityError` から診断コード/Stage/Descriptor を橋渡しする。

## 5. 参照リンク
- 実装: `compiler/runtime/src/capability/registry.rs`
- テスト: `compiler/runtime/tests/capability_registry.rs`
- エラー行列更新: `docs/plans/bootstrap-roadmap/assets/capability-error-matrix.csv`
