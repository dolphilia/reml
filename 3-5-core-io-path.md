# 4.6 Core IO & Path（フェーズ3 ドラフト）

Status: Draft（内部レビュー中）

> 目的：ファイル・ストリーム・パス操作と効果タグ (`effect {io}`) を標準化し、`defer` によるリソース解放や監査ログとの連携を保証する。

## 0. ドラフトメタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | Draft（フェーズ3） |
| 効果タグ | `effect {io}`, `effect {mut}`, `effect {mem}`, `effect {blocking}`, `effect {async}` |
| 依存モジュール | `Core.Prelude`, `Core.Text`, `Core.Collections`, `Core.Diagnostics`, `Core.Numeric & Time` |
| 相互参照 | [2.6 実行戦略](2-6-execution-strategy.md), [4.5 Core Numeric & Time](4-5-core-numeric-time.md), Guides: [ランタイム連携](guides/runtime-bridges.md) |

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

- `IoError` は `kind: IoErrorKind` と `message: Str`、`path: Option<Path>` を保持し `Diagnostic` へ変換可能。
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

### 4.1 ファイル監視（オプション）

```reml
type WatchEvent = Created(Path) | Modified(Path) | Deleted(Path)

fn watch(paths: List<Path>, callback: (WatchEvent) -> ()) -> Result<Watcher, IoError> // `effect {io, async}`
fn close(watcher: Watcher) -> Result<(), IoError>                                      // `effect {io}`
```

- `effect {async}` を明示し、イベントループとの連携を必要とする。

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

- `with_reader` と `buffered` を組み合わせ、`Config.parse_yaml`（Chapter 4.8 予定）に渡す例。
- 所要時間を `log_io` で監査ログに記録し、`audit_id` を伝播。

> 関連: [4.5 Core Numeric & Time](4-5-core-numeric-time.md), [2.6 実行戦略](2-6-execution-strategy.md), [guides/runtime-bridges.md](guides/runtime-bridges.md)
