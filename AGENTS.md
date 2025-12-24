# AGENTS.md

## 目的と前提
- このリポジトリは **Reml 言語の総合開発リポジトリ** です。言語仕様の策定だけでなく、コンパイラの実装（Rust/OCaml）、開発ツール、サンプルコードの管理を行います。
- 作業する AI エージェントはこの文書と `CLAUDE.md` に従ってください。
- すべての対話とテキストは **日本語** で行います。
- 実装言語は主に **Rust** です。参考実装として OCaml コードも存在しますが、新規開発の主力は Rust 版です。

## リポジトリ概観
### ドキュメント (`docs/`)
- `0-x` 系列: 導入資料・プロジェクト指針。
- `1-x` 系列: 言語コア仕様（構文、型、意味論）。
- `2-x` 系列: 標準パーサー API 仕様。
- `3-x` 系列: 標準ライブラリ仕様。
- `4-x` 系列: 公式プラグイン仕様ドラフト。
- `5-x` 系列: エコシステム仕様ドラフト。
- `guides/`: 実務ガイド（AI 連携、プラグイン開発など）。
- `notes/`: 調査ノート、将来計画、RFC 相当の文書。

### 実装 (`compiler/`, `runtime/`, etc.)
- `compiler/rust`: **[Active]** Rust によるメインコンパイラ実装。
- `compiler/ocaml`: **[Reference]** OCaml によるプロトタイプ/参照実装。
- `runtime/`: ランタイムライブラリの実装。
- `tooling/`: LSP サーバー、フォーマッタ、CLI ツールなど。
- `examples/`: Reml 言語のサンプルコード、テストケース。

## 作業の基本原則
1. **仕様と実装の同期**: 仕様（`docs/spec`）と実装（`compiler/`）に乖離が生じないように注意します。実装を変更した場合は仕様書を、仕様を変更した場合は実装を更新するチケットやタスクを確認・提案します。
2. **言語使用**: 
    - コミュニケーション: 日本語。
    - Reml コード: 仕様に準拠した最新の構文。
    - 実装コード: Rust (2021 edition以上), OCaml (参照用)。
3. **Rust 開発**: 
    - `cargo test`, `cargo fmt`, `cargo clippy` を活用し、品質を保ちます。
    - ルートには衝突回避のため `Cargo.toml.ws` のみがあり、各作業単位は `compiler/rust/frontend/` や `compiler/rust/backend/`、`compiler/rust/runtime/` などの個別 `Cargo.toml` を直接指定して実行します（例: `cargo test --manifest-path compiler/rust/frontend/Cargo.toml`）。
    - 実行手順:
        - frontend: `cargo build --manifest-path compiler/rust/frontend/Cargo.toml`
        - backend: `cargo build --manifest-path compiler/rust/backend/llvm/Cargo.toml`
        - runtime: `cargo build --manifest-path compiler/rust/runtime/Cargo.toml`
        - ルート: `mv Cargo.toml.ws Cargo.toml` → `cargo build` → `mv Cargo.toml Cargo.toml.ws`
    - エラーメッセージや識別子にはプロジェクトの命名規則に従います。
4. **非破壊的編集**: 既存の資産（特に OCaml の参照実装や古いノート）を無断で削除せず、必要なら `deprecated` 扱いにして残します。

## 推奨ワークフロー
1. **コンテキスト把握**: 
    - ドキュメント修正の場合: 関連する `docs/spec` を確認。
    - 実装修正の場合: `compiler/rust` 内の該当コードと、対応する仕様書を確認。
2. **計画**: 変更の影響範囲（仕様 vs 実装）を特定します。
3. **実行**:
    - ドキュメント: Markdown の修正。リンク整合性の確認。
    - コード: 実装、テスト追加、ローカルでのビルド検証。
4. **検証**: 
    - `cargo test` 等のパスを確認。
    - ドキュメントのプレビュー（必要であれば）。

## ディレクトリ別ガイド
- **`docs/`**: `README.md` が各種仕様へのインデックスになっています。
- **`compiler/rust`**: Rust 版コンパイラのルート。`cargo build` でビルド可能。
- **`examples`**: ユーザー向けの例。実装の動作検証にも使われます。
