# 4.10 Core Async / FFI / Unsafe（ドラフトメモ）

Status: Draft（調査メモ）

> 目的：Reml の非同期実行 (`Core.Async`)・FFI (`Core.Ffi`)・unsafe ブロック (`Core.Unsafe`) に関する基本方針と効果タグの枠組みを整理し、今後の詳細仕様策定に備える。

## 0. ドラフトメタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | Draft（調査メモ） |
| 効果タグ | `effect {io.async}`, `effect {ffi}`, `effect {unsafe}`, `effect {blocking}` |
| 依存モジュール | `Core.Prelude`, `Core.Iter`, `Core.IO`, `Core.Runtime`, `Core.Diagnostics` |
| 相互参照 | [2.6 実行戦略](2-6-execution-strategy.md), [guides/runtime-bridges.md](guides/runtime-bridges.md), [guides/reml-ffi-handbook.md](guides/reml-ffi-handbook.md) |

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

## 4. Capability Registry との連携

- `Core.Runtime` の `CapabilityRegistry` から `SchedulerHandle`, `FfiCapability`, `UnsafeChecker` を取得し、標準的な実装に委譲する。
- `CapabilityId` 例: `"io.async"`, `"ffi"`, `"unsafe"`。

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

> 関連: [guides/runtime-bridges.md](guides/runtime-bridges.md), [guides/reml-ffi-handbook.md](guides/reml-ffi-handbook.md), [2.6 実行戦略](2-6-execution-strategy.md)
