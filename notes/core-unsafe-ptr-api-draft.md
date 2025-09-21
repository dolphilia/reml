# Core.Unsafe.Ptr API 草案

> 目的：Reml の `unsafe` セクションで利用する原始ポインタ API を整理し、FFI・バッファ操作・GC 連携のユースケースを検証可能な形で定義する。

## 1. 型定義

```reml
module Core.Unsafe.Ptr {
  type Ptr<T>
  type MutPtr<T>
  type NonNullPtr<T>
  type VoidPtr = Ptr<void>
  type FnPtr<Args, Ret>
  type Span<T> = { base: NonNullPtr<T>, len: usize }
}
```

- `Ptr<T>`: NULL 許容。読み取り専用操作のみ許可。
- `MutPtr<T>`: 可変参照相当。重複書き込みで未定義動作の可能性。
- `NonNullPtr<T>`: 非NULL保証を持つ `Ptr<T>`。`Span<T>` など境界検査付きラッパの基礎。
- `VoidPtr`: 型不明境界。FFI でのキャスト前提。
- `FnPtr<Args, Ret>`: FFI の関数ポインタ、クロージャを含まない素のコードポインタ。
- `Span<T>`: `base` + `len` の境界付きビュー。`unsafe` なしでも読み取り可能な API を別モジュールで提供する予定。

## 2. 生成・変換 API

```reml
fn addr_of<T>(value: &T) -> Ptr<T>
fn addr_of_mut<T>(value: &mut T) -> MutPtr<T>
fn from_option<T>(opt: Option<NonNullPtr<T>>) -> Ptr<T>
fn require_non_null<T>(ptr: Ptr<T>) -> Result<NonNullPtr<T>, NullError>
fn cast<T, U>(ptr: Ptr<T>) -> Ptr<U> unsafe
fn cast_mut<T, U>(ptr: MutPtr<T>) -> MutPtr<U> unsafe
fn to_int<T>(ptr: Ptr<T>) -> usize unsafe
fn from_int<T>(addr: usize) -> Ptr<T> unsafe
```

- `addr_of/addr_of_mut`: 評価順序を固定し、未初期化の借用に頼らずにアドレス取得。
- `require_non_null`: 安全境界で Option 化。`NullError` はメッセージとアドレス値を保持。
- `cast*` / `to_int` / `from_int`: 常に `unsafe`。整列・サイズ制約違反が UB になることを仕様に記載。

## 3. 読み書き・コピー API

```reml
fn read<T>(ptr: Ptr<T>) -> T unsafe
fn read_unaligned<T>(ptr: Ptr<T>) -> T unsafe
fn write<T>(ptr: MutPtr<T>, value: T) unsafe
fn write_unaligned<T>(ptr: MutPtr<T>, value: T) unsafe
fn copy_to<T>(src: Ptr<T>, dst: MutPtr<T>, count: usize) unsafe
fn copy_nonoverlapping<T>(src: Ptr<T>, dst: MutPtr<T>, count: usize) unsafe
fn fill<T>(dst: MutPtr<T>, value: T, count: usize) unsafe
```

- `read`/`write`: 標準整列を要求。違反時は UB。`*_unaligned` で回避可能。
- `copy_to`: 重複許容（`memmove`）。
- `copy_nonoverlapping`: 非重複前提で `memcpy` 最適化を可能に。
- `fill`: 既知値で領域初期化。`T: Copy` 制約を仕様に追加予定。

## 4. アドレス計算

```reml
fn add<T>(ptr: Ptr<T>, count: usize) -> Ptr<T>
fn add_mut<T>(ptr: MutPtr<T>, count: usize) -> MutPtr<T>
fn offset<T>(ptr: Ptr<T>, delta: isize) -> Ptr<T>
fn byte_offset<T>(ptr: Ptr<T>, bytes: isize) -> Ptr<T>
```

- `add`/`add_mut`: 正方向だけを対象にし、同一アロケーション内での使用を想定。
- `offset`: 正負の任意移動。境界外に出ると UB。
- `byte_offset`: バイト単位移動。構造体ビュー構築に利用。

## 5. 監査・診断補助

```reml
fn tag<T>(ptr: Ptr<T>, label: String) -> TaggedPtr<T>
fn debug_repr<T>(ptr: Ptr<T>) -> String
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
  let field_ptr = unsafe { bytes.base.add(OFFSET_VERSION) }
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
  fn new(ptr: NonNullPtr<Object>) -> RootGuard = {
    unsafe { runtime::register_root(ptr) }
    RootGuard { ptr }
  }

  fn release(self) {
    unsafe { runtime::unregister_root(self.ptr) }
  }
}

impl Drop for RootGuard {
  fn drop(self) = self.release()
}
```

* `register_root`/`unregister_root` は `unsafe`。
* `Drop` 実装で `defer` 相当の解放を保証する。

## 8. CI スモークテスト要件

1. **ffi-smoke**: C 側の `strlen` と同等の関数を呼び出し、`Ptr<u8>` の NULL 非許容／NULL 許容双方を検証する。`audit.log` に `ffi.call` が残ることをアサート。
2. **buffer-span**: `Span<u8>` から `Ptr<u8>` を降格後、`read`/`write`/`copy_nonoverlapping` を試し、境界外アクセス時に安全 API がエラーを返すことをテスト。
3. **gc-root-guard**: `RootGuard` 相当のユーティリティで `register_root`/`unregister_root` がペアになることを確認し、`defer`/`Drop` 経由でも確実に解放されることを追跡。
4. **alignment-check**: `read_unaligned`/`write_unaligned` の挙動をアライメント違反ケースで検証し、`read`/`write` が UB になる条件を文書化した診断メッセージと整合することを確認。
5. **thread-send-audit**: `@requires(effect={runtime, unsafe})` を付与したポインタ共有ユースケースで `audit` ログが作成されるか検証し、`Send`/`Sync` マーカー無しで共有した場合にビルドエラーとなることを確認。

各スモークテストは CI で `core-unsafe-ptr` ジョブとして実行し、失敗時は `Diagnostic` に `effect_flags` と `ptr_label` が含まれることを必須要件とする。


---

> TODO: 上記 API を仕様書の該当章へ取り込み、同時に `Core.Unsafe` 内の命名規則 (`snake_case`) とドキュメント整備ポリシーを定義する。
