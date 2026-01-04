# Core.Unsafe.Ptr 運用ガイド

> 目的：仕様章 [3.9 Core Async / FFI / Unsafe](../../spec/3-9-core-async-ffi-unsafe.md#3-coreunsafeptr-api) で正式化された `Core.Unsafe.Ptr` API を安全かつ効率的に活用するためのベストプラクティスをまとめる。
> 仕様参照：型・関数の定義は 3.9 §3 に従う。本ガイドでは利用時のチェックリスト、監査フロー、補助的なサンプルを提供する。

## 1. 型定義

```reml
module Core.Unsafe.Ptr {
  type Ptr<T>
  type MutPtr<T>
  type NonNullPtr<T>
  type VoidPtr = Ptr<void>
  type FnPtr<Args, Ret>
  type Span<T> = { ptr: NonNullPtr<T>, len: usize }
  type TaggedPtr<T> = { raw: Ptr<T>, label: Option<Str> }
}
```

- `Ptr<T>`: NULL 許容。読み取り専用操作のみ許可。
- `MutPtr<T>`: 可変参照相当。重複書き込みで未定義動作の可能性。
- `NonNullPtr<T>`: 非 NULL 保証を持つ `Ptr<T>`。`Span<T>` など境界検査付きラッパの基礎。
- `VoidPtr`: 型不明境界。FFI でのキャスト前提。
- `FnPtr<Args, Ret>`: FFI の関数ポインタ、クロージャを含まない素のコードポインタ。
- `Span<T>`: `ptr` + `len` の境界付きビュー。`len = 0` の場合でも `ptr` は非 NULL を維持する。
- `TaggedPtr<T>`: 監査やテスト診断に利用するラベル付きポインタ。`tag` API で生成する。

## 2. 生成・変換 API

```reml
fn addr_of<T>(value: &T) -> Ptr<T>
fn addr_of_mut<T>(value: &mut T) -> MutPtr<T>
fn from_option<T>(opt: Option<NonNullPtr<T>>) -> Ptr<T>
fn require_non_null<T>(ptr: Ptr<T>) -> Result<NonNullPtr<T>, UnsafeError>
fn cast<T, U>(ptr: Ptr<T>) -> Ptr<U> unsafe
fn cast_mut<T, U>(ptr: MutPtr<T>) -> MutPtr<U> unsafe
fn to_int<T>(ptr: Ptr<T>) -> usize unsafe
fn from_int<T>(addr: usize) -> Ptr<T> unsafe
```

- `addr_of/addr_of_mut`: 評価順序を固定し、未初期化の借用に頼らずにアドレス取得。
- `require_non_null`: 安全境界で Option 化。失敗時は `UnsafeErrorKind::NullPointer` を返し、メッセージにアドレス値を含める。
- `cast*` / `to_int` / `from_int`: 常に `unsafe`。整列・サイズ制約違反が UB になることを仕様に記載。

## 3. 読み書き・コピー API

```reml
fn read<T>(ptr: Ptr<T>) -> Result<T, UnsafeError> unsafe
fn read_unaligned<T>(ptr: Ptr<T>) -> Result<T, UnsafeError> unsafe
fn write<T>(ptr: MutPtr<T>, value: T) -> Result<(), UnsafeError> unsafe
fn write_unaligned<T>(ptr: MutPtr<T>, value: T) -> Result<(), UnsafeError> unsafe
fn copy_to<T>(src: Ptr<T>, dst: MutPtr<T>, count: usize) -> Result<(), UnsafeError> unsafe
fn copy_nonoverlapping<T>(src: Ptr<T>, dst: MutPtr<T>, count: usize) -> Result<(), UnsafeError> unsafe
fn fill<T: Copy>(dst: MutPtr<T>, value: T, count: usize) -> Result<(), UnsafeError> unsafe
```

- `read`/`write`: 標準整列を要求。違反時は UB。`*_unaligned` で回避可能。
- `copy_to`: 重複許容（`memmove`）。
- `copy_nonoverlapping`: 非重複前提で `memcpy` 最適化を可能に。
- `fill`: 既知値で領域初期化。`T: Copy` 制約を仕様に追加予定。

## 4. アドレス計算

```reml
fn add<T>(ptr: Ptr<T>, count: usize) -> Ptr<T> unsafe
fn add_mut<T>(ptr: MutPtr<T>, count: usize) -> MutPtr<T> unsafe
fn offset<T>(ptr: Ptr<T>, delta: isize) -> Ptr<T> unsafe
fn byte_offset<T>(ptr: Ptr<T>, bytes: isize) -> Ptr<T> unsafe

fn span_from_raw_parts<T>(ptr: Ptr<T>, len: usize) -> Result<Span<T>, UnsafeError>
fn span_split_at<T>(span: Span<T>, index: usize) -> Result<(Span<T>, Span<T>), UnsafeError>
fn span_as_ptr<T>(span: Span<T>) -> Ptr<T>
fn span_as_mut_ptr<T>(span: Span<T>) -> MutPtr<T>
```

- `add`/`add_mut`: 正方向だけを対象にし、同一アロケーション内での使用を想定。
- `offset`: 正負の任意移動。境界外に出ると UB。
- `byte_offset`: バイト単位移動。構造体ビュー構築に利用。
- `span_from_raw_parts`: `Ptr<T>` と長さから `Span<T>` を生成。`len = 0` の場合も `ptr` が無効な非NULLにならないよう検証する。
- `span_split_at`: スパンを安全に分割。境界外インデックスでは `UnsafeErrorKind::OutOfBounds`。
- `span_as_ptr` / `span_as_mut_ptr`: `Span<T>` から `Ptr`/`MutPtr` を得る際は、後続操作が `effect {memory}` を伴うことをドキュメントする。

## 5. 監査・診断補助

```reml
fn tag<T>(ptr: Ptr<T>, label: Str) -> TaggedPtr<T>
fn debug_repr<T>(ptr: Ptr<T>) -> Str
```

- `tag`: デバッグビルドでアサーションや監査ログへメタデータを添付するためのフック（Release では no-op 想定）。
- `debug_repr`: `0x` 付き16進表示と `label` を出力。効果は `unsafe` に分類しない。

## 6. テスト可能なユースケース

### 6.1 FFI コール境界
- C ライブラリへ `Ptr<u8>` を渡し、戻りポインタを `require_non_null` で検証。
- `FnPtr` を受け取るコールバック API で `cast` を利用し、`audit.log` に `tag` 付きで記録。

### 6.2 バッファ操作
- `Span<u8>` を生成し、`copy_to`/`fill` でパケット操作を行うベンチマーク。
- ASCII 専用バッファに `write_unaligned` を利用し、`@no_blocking` の I/O API から呼び出すパスを検証。

### 6.3 GC ルート登録
- `NonNullPtr<Object>` を `runtime::register_root` に渡し、`defer` で `unregister_root` を保証するテスト。
- `byte_offset` でフィールドアドレスを算出し、書き込みバリア (`write_barrier`) と組み合わせて世代間更新を検証。


## 7. 動作例ドラフト

### 7.1 FFI: C の `strlen` を呼び出す

```reml
extern "C" fn strlen(ptr: Ptr<u8>) -> usize

fn c_strlen(input: String) -> usize = {
  unsafe {
    let bytes = input.asBytes();
    let ptr = bytes.asPtr();
    // UTF-8 の途中でコピーしないため、NULL は含まれない前提
    strlen(ptr)
  }
}
```

* `asBytes`: `Span<u8>` を返す想定。境界チェック付きで NULL 終端を検査。
* `strlen` は `ffi` + `unsafe` 効果を持つため、呼び出し側関数も `unsafe` 効果を記録する。

### 7.2 バッファ操作: 固定長ヘッダの読み取り

```reml
fn parse_header(bytes: Span<u8>) -> Result<Header, ParseError> = {
  if bytes.len < HEADER_LEN { return Err(ParseError::Truncated) }
  let field_ptr = unsafe { bytes.ptr.add(OFFSET_VERSION) }
  let version = unsafe { field_ptr.read() }
  ...
}
```

* `Span<u8>` による長さチェックの後で `add` を使用。
* `read` は `unsafe` なので局所的にブロックを閉じ込め、境界チェック済みであることをコメントで明示。

### 7.3 GC ルート登録: RAII 風ハンドル

```reml
struct RootGuard {
  ptr: NonNullPtr<Object>
}

impl RootGuard {
  fn new(ptr: NonNullPtr<Object>) -> Result<RootGuard, UnsafeError> = {
    unsafe { runtime::register_root(ptr)? }
    Ok(RootGuard { ptr })
  }

  fn release(self) -> Result<(), UnsafeError> = {
    unsafe { runtime::unregister_root(self.ptr) }
  }
}

impl Drop for RootGuard {
  fn drop(self) {
    let _ = self.release();
  }
}
```

* `register_root`/`unregister_root` は `unsafe`。
* `Drop` 実装で `defer` 相当の解放を保証する。

### 7.4 FFI コールバック: `bind_fn_ptr` の利用

```reml
extern "C" {
  fn register_callback(cb: FnPtr<(i32,), ()>);
}

fn install_callback(audit: AuditSink) -> Result<(), Diagnostic> = {
  unsafe {
    let stub = bind_fn_ptr(|value: i32| {
      AuditContext::new("ffi", "callback")?
        .log("ffi.callback", json!({ "value": value }))?;
      Ok(())
    })?; // Result<ForeignStub<(i32,), ()>, UnsafeError>

    register_callback(stub.raw);
  }
  Ok(())
}
```

* `bind_fn_ptr` は Reml クロージャを ABI 検証済みの `ForeignStub` に変換し、シンボル登録前に `UnsafeErrorKind::InvalidSignature` を検出できる。
* コールバック内部では `AuditContext` を使用して `effect {audit}` を発生させ、FFI 経由の非同期イベントでも監査ログと Capability 設定を同期する。

## 8. CI スモークテスト要件

1. **ffi-smoke**: C 側の `strlen` と同等の関数を呼び出し、`Ptr<u8>` の NULL 非許容／NULL 許容双方を検証する。`audit.log` に `ffi.call` が残ることをアサート。
2. **buffer-span**: `Span<u8>` から `Ptr<u8>` を降格後、`read`/`write`/`copy_nonoverlapping` を試し、境界外アクセス時に安全 API がエラーを返すことをテスト。


## 9. 改訂タスクリスト（状態トラッカー）

| ステータス | 項目 |
| --- | --- |
| ✅ | `Span<T>` 定義の更新 (`ptr: NonNullPtr<T>, len: usize`) と `span_from_raw_parts` 系ユーティリティの追記 |
| ✅ | `bind_fn_ptr` のコールバック例を追加し、監査ログと併用するパターンを明示 |
| 🔄 | `effect {memory}` を伴う操作と `CapabilitySecurity.effect_scope` の対応チェックリストを作成 |
| 🔄 | `MappedMemory` ⇄ `Span<u8>` 変換ガイドラインを追加（Core.Memory 連携） |
| 🔄 | `audited_unsafe_block` + `AuditContext` を用いた低レベル監査サンプルを整備 |
| 🔄 | `alignment-check` / `thread-send-audit` テストのサンプルコードを補完 |

開発中の CI スモークテスト (`ffi-smoke` / `buffer-span` / `gc-root-guard` / `alignment-check` / `thread-send-audit`) は、`core-unsafe-ptr` ジョブで実行し、失敗時に `Diagnostic` が `effect_flags` と `ptr_label` を含むことを検証する。


---
> TODO: 監査ログテンプレートと CI スクリプト断片（`alignment-check`, `thread-send-audit`）を追記し、仕様 3.9 §3 との整合テーブルを付録化する。
