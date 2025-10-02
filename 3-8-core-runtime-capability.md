# 3.8 Core Runtime & Capability Registry

> 目的：Reml ランタイムの能力（GC、メトリクス、監査、プラグイン）を統一的に管理する `Capability Registry` を定義し、標準ライブラリ各章から利用できる公式 API を提供する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {runtime}`, `effect {audit}`, `effect {unsafe}`, `effect {security}` |
| 依存モジュール | `Core.Prelude`, `Core.Diagnostics`, `Core.Numeric & Time`, `Core.IO`, `Core.Config`, `Core.Env` |
| 相互参照 | [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3.9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md), [3-10 Core Env & Platform Bridge](3-10-core-env.md) |

> **段階的導入ポリシー**: Capability の追加や効果カテゴリの拡張は `-Z` 実験フラグ経由で opt-in し、`CapabilityRegistry::register` に渡すメタデータで `stage = Experimental | Beta | Stable` を明示する。`stage` が `Experimental` の Capability は `@requires_capability(stage="experimental")` を伴う API からのみ呼び出せる。ベータ／安定化の手順は `notes/algebraic-effects-implementation-roadmap-revised.md` を参照し、`@pure`/`@dsl_export` 契約との整合チェックを完了してから `stage = Stable` へ更新すること。

## 1. Capability Registry の基本構造

```reml
pub type CapabilityId = Str

pub struct CapabilityRegistry {
  gc: Option<GcCapability>,
  io: IoCapability,
  async_runtime: Option<AsyncCapability>,
  actor: Option<ActorRuntimeCapability>,
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
  | Async(AsyncCapability)
  | Actor(ActorRuntimeCapability)
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

各 Capability の概要は以下の通りで、OS／プラットフォーム依存コンポーネントは公式プラグイン仕様（Chapter 4）に詳細を委ねる。標準配布では未登録状態を既定とし、導入時に `SecurityCapability` で審査する。

| Capability | 主担当効果 | 役割 | 参照章 |
| --- | --- | --- | --- |
| `AsyncCapability` | `io.async`, `io.timer` | スケジューラ・Waker・バックプレッシャ API を公開し、`Core.Async` を支える | 本章 §1.4, [3-9](3-9-core-async-ffi-unsafe.md) §1.1 |
| `ActorRuntimeCapability` | `io.async`, `audit` | Mailbox や分散トランスポート、Actor 監査フックを提供 | 本章 §1.4, [3-9](3-9-core-async-ffi-unsafe.md) §1.9 |
| `SyscallCapability` | `syscall`, `memory`, `unsafe` | OS システムコール呼び出しと監査フック | [4-1 System Capability プラグイン](4-1-system-plugin.md) |
| `ProcessCapability` | `process`, `thread` | プロセス生成・スレッド管理 | [4-2 Process Capability プラグイン](4-2-process-plugin.md) |
| `MemoryCapability` | `memory` | メモリマップ/共有メモリ/保護制御 | [4-3 Memory Capability プラグイン](4-3-memory-plugin.md) |
| `SignalCapability` | `signal` | シグナル登録・送信・待機 | [4-4 Signal Capability プラグイン](4-4-signal-plugin.md) |
| `HardwareCapability` | `hardware` | CPU 検出・性能カウンタ・NUMA | [4-5 Hardware Capability プラグイン](4-5-hardware-plugin.md) |
| `RealTimeCapability` | `realtime`, `io.timer` | リアルタイムスケジューラ・高精度タイマ | [4-6 RealTime Capability プラグイン](4-6-realtime-plugin.md) |
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

#### 効果ステージとハンドラ契約 {#capability-stage-contract}

| 効果 `stage` | 必須属性 | Capability Registry の検証 | 備考 |
| --- | --- | --- | --- |
| `Experimental` | `@requires_capability(stage="experimental")` を必須とし、`@handles` で捕捉する際も同属性を付与する。 | `register` 時に `effect_scope` と合わせて Stage 情報を格納し、ハンドラ適用時に `CapabilityRegistry::get` へ問い合わせて許可済みか検査する。 | PoC/社内実験用途。CI では `--deny experimental` フラグで一括拒否可能。 |
| `Beta` | `@requires_capability(stage="beta")` を推奨（省略時は `experimental` と同等の検査を行う）。 | Stage 昇格時に `SecurityCapability` が監査証跡を確認し、`effect_scope` の差分が無いか検証する。 | 機能フリーズ前の互換性検証フェーズ。 |
| `Stable` | Capability 属性は任意。`@handles` のみで捕捉可能。 | `register` 後は Stage 情報を `CapabilityRegistry::stage_of(effect_tag)` にキャッシュし、ドキュメント生成と IDE へ公開する。 | LTS ポリシー対象。 |

ハンドラが捕捉する効果タグ `Σ_handler` を評価する際、ランタイムは次の手順で認証を行う。

1. `EffectDecl`（1-3 §I.1）から `stage` と `effect_scope` を取得し、Capability Registry に登録済みか照会する。
2. ハンドラに付与された `@requires_capability` / `@handles` 属性から宣言済みの Stage を抽出し、表の条件を満たしているかを比較する。
3. 不一致の場合は `CapabilityError::SecurityViolation` を生成し、`effects.contract.stage_mismatch` 診断で呼び出し元に伝播する。
4. 診断メッセージは 0-1-project-purpose.md §1.2 の安全性基準に従い、拒否理由と必要な Capability/Stage を列挙する。

この検査結果は §4 で定義する `AuditCapability` のシンクへ送信され、`Core.Diagnostics`（3.6 節）の `audit` 効果経由で共有される。IDE/LSP から参照する場合は、[notes/dsl-plugin-roadmap.md §5](notes/dsl-plugin-roadmap.md#effect-handling-matrix) の比較表を利用してガイダンスを提示する。

### 1.3 プラットフォーム情報と能力 {#platform-info}

```reml
pub type PlatformInfo = {
  os: OS,
  arch: Architecture,
  family: TargetFamily,
  variant: Option<Str>,
  features: Set<Str>,
  runtime_capabilities: Set<RuntimeCapability>,
  target_capabilities: Set<TargetCapability>,
  profile_id: Option<Str>,
  triple: Option<Str>,
  stdlib_version: Option<SemVer>,
  runtime_revision: Option<Str>,
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
  AsyncScheduler,
  AsyncBackpressure,
  ActorMailbox,
  DistributedActor,
  AsyncTracing,
  Vector512,
  RegexJit,
  RegexMetrics,
}

pub enum TargetCapability = {
  UnicodeNfc,                 // 対応: `unicode.nfc`
  UnicodeExtendedGrapheme,    // 対応: `unicode.grapheme`
  FilesystemCaseSensitive,    // 対応: `fs.case_sensitive`
  FilesystemCaseInsensitive,  // 対応: `fs.case_insensitive`
  FilesystemCasePreserving,   // 対応: `fs.case_preserving`
  PathUtf8Encoding,           // 対応: `fs.path_utf8`
  ThreadLocalStorage,         // 対応: `thread.local`
  JobControl,                 // 対応: `process.job_control`
  MonotonicClock,             // 対応: `clock.monotonic`
  HighResolutionClock,        // 対応: `clock.highres`
  FfiCallConvC,               // 対応: `ffi.callconv.c`
  FfiCallConvSysv,            // 対応: `ffi.callconv.sysv`
  FfiCallConvWin64,           // 対応: `ffi.callconv.win64`
  FfiCallConvWasm,            // 対応: `ffi.callconv.wasm`
}

fn platform_info() -> PlatformInfo                       // `effect {runtime}`
fn platform_features() -> Set<Str>                        // `effect {runtime}`
fn platform_capabilities() -> Set<RuntimeCapability>      // `effect {runtime}`
fn runtime_capabilities() -> Set<RuntimeCapability>       // `effect {runtime}`
fn target_capabilities() -> Set<TargetCapability>         // `effect {runtime}`
fn platform_variant() -> Option<Str>                      // `effect {runtime}`
fn has_capability(cap: RuntimeCapability) -> Bool         // `effect {runtime}`
fn has_target_capability(cap: TargetCapability) -> Bool   // `effect {runtime}`
fn capability_name(cap: TargetCapability) -> &'static Str // `@pure`
fn family_tag(info: PlatformInfo) -> Str                  // `@pure`
```

* `PlatformInfo` は `Core.Env`（[3-10](3-10-core-env.md)）や `RunConfig.extensions["target"]` と同期させる。CLI が指定したターゲットと実行時情報が乖離した場合は `DiagnosticDomain::Target` で `target.config.mismatch` を発行し、`Diagnostic.extensions["target"]` に `profile_id` / `triple` / 差異一覧を記録する。
* `features` は `@cfg(feature = "...")` と連携し、ビルドプロファイルや CLI オプションで有効にした拡張機能の集合を表す。`runtime_capabilities` にはハードウェア検出結果を格納し、`RunConfig` の最適化スイッチ（Packrat/左再帰/トレース等）の既定値に利用できる。`target_capabilities` はターゲット固有挙動（Unicode/ファイルシステム/ABI 等）を表し、`@cfg(capability = "...")` や `RunConfigTarget.capabilities` と同期する。
* `profile_id` / `triple` / `stdlib_version` / `runtime_revision` は `TargetProfile` 由来のメタデータを保持し、コンパイラが生成した `RunArtifactMetadata`（2-6 §B-2-1-a）と一致することを保証する。
* `family_tag` は `"unix"` や `"windows"` といったスカラー文字列を返し、`RunConfig.extensions["target"]` の `family` フィールドを埋める際に使用する。
* Capability Registry は `register("platform", handle)` を通じてプラットフォーム情報提供者を差し替え可能。未登録時はホスト依存の既定実装が自動登録される。
* `platform_features()` はビルド時フィーチャ集合を直接返し、`platform_capabilities()` / `runtime_capabilities()` は検出済みハードウェア機能（`RuntimeCapability`）を提供する。`target_capabilities()` はターゲット挙動能力（`TargetCapability`）を返す。`platform_variant()` には libc バージョンやベンダー拡張など追加識別子を格納できる。

#### 1.3.1 `@dsl_export` との整合

- `@dsl_export` で宣言された `allows_effects` と効果宣言の `effect_scope` は、Capability Registry 登録時に比較される。差分がある場合は登録を拒否し、`CapabilityError::SecurityViolation` を返す。
- Stage 昇格 (`reml capability stage promote`) のたびに `Diagnostic.extensions["effects"].stage` と `DslExportSignature.stage` が一致しているか確認し、マニフェストの `expect_effects` / `expect_effects_stage` を更新する。
- CLI は今後追加予定の `manifest.dsl.stage_mismatch` 診断を通じて、未更新の DSL エントリが残っていないか検査する。

* `TargetCapability` の列挙値は `capability_name(cap)` により `unicode.nfc` 等のカノニカル文字列へ変換され、`@cfg(capability = "...")`、`RunConfigTarget.capabilities`、および環境変数 `REML_TARGET_CAPABILITIES` で利用される。列挙外のカスタム Capability を導入する際は、実装側で `CapabilityRegistry::register_custom_target_capability(name: Str)` を提供し、名前と診断を登録することが推奨される。
* ランタイム最適化時は次のように利用する：

```reml
let info = platform_info();
if info.runtime_capabilities.contains(RuntimeCapability::SIMD) {
  enable_simd_pipeline();
}
if platform_features().contains("packrat_default") {
  cfg.extensions["target"].features.insert("packrat_default");
}
if has_target_capability(TargetCapability::FilesystemCaseInsensitive) {
  cfg.extensions["target"].extra.insert("fs.case", "insensitive");
}
```

* `FfiCapability`（[3-9](3-9-core-async-ffi-unsafe.md)）は `platform_info()` と `resolve_calling_convention` を参照し、ターゲットごとの ABI を自動選択する。Capability Registry でプラットフォーム情報を更新すると FFI バインディングも同時に反映される。
* `Core.Env.resolve_run_config_target` / `merge_runtime_target`（3-10 §4）は `target_capabilities()` を利用して `RunConfigTarget.capabilities` を初期化し、`TargetProfile` の宣言値と実行時検出結果を統合する。不一致は `DiagnosticDomain::Target` の `target.capability.unknown` または `target.config.mismatch` として報告される。

### 1.4 非同期・Actor Capability {#async-actor-capability}

| Capability | 説明 | 主な利用者 |
| --- | --- | --- |
| `RuntimeCapability::AsyncScheduler` | マルチスレッドスケジューラと Waker 実装を提供し、`Core.Async` の `spawn`/`block_on` を安定化させる。 | `Core.Async`, `RunConfig.execution`, `guides/core-parse-streaming.md` |
| `RuntimeCapability::AsyncBackpressure` | メールボックスやストリームの高水位制御をサポートし、`send`/`run_stream_async` が `Pending` を返せる。 | `Core.Async`, `StreamDriver`, `guides/runtime-bridges.md` |
| `RuntimeCapability::ActorMailbox` | 固定長リングバッファ付き Mailbox と `link`/`monitor` 用の監査フックを有効化する。 | `Core.Async` §1.9, `3-6` 監査拡張 |
| `RuntimeCapability::DistributedActor` | `TransportHandle` によるリモート mail box 統合と TLS 設定検証を提供する。 | `Core.Async` §1.9.2, `guides/runtime-bridges.md` §11 |
| `RuntimeCapability::AsyncTracing` | 非同期タスクの span 追跡（`DiagnosticSpan` の継承と `async.trace.*` メトリクス）を記録する。 | `Core.Diagnostics`, LSP トレース, 監査ログ |

- これら Capability は 0-1-project-purpose.md §1.1 の性能基準を満たすため、最低でも `AsyncScheduler` を安定ステージで登録することを求める。未登録の場合は `Core.Async` が逐次実行フォールバックへ切り替わり、`async.actor.capability_missing` 診断で通知される。
- `AsyncBackpressure` が無い環境では `send` の `Pending` が `DropNew` に置き換わるため、DSL は高水位閾値を保守的に設定し、`guides/runtime-bridges.md §11` のテーブルに従って警告を発行する。
- `DistributedActor` を利用する場合は `SecurityCapability.permissions` に `Network` を含めること。暗号化オプションが未設定なら `SecurityCapability` が `CapabilityError::SecurityViolation` を返す。
- `AsyncTracing` が有効な環境では `DiagnosticSpan` を `CapabilityRegistry::get("tracing")` から取得し、`ActorContext.span` に継承する。メトリクス未対応環境ではトレースセクションをスキップする。
- `CapabilityRegistry::stage_of(RuntimeCapability)` はこれらの Capability についても Stage 管理を提供し、`experimental` から `beta` へ昇格させる際は `notes/dsl-plugin-roadmap.md` のチェックリストを満たす必要がある。

```reml
pub struct AsyncCapability {
  scheduler: SchedulerHandle,
  spawn_task: fn(Future<Any>) -> TaskHandle,
  supports_mailbox: fn() -> Bool,
  tracing: Option<AsyncTracingHooks>,
}

pub struct TaskHandle {
  id: Uuid,
  cancel: fn() -> Bool,
  metrics: TaskMetrics,
}

pub struct ActorRuntimeCapability {
  allocate_mailbox: fn<Message>(ActorId, MailboxConfig) -> Result<MailboxHandle<Message>, CapabilityError>,
  register_transport: fn(TransportHandle) -> Result<(), CapabilityError>,
  diagnostics: ActorDiagnosticsHooks,
}

pub struct MailboxConfig {
  capacity: usize,
  overflow: OverflowPolicy,
  priority: Option<PriorityPolicy>,
}

pub struct ActorDiagnosticsHooks {
  on_spawn: fn(ActorId, NodeId) -> (),
  on_exit: fn(ActorId, ExitStatus) -> (),
  on_backpressure: fn(ActorId, MailboxStats) -> (),
}

pub struct MailboxStats {
  pending: usize,
  dropped: usize,
  high_watermark: usize,
}

pub enum PriorityPolicy = FIFO | Priority { levels: u8 }
```



### 1.5 Regex Capability {#regex-capability}

| Capability | 説明 | 主な利用者 |
| --- | --- | --- |
| `RuntimeCapability::RegexJit` | 正規表現の JIT コンパイルとネイティブ実行を許可。JIT 未対応プラットフォームでは `Core.Regex` が自動的に NFA 実装へフォールバック。 | `Core.Regex`, `Core.Parse.Regex`, `RunConfig` (`RegexRunConfig.engine = "auto"`) |
| `RuntimeCapability::RegexMetrics` | マッチング時間・バックトラック深度などの計測値を収集し、監査ログとメトリクスに公開。 | `Core.Regex`, `Core.Diagnostics`, `AuditSink` |

* `RuntimeCapability::RegexJit` が無効な場合、`PatternFlag::Jit` は `RegexErrorKind::CapabilityRequired` を返し、`feature {regex}` は NFA/Hybrid 実装のみを利用する（2.6 §F）。
* `RuntimeCapability::RegexMetrics` を有効化すると `RunConfig.extensions["regex"].metrics=true` が要求され、`regex.match.duration` / `regex.backtrack.depth` をメトリクスストリームへ送信する。無効な場合はメトリクス計測を省略する。
* Capability Registry は `register("regex", CapabilityHandle::Plugin(...))` を通じてサードパーティエンジンを差し替え可能とし、登録時に `UnicodeClassProfile.version` を `platform_features()` と照合する。
* 監査強度ポリシー（3-6 §2.7）が `High` のときは `RegexRunConfig.audit` を省略できず、`RuntimeCapability::RegexMetrics` が未登録であれば `regex.audit.capability_missing` を発行する。

### 1.6 CapabilityError

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
  memory_limit: Option<MemoryLimit>,
  cpu_limit: Option<CpuQuota>,
  network_access: NetworkAccess,
  file_access: FileAccess,
}

pub enum NetworkAccess = None | Restricted(List<NetworkPattern>) | Full
pub enum FileAccess = None | ReadOnly(List<PathPattern>) | Restricted(List<PathPattern>) | Full
```

- `memory_limit` と `cpu_limit` は 3.5 §9 の `MemoryLimit` / `CpuQuota` を利用し、`load_plugin_sandboxed` 内で `MemoryLimit::resolve` と `CpuQuota::normalize` を必ず実行する。正規化結果は `CapabilityRegistry::registry().memory` 等と統合され、Stage/Capability 審査で監査ログへ記録する。
- 物理メモリや論理コア数は `PlatformInfo`（本章 §1.3）から取得し、`Relative` や `Fraction` 指定の妥当性を検証する。制限超過時は `CapabilityError::SecurityViolation` を返し、診断 `sandbox.limit.invalid`（3-6 §6.1.2）を生成する。


### 6.2 DSLプラグイン指針

- DSL テンプレート／オブザーバビリティ拡張は `PluginCapability.register` で Capability Registry に自己記述メタデータを登録する。
- プラグインの責務と配布ポリシーは [notes/dsl-plugin-roadmap.md](notes/dsl-plugin-roadmap.md) および [AGENTS.md](AGENTS.md) を参照し、互換テストを必須化する。
- `plugins` セクションで FfiCapability や AsyncCapability を要求する場合は、Conductor 側の `with_capabilities` と同一IDを使用して権限を同期させる。

## 7. DSL Capability Utility {#dsl-capability-utility}

DSL エクスポートの互換性検証と性能推定をランタイムで支援するために、`Core.Runtime.DslCapability` 名前空間を定義する。Chapter 1 の `DslExportSignature`（1-2 §G）と Chapter 3.7 のマニフェスト API を連携させ、CLI や LSP からのクエリに応答できるようにする。

```reml
pub type DslCapabilityProfile = {
  category: DslCategory,
  produces: DslCategory,
  requires: List<DslCategory>,
  allows_effects: Set<EffectTag>,
  capabilities: Set<CapabilityId>,
  manifest: Option<DslEntry>,
  performance: Option<DslPerformanceHints>,
}

pub type DslPerformanceHints = {
  baseline_latency: Duration,
  throughput: Option<Float>,
  memory_ceiling: Option<usize>,
  notes: Option<Str>,
}

pub type DslCompatibilityReport = {
  compatible: Bool,
  missing_capabilities: Set<CapabilityId>,
  effect_delta: Set<EffectTag>,
  category_mismatch: Option<(DslCategory, DslCategory)>,
  notes: List<Str>,
}

fn register_dsl_profile(profile: DslCapabilityProfile) -> Result<(), CapabilityError>       // `effect {runtime}`
fn resolve_dsl_profile(name: Str) -> Option<DslCapabilityProfile>                          // `effect {runtime}`
fn analyze_dsl_compatibility(lhs: DslCapabilityProfile, rhs: DslCapabilityProfile) -> DslCompatibilityReport // `@pure`
fn benchmark_dsl(entry: DslExportSignature<Json>, harness: BenchmarkHarness) -> Result<DslPerformanceHints, CapabilityError> // `effect {runtime, audit}`
```

```reml
pub type BenchmarkHarness = {
  input_generator: fn() -> Json,                     // `effect {runtime}`
  iterations: u32,
  warmup: u32,
  metrics: List<MetricPoint<Float>>,                // `@pure`
  trace: Option<TraceSink>,
}
```

- `register_dsl_profile` は `reml.toml` とコンパイラが収集した `DslExportSignature` を統合し、Capability Registry にキャッシュする。
- `resolve_dsl_profile` は CLI が `reml dsl info <name>` のようなコマンドで利用し、互換性情報を JSON で返す際の基礎メタデータを提供する。
- `analyze_dsl_compatibility` は `requires` / `capabilities` / `allows_effects` を比較し、Chapter 1.3 §I.1 の効果境界検査を再利用して差分を報告する。`effect_delta` が空で `missing_capabilities` も空の場合に互換と判定する。
- `benchmark_dsl` はパーサーのホットパスを計測し、`MetricsCapability.emit` を通じて `dsl.performance.*` メトリクスを収集する。`TraceSink` が指定された場合は [3-6](3-6-core-diagnostics-audit.md) のトレース API と連携して結果を可視化する。
- `DslPerformanceHints` は 0-2 指針の性能基準（10MB 線形解析等）を記録し、CLI/IDE が閾値を超過した場合に警告を出せるようにする。

### 7.1 プロファイル生成フロー

1. `load_manifest`（3-7 §1.2）で取得した DSL セクションを `register_dsl_profile` に渡す。
2. コンパイラが `@dsl_export` を解析して `DslExportSignature` を得たら、`benchmark_dsl` の事前ウォームアップにより性能ヒントを更新する。
3. `analyze_dsl_compatibility` を用いて、Conductor が依存する DSL の `requires` セットが満たされているか、`allows_effects` の差異が許容範囲かを確認する。
4. 結果は `CliDiagnosticEnvelope.summary.stats["dsl_compat"]`（3-6 §9）に集計され、CLI 出力や LSP が利用できる。

### 7.2 互換性診断との連携

- `DslCompatibilityReport` が `compatible=false` を返した場合、`diagnostic("dsl.compatibility.failed")` を生成し、`missing_capabilities` と `effect_delta` を期待集合として提示する。
- `category_mismatch` が発生した場合は型検査段階のエラー (`manifest.dsl.category_mismatch`) と同期し、重複報告を避ける。
- `performance.notes` に `baseline_latency` が 0-2 指針の閾値を超えた旨が記録されている場合は、`Severity::Warning` で CLI に表示し、CI では `--fail-on-performance` フラグでエラーに昇格できる。

### 7.3 テンプレート Capability プリセット

```reml
pub enum TemplateCapability =
  | RenderHtml
  | RenderText
  | RegisterFilter
  | BypassEscape

fn capability_name(cap: TemplateCapability) -> CapabilityId
fn requires_stage(cap: TemplateCapability) -> StageId
```

- `RenderHtml` は HTML/XML などエスケープ前提のテンプレート出力を許可し、Stage `runtime` を要求する。`Core.Text.Template.render` が HTML エスケープを有効にした状態で利用されることを前提とし、`template.escape.bypassed` が Warning を発生させた際は監査ログへ記録する。
- `RenderText` はログ・診断メッセージなどプレーンテキスト出力向けで、`Stage::audit` または `Stage::cli` と組み合わせる。`EscapePolicy` は緩和できるが、`RenderHtml` を要求するテンプレートと同時に利用する場合は `with_escape_policy` で明示する。
- `RegisterFilter` は `TemplateFilterRegistry.register_secure` に必要な Capability であり、フィルター実装側で追加の Capability を再確認する。プラグインがこの Capability を要求する場合、`CapAuthPolicy` は署名検証 (`verify_signature`) の成功を必須とする。
- `BypassEscape` は危険操作のためデフォルトで無効。`Stage::unsafe` を要求し、`CapabilityRegistry` 側でプロジェクト単位の許可が必要。CI では `--deny-capability template.bypass_escape` を推奨する。
- `capability_name(TemplateCapability::RenderHtml)` は `"template.render_html"` のようなカノニカル名を返し、`DSL` マニフェスト・`conductor` 設定と一致させる。
- `requires_stage` は Stage/Capability 整合を強制し、`RenderHtml`/`RenderText` は `Stage::runtime`、`RegisterFilter` は `Stage::build`、`BypassEscape` は `Stage::unsafe` を返す。Stage 不一致時は `CapabilityError::StageViolation` を返し、`diagnostic("template.capability.stage_violation")` を生成する。

---


## 8. 使用例（GC + Metrics 登録）

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

## 9. ランタイム監視とデバッグ

### 9.1 リアルタイムメトリクス

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

### 9.2 パフォーマンスプロファイリング

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

### 9.3 ランタイムデバッグ

```reml
fn attach_debugger(config: DebuggerConfig) -> Result<Debugger, CapabilityError>  // `effect {debug, unsafe}`
fn set_breakpoint(location: CodeLocation) -> Result<BreakpointId, DebugError>    // `effect {debug}`
fn inspect_capability_state(cap_id: CapabilityId) -> Result<CapabilityState, DebugError> // `effect {debug}`
```

> 関連: [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3.9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md)

> 注意: 本章は 2.9 実行時基盤ドラフトの内容を Chapter 3 に移行し、正式化したものです。
