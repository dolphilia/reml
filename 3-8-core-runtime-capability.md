# 3.8 Core Runtime & Capability Registry（フェーズ3 ドラフト）

Status: Draft（内部レビュー中）

> 目的：Reml ランタイムの能力（GC、メトリクス、監査、プラグイン）を統一的に管理する `Capability Registry` を定義し、標準ライブラリ各章から利用できる公式 API を提供する。

## 0. ドラフトメタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | Draft（フェーズ3） |
| 効果タグ | `@pure`, `effect {runtime}`, `effect {audit}`, `effect {unsafe}` |
| 依存モジュール | `Core.Prelude`, `Core.Diagnostics`, `Core.Numeric & Time`, `Core.IO`, `Core.Config` |
| 相互参照 | [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) |

## 1. Capability Registry の基本構造

```reml
pub type CapabilityId = Str

pub struct CapabilityRegistry {
  gc: Option<GcCapability>,
  io: IoCapability,
  audit: AuditCapability,
  metrics: MetricsCapability,
  plugins: PluginCapability,
}

fn registry() -> &'static CapabilityRegistry                  // `effect {runtime}`
fn register(cap: CapabilityId, value: CapabilityHandle) -> Result<(), CapabilityError> // `effect {runtime}`
fn get(cap: CapabilityId) -> Option<CapabilityHandle>          // `effect {runtime}`
```

- `CapabilityHandle` は実装依存のポインタ/関数テーブルをラップする型（不透明指針）。
- `register` は起動時に呼び出され、重複登録時は `CapabilityError::AlreadyRegistered` を返す。

### 1.1 CapabilityError

```reml
pub type CapabilityError = {
  kind: CapabilityErrorKind,
  message: Str,
}

pub enum CapabilityErrorKind = AlreadyRegistered | NotFound | InvalidHandle | UnsafeViolation
```

- `InvalidHandle` は型不一致や ABI 不整合を検出した際に報告する。
- `UnsafeViolation` は `effect {unsafe}` 経由でのみ返される。

## 2. GC Capability インターフェイス

Chapter 2.9 のドラフトを正式化する。

```reml
pub type GcCapability = {
  configure: fn(GcConfig) -> Result<(), CapabilityError>;
  register_root: fn(RootSet) -> Result<(), CapabilityError>;
  unregister_root: fn(RootSet) -> Result<(), CapabilityError>;
  write_barrier: fn(ObjectRef, FieldRef) -> Result<(), CapabilityError>;
  metrics: fn() -> Result<GcMetrics, CapabilityError>;
  trigger: fn(GcReason) -> Result<(), CapabilityError>;
}
```

- すべて `Result` を返し、失敗時は `CapabilityError` にラップする。
- `GcMetrics` は [3.4](3-4-core-numeric-time.md) の `MetricPoint` と互換のフィールド構造を持つ。

## 3. Metrics & Audit Capability

```reml
pub type MetricsCapability = {
  emit: fn(MetricPoint<Float>) -> Result<(), CapabilityError>,
  list: fn() -> Result<List<MetricDescriptor>, CapabilityError>,
}

pub type AuditCapability = {
  emit: fn(Diagnostic) -> Result<(), CapabilityError>,       // `effect {audit}`
  status: fn() -> Result<AuditStatus, CapabilityError>,
}
```

- `MetricDescriptor` は登録済みメトリクスのメタデータ（名前、型、説明）。
- `AuditStatus` は監査シンクの状態（接続/遅延/停止）を表す。

## 4. IO Capability

```reml
pub type IoCapability = {
  open: fn(Path, FileOptions) -> Result<File, CapabilityError>,
  read: fn(File, Bytes) -> Result<usize, CapabilityError>,
  write: fn(File, Bytes) -> Result<usize, CapabilityError>,
  close: fn(File) -> Result<(), CapabilityError>,
}
```

- 3.5 の同期 IO API が内部で利用するバックエンドとして定義。
- 実装は OS ごとに差し替え可能。

## 5. プラグイン Capability

```reml
pub type PluginCapability = {
  register: fn(PluginMetadata) -> Result<(), CapabilityError>,
  verify_signature: fn(PluginMetadata) -> Result<(), CapabilityError>,
  load: fn(Path) -> Result<PluginHandle, CapabilityError>,
}

pub type PluginMetadata = {
  id: Str,
  version: SemVer,
  capabilities: List<CapabilityId>,
  signature: Option<Bytes>,
}
```

- `SemVer` と `PluginHandle` は将来のプラグイン拡張章（予定）と整合する。
- `verify_signature` は 3.6 の監査モジュールと連携して署名検証結果をログ化する。

## 6. 使用例（GC + Metrics 登録）

```reml
use Core;
use Core.Runtime;
use Core.Numeric;

fn bootstrap_runtime() -> Result<(), CapabilityError> =
  register("gc", CapabilityHandle::Gc(my_gc_capability()))?;
  register("metrics", CapabilityHandle::Metrics(my_metrics_capability()))?;
  Ok(())

fn collect_gc_metrics() -> Result<MetricPoint<Float>, CapabilityError> =
  let metrics = registry().metrics.metrics()?;
  Ok(metric_point("gc.pause_ms", metrics.last_pause_ms))
```

- 起動時に `gc` と `metrics` を登録し、`registry()` 経由で取得可能とする。
- 取得したメトリクスは Chapter 3.4 の `metric_point` を再利用して監査へ送出する。

> 関連: [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md)

> 注意: 本章は 2.9 実行時基盤ドラフトの内容を Chapter 3 に移行し、正式化したものです。
