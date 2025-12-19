# FFI DSL ガイド（ドラフト）

## 目的
`Core.Ffi.Dsl` を使って低レベル FFI を安全な API として利用する。

## 想定読者
- `reml-bindgen` の生成物を安全に扱いたい開発者
- FFI の `unsafe` を最小化したい実装者

## コア API
- `bind_library`: ライブラリの解決。`effect {ffi}` を要求。
- `bind_fn`: シンボルと署名の紐付け。`effect {ffi, unsafe}` を要求。
- `wrap`: `unsafe` な呼び出しを安全化し、`effect {ffi}` の API を返す。
- `FfiType` DSL: `ffi.double` / `ffi.ptr(ffi.I8)` / `ffi.Struct(...)` などの型表現。

## 安全境界の考え方
- `bind_fn` は低レベル API のため **必ず `unsafe` 境界**で扱う。
- `wrap` は引数数・戻り値の NULL・所有権前提を検証し、`Result` で失敗を明示する。
- `wrap` 経由の呼び出しは監査ログへ `ffi.wrapper.*` を記録する。

## 最小例
```reml
use Core.Ffi.Dsl as ffi

let cos = effect {ffi, unsafe} {
  let lib = ffi.bind_library("m")?
  let raw = lib.bind_fn("cos", ffi.fn_sig([ffi.double], ffi.double, false))?
  ffi.wrap(raw, { name: "libm.cos", null_check: false, ownership: None, error_map: None })?
}
let value = cos(0.5)?
```

## `unsafe` 直呼びと `wrap` の対比
```reml
use Core.Ffi.Dsl as ffi

// 低レベル呼び出し（unsafe）
let value = effect {ffi, unsafe} {
  let lib = ffi.bind_library("m")?
  let raw = lib.bind_fn("cos", ffi.fn_sig([ffi.double], ffi.double, false))?
  raw(0.5)?
}

// 安全化ラッパー
let cos = effect {ffi, unsafe} {
  let lib = ffi.bind_library("m")?
  let raw = lib.bind_fn("cos", ffi.fn_sig([ffi.double], ffi.double, false))?
  ffi.wrap(raw, { name: "libm.cos", null_check: false, ownership: None, error_map: None })?
}
let safe_value = cos(0.5)?
```

## `reml-bindgen` 併用フロー
1. `reml-bindgen` で `extern` 生成（`generated/`）と `bindings.manifest.json` を作成する。
2. `Core.Ffi.Dsl` のラッパーを `wrapper/` 等に実装し、`unsafe` 境界を局所化する。
3. アプリケーション側は **ラッパーのみ**を利用し、監査ログを確認する。

## 監査ログのポイント
- `AuditEnvelope.metadata["ffi"].wrapper = "ffi.wrap"` を付与し、`ffi.wrapper.*` を必須記録する。
- 失敗時は `ffi.wrap.invalid_argument` / `ffi.wrap.null_return` / `ffi.wrap.ownership_violation` を診断キーに使う。

## サンプル配置
- `examples/ffi/bindgen/minimal`: 生成物と手書きラッパーの分離例。
- `examples/ffi/dsl`: `unsafe` 直呼びと `ffi.wrap` の比較例。
