# FFI DSL ガイド（ドラフト）

## 目的
`Core.Ffi.Dsl` を使って低レベル FFI を安全な API として利用する。

## 想定読者
- `reml-bindgen` の生成物を安全に扱いたい開発者
- FFI の `unsafe` を最小化したい実装者

## 基本方針
- `bind_library` / `bind_fn` は低レベル API とし、`ffi.wrap` で安全化する。
- 監査ログには `wrapper = "ffi.wrap"` を付与する。

## 最小例
```reml
let lib = ffi.bind_library("m")?
let raw = lib.bind_fn("cos", ffi.double -> ffi.double)?
let cos = ffi.wrap(raw, effect {ffi})
```

## エラーハンドリング
- `ffi.wrap` は `Result` を返し、`null` や不正引数は明示的に失敗させる。
- 失敗時の診断キーは `ffi.wrap.invalid_argument` / `ffi.wrap.null_return` を用いる。

## 併用フロー
- `reml-bindgen` で生成した `extern "C"` を DSL で包み、
  `examples/ffi` では「生成物」「ラッパー」「利用コード」を分離する。
