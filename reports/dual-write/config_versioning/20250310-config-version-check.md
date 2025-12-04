# 2025-03-10 Manifest/Schema バージョン互換チェック（Run ID: 20250310-config-version）

- ブランチ: `git rev-parse --short HEAD`（作業時点） → 実行環境側ログ参照
- 実行環境: macOS（Codex CLI, sandbox workspace-write）

## 1. `cargo test schema_version_check`
- 作業ディレクトリ: `compiler/rust/runtime`
- コマンド: `cargo test schema_version_check`
- 結果: 正常終了（終了コード 0）。`tests/manifest.rs` に追加した 5 ケース（互換成功、major 不一致、minor 超過、schema バージョン未設定、SemVer 解析失敗）をすべて通過。
- 代表的な警告: 既存の `unused_imports` 等（`collection_diff.rs`, `config/mod.rs` など）で従来から出ているワーニングのみ。今回の変更では新規ワーニングなし。

## 2. 観測事項
- `config.schema.version_incompatible` / `config.project.version_invalid` 診断コードが JSON へ `config.version_reason`（`major`/`schema_ahead`/`parse_error`）を残すことをテストで確認。
- 互換判定は `Schema.version` が `None` の場合スキップされ、Manifest だけでは覆せないため CLI 側でも同じ API を呼び出せばよい。

## フォローアップ
1. CLI/CI へ `ensure_schema_version_compatibility` を組み込む時点で、本 Run ID をベースラインとして差分を比較する。
2. スキーマ実例（`examples/core_config/*`）の整備後、`schema_version_check_passes_when_manifest_is_newer` を実データで再検証し、ログを追加予定。
