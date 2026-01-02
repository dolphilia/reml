# CLAUDE.md

このファイルは、このリポジトリで作業する Claude Code (claude.ai/code) 向けの指針です。

**重要**: ユーザーとの対話はすべて **日本語** で行ってください。

## プロジェクト概要

**Reml (Readable & Expressive Meta Language)** の言語仕様および公式実装（コンパイラ・ツール）を管理するモノレポです。
以前は仕様書専用リポジトリでしたが、現在は Rust によるコンパイラ実装を含む総合開発リポジトリとなっています。

## リポジトリ構造

### ドキュメント (`docs/`)

- `docs/spec/`: 言語仕様書 (Core, Parser, Stdlib, Plugins, Ecosystem)。
- `docs/guides/`: 開発者向けガイド。
- `docs/notes/`: 設計ノート、リサーチ、RFC。
- `docs/plans/`: 実装計画、ロードマップ。
- `docs/schemas/`: JSON Schema (Diagnostics, Plugins)。

### 実装 (`compiler/`, `tooling/`)

- **`compiler/rust/`**: 現在のメイン開発対象。Rust 製コンパイラ。
- `compiler/ocaml/`: 参照用実装。
- `tooling/`: エコシステムツール（LSP 等）。
- `runtime/`: ランタイムライブラリ。
- `reports/`: 監査ログ、メトリクスレポート。

### その他

- `examples/`: Reml コードのサンプル。
- `tests/`: 統合テスト。

## 開発ワークフロー

### Rust コンパイラ開発 (`compiler/rust`)

アクティブな開発はここで行われます。

- ルートには衝突回避のため `Cargo.toml.ws` のみがあり、原則 `--manifest-path` で作業対象の `Cargo.toml` を直接指定して実行します（例: `compiler/rust/frontend/`、`compiler/rust/backend/`、`compiler/rust/runtime/`）。
- ルートビルドは必要な時のみ実行し、作業後に `Cargo.toml.ws` を必ず元に戻します。
- **ビルド**: `cargo build --manifest-path compiler/rust/frontend/Cargo.toml`
- **テスト**: `cargo test --manifest-path compiler/rust/frontend/Cargo.toml`
- **リント**: `cargo clippy --manifest-path compiler/rust/frontend/Cargo.toml`
- **フォーマット**: `cargo fmt --manifest-path compiler/rust/frontend/Cargo.toml`
- **実行例（確認済み）**:
  - frontend: `cargo build --manifest-path compiler/rust/frontend/Cargo.toml`
  - backend: `cargo build --manifest-path compiler/rust/backend/llvm/Cargo.toml`
  - runtime: `cargo build --manifest-path compiler/rust/runtime/Cargo.toml`
  - ルート: `mv Cargo.toml.ws Cargo.toml` → `cargo build` → `mv Cargo.toml Cargo.toml.ws`

### ドキュメント編集

- 仕様書は `docs/spec/` にあります。
- 変更時は、関連する実装（`compiler/`）との整合性を常に意識してください。
- リンク切れを防ぐため、相対パスの整合性を確認してください。

### examples の `.reml` 実行

- スイート実行: `tooling/examples/run_examples.sh --suite spec_core` / `--suite practical`
- 単体実行（ビルド済み想定）: `compiler/rust/frontend/target/debug/reml_frontend --output json examples/.../*.reml`
- 単体実行（cargo 経由）: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/.../*.reml`

### よく使う `cargo test` / `cargo fmt` / `cargo clippy`

- フロントエンド全体テスト: `cargo test --manifest-path compiler/rust/frontend/Cargo.toml`
- フロントエンドの範囲指定: `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::module -- --nocapture`
- フロントエンド fmt: `cargo fmt --manifest-path compiler/rust/frontend/Cargo.toml`
- フロントエンド clippy: `cargo clippy --manifest-path compiler/rust/frontend/Cargo.toml`
- adapter テスト: `cargo test --manifest-path compiler/rust/adapter/Cargo.toml`
- backend テスト: `cargo test --manifest-path compiler/rust/backend/llvm/Cargo.toml`
- backend fmt: `cargo fmt --manifest-path compiler/rust/backend/llvm/Cargo.toml`
- backend clippy: `cargo clippy --manifest-path compiler/rust/backend/llvm/Cargo.toml`
- runtime テスト: `cargo test --manifest-path compiler/rust/runtime/Cargo.toml`
- runtime fmt: `cargo fmt --manifest-path compiler/rust/runtime/Cargo.toml`
- runtime clippy: `cargo clippy --manifest-path compiler/rust/runtime/Cargo.toml`

## コーディング規約

### 一般

- **言語**: コメント、ドキュメント、コミットメッセージ等は日本語。
- **用語**: `docs/spec/0-2-glossary.md` に準拠。

### Rust

- 標準的な Rust イディオムに従ってください。
- エラーハンドリングは `Result` を適切に使用し、`unwrap()` は避けてください（テストコードを除く）。

### Reml (サンプルコード等)

- `docs/spec/0-3-code-style-guide.md` に従ってください。

## アーキテクチャと設計

- **パーサーコンビネーター**: Reml の核となる概念です。`docs/spec/2-x` 系列を参照してください。
- **Unicode**: Byte / Char / Grapheme の 3 層モデルを採用しています。
- **DSL 指向**: 言語拡張や埋め込み DSL を強力にサポートする設計です。

## エージェントへの指示

1. **日時の取得**: タスク開始時に必ずコマンド（例: `date`）を実行し、現在の正確な日付・時刻を取得してください。
2. **調査**: 質問に答える前に、必ず関連する `docs/` やコードを確認してください。
3. **整合性**: 仕様と実装の不一致を見つけた場合は、それを指摘し、どちらを正とするかユーザーに確認してください。
4. **実行**: コードの変更を行う際は、必ずテスト（`cargo test`）を実行し、動作を確認してください。
