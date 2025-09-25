# 3.8 Core Runtime & Capability Registry

> 目的：Reml ランタイムの能力（GC、メトリクス、監査、プラグイン）を統一的に管理する `Capability Registry` を定義し、標準ライブラリ各章から利用できる公式 API を提供する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {runtime}`, `effect {audit}`, `effect {unsafe}`, `effect {security}` |
| 依存モジュール | `Core.Prelude`, `Core.Diagnostics`, `Core.Numeric & Time`, `Core.IO`, `Core.Config`, `Core.Env` |
| 相互参照 | [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3.9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md), [3-10 Core Env & Platform Bridge](3-10-core-env.md) |

## 1. Capability Registry の基本構造

```reml
pub type CapabilityId = Str

pub struct CapabilityRegistry {
  gc: Option<GcCapability>,
  io: IoCapability,
  audit: AuditCapability,
  metrics: MetricsCapability,
  plugins: PluginCapability,
  system: Option<SyscallCapability>,
  process: Option<ProcessCapability>,
  memory: Option<MemoryCapability>,
  signal: Option<SignalCapability>,
  hardware: Option<HardwareCapability>,
  realtime: Option<RealTimeCapability>,
  security: SecurityCapability,
}

fn registry() -> &'static CapabilityRegistry                  // `effect {runtime}`
fn register(cap: CapabilityId, value: CapabilityHandle) -> Result<(), CapabilityError> // `effect {runtime}`
fn get(cap: CapabilityId) -> Option<CapabilityHandle>          // `effect {runtime}`
```

- `CapabilityHandle` は実装依存のポインタ/関数テーブルをラップする型（不透明指針）。
- `register` は起動時に呼び出され、重複登録時は `CapabilityError::AlreadyRegistered` を返す。
- セキュリティ強化のため、Capability は署名検証とアクセス制御をサポートする。

### 1.1 CapabilityHandle のバリアント

```reml
pub enum CapabilityHandle =
  | Gc(GcCapability)
  | Io(IoCapability)
  | Audit(AuditCapability)
  | Metrics(MetricsCapability)
  | Plugin(PluginCapability)
  | System(SyscallCapability)
  | Process(ProcessCapability)
  | Memory(MemoryCapability)
  | Signal(SignalCapability)
  | Hardware(HardwareCapability)
  | RealTime(RealTimeCapability)
  | Security(SecurityCapability);
```

各 Capability の概要は以下の通りで、詳細仕様は該当章（3-11〜3-16）に委ねる。

| Capability | 主担当効果 | 役割 | 参照章 |
| --- | --- | --- | --- |
| `SyscallCapability` | `syscall`, `memory`, `unsafe` | OS システムコール呼び出しと監査フック | 3-11 Core System (予定) |
| `ProcessCapability` | `process`, `thread` | プロセス生成・スレッド管理 | 3-12 Core Process (予定) |
| `MemoryCapability` | `memory` | メモリマップ/共有メモリ/保護制御 | 3-13 Core Memory (予定) |
| `SignalCapability` | `signal` | シグナル登録・送信・待機 | 3-14 Core Signal (予定) |
| `HardwareCapability` | `hardware` | CPU 検出・性能カウンタ・NUMA | 3-15 Core Hardware (予定) |
| `RealTimeCapability` | `realtime`, `io.timer` | リアルタイムスケジューラ・高精度タイマ | 3-16 Core RealTime (予定) |
| `SecurityCapability` | `security`, `audit` | セキュリティポリシー適用と証跡記録 | 本章 §1.2, §2.7 |

Capability Registry は上記バリアントを通じてシステム API を表面化し、効果タグと runtime 権限を整合させる。

### 1.2 セキュリティモデル

```reml
pub type CapabilitySecurity = {
  signature: Option<DigitalSignature>,
  permissions: Set<Permission>,
  isolation_level: IsolationLevel,
  audit_required: Bool,
  effect_scope: Set<EffectTag>,
  policy: Option<SecurityPolicyRef>,
  sandbox: Option<SandboxProfile>,
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

* `EffectTag` は [1.3 効果と安全性](1-3-effects-safety.md) に定義された `Σ` のタグ名。Capability が生成し得る効果を宣言し、`register` 時に `effect_scope ⊇ actual_effects` を検証する。
* `SecurityPolicyRef` は `SecurityCapability` が管理するポリシーへの参照で、`enforce_security_policy` 実行時に `verify_capability_security` と一致することを要求する。
* `SandboxProfile` は CPU・メモリ・ネットワークの制約を記述する共通構造体で、§6.1 の `SandboxConfig` を再利用して定義する。

### 1.3 プラットフォーム情報と能力 {#platform-info}

```reml
pub type PlatformInfo = {
  os: OS,
  arch: Architecture,
  family: TargetFamily,
  variant: Option<Str>,
  features: Set<Str>,
  capabilities: Set<RuntimeCapability>,
}

pub enum OS = Windows | Linux | MacOS | FreeBSD | Wasm | Other(Str)
pub enum Architecture = X64 | ARM64 | X86 | ARM | WASM32 | RISCV64 | Other(Str)
pub enum TargetFamily = Unix | Windows | Wasm | Other(Str)

pub enum RuntimeCapability = {
  SIMD,
  HardwareRng,
  CryptoExtensions,
  GPU,
  ThreadLocal,
  Vector512,
}

fn platform_info() -> PlatformInfo                       // `effect {runtime}`
fn platform_features() -> Set<Str>                        // `effect {runtime}`
fn platform_capabilities() -> Set<RuntimeCapability>      // `effect {runtime}`
fn platform_variant() -> Option<Str>                      // `effect {runtime}`
fn has_capability(cap: RuntimeCapability) -> Bool         // `effect {runtime}`
fn family_tag(info: PlatformInfo) -> Str                  // `@pure`
```

* `PlatformInfo` は `Core.Env`（[3-10](3-10-core-env.md)）や `RunConfig.extensions["target"]` と同期させる。CLI が指定したターゲットと実行時情報が乖離した場合は `target.config.unsupported_value` を発行し、`Diagnostic.extensions["cfg"].evaluated` に両者を記録する。
* `features` は `@cfg(feature = "...")` と連携し、ビルドプロファイルや CLI オプションで有効にした拡張機能の集合を表す。`capabilities` にはハードウェア検出結果を格納し、`RunConfig` の最適化スイッチ（Packrat/左再帰/トレース等）の既定値に利用できる。
* `family_tag` は `"unix"` や `"windows"` といったスカラー文字列を返し、`RunConfig.extensions["target"]` の `family` フィールドを埋める際に使用する。
* Capability Registry は `register("platform", handle)` を通じてプラットフォーム情報提供者を差し替え可能。未登録時はホスト依存の既定実装が自動登録される。
* `platform_features()` はビルド時フィーチャ集合を直接返し、`platform_capabilities()` は検出済みハードウェア機能（`RuntimeCapability`）を提供する。`platform_variant()` には libc バージョンやベンダー拡張など追加識別子を格納できる。
* ランタイム最適化時は次のように利用する：

```reml
let info = platform_info();
if info.capabilities.contains(RuntimeCapability::SIMD) {
  enable_simd_pipeline();
}
if platform_features().contains("packrat_default") {
  cfg.extensions["target"].features.insert("packrat_default");
}
```

* `FfiCapability`（[3-9](3-9-core-async-ffi-unsafe.md)）は `platform_info()` と `resolve_calling_convention` を参照し、ターゲットごとの ABI を自動選択する。Capability Registry でプラットフォーム情報を更新すると FFI バインディングも同時に反映される。

### 1.4 CapabilityError

```reml
pub type CapabilityError = {
  kind: CapabilityErrorKind,
  message: Str,
}

pub enum CapabilityErrorKind = AlreadyRegistered | NotFound | InvalidHandle | UnsafeViolation | SecurityViolation
```

- `InvalidHandle` は型不一致や ABI 不整合を検出した際に報告する。
- `UnsafeViolation` は `effect {unsafe}` 経由でのみ返される。
- `SecurityViolation` はアクセス制御違反や不正なケーパビリティ操作時に発生する。

## 2. システムプログラミング Capability 概要

`Σ_system` に対応する Capability は、低レベル API をランタイム経由で公開しつつ、安全性と監査を維持するためのゲートとして機能する。各 Capability は `CapabilitySecurity.effect_scope` と一致する効果タグを生成し、登録時に署名およびポリシー検証を受ける。

### 2.1 SyscallCapability

```reml
pub type SyscallCapability = {
  raw_syscall: fn(SyscallNumber, [i64; 6]) -> Result<i64, SyscallError>,      // effect {syscall, unsafe}
  platform_syscalls: PlatformSyscalls,                                        // effect {syscall}
  audited_syscall: fn(SyscallDescriptor, SyscallThunk) -> Result<SyscallRet, SyscallError>, // effect {syscall, audit}
  supports: fn(SyscallId) -> Bool,
}

pub type SyscallThunk = fn() -> Result<SyscallRet, SyscallError>;
```

* `PlatformSyscalls` は OS 別ラッパ（Linux/Windows/macOS 等）をカプセル化し、型安全な高レベル API を提供する。
* `audited_syscall` は [3-6](3-6-core-diagnostics-audit.md) の監査ロガーと統合し、`audit` 効果を標準化する。

### 2.2 ProcessCapability

```reml
pub type ProcessCapability = {
  spawn_process: fn(Command, Environment) -> Result<ProcessHandle, ProcessError>,    // effect {process}
  kill_process: fn(ProcessHandle, Signal) -> Result<(), ProcessError>,               // effect {process, signal}
  wait_process: fn(ProcessHandle, Option<Duration>) -> Result<ExitStatus, ProcessError>, // effect {process, blocking}
  create_thread: fn(ThreadStart, ThreadOptions) -> Result<ThreadHandle, ThreadError>,    // effect {thread}
  join_thread: fn(ThreadHandle, Option<Duration>) -> Result<ThreadResult, ThreadError>,  // effect {thread, blocking}
  set_thread_affinity: fn(ThreadHandle, Set<CpuId>) -> Result<(), ThreadError>,          // effect {thread, hardware}
}
```

### 2.3 MemoryCapability

```reml
pub type MemoryCapability = {
  mmap: fn(MmapRequest) -> Result<MappedMemory, MemoryError>,          // effect {memory, unsafe}
  munmap: fn(MappedMemory) -> Result<(), MemoryError>,                 // effect {memory}
  mprotect: fn(&mut MappedMemory, MemoryProtection) -> Result<(), MemoryError>, // effect {memory}
  shared_open: fn(SharedMemoryRequest) -> Result<SharedMemory, MemoryError>,    // effect {memory, process}
  msync: fn(&MappedMemory, SyncFlags) -> Result<(), MemoryError>,      // effect {memory, io}
}
```

### 2.4 SignalCapability

```reml
pub type SignalCapability = {
  register_handler: fn(Signal, SignalHandler) -> Result<PreviousHandler, SignalError>, // effect {signal, unsafe}
  mask: fn(Set<Signal>) -> Result<SignalMask, SignalError>,                             // effect {signal}
  unmask: fn(SignalMask) -> Result<(), SignalError>,                                   // effect {signal}
  send: fn(ProcessId, Signal) -> Result<(), SignalError>,                              // effect {signal, process}
  wait: fn(Set<Signal>, Option<Duration>) -> Result<SignalInfo, SignalError>,          // effect {signal, blocking}
  raise: fn(Signal) -> Result<(), SignalError>,                                        // effect {signal}
}
```

### 2.5 HardwareCapability

```reml
pub type HardwareCapability = {
  read_cpu_id: fn() -> CpuId,                                  // effect {hardware}
  cpu_features: fn() -> CpuFeatures,                           // effect {hardware}
  rdtsc: fn() -> u64,                                          // effect {hardware, timing}
  rdtscp: fn() -> (u64, u32),                                  // effect {hardware, timing}
  prefetch: fn<T>(Ptr<T>, PrefetchLocality) -> (),              // effect {hardware}
  numa_nodes: fn() -> List<NumaNode>,                           // effect {hardware}
  bind_numa: fn(NumaNode) -> Result<(), HardwareError>,        // effect {hardware, thread}
}
```

### 2.6 RealTimeCapability

```reml
pub type RealTimeCapability = {
  set_scheduler: fn(SchedulingPolicy, Priority) -> Result<PreviousScheduler, RealTimeError>, // effect {realtime}
  lock_memory: fn(VoidPtr, usize) -> Result<(), MemoryError>,                                 // effect {realtime, memory}
  unlock_memory: fn(VoidPtr, usize) -> Result<(), MemoryError>,                               // effect {realtime, memory}
  sleep_precise: fn(Duration) -> Result<Duration, RealTimeError>,                             // effect {realtime, blocking}
  create_timer: fn(Duration, TimerHandler) -> Result<TimerHandle, RealTimeError>,             // effect {realtime, io.timer}
}
```

### 2.7 SecurityCapability

```reml
pub type SecurityCapability = {
  enforce_security_policy: fn(SecurityPolicy) -> Result<(), SecurityError>,    // effect {security, audit}
  current_policy: fn() -> SecurityPolicy,                                       // effect {security}
  verify_signature: fn(CapabilityId, DigitalSignature) -> Result<(), SecurityError>, // effect {security}
  audit_violation: fn(SecurityViolationReport) -> Result<(), CapabilityError>,  // effect {audit}
  policy_digest: fn() -> PolicyDigest,                                          // @pure
}
```

`SecurityPolicy` は [system-programming-analysis.md](system-programming-analysis.md) で提案された構造（許可システムコール、メモリ制限、ネットワーク範囲等）を採用し、`policy_digest` は監査ログやキャッシュで使用するハッシュ値を返す。

---

## 3. GC Capability インターフェイス

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

### 3.1 メモリ管理の高度制御

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

## 4. Metrics & Audit Capability

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


### 4.1 DSLメトリクス連携

- Conductor で宣言された DSL ID ごとに `register_dsl_metrics` を呼び出し、`MetricsCapability.emit` を通じて `dsl.latency` などのメトリクスを登録する。
- `MetricsCapability.list` は DSL メトリクスを含むディスクリプタを返し、ダッシュボードプラグインが自動検出できるようにする。
- トレース連携は [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) の `start_dsl_span` を利用し、`TraceContext` を Capability Registry 経由で伝搬させる。

## 5. IO Capability

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

## 6. プラグイン Capability

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

### 6.1 プラグインサンドボックス

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


### 6.2 DSLプラグイン指針

- DSL テンプレート／オブザーバビリティ拡張は `PluginCapability.register` で Capability Registry に自己記述メタデータを登録する。
- プラグインの責務と配布ポリシーは [notes/dsl-plugin-roadmap.md](notes/dsl-plugin-roadmap.md) および [AGENTS.md](AGENTS.md) を参照し、互換テストを必須化する。
- `plugins` セクションで FfiCapability や AsyncCapability を要求する場合は、Conductor 側の `with_capabilities` と同一IDを使用して権限を同期させる。

## 7. 使用例（GC + Metrics 登録）

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

## 8. ランタイム監視とデバッグ

### 8.1 リアルタイムメトリクス

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

### 8.2 パフォーマンスプロファイリング

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

### 8.3 ランタイムデバッグ

```reml
fn attach_debugger(config: DebuggerConfig) -> Result<Debugger, CapabilityError>  // `effect {debug, unsafe}`
fn set_breakpoint(location: CodeLocation) -> Result<BreakpointId, DebugError>    // `effect {debug}`
fn inspect_capability_state(cap_id: CapabilityId) -> Result<CapabilityState, DebugError> // `effect {debug}`
```

> 関連: [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3.9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md)

> 注意: 本章は 2.9 実行時基盤ドラフトの内容を Chapter 3 に移行し、正式化したものです。
