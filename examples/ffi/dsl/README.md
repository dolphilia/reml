# FFI DSL サンプル

`Core.Ffi.Dsl` を使って `unsafe` 直呼びと `ffi.wrap` の差分を比較する。

## 構成
- `unsafe_direct.reml`: 低レベル呼び出しを直接使う例
- `wrapped_safe.reml`: `ffi.wrap` で安全化した例

## ねらい
- `unsafe` 境界をラッパー側へ集中させる
- `ffi.wrap` の監査メタデータと診断キーを明確にする
