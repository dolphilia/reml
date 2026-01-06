# C-1 サードパーティライブラリ調査メモ

## 調査日
- 2026-01-06

## 参照ファイル
- `compiler/frontend/Cargo.toml:26-48`
- `compiler/runtime/Cargo.toml:39-70`
- `compiler/runtime/ffi/Cargo.toml:17-39`
- `compiler/backend/llvm/Cargo.toml:8-11`
- `compiler/adapter/Cargo.toml:11-14`
- `compiler/ffi_bindgen/Cargo.toml:15-21`
- `compiler/xtask/Cargo.toml:7-9`

## 直接依存クレートの抽出結果

以下は Cargo.toml に記載された直接依存（dev-dependencies を含む）の一覧。
用途は一般的な役割に基づくメモであり、個別の実装の詳細な使用箇所は別途確認が必要。

- `chumsky`（frontend）: パーサコンビネータ。
- `logos`（frontend）: レキサ生成。
- `serde` / `serde_json`（frontend/runtime/runtime_ffi/backend/adapter/ffi_bindgen/xtask）: シリアライズ/デシリアライズ。
- `indexmap`（frontend/runtime/runtime_ffi）: 挿入順を保つ連想配列。
- `smallvec`（frontend）: 小サイズ最適化の可変長配列。
- `smol_str`（frontend）: 小さな文字列最適化。
- `once_cell`（frontend/runtime/runtime_ffi）: 遅延初期化。
- `thiserror`（frontend/runtime/runtime_ffi/adapter/ffi_bindgen）: エラー型の derive。
- `unicode-ident`（frontend/runtime/runtime_ffi optional）: Unicode 識別子判定。
- `unicode-normalization`（frontend/runtime/runtime_ffi）: Unicode 正規化。
- `unicode-width`（frontend/runtime/runtime_ffi）: 表示幅計算。
- `unicode-segmentation`（runtime/runtime_ffi）: Unicode の分割（grapheme 等）。
- `uuid`（frontend/runtime/runtime_ffi optional）: UUID 生成/操作。
- `schemars`（frontend optional/runtime/runtime_ffi optional）: JSON Schema 生成。
- `regex`（runtime/ffi_bindgen）: 正規表現。
- `ordered-float`（runtime/runtime_ffi）: 浮動小数の順序付け。
- `toml`（runtime/runtime_ffi/ffi_bindgen/xtask）: TOML 解析。
- `time`（runtime/runtime_ffi）: 時刻/日付処理。
- `notify`（runtime/runtime_ffi）: ファイル監視。
- `glob`（runtime/runtime_ffi）: glob パターン。
- `rust_decimal`（runtime optional）: 10進数演算。
- `num-bigint`（runtime optional）: 任意精度整数。
- `num-rational`（runtime optional）: 有理数。
- `sha2`（runtime/runtime_ffi optional/ffi_bindgen）: SHA-2 ハッシュ。
- `wasmtime`（runtime/runtime_ffi optional）: WebAssembly 実行基盤。
- `getrandom`（adapter）: OS 乱数取得。

## 主要クレートの具体的な使用箇所（実装側の参照）

- `chumsky`:
  - `compiler/frontend/src/parser/mod.rs:3-7`（パーサ基盤の import）
  - `compiler/frontend/src/parser/mod.rs:116-130`（`chumsky::Parser` を拡張したトレイト定義）
  - `compiler/frontend/src/parser/mod.rs:2094-2140`（`choice` / `just` などの組み合わせ）
- `logos`:
  - `compiler/frontend/src/lexer/mod.rs:7`（`Logos` derive の利用）
  - `compiler/frontend/src/lexer/mod.rs:83-90`（`RawToken` の `#[derive(Logos)]` と `logos::skip`）
  - `compiler/frontend/src/lexer/mod.rs:471-490`（`RawToken::lexer` を用いたトークン化）
- `wasmtime`:
  - `compiler/runtime/src/runtime/plugin_bridge.rs:10-10`（`Engine`, `Module`, `Instance`, `Store` の import）
  - `compiler/runtime/src/runtime/plugin_bridge.rs:111-246`（Wasm ブリッジのロード/実行）
  - `compiler/runtime/src/runtime/bridge.rs:184-191`（メタデータに `engine=wasmtime` を付与）
  - `compiler/runtime/tests/plugin_wasm_bridge.rs:137-142`（テストで `wasmtime` を検証）
- `serde` / `serde_json`:
  - `compiler/frontend/src/parser/ast.rs:1-6`（AST の `Serialize` derive）
  - `compiler/runtime/src/data/schema.rs:1-16`（`Serialize` / `Deserialize` の付与）
  - `compiler/backend/llvm/src/integration.rs:15-16`（統合レイヤの `Deserialize`）
- `thiserror`:
  - `compiler/frontend/src/diagnostic/model.rs:10`（診断モデルのエラー型）
  - `compiler/runtime/src/runtime/plugin.rs:11`（プラグイン実行のエラー型）
  - `compiler/adapter/src/env.rs:3`（環境 API のエラー型）
- `regex`:
  - `compiler/runtime/src/parse/combinator.rs:9`（パーサ補助の正規表現）
  - `compiler/runtime/src/parse/combinator.rs:291-293`（絵文字判定の Regex）
  - `compiler/ffi_bindgen/src/lib.rs:1`（bindgen 設定の正規表現）

## dev-dependencies（テスト/ベンチ用途）

- `insta`（frontend/runtime）: スナップショットテスト。
- `tempfile`（frontend/runtime/runtime_ffi）: 一時ファイル。
- `test-case`（frontend）: パラメタライズドテスト。
- `humantime`（runtime）: 人間可読な時間表現。
- `criterion`（runtime）: ベンチマーク。
- `proptest`（runtime）: プロパティベーステスト。
- `static_assertions`（runtime）: コンパイル時アサート。
- `wat`（runtime）: WebAssembly Text 解析。
