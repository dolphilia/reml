# ffi_bindgen

C ヘッダから Reml の FFI シグネチャを生成するツール群です。`reml-bindgen` が `.reml` と `bindings.manifest.json` を出力し、診断ログを JSON で出力します。

## 構成
- `src/lib.rs`: 設定読み込み、解析、生成、診断の本体
- `src/main.rs`: `reml-bindgen` CLI エントリ

## 使い方
```
cargo run --manifest-path compiler/ffi_bindgen/Cargo.toml --bin reml-bindgen -- --config reml-bindgen.toml
```

CLI オプション（抜粋）:
- `--config <path>`: 設定ファイル（既定: `reml-bindgen.toml`）
- `--header <path>` / `--include-path <path>` / `--define <name[=value]>`
- `--output <path>` / `--manifest <path>` / `--exclude <pattern>`

## 設定ファイルの必須項目
`reml-bindgen.toml` では次の項目が必須です。
- `headers`
- `include_paths`
- `output`
- `manifest`

## 関連
- `remlc build` から `ffi.bindgen` セクション経由で呼び出されます。
