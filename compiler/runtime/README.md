# runtime

Reml の Rust ランタイム実装です。Capability/IO/Text/Parse/Prelude/Plugin などの中核モジュールと、C ランタイム/FFI ラッパを統合的に扱います。

## 構成
- `src/`: Rust ランタイム本体（Capability、IO、Text、Parse、Runtime など）
- `native/`: C ランタイム実装（ビルド/テストは `native/README.md` を参照）
- `ffi/`: Rust 側 FFI ラッパ（`ffi/README.md` を参照）
- `examples/` / `benches/` / `tests/`: 検証用コード

## 機能フラグ（抜粋）
- `core_numeric` / `core_time` / `core_io` / `core_path` / `core_async`
- `metrics` / `unicode_full` / `experimental_migration`

## ビルド/テスト
```
cargo build --manifest-path compiler/runtime/Cargo.toml
cargo test --manifest-path compiler/runtime/Cargo.toml
```

## 参照先
- `compiler/runtime/native/README.md`
- `compiler/runtime/ffi/README.md`
