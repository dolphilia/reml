# 3.9 Core Async / FFI / Unsafe

> 目的：Reml の非同期実行 (`Core.Async`)・FFI (`Core.Ffi`)・unsafe ブロック (`Core.Unsafe`) に関する基本方針と効果タグの枠組みを整理し、今後の詳細仕様策定に備える。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `effect {io.async}`, `effect {io.blocking}`, `effect {io.timer}`, `effect {ffi}`, `effect {unsafe}`, `effect {security}`, `effect {audit}` |
| 依存モジュール | `Core.Prelude`, `Core.Iter`, `Core.IO`, `Core.Runtime`, `Core.Diagnostics` |
| 相互参照 | [2.6 実行戦略](2-6-execution-strategy.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [guides/runtime-bridges.md](guides/runtime-bridges.md), [guides/reml-ffi-handbook.md](guides/reml-ffi-handbook.md) |

## 1. Core.Async の枠組み

```reml
pub type Future<T> = {
  poll: fn(&mut Context) -> Poll<T>,
}

pub enum Poll<T> = Ready(T) | Pending

pub struct Task<T> {
  future: Future<T>,
  scheduler: SchedulerHandle,
}

fn spawn<T>(future: Future<T>, scheduler: SchedulerHandle) -> Task<T>        // `effect {io.async}`
fn block_on<T>(future: Future<T>) -> Result<T, AsyncError>                    // `effect {io.blocking}`
fn sleep_async(duration: Duration) -> Future<()>                             // `effect {io.async, io.timer}`
```

- `Context` は Waker を含む非同期実行コンテキスト。
- `SchedulerHandle` は `Core.Runtime` の Capability Registry から取得する。
- `block_on` は同期ブロックするため `effect {blocking}` を要求し、CLI ツールなどで使用する際は注意が必要。

### 1.2 高度な非同期パターン

```reml
fn join<T, U>(future1: Future<T>, future2: Future<U>) -> Future<(T, U)>         // `effect {io.async}`
fn select<T>(futures: List<Future<T>>) -> Future<(usize, T)>                   // `effect {io.async}`
fn timeout<T>(future: Future<T>, duration: Duration) -> Future<Result<T, TimeoutError>> // `effect {io.async}`
fn retry<T>(future: () -> Future<T>, policy: RetryPolicy) -> Future<T>         // `effect {io.async}`

pub type RetryPolicy = {
  max_attempts: u32,
  backoff: BackoffStrategy,
  should_retry: (AsyncError) -> Bool,
}

pub enum BackoffStrategy = Linear(Duration) | Exponential { base: Duration, max: Duration } | Custom((u32) -> Duration)
```

### 1.3 ストリームとアシンクイテレータ

```reml
pub type AsyncStream<T> = {
  next: fn() -> Future<Option<T>>,
}

fn from_iter<T>(iter: Iter<T>) -> AsyncStream<T>                               // `effect {io.async}`
fn buffer<T>(stream: AsyncStream<T>, size: usize) -> AsyncStream<T>             // `effect {io.async, memory}`
fn map_async<T, U>(stream: AsyncStream<T>, f: (T) -> Future<U>) -> AsyncStream<U> // `effect {io.async}`
fn filter_async<T>(stream: AsyncStream<T>, pred: (T) -> Future<Bool>) -> AsyncStream<T> // `effect {io.async}`
fn collect_async<T>(stream: AsyncStream<T>) -> Future<List<T>>                  // `effect {io.async, memory}`
```

### 1.4 DSLオーケストレーション支援 API

```reml
pub struct Channel<Send, Recv> where Send: Serialize, Recv: Deserialize {
  sender: DslSender<Send>,
  receiver: DslReceiver<Recv>,
  codec: Codec<Send, Recv>,
  buffer_size: usize,
  overflow: OverflowPolicy,
}

fn create_channel<S, R>(buffer_size: usize, codec: Codec<S, R>) -> Result<(DslSender<S>, DslReceiver<R>), AsyncError> // `effect {io.async}`
fn merge_channels<T>(channels: List<DslReceiver<T>>) -> DslReceiver<T>                                             // `effect {io.async}`
fn split_channel<T>(channel: DslReceiver<T>, predicate: (T) -> Bool) -> (DslReceiver<T>, DslReceiver<T>)           // `effect {io.async}`
fn with_execution_plan<T>(dsl: DslSpec<T>, plan: ExecutionPlan) -> DslSpec<T>                                     // `effect {io.async}`

struct ExecutionPlan = {
  strategy: ExecutionStrategy,
  backpressure: BackpressurePolicy,
  error: ErrorPropagationPolicy,
  scheduling: SchedulingPolicy,
}

enum BackpressurePolicy = Drop | DropOldest | Buffer(usize) | Block | Adaptive { high_watermark: usize, low_watermark: usize, strategy: AdaptiveStrategy }

enum AdaptiveStrategy = DropNewest | SlowProducer | SignalDownstream
```

- `Channel<Send, Recv>` は DSL 間通信を型安全に扱い、コード変換を `Codec<Send, Recv>` へ委譲する。
- `ExecutionPlan` は `conductor` の `execution { ... }` ブロックと 1:1 で対応し、スケジューラーへ渡す実行ポリシーを保持する。
- `with_execution_plan` は DSL 定義時に計画を合成するコンビネータであり、バックプレッシャー制御やエラー隔離を `Core.Async` ランタイムへ伝える。

### 1.5 プラットフォーム適応スケジューラ

```reml
fn default_scheduler_config() -> SchedulerConfig = {
  let info = platform_info();
  let hints = scheduler_hints(info); // Core.Async が提供する CPU/IO 推奨値
  SchedulerConfig {
    worker_threads: Some(if has_capability(RuntimeCapability::Vector512) {
      hints.prefer_physical_threads
    } else {
      hints.prefer_logical_threads
    }),
    max_blocking_threads: if platform_features().contains("io.blocking.strict") {
      Some(hints.blocking_guard_threads)
    } else {
      None
    },
    io_driver: info.family == TargetFamily::Unix,
    time_driver: true,
  }
}
```

* `platform_info()` は `Core.Runtime` から取得した実行環境を返し、`RuntimeCapability` に応じてスケジューラ構成を切り替えられる。
* `platform_features()` で `RunConfig.extensions["target"].features` と同期したフラグ（例: `feature = "io.blocking.strict"`）を参照し、DSL ごとのバックプレッシャ設定やタスクプールサイズを調整する。
* Core.DSL モジュールはこの関数を利用して `ExecutionPlan` の既定値を決定し、`@cfg` で有効化した機能と矛盾しないようにする。

### 1.1 AsyncError


```reml
pub type AsyncError = {
  kind: AsyncErrorKind,
  message: Str,
}

pub enum AsyncErrorKind = Cancelled | Timeout | RuntimeUnavailable
```

## 2. Core.Ffi の枠組み

```reml
pub type ForeignFunction = FnPtr<(VoidPtr,), VoidPtr>

fn bind_library(path: Path) -> Result<LibraryHandle, FfiError>               // `effect {ffi}`
fn get_function(handle: LibraryHandle, name: Str) -> Result<ForeignFunction, FfiError> // `effect {ffi}`
fn call_ffi(fn_ptr: ForeignFunction, args: FfiArgs) -> Result<FfiValue, FfiError> // `effect {ffi, unsafe}`

pub type FfiArgs = Span<u8>
pub type FfiValue = Span<u8>
```

### 2.0 バインディング生成と Capability 連携

```reml
fn auto_bind(handle: LibraryHandle, name: Str, signature: FfiSignature) -> Result<TypedForeignFn, FfiError> // `effect {ffi}`
fn auto_bind_all(handle: LibraryHandle, spec: [FfiBinding]) -> Result<BoundLibrary, FfiError>               // `effect {ffi}`
fn call_with_capability(cap: FfiCapability, symbol: ForeignFunction, args: FfiArgs) -> Result<FfiValue, FfiError> // `effect {ffi, security, audit}`

struct FfiSignature = { params: [FfiType], return_type: FfiType }
struct FfiBinding   = { name: Str, signature: FfiSignature, conventions: CallingConvention }
struct SymbolHandle = { library: LibraryHandle, function: ForeignFunction }
struct TypedForeignFn = { call: fn(FfiArgs) -> Result<FfiValue, FfiError>, symbol: ForeignFunction, metadata: FfiBinding }
struct FfiCapability = { call_function: fn(SymbolHandle, FfiArgs) -> Result<FfiValue, FfiError>, sandbox: Option<FfiSandbox>, audit: AuditHandle }
struct LibraryMetadata = { path: Path, preferred_convention: Option<CallingConvention>, required_capabilities: Set<RuntimeCapability> }
```

- `auto_bind` は署名情報からシリアライザ/デシリアライザを自動生成し、返却された `TypedForeignFn` 経由で型安全な `call` を提供する。
- `auto_bind_all` は複数シンボルを一括登録し、Capability Registry と連携する `BoundLibrary` を構築する。
- `call_with_capability`（および `FfiCapability.call_function`）は [3.8](3-8-core-runtime-capability.md) の `CapabilityRegistry` 経由で取得した権限を通じて FFI 呼び出しを実行し、監査ログやサンドボックスを適用する。

- `FfiError` は OS 依存エラーやシンボル解決失敗をラップ。

```reml
pub type FfiError = {
  kind: FfiErrorKind,
  message: Str,
  library_path: Option<Path>,
  symbol_name: Option<Str>,
}

pub enum FfiErrorKind = {
  LibraryNotFound,
  SymbolNotFound,
  InvalidSignature,
  CallFailed,
  SecurityViolation,
  UnsupportedPlatform,
}
```

### 2.1 タイプセーフな FFI ラッパー

```reml
// 自動的なラッパー生成
macro foreign_fn(lib: Str, name: Str, signature: Str) -> ForeignFunction

// 使用例
let add_numbers = foreign_fn!("math", "add", "fn(i32, i32) -> i32");
let args = ffi::encode_args(&(42i32, 24i32));
let raw = add_numbers.call(args)?;            // raw は FfiValue (Span<u8>)
let sum: i32 = ffi::decode_result(raw)?;
```

`ffi::encode_args` / `ffi::decode_result` は `FfiSignature` と互換のシリアライズヘルパで、`Span<u8>` を安全に生成・復元する。低レベル API を直接利用する場合は `span_from_raw_parts` と `CapabilitySecurity.effect_scope` を併用し、境界検査と監査記録を怠らないこと。

### 2.2 呼出規約とプラットフォーム適応

```reml
pub enum CallingConvention = C | StdCall | FastCall | SysV | WasmSystemV | Custom(Str)

fn resolve_calling_convention(target: PlatformInfo, foreign: LibraryMetadata) -> Result<CallingConvention, FfiError> // `effect {runtime}`
fn link_foreign_library(path: Path, target: PlatformInfo) -> Result<LibraryHandle, FfiError> // `effect {ffi}`
fn with_abi_adaptation(fn_ptr: ForeignFunction, conv: CallingConvention) -> Result<ForeignFunction, FfiError> // `effect {ffi, unsafe}`
```

* 既定では `RunConfig.extensions["target"]` を用いて呼出規約を決定し、`platform_info()`（[3-8](3-8-core-runtime-capability.md)）が提供する実行時情報と突き合わせる。
* ターゲットの `family` が `Windows` かつ `arch = X64` の場合は `StdCall` を採用し、`Unix` ファミリでは `SysV` を既定とする。WASM ターゲットでは `WasmSystemV` を利用し、サポート外の場合は `FfiErrorKind::UnsupportedPlatform` を返す。
* `resolve_calling_convention` は `LibraryMetadata` に含まれる `preferred_convention` を尊重しつつ、実行環境で利用できない場合は `target.config.unsupported_value` 診断を併せて発行する。診断は `Diagnostic.extensions["cfg"].evaluated` にターゲット値とライブラリ要求を記録する。
* `with_abi_adaptation` は必要に応じてシム層を挿入し、レジスタ引数配置やスタック整列を調整する。性能への影響を抑えるため、変換は初回呼び出し時にキャッシュする。

### 2.3 メモリ管理と所有権境界

```reml
pub type ForeignPtr<T> = {
  raw: Ptr<T>,
  layout: Option<Layout>,
  release: Option<FnPtr<(VoidPtr,), ()>>,
}

pub type ForeignBuffer = {
  span: Span<u8>,
  release: Option<FnPtr<(VoidPtr,), ()>>,
  ownership: Ownership,
}

pub enum Ownership = Borrowed | Owned | Transferred

fn wrap_foreign_ptr<T>(raw: Ptr<T>, layout: Option<Layout>) -> Result<ForeignPtr<T>, FfiError>      // `effect {unsafe}`
fn borrow_span<T>(raw: Ptr<T>, len: usize) -> Result<Span<T>, FfiError>                             // `effect {unsafe}`
fn acquire_mut_span<T>(raw: MutPtr<T>, len: usize) -> Result<Span<T>, FfiError>                     // `effect {unsafe}`
fn release_foreign_ptr<T>(ptr: ForeignPtr<T>) -> Result<(), FfiError>                               // `effect {unsafe, memory}`
fn transfer_buffer(buffer: ForeignBuffer, release: FnPtr<(VoidPtr,), ()>) -> Result<(), FfiError>   // `effect {unsafe, memory}`
```

- `ForeignPtr<T>` は `Ptr<T>` を内包し、必要に応じて `NonNullPtr<T>` へ昇格して利用する。`layout` には [3-13 Core Memory](3-13-core-memory.md) で定義する `Layout` 情報を格納する。
- `ForeignBuffer` は `Span<u8>` と所有権メタデータを保持し、`Ownership::Borrowed` の場合は解放禁止とする。
- `call_ffi` は `unsafe` を要求し、境界で `AuditEnvelope` を付与することが推奨される。`transfer_buffer` では Capability Registry を通じて `MemoryCapability` の監査フックを呼び出す。

## 3. Core.Unsafe.Ptr API

```reml
// 基本ポインタ型
type Ptr<T>
type MutPtr<T>
type NonNullPtr<T>
type VoidPtr
type FnPtr<Args, Ret>
type Span<T> = { ptr: NonNullPtr<T>, len: usize }

// 基本操作
fn read<T>(ptr: Ptr<T>) -> Result<T, UnsafeError>                               // `effect {unsafe}`
fn write<T>(ptr: MutPtr<T>, value: T) -> Result<(), UnsafeError>                // `effect {unsafe}`
fn copy_to<T>(src: Ptr<T>, dst: MutPtr<T>, count: usize) -> Result<(), UnsafeError> // `effect {unsafe, memory}`
fn cast<T, U>(ptr: Ptr<T>) -> Ptr<U>                                            // `effect {unsafe}`
fn offset<T>(ptr: Ptr<T>, count: isize) -> Ptr<T>                               // `effect {unsafe}`
fn as_non_null<T>(ptr: Ptr<T>) -> Result<NonNullPtr<T>, UnsafeError>            // `effect {unsafe}`

// Span ユーティリティ
fn span_from_raw_parts<T>(ptr: Ptr<T>, len: usize) -> Result<Span<T>, UnsafeError>     // `effect {unsafe}`
fn span_split_at<T>(span: Span<T>, index: usize) -> Result<(Span<T>, Span<T>), UnsafeError> // `effect {unsafe}`
fn span_as_ptr<T>(span: Span<T>) -> Ptr<T>                                              // `effect {unsafe}`
fn span_as_mut_ptr<T>(span: Span<T>) -> MutPtr<T>                                       // `effect {unsafe}`

// VoidPtr / FnPtr ブリッジ
fn to_void_ptr<T>(ptr: Ptr<T>) -> VoidPtr                                      // `effect {unsafe}`
fn from_void_ptr<T>(ptr: VoidPtr) -> Ptr<T>                                    // `effect {unsafe}`
fn bind_fn_ptr<Args, Ret>(ptr: FnPtr<Args, Ret>) -> Result<ForeignStub<Args, Ret>, UnsafeError> // `effect {unsafe}`

pub type ForeignStub<Args, Ret> = {
  call: fn(Args) -> Ret,
  raw: FnPtr<Args, Ret>,
}
```

* `Span<T>` は `len = 0` の場合でも `ptr` は無効な非NULLダングリング値を許容しないため、ゼロ長スライスは安全に扱える。
* `copy_to` は `memory` 効果を併発し、`CapabilitySecurity.effect_scope` に `memory` を含む API からのみ呼び出す。
* `bind_fn_ptr` は FFI の `foreign_fn!` マクロと連携し、ABI 検証後に型安全な呼び出しを生成する。

---

## 4. Core.Unsafe の指針

```reml
fn unsafe_block<T>(f: () -> T) -> T                      // `effect {unsafe}`
fn assume(cond: Bool, message: Str) -> ()                // `effect {unsafe}`
fn transmute<T, U>(value: T) -> U                        // `effect {unsafe}`
```

- `unsafe_block` は安全性検証済みのコード領域を明示的に囲む。
- `assume` はコンパイラに対するヒントであり、偽の場合は未定義動作となる。
- `transmute` は型の同じビット表現を再解釈する際に使用。

### 4.1 安全性検証メカニズム

```reml
fn verify_memory_safety(ptr: Ptr<u8>, size: usize) -> Result<(), UnsafeError>  // `effect {unsafe}`
fn check_alignment<T>(ptr: Ptr<T>) -> Bool                                     // `effect {unsafe}`
fn bounds_check(ptr: Ptr<u8>, offset: isize, bounds: (usize, usize)) -> Result<(), UnsafeError> // `effect {unsafe}`

pub type UnsafeError = {
  kind: UnsafeErrorKind,
  message: Str,
  location: Option<CodeLocation>,
}

pub enum UnsafeErrorKind = {
  NullPointer,
  OutOfBounds,
  InvalidAlignment,
  UseAfterFree,
  DoubleFree,
  MemoryLeak,
}
```

### 4.2 監査された unsafe 操作

```reml
fn audited_unsafe_block<T>(operation: Str, f: () -> T) -> T                    // `effect {unsafe, audit}`
fn log_unsafe_operation(op: UnsafeOperation, context: UnsafeContext) -> ()     // `effect {audit}`

pub type UnsafeOperation = {
  operation_type: UnsafeOperationType,
  memory_address: Option<usize>,
  size: Option<usize>,
  stack_trace: List<CodeLocation>,
}

pub enum UnsafeOperationType = {
  PointerDereference,
  MemoryAllocation,
  MemoryDeallocation,
  TypeTransmutation,
  ForeignCall,
}
```

## 5. Capability Registry との連携

### 5.1 非同期 Capability

```reml
pub type AsyncCapability = {
  create_runtime: fn(AsyncRuntimeConfig) -> Result<AsyncRuntime, CapabilityError>,
  spawn_task: fn(Future<T>) -> Result<TaskHandle<T>, CapabilityError>,
  schedule_timer: fn(Duration, TimerCallback) -> Result<TimerId, CapabilityError>,
  get_metrics: fn() -> Result<AsyncMetrics, CapabilityError>,
}

pub type AsyncRuntimeConfig = {
  worker_threads: Option<usize>,
  max_blocking_threads: Option<usize>,
  thread_stack_size: Option<usize>,
  enable_io_driver: Bool,
  enable_time_driver: Bool,
}
```

### 5.2 FFI Capability

```reml
pub type FfiCapability = {
  load_library: fn(Path, FfiSecurity) -> Result<LibraryHandle, CapabilityError>,
  resolve_symbol: fn(LibraryHandle, Str) -> Result<SymbolHandle, CapabilityError>,
  verify_abi: fn(SymbolHandle, FfiSignature) -> Result<(), CapabilityError>,
  call_function: fn(SymbolHandle, FfiArgs) -> Result<FfiValue, CapabilityError>,
}

pub type FfiSecurity = {
  allowed_libraries: Option<List<PathPattern>>,
  signature_verification: Bool,
  sandbox_calls: Bool,
}
```

### 5.3 Unsafe Capability

```reml
pub type UnsafeCapability = {
  enable_raw_pointers: fn(UnsafePolicy) -> Result<(), CapabilityError>,
  allocate_raw: fn(usize, usize) -> Result<MutPtr<u8>, CapabilityError>,
  deallocate_raw: fn(MutPtr<u8>, usize, usize) -> Result<(), CapabilityError>,
  track_allocation: fn(NonNullPtr<u8>, usize) -> Result<AllocationId, CapabilityError>,
  verify_pointer: fn(Ptr<u8>) -> Result<PointerInfo, CapabilityError>,
}

pub type UnsafePolicy = {
  enable_bounds_checking: Bool,
  enable_use_after_free_detection: Bool,
  enable_double_free_detection: Bool,
  max_allocations: Option<usize>,
  allocation_size_limit: Option<usize>,
}
```

## 6. 使用例（調査メモ）

```reml
use Core;
use Core.Async;
use Core.Ffi;

fn async_file_copy(src: Path, dest: Path) -> Result<(), Diagnostic> =
  spawn(async move {
    let mut reader = AsyncFile::open(src).await?;
    let mut writer = AsyncFile::create(dest).await?;
    while let Some(chunk) = reader.next_chunk().await? {
      writer.write_all(chunk).await?;
    }
    Ok(())
  }, scheduler())
    .await
    .map_err(|err| Diagnostic::from_async_error(err))
```

- 将来的な AsyncFile API の利用例（現時点では概念メモ）。`await` 構文は Reml の非同期拡張候補。
- エラーは `Diagnostic` へ変換し、監査連携の対象にする思考過程を示す。

## 7. セキュリティとベストプラクティス

### 7.1 非同期セキュリティ

```reml
// タイムアウトとリソース制限
fn with_async_limits<T>(limits: AsyncLimits, future: Future<T>) -> Future<Result<T, LimitError>>

pub type AsyncLimits = {
  execution_timeout: Option<Duration>,
  memory_limit: Option<usize>,
  concurrent_tasks_limit: Option<usize>,
}
```

### 7.2 FFI セキュリティ

```reml
// サンドボックス内での FFI 呼び出し
fn call_sandboxed<T>(foreign_fn: ForeignFunction, args: FfiArgs, sandbox: FfiSandbox) -> Result<T, FfiError> // `effect {ffi, unsafe, security, audit}`

pub type FfiSandbox = {
  memory_limit: usize,
  cpu_time_limit: Duration,
  syscall_whitelist: Option<List<SyscallId>>,
  network_access: Bool,
  file_access: FileAccessPolicy,
}
```

### 7.3 Unsafe セキュリティ

```reml
// メモリ安全性の動的検証
fn enable_memory_sanitizer(config: SanitizerConfig) -> Result<(), UnsafeError>  // `effect {unsafe, debug}`

pub type SanitizerConfig = {
  detect_out_of_bounds: Bool,
  detect_use_after_free: Bool,
  detect_double_free: Bool,
  detect_memory_leaks: Bool,
  quarantine_freed_memory: Bool,
}
```

## 8. パフォーマンス最適化

### 8.1 非同期最適化

```reml
// タスクスケジューリングの調整
fn tune_async_runtime(metrics: AsyncMetrics) -> AsyncRuntimeConfig
fn adaptive_scheduling(workload: WorkloadProfile) -> SchedulingStrategy

// バッチ処理とストリーミングの最適化
fn batch_futures<T>(futures: List<Future<T>>, batch_size: usize) -> Future<List<T>>
fn stream_with_backpressure<T>(stream: AsyncStream<T>, buffer_size: usize) -> AsyncStream<T>
```

### 8.2 FFI 最適化

```reml
// 関数呼び出しのキャッシュ
fn cache_ffi_function(foreign_fn: ForeignFunction, cache_size: usize) -> CachedForeignFunction
fn batch_ffi_calls(calls: List<FfiCall>) -> Result<List<FfiValue>, FfiError>

// JIT コンパイルされた FFI ラッパー
fn compile_ffi_wrapper(signature: FfiSignature) -> Result<CompiledWrapper, FfiError>  // `effect {jit}`
```

## 9. デバッグとテストサポート

### 9.1 非同期デバッグ

```reml
fn trace_async_execution(future: Future<T>) -> Future<(T, ExecutionTrace)>      // `effect {debug}`
fn debug_deadlock_detection() -> Result<List<DeadlockInfo>, DebugError>        // `effect {debug}`
fn async_test_harness<T>(test: Future<T>, timeout: Duration) -> TestResult<T>   // `effect {test}`
```

### 9.2 FFI テスト

```reml
fn mock_foreign_function(signature: FfiSignature, behavior: MockBehavior) -> MockForeignFunction
fn verify_ffi_contract(foreign_fn: ForeignFunction, contract: FfiContract) -> Result<(), FfiError>
```

### 9.3 Unsafe テスト

```reml
fn simulate_memory_corruption(pattern: CorruptionPattern) -> ()                 // `effect {unsafe, test}`
fn test_unsafe_invariants(invariants: List<UnsafeInvariant>) -> TestResult<()> // `effect {unsafe, test}`
```

> 関連: [guides/runtime-bridges.md](guides/runtime-bridges.md), [guides/reml-ffi-handbook.md](guides/reml-ffi-handbook.md), [2.6 実行戦略](2-6-execution-strategy.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md)
