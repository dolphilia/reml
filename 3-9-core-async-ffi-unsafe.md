# 3.9 Core Async / FFI / Unsafe

> 目的：Reml の非同期実行 (`Core.Async`)・FFI (`Core.Ffi`)・unsafe ブロック (`Core.Unsafe`) に関する基本方針と効果タグの枠組みを整理し、今後の詳細仕様策定に備える。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `effect {io.async}`, `effect {ffi}`, `effect {unsafe}`, `effect {blocking}`, `effect {security}` |
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
fn block_on<T>(future: Future<T>) -> Result<T, AsyncError>                    // `effect {blocking}`
fn sleep_async(duration: Duration) -> Future<()>                             // `effect {io.async}`
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
fn buffer<T>(stream: AsyncStream<T>, size: usize) -> AsyncStream<T>             // `effect {io.async, mem}`
fn map_async<T, U>(stream: AsyncStream<T>, f: (T) -> Future<U>) -> AsyncStream<U> // `effect {io.async}`
fn filter_async<T>(stream: AsyncStream<T>, pred: (T) -> Future<Bool>) -> AsyncStream<T> // `effect {io.async}`
fn collect_async<T>(stream: AsyncStream<T>) -> Future<List<T>>                  // `effect {io.async, mem}`
```

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
pub type ForeignFunction = unsafe fn(*mut c_void) -> *mut c_void

fn bind_library(path: Path) -> Result<LibraryHandle, FfiError>               // `effect {ffi}`
fn get_function(handle: LibraryHandle, name: Str) -> Result<ForeignFunction, FfiError> // `effect {ffi}`
fn call_ffi(fn_ptr: ForeignFunction, args: Bytes) -> Result<Bytes, FfiError>  // `effect {ffi, unsafe}`
```

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
let result = add_numbers.call([42, 24])?; // タイプセーフな呼び出し
```

### 2.2 メモリ管理と所有権

```reml
pub type ForeignPtr<T> = {
  ptr: *mut T,
  size: Option<usize>,
  deallocator: Option<fn(*mut T)>,
}

fn wrap_foreign_ptr<T>(ptr: *mut T, size: Option<usize>) -> ForeignPtr<T>       // `effect {unsafe}`
fn foreign_slice<T>(ptr: ForeignPtr<T>, len: usize) -> Result<ForeignSlice<T>, FfiError> // `effect {unsafe}`
fn copy_from_foreign<T: Copy>(ptr: ForeignPtr<T>) -> Result<T, FfiError>       // `effect {unsafe, mem}`
fn copy_to_foreign<T: Copy>(value: T, ptr: ForeignPtr<T>) -> Result<(), FfiError> // `effect {unsafe}`
```

- `call_ffi` は `unsafe` を要求し、境界で `AuditEnvelope` を付与することが推奨される。

## 3. Core.Unsafe の指針

```reml
fn unsafe_block<T>(f: () -> T) -> T                      // `effect {unsafe}`
fn assume(cond: Bool, message: Str) -> ()                // `effect {unsafe}`
fn transmute<T, U>(value: T) -> U                        // `effect {unsafe}`
```

- `unsafe_block` は安全性検証済みのコード領域を明示的に囲む。
- `assume` はコンパイラに対するヒントであり、偽の場合は未定義動作となる。
- `transmute` は型の同じビット表現を再解釈する際に使用。

### 3.1 安全性検証メカニズム

```reml
fn verify_memory_safety(ptr: *const u8, size: usize) -> Result<(), UnsafeError> // `effect {unsafe}`
fn check_alignment<T>(ptr: *const T) -> Bool                                   // `effect {unsafe}`
fn bounds_check(ptr: *const u8, offset: isize, bounds: (usize, usize)) -> Result<(), UnsafeError> // `effect {unsafe}`

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

### 3.2 監査された unsafe 操作

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

## 4. Capability Registry との連携

### 4.1 非同期 Capability

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

### 4.2 FFI Capability

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

### 4.3 Unsafe Capability

```reml
pub type UnsafeCapability = {
  enable_raw_pointers: fn(UnsafePolicy) -> Result<(), CapabilityError>,
  allocate_raw: fn(usize, usize) -> Result<*mut u8, CapabilityError>,
  deallocate_raw: fn(*mut u8, usize, usize) -> Result<(), CapabilityError>,
  track_allocation: fn(*mut u8, usize) -> Result<AllocationId, CapabilityError>,
  verify_pointer: fn(*const u8) -> Result<PointerInfo, CapabilityError>,
}

pub type UnsafePolicy = {
  enable_bounds_checking: Bool,
  enable_use_after_free_detection: Bool,
  enable_double_free_detection: Bool,
  max_allocations: Option<usize>,
  allocation_size_limit: Option<usize>,
}
```

## 5. 使用例（調査メモ）

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

## 6. セキュリティとベストプラクティス

### 6.1 非同期セキュリティ

```reml
// タイムアウトとリソース制限
fn with_async_limits<T>(limits: AsyncLimits, future: Future<T>) -> Future<Result<T, LimitError>>

pub type AsyncLimits = {
  execution_timeout: Option<Duration>,
  memory_limit: Option<usize>,
  concurrent_tasks_limit: Option<usize>,
}
```

### 6.2 FFI セキュリティ

```reml
// サンドボックス内での FFI 呼び出し
fn call_sandboxed<T>(foreign_fn: ForeignFunction, args: FfiArgs, sandbox: FfiSandbox) -> Result<T, FfiError>

pub type FfiSandbox = {
  memory_limit: usize,
  cpu_time_limit: Duration,
  syscall_whitelist: Option<List<SyscallId>>,
  network_access: Bool,
  file_access: FileAccessPolicy,
}
```

### 6.3 Unsafe セキュリティ

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

## 7. パフォーマンス最適化

### 7.1 非同期最適化

```reml
// タスクスケジューリングの調整
fn tune_async_runtime(metrics: AsyncMetrics) -> AsyncRuntimeConfig
fn adaptive_scheduling(workload: WorkloadProfile) -> SchedulingStrategy

// バッチ処理とストリーミングの最適化
fn batch_futures<T>(futures: List<Future<T>>, batch_size: usize) -> Future<List<T>>
fn stream_with_backpressure<T>(stream: AsyncStream<T>, buffer_size: usize) -> AsyncStream<T>
```

### 7.2 FFI 最適化

```reml
// 関数呼び出しのキャッシュ
fn cache_ffi_function(foreign_fn: ForeignFunction, cache_size: usize) -> CachedForeignFunction
fn batch_ffi_calls(calls: List<FfiCall>) -> Result<List<FfiValue>, FfiError>

// JIT コンパイルされた FFI ラッパー
fn compile_ffi_wrapper(signature: FfiSignature) -> Result<CompiledWrapper, FfiError>  // `effect {jit}`
```

## 8. デバッグとテストサポート

### 8.1 非同期デバッグ

```reml
fn trace_async_execution(future: Future<T>) -> Future<(T, ExecutionTrace)>      // `effect {debug}`
fn debug_deadlock_detection() -> Result<List<DeadlockInfo>, DebugError>        // `effect {debug}`
fn async_test_harness<T>(test: Future<T>, timeout: Duration) -> TestResult<T>   // `effect {test}`
```

### 8.2 FFI テスト

```reml
fn mock_foreign_function(signature: FfiSignature, behavior: MockBehavior) -> MockForeignFunction
fn verify_ffi_contract(foreign_fn: ForeignFunction, contract: FfiContract) -> Result<(), FfiError>
```

### 8.3 Unsafe テスト

```reml
fn simulate_memory_corruption(pattern: CorruptionPattern) -> ()                 // `effect {unsafe, test}`
fn test_unsafe_invariants(invariants: List<UnsafeInvariant>) -> TestResult<()> // `effect {unsafe, test}`
```

> 関連: [guides/runtime-bridges.md](guides/runtime-bridges.md), [guides/reml-ffi-handbook.md](guides/reml-ffi-handbook.md), [2.6 実行戦略](2-6-execution-strategy.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md)
