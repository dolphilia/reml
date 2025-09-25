# 3.8 Core Runtime & Capability Registry

> 目的：Reml ランタイムの能力（GC、メトリクス、監査、プラグイン）を統一的に管理する `Capability Registry` を定義し、標準ライブラリ各章から利用できる公式 API を提供する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {runtime}`, `effect {audit}`, `effect {unsafe}`, `effect {security}` |
| 依存モジュール | `Core.Prelude`, `Core.Diagnostics`, `Core.Numeric & Time`, `Core.IO`, `Core.Config` |
| 相互参照 | [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3.9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md) |

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
- セキュリティ強化のため、Capability は署名検証とアクセス制御をサポートする。

### 1.2 セキュリティモデル

```reml
pub type CapabilitySecurity = {
  signature: Option<DigitalSignature>,
  permissions: Set<Permission>,
  isolation_level: IsolationLevel,
  audit_required: Bool,
}

pub enum Permission = {
  ReadConfig,
  WriteConfig,
  FileSystem(PathPattern),
  Network(NetworkPattern),
  Runtime(RuntimeOperation),
}

pub enum IsolationLevel = None | Sandboxed | FullIsolation

fn verify_capability_security(handle: CapabilityHandle, security: CapabilitySecurity) -> Result<(), SecurityError>
```

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
- `SecurityViolation` はアクセス制御違反や不正なケーパビリティ操作時に発生する。

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
- GC 操作は監査ログに記録され、パフォーマンス監視とデバッグを支援する。

### 2.1 メモリ管理の高度制御

```reml
fn configure_gc_advanced(config: AdvancedGcConfig) -> Result<(), CapabilityError>;

pub type AdvancedGcConfig = {
  heap_size_limit: Option<usize>,
  collection_frequency: GcFrequency,
  concurrent_collection: Bool,
  memory_pressure_threshold: Float,
  debug_mode: Bool,
}

pub enum GcFrequency = Aggressive | Normal | Conservative | Manual
```

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


### 3.1 DSLメトリクス連携

- Conductor で宣言された DSL ID ごとに `register_dsl_metrics` を呼び出し、`MetricsCapability.emit` を通じて `dsl.latency` などのメトリクスを登録する。
- `MetricsCapability.list` は DSL メトリクスを含むディスクリプタを返し、ダッシュボードプラグインが自動検出できるようにする。
- トレース連携は [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) の `start_dsl_span` を利用し、`TraceContext` を Capability Registry 経由で伝搬させる。

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
- プラグインのライフサイクル管理（ロード、アンロード、アップデート）を安全に行うメカニズムを提供。

### 5.1 プラグインサンドボックス

```reml
fn load_plugin_sandboxed(metadata: PluginMetadata, sandbox: SandboxConfig) -> Result<PluginHandle, CapabilityError>

pub type SandboxConfig = {
  allowed_capabilities: Set<CapabilityId>,
  memory_limit: Option<usize>,
  cpu_limit: Option<Duration>,
  network_access: NetworkAccess,
  file_access: FileAccess,
}

pub enum NetworkAccess = None | Restricted(List<NetworkPattern>) | Full
pub enum FileAccess = None | ReadOnly(List<PathPattern>) | Restricted(List<PathPattern>) | Full
```


### 5.2 DSLプラグイン指針

- DSL テンプレート／オブザーバビリティ拡張は `PluginCapability.register` で Capability Registry に自己記述メタデータを登録する。
- プラグインの責務と配布ポリシーは [notes/dsl-plugin-roadmap.md](notes/dsl-plugin-roadmap.md) および [AGENTS.md](AGENTS.md) を参照し、互換テストを必須化する。
- `plugins` セクションで FfiCapability や AsyncCapability を要求する場合は、Conductor 側の `with_capabilities` と同一IDを使用して権限を同期させる。

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

## 7. ランタイム監視とデバッグ

### 7.1 リアルタイムメトリクス

```reml
fn start_metrics_collection(interval: Duration) -> Result<MetricsCollector, CapabilityError>
fn get_runtime_statistics() -> RuntimeStatistics

pub type RuntimeStatistics = {
  uptime: Duration,
  memory_usage: MemoryUsage,
  gc_statistics: GcStatistics,
  capability_usage: Map<CapabilityId, UsageStats>,
  thread_pool_status: ThreadPoolStatus,
}

pub type MemoryUsage = {
  heap_used: usize,
  heap_total: usize,
  stack_size: usize,
  gc_overhead: Float,
}
```

### 7.2 パフォーマンスプロファイリング

```reml
fn enable_profiling(config: ProfilingConfig) -> Result<Profiler, CapabilityError>
fn collect_profile_data(profiler: Profiler) -> ProfileData

pub type ProfilingConfig = {
  sample_rate: Float,
  track_allocations: Bool,
  track_io: Bool,
  track_capability_calls: Bool,
}

pub type ProfileData = {
  call_graph: CallGraph,
  allocation_profile: AllocationProfile,
  hotspots: List<Hotspot>,
}
```

### 7.3 ランタイムデバッグ

```reml
fn attach_debugger(config: DebuggerConfig) -> Result<Debugger, CapabilityError>  // `effect {debug, unsafe}`
fn set_breakpoint(location: CodeLocation) -> Result<BreakpointId, DebugError>    // `effect {debug}`
fn inspect_capability_state(cap_id: CapabilityId) -> Result<CapabilityState, DebugError> // `effect {debug}`
```

> 関連: [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3.9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md)

> 注意: 本章は 2.9 実行時基盤ドラフトの内容を Chapter 3 に移行し、正式化したものです。
