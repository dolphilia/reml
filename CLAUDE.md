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

### 実装 (`compiler/`, `tooling/`)
- **`compiler/rust/`**: 現在のメイン開発対象。Rust 製コンパイラ。
- `compiler/ocaml/`: 参照用実装。
- `tooling/`: エコシステムツール（LSP 等）。
- `runtime/`: ランタイムライブラリ。

### その他
- `examples/`: Reml コードのサンプル。
- `tests/`: 統合テスト（存在する場合）。

## 開発ワークフロー

### Rust コンパイラ開発 (`compiler/rust`)
アクティブな開発はここで行われます。
- ルートには衝突回避のため `Cargo.toml.ws` のみがあり、作業対象の `Cargo.toml` を直接指定して実行します（例: `compiler/rust/frontend/`、`compiler/rust/backend/`、`compiler/rust/runtime/`）。
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
1. **調査**: 質問に答える前に、必ず関連する `docs/` やコードを確認してください。
2. **整合性**: 仕様と実装の不一致を見つけた場合は、それを指摘し、どちらを正とするかユーザーに確認してください。
3. **実行**: コードの変更を行う際は、必ずテスト（`cargo test`）を実行し、動作を確認してください。
