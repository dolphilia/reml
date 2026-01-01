# 0.1 ワークストリーム追跡

## 追跡ルール
- 各ストリームは 1 つの仕様章（3-x）と 1 つのガイド更新に紐づける。
- DSL 開発者が触れる API には必ず **サンプル + 期待出力** を用意し、回帰計画の監視対象へ登録する。
- 仕様変更は `docs/spec/0-1-project-purpose.md` の優先度に沿って評価し、安全性・診断・性能を優先する。

## ストリーム一覧
| ストリーム | 優先度 | 主目的 | 主要参照 | 成果物（ドラフト） |
| --- | --- | --- | --- | --- |
| Core.Test | 高 | DSL の統合/ゴールデン/ファジング基盤 | `docs/spec/3-6-core-diagnostics-audit.md` | `docs/spec/3-11-core-test.md` と `docs/guides/tooling/testing.md` |
| Core.Cli | 高 | DSL 用 CLI の宣言的構築 | `docs/spec/3-10-core-env.md` | `docs/spec/3-12-core-cli.md` と `docs/guides/tooling/cli-authoring.md` |
| Core.Text.Pretty | 中 | フォーマッタ/コード生成の整形基盤 | `docs/spec/3-3-core-text-unicode.md` | `docs/spec/3-13-core-text-pretty.md` と `docs/guides/dsl/formatter-authoring.md` |
| Core.Lsp | 中 | IDE 連携の標準ツールキット | `docs/spec/3-5-core-io-path.md` | `docs/spec/3-14-core-lsp.md` と `docs/guides/lsp/lsp-authoring.md` |
| Core.Doc | 低 | ドキュメント/Doctest 基盤 | `docs/spec/3-6-core-diagnostics-audit.md` | `docs/spec/3-15-core-doc.md` と `docs/guides/dsl/doc-authoring.md` |

## Phase 4 との接続
- 各ストリームの成果物は `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に登録し、回帰シナリオの対象へ追加する。
- 追加シナリオは `chapter3` 配下の標準ライブラリサンプルとして管理し、診断キー・期待出力を統一する。

## 追記ルール
- 新規 API 名は `docs/spec/0-2-glossary.md` に登録する。
- 仕様変更の理由は脚注または TODO として `docs/notes/` に根拠リンクを残す。
