# Reml CLI ワークフローガイド（Draft）

> `reml` コマンドを用いた日常開発〜CI/CD 運用までの手順を整理する。

## 1. 基本コマンド
- `reml new`, `reml add`, `reml build`, `reml test`, `reml fmt`, `reml check` の概要。
- `CliDiagnosticEnvelope` 出力の読み方。

## 2. 開発ワークフロー
- ローカル環境での推奨フロー（Git hooks, fmt/check, test）。
- DSL プロファイル更新と互換性診断 (`reml dsl info`).

## 3. CI/CD 連携
- サンプルパイプライン（GitHub Actions 等）。
- `--output json` / `--summary` / `--fail-on-*` オプション活用。

## 4. 監査ログ運用
- `AuditEnvelope` の格納場所とローテーション。
- セキュリティ監査との連携。

## 5. トラブルシューティング
- 代表的な CLI エラーコード一覧予定。
- サポートリソースへのリンク（FAQ, コミュニティ）。

> Draft 版。Chapter 4.1 完成時に更新予定。
