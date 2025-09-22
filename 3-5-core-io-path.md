# 3.5 Core IO & Path

Status: 正式仕様（2025年版）

> 目的：ファイル・ストリーム・パス操作と効果タグ (`effect {io}`) を標準化し、`defer` によるリソース解放や監査ログとの連携を保証する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `effect {io}`, `effect {mut}`, `effect {mem}`, `effect {blocking}`, `effect {async}`, `effect {security}` |
| 依存モジュール | `Core.Prelude`, `Core.Text`, `Core.Collections`, `Core.Diagnostics`, `Core.Numeric & Time` |
| 相互参照 | [2.6 実行戦略](2-6-execution-strategy.md), [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), Guides: [ランタイム連携](guides/runtime-bridges.md) |

## 1. IO モジュール構成

- `use Core.IO;` は同期 IO API（`Reader`, `Writer`, `File`, `Stdin`, `Stdout`）を公開する。
- `use Core.Path;` はパス抽象（`Path`, `PathBuf`, `Glob`, `Watcher`）を提供する。
- すべての IO 関数は `effect {io}` を明記し、`effect {blocking}` フラグで同期ブロッキングの可能性を示す。
- `defer` キーワードと `ScopeGuard` 型を利用してリソース解放を保証する設計を前提とする。

## 2. Reader / Writer 抽象

```reml
trait Reader {
  fn read(&mut self, buf: &mut Bytes) -> Result<usize, IoError>;            // `effect {io, blocking}`
  fn read_exact(&mut self, size: usize) -> Result<Bytes, IoError>;          // `effect {io, blocking}`
}

trait Writer {
  fn write(&mut self, buf: Bytes) -> Result<usize, IoError>;                // `effect {io, blocking}`
  fn flush(&mut self) -> Result<(), IoError>;                               // `effect {io, blocking}`
}

fn copy<R: Reader, W: Writer>(reader: &mut R, writer: &mut W) -> Result<u64, IoError> // `effect {io, blocking}`
fn with_reader<T>(path: Path, f: (FileReader) -> Result<T, IoError>) -> Result<T, IoError> // `effect {io, blocking}`
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
}

pub type IoContext = {
  operation: Str,
  bytes_processed: Option<u64>,
  timestamp: Timestamp,
}
```
- `with_reader` はファイルを開きクロージャへ渡した後、`defer` 相当で自動的に閉じる。

## 3. ファイルとストリーム

```reml
struct File {
  fd: Fd,
  path: PathBuf,
}

fn open(path: Path) -> Result<File, IoError>                       // `effect {io, blocking}`
fn create(path: Path, options: FileOptions) -> Result<File, IoError> // `effect {io, blocking}`
fn metadata(path: Path) -> Result<FileMetadata, IoError>           // `effect {io, blocking}`
fn remove(path: Path) -> Result<(), IoError>                       // `effect {io, blocking}`
fn sync(file: &mut File) -> Result<(), IoError>                    // `effect {io, blocking}`
```

- `FileOptions` は `append`, `truncate`, `permissions` を保持し、`UnixMode`/`WindowsAttributes` を統一的に扱う。
- `FileMetadata` は `size`, `created`, `modified`, `permissions` を含み `Core.Numeric & Time` の `Timestamp`/`Duration` 型と整合。

### 3.1 ストリーミングとバッファ

```reml
type BufferedReader<R: Reader>
fn buffered<R: Reader>(reader: R, capacity: usize) -> BufferedReader<R> // `effect {mem}`
fn read_line(reader: &mut BufferedReader<Reader>) -> Result<Option<Str>, IoError> // `effect {io, blocking}`
```

- バッファ確保時に `effect {mem}` を要求。
- `read_line` は `Str` を返す。`Core.Text` の正規化は呼び出し側で行う。

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
fn glob(pattern: Str) -> Result<List<Path>, PathError>           // `effect {io}`
```

- `PathError` はプラットフォーム依存文字列や無効な UTF-8 バイト列を報告する。
- `normalize` は `.` や `..` を処理し、危険なエスケープを取り除く。
- パストラバーサル攻撃、シンボリックリンク攻撃などのセキュリティ脆弱性を緩和する。

### 4.2 セキュリティヘルパ

```reml
fn validate_path(path: Path, policy: SecurityPolicy) -> Result<Path, SecurityError>  // `effect {security}`
fn sandbox_path(path: Path, root: Path) -> Result<Path, SecurityError>             // `effect {security}`
fn is_safe_symlink(path: Path) -> Result<Bool, IoError>                           // `effect {io, security}`
```

### 4.1 ファイル監視（オプション）

```reml
type WatchEvent = Created(Path) | Modified(Path) | Deleted(Path)

fn watch(paths: List<Path>, callback: (WatchEvent) -> ()) -> Result<Watcher, IoError> // `effect {io, async}`
fn close(watcher: Watcher) -> Result<(), IoError>                                      // `effect {io}`
```

- `effect {async}` を明示し、イベントループとの連携を必要とする。
- 大量のファイル変更監視時にはシステムリソースを保護するメカニズムを提供。

```reml
fn watch_with_limits(paths: List<Path>, limits: WatchLimits, callback: (WatchEvent) -> ()) -> Result<Watcher, IoError>

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

- IO 操作の所要時間を `Core.Numeric & Time` の `Duration` で記録し、`audit_id` と `change_set` を付与するテンプレートを提供。

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

> 関連: [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [2.6 実行戦略](2-6-execution-strategy.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [guides/runtime-bridges.md](guides/runtime-bridges.md)
