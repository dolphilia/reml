# 3.5 Core IO & Path

> 目的：ファイル・ストリーム・パス操作と効果タグ (`effect {io}`) を標準化し、`defer` によるリソース解放や監査ログとの連携を保証する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `effect {io}`, `effect {mut}`, `effect {mem}`, `effect {io.blocking}`, `effect {io.async}`, `effect {security}` |
| 依存モジュール | `Core.Prelude`, `Core.Text`, `Core.Collections`, `Core.Diagnostics`, `Core.Numeric & Time` |
| 相互参照 | [2.6 実行戦略](2-6-execution-strategy.md), [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), Guides: [ランタイム連携](../guides/runtime/runtime-bridges.md) |

## 1. IO モジュール構成

- `use Core.IO;` は同期 IO API（`Reader`, `Writer`, `File`, `Stdin`, `Stdout`）を公開する。
- `use Core.Path;` はパス抽象（`Path`, `PathBuf`, `Glob`, `Watcher`）を提供する。
- すべての IO 関数は `effect {io}` を明記し、`effect {io.blocking}` フラグで同期ブロッキングの可能性を示す。
- `defer` キーワードと `ScopeGuard` 型を利用してリソース解放を保証する設計を前提とする。

## 2. Reader / Writer 抽象

```reml
trait Reader {
  fn read(&mut self, buf: &mut Bytes) -> Result<usize, IoError>;            // `effect {io, io.blocking}`
  fn read_exact(&mut self, size: usize) -> Result<Bytes, IoError>;          // `effect {io, io.blocking}`
}

trait Writer {
  fn write(&mut self, buf: Bytes) -> Result<usize, IoError>;                // `effect {io, io.blocking}`
  fn flush(&mut self) -> Result<(), IoError>;                               // `effect {io, io.blocking}`
}

fn copy<R: Reader, W: Writer>(reader: &mut R, writer: &mut W) -> Result<u64, IoError> // `effect {io, io.blocking}`
fn with_reader<T>(path: Path, f: (FileReader) -> Result<T, IoError>) -> Result<T, IoError> // `effect {io, io.blocking}`
```

- `IoError` は `kind: IoErrorKind` と `message: Str`、`path: Option<Path>` を保持し `IntoDiagnostic` トレイト経由で診断システムと連携する。

```reml
pub type IoError = {
  kind: IoErrorKind,
  message: Str,
  path: Option<Path>,
  context: Option<IoContext>,
}

pub enum IoErrorKind = {
  NotFound,
  PermissionDenied,
  ConnectionRefused,
  InvalidInput,
  TimedOut,
  WriteZero,
  Interrupted,
  UnexpectedEof,
  OutOfMemory,
  SecurityViolation,
  UnsupportedPlatform,
}

pub type IoContext = {
  operation: Str,
  path: Option<Path>,
  capability: Option<CapabilityId>,
  bytes_processed: Option<u64>,
  timestamp: Timestamp,
  effects: EffectLabels,
  buffer: Option<BufferStats>,
  watch: Option<WatchStats>,
  glob: Option<GlobStats>,
}
```

- `path` は操作対象の `Path`/`PathBuf` を保持し、`capability` は `CapabilityRegistry::verify_capability_stage` の結果（例: `io.fs.read`, `memory.buffered_io`, `security.fs.policy`）を格納する。
- `effects` は `effect {io}` / `{io.blocking}` / `{mem}` / `{security}` などの真偽値と `io_blocking_calls` / `mem_bytes` 等のカウンタを含み、`buffer`・`watch`・`glob` は `BufferedReader`, `Watcher`, `glob` API が収集した補助メタデータ（容量/残量、キューサイズ/遅延、glob パターン/拒否パス）を保持する。

- Rust Runtime では `Reader`/`Writer` 呼び出しの直前に `take_io_effects_snapshot()` を実行し、`IoContext.operation`（例: `"reader.read"`, `"io.copy"`）と `metadata.io.helper`（`copy`/`with_reader` 等のヘルパ名）を常に記録する。`metadata.io.bytes_processed` は `buf.len()` または実際に転送したバイト数で更新され、`core_io.reader_writer_effects_pass_rate` の検証対象となる。
- `IoError::into_diagnostic()` は `metadata.io.*` / `extensions.io.*` に `operation`, `path`, `capability`, `bytes_processed`, `buffer.capacity/fill`, `watch.queue_size/delay_ns`, `glob.pattern/offending_path` を転記し、`extensions.effects.*` に `EffectLabels` を展開する。`audit_metadata["io.*"]` にも同じキーが保存され、監査シナリオ（[3-6](3-6-core-diagnostics-audit.md)）から参照できる。
- Windows 固有 API や POSIX 拡張に依存する操作は、対象プラットフォームで提供されない場合に `IoErrorKind::UnsupportedPlatform` を返す。診断 `target.config.unsupported_value`（[2-5](2-5-error.md#b-9-条件付きコンパイル関連診断)）と連動し、`notes` に要求した機能と検出済みプラットフォームを記録する。
- `with_reader` はファイルを開きクロージャへ渡した後、`defer` 相当で自動的に閉じる。

## 3. ファイルとストリーム

```reml
struct File {
  fd: Fd,
  path: PathBuf,
}

fn open(path: Path) -> Result<File, IoError>                       // `effect {io, io.blocking}`
fn create(path: Path, options: FileOptions) -> Result<File, IoError> // `effect {io, io.blocking}`
fn metadata(path: Path) -> Result<FileMetadata, IoError>           // `effect {io, io.blocking}`
fn remove(path: Path) -> Result<(), IoError>                       // `effect {io, io.blocking}`
fn sync(file: &mut File) -> Result<(), IoError>                    // `effect {io, io.blocking}`
```

- `FileOptions` は `append`, `truncate`, `permissions` を保持し、`UnixMode`/`WindowsAttributes` を統一的に扱う。
- `FileMetadata` は `size`, `created`, `modified`, `permissions` を含み `Core.Numeric & Time` の `Timestamp`/`Duration` 型と整合。

### 3.1 ストリーミングとバッファ

```reml
type BufferedReader<R: Reader>
fn buffered<R: Reader>(reader: R, capacity: usize) -> BufferedReader<R> // `effect {mem}`
fn read_line(reader: &mut BufferedReader<Reader>) -> Result<Option<Str>, IoError> // `effect {io, io.blocking}`
```

- バッファ確保時に `effect {mem}` を要求。
- `read_line` は `Str` を返す。`Core.Text` の正規化は呼び出し側で行う。
- `Core.Memory`（公式プラグイン [5-3](5-3-memory-plugin.md)）で定義する `MappedMemory` や `Span<u8>` と連携する場合は、`memory` 効果が `CapabilitySecurity.effect_scope` に含まれていることを確認する。

## 4. Path 抽象

```reml
struct Path {
  raw: Bytes,
}

fn path(str: Str) -> Result<Path, PathError>                     // `@pure`
fn join(base: Path, segment: Str) -> Path                        // `@pure`
fn parent(path: Path) -> Option<Path>                            // `@pure`
fn normalize(path: Path) -> Path                                 // `@pure`
fn is_absolute(path: Path) -> Bool                               // `@pure`
fn glob(pattern: Str) -> Result<List<Path>, PathError>           // `effect {io, io.blocking}`
```

- `PathError` はプラットフォーム依存文字列や無効な UTF-8 バイト列を報告する。
- `normalize` は `.` や `..` を処理し、危険なエスケープを取り除く。
- `glob` は `FsAdapter` / `CapabilityRegistry` を介して `CapabilityId = "io.fs.read"` を検証し、`metadata.io.glob.pattern`・`metadata.io.glob.offending_path`・`metadata.io.helper = "path.glob"` を診断へ記録する。失敗時は `PathErrorKind::InvalidPattern` または `IoErrorKind::UnsupportedPlatform` を `core.path.glob.*` 診断コードへ変換する。
- パストラバーサル攻撃、シンボリックリンク攻撃などのセキュリティ脆弱性を緩和する。

### 4.2 セキュリティヘルパ

```reml
fn validate_path(path: Path, policy: SecurityPolicy) -> Result<Path, SecurityError>  // `effect {security}`
fn sandbox_path(path: Path, root: Path) -> Result<Path, SecurityError>             // `effect {security}`
fn is_safe_symlink(path: Path) -> Result<Bool, IoError>                           // `effect {io, io.blocking, security}`
```

- セキュリティポリシーの適用と診断メタデータの整合例は [examples/practical/core_path/security_check/relative_denied.reml](../../examples/practical/core_path/security_check/relative_denied.reml) を参照。`SecurityPolicy` で許可ルートを定義 → `validate_path` で拒否理由を `metadata.security.reason` に記録 → `sandbox_path` で canonical path を得てから `is_safe_symlink` で `effect {security}` を計測する順序を示している。

### 4.3 文字列ユーティリティ（クロスプラットフォーム）

```reml
pub enum PathStyle = Native | Posix | Windows

fn normalize_path(text: Str, style: PathStyle = Native) -> Result<Str, PathError>    // `@pure`
fn join_paths(parts: List<Str>, style: PathStyle = Native) -> Result<Str, PathError>  // `@pure`
fn is_absolute_str(text: Str, style: PathStyle = Native) -> Bool                      // `@pure`
```

* `Native` は `platform_info().os`（[3-8](3-8-core-runtime-capability.md)）に基づいてセパレータとドライブ表現を選択する。`Posix` は `/` 区切り、`Windows` は `\` 区切りとドライブレター/UNC を許可する。
* `normalize_path` は重複セパレータと `.` / `..` を畳み込み、危険なドライブプレフィックスを拒否する。結果は選択した `style` に揃えた文字列で返す。`Path` へ変換する場合は `path(normalize_path(...))` を利用する。
* `join_paths` は `parts` の各要素を検証し、`style` に従ってエスケープやセパレータ補完を行う。絶対パスが途中に現れた場合は先頭に揃えて後続部分を安全に結合する。
* `is_absolute_str` は入力が完全修飾パスかどうかを判定する。Windows ではドライブレター (`C:`) や UNC (`\\server\\share`) を認識する。
* 文字列ユーティリティでエラーが発生した場合は `PathError` に `PathErrorKind::InvalidEncoding` / `UnsupportedPlatform` を設定し、併せて `IoErrorKind::UnsupportedPlatform` へ変換可能とする。IDE へは `Diagnostic.extensions["cfg"].evaluated` を通じてプラットフォーム差異を説明することを推奨する。

### 4.4 ファイル監視（オプション）

```reml
type WatchEvent = Created(Path) | Modified(Path) | Deleted(Path)

fn watch(paths: List<Path>, callback: (WatchEvent) -> ()) -> Result<Watcher, IoError> // `effect {io, io.async}`
fn close(watcher: Watcher) -> Result<(), IoError>                                      // `effect {io}`
```

- `effect {io.async}` を明示し、イベントループとの連携を必要とする。
- 大量のファイル変更監視時にはシステムリソースを保護するメカニズムを提供。

```reml
fn watch_with_limits(paths: List<Path>, limits: WatchLimits, callback: (WatchEvent) -> ()) -> Result<Watcher, IoError> // `effect {io, io.async}`

pub type WatchLimits = {
  max_events_per_second: u32,
  max_depth: Option<u8>,
  exclude_patterns: List<Str>,
}
```

## 5. リソース解放と `defer`

`ScopeGuard` により確実な解放を支援する。

```reml
struct ScopeGuard<T> {
  value: T,
  on_drop: fn(T),
}

fn guard<T>(value: T, on_drop: fn(T)) -> ScopeGuard<T>             // `@pure`
fn into_inner<T>(guard: ScopeGuard<T>) -> T                        // `@pure`
```

- `with_reader` / `with_writer` は内部で `ScopeGuard` を利用して `File.close` を保証する。
- `defer` キーワードは `ScopeGuard` の糖衣であり、`effect` を変化させない。
- リソースリークを防ぐために、未解放リソースの監視機能を提供。

```reml
fn track_resources(enabled: Bool) -> ()                           // `effect {debug}`
fn list_open_resources() -> List<ResourceHandle>                  // `effect {debug}`
fn force_cleanup_resources() -> Result<u32, IoError>              // `effect {io, debug}`
```

## 6. 監査ログ連携

```reml
fn log_io(event: Str, path: Option<Path>, duration: Duration, audit: AuditSink) -> Result<(), Diagnostic> // `effect {audit}`
```

- IO 操作の所要時間を `Core.Numeric & Time` の `Duration` で記録し、`audit_id` と `change_set` を付与するテンプレートを提供。`AuditContext`（[3.6](3-6-core-diagnostics-audit.md)）と組み合わせることで、`SyscallCapability.audited_syscall` の前後で統一された監査レコードを生成できる。

## 7. 使用例（設定ファイル読み込み）

```reml
use Core;
use Core.IO;
use Core.Path;
use Core.Config;
use Core.Numeric;

fn load_config(path: Str, audit: AuditSink) -> Result<AppConfig, Diagnostic> =
  let file_path = path(path)?;
  let start = Core.Numeric.now()?;

  let config = with_reader(file_path.clone(), |reader| {
    reader
      |> buffered(capacity=64 * 1024)
      |> read_line
      |> Iter.from
      |> Iter.take_while(|line| line.is_some())
      |> Iter.map(|line| line.expect("line"))
      |> Config.parse_yaml()
  })?;

  let elapsed = duration_between(start?, Core.Numeric.now()?);
  log_io("config.load", Some(file_path.clone()), elapsed, audit)?;
  Ok(config)
```

- `with_reader` と `buffered` を組み合わせ、`Config.parse_yaml`（Chapter 3.7）に渡す例。
- 所要時間を `log_io` で監査ログに記録し、`audit_id` を伝播。
- ファイル入出力とパスサンドボックスを同時に扱うサンプルは [examples/practical/core_io/file_copy/canonical.reml](../../examples/practical/core_io/file_copy/canonical.reml) へ収録している。`with_reader` / `with_writer` / `copy` を `sandbox_path` と組み合わせ、`log_io("examples.core_io.file_copy", ...)` で `metadata.io.helper` と `effect.stage.*` を監査ログへ残す実装例である。

## 8. 非同期 IO との統合

### 8.1 同期・非同期ブリッジ

```reml
// 同期 IO を非同期コンテキストで使用
fn async_read<R: Reader>(reader: R) -> AsyncResult<Bytes>          // `effect {io.async}`
fn async_write<W: Writer>(writer: W, data: Bytes) -> AsyncResult<usize> // `effect {io.async}`

// バッチ処理とストリーミング
fn batch_process_files(paths: List<Path>, processor: (Path) -> Result<T, IoError>) -> AsyncResult<List<T>>
fn stream_file_contents(path: Path) -> AsyncResult<AsyncIter<Bytes>>
```

### 8.2 リソースプールと最適化

```reml
pub type IoPool = {
  max_concurrent: u32,
  timeout: Duration,
  retry_policy: RetryPolicy,
}

fn with_io_pool<T>(pool: IoPool, operation: () -> Result<T, IoError>) -> Result<T, IoError>
fn parallel_file_ops<T>(operations: List<() -> Result<T, IoError>>, pool: IoPool) -> Result<List<T>, IoError>
```

> 関連: [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [2.6 実行戦略](2-6-execution-strategy.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [guides/runtime-bridges.md](../guides/runtime/runtime-bridges.md)

## 9. Resource Limit ユーティリティ (`Core.Resource`)

> 目的：Conductor、Sandbox、RunConfig が共通のリソース制限表現を共有し、0-1 §1.1（性能）および §1.2（安全性）の基準に沿って静的検証できるようにする。

```reml
pub module Core.Resource

pub enum MemoryLimit =
  | Unlimited
  | Absolute { bytes: NonZeroU64 }
  | Relative { percent_of_physical: Float }
  | Soft { soft_bytes: NonZeroU64, hard_bytes: Option<NonZeroU64> };

pub enum CpuQuota =
  | Unlimited
  | Fraction { share: Float }
  | MilliCores(NonZeroU32)
  | FixedCores(NonZeroU16);

pub type MemoryLimitResolved = {
  declaration: MemoryLimit,
  hard_bytes: NonZeroU64,
  soft_bytes: Option<NonZeroU64>,
}

pub type CpuQuotaNormalized = {
  declaration: CpuQuota,
  scheduler_slots: NonZeroU16,
  share: Float,
}

pub enum ResourceLimitError =
  | ZeroOrNegative
  | PercentageOutOfRange { min: Float, max: Float }
  | MissingBaseline
  | ExceedsPhysicalMemory { requested: u64, available: u64 }
  | SchedulingOverflow { requested: u16, available: u16 };

fn MemoryLimit::hard(bytes: NonZeroU64) -> MemoryLimit
fn MemoryLimit::mebibytes(mib: NonZeroU64) -> MemoryLimit
fn MemoryLimit::relative(percent: Float) -> Result<MemoryLimit, ResourceLimitError>
fn MemoryLimit::resolve(total_physical: Option<NonZeroU64>) -> Result<MemoryLimitResolved, ResourceLimitError>

fn CpuQuota::fraction(share: Float) -> Result<CpuQuota, ResourceLimitError>
fn CpuQuota::milli_cores(mcores: NonZeroU32) -> CpuQuota
fn CpuQuota::cores(cores: NonZeroU16) -> CpuQuota
fn CpuQuota::normalize(logical_cores: NonZeroU16, scheduler_parallelism: NonZeroU16) -> Result<CpuQuotaNormalized, ResourceLimitError>
```

- `Relative.percent_of_physical` は `0.01 <= share <= 1.0` を要求し、`resolve` 時に物理メモリ総量が提供されない場合は `ResourceLimitError::MissingBaseline` を返す。要求値が総量を超えた場合は `ExceedsPhysicalMemory` を報告し、診断 `conductor.resource.limit_exceeded`（3-6 §6.1.2）に変換される。
- `Soft` はガーベジコレクタやページングを許容する設定であり、`hard_bytes` が指定された場合は `hard_bytes >= soft_bytes` を保証する。省略した場合でも `resolve` は `soft_bytes` を返却し、ランタイムが監視する閾値となる。
- `CpuQuota::fraction` は `0.05 <= share <= 1.0` を満たす必要がある。`normalize` は論理コア数と `ExecutionPlan.strategy` から得られる並列度を考慮し、必要スロット数を切り上げて算出する。スロットが `scheduler_parallelism` を超えた場合は `SchedulingOverflow`。
- これらの型は `serde` 互換なリテラル（例: `{ memory = { absolute = { bytes = 134217728 } } }`）と DSL からのビルダー（例: `MemoryLimit::mebibytes(128)`、`CpuQuota::fraction(0.5)`）の双方で構築できる。文字列表現（"128MB" 等）は廃止し、コンパイラが型チェック可能な API へ移行する。
- Conductor/RunConfig/Sandbox は `MemoryLimitResolved` と `CpuQuotaNormalized` を共有して監査・診断へ記録する。正規化ロジックを再利用し、実行前に単位換算が完了している状態を標準とする。
