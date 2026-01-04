# docs/ 目次

`docs/` 配下には Reml プロジェクトの公式仕様・実装ガイド・調査ノート・計画書が整理されています。本ファイルは各カテゴリへの入口として機能します。

## 1. 公式仕様 (`docs/spec/`)

| 節 | 内容 | ファイル |
| --- | --- | --- |
| 0.x | 導入資料・プロジェクト方針 | [0-0-overview.md](spec/0-0-overview.md), [0-1-project-purpose.md](spec/0-1-project-purpose.md), [0-2-glossary.md](spec/0-2-glossary.md), [0-3-code-style-guide.md](spec/0-3-code-style-guide.md) |
| 1.x | 言語コア仕様 | [1-0-language-core-overview.md](spec/1-0-language-core-overview.md) 〜 [1-5-formal-grammar-bnf.md](spec/1-5-formal-grammar-bnf.md) |
| 2.x | 標準パーサー API | [2-0-parser-api-overview.md](spec/2-0-parser-api-overview.md) 〜 [2-7-core-parse-streaming.md](spec/2-7-core-parse-streaming.md) |
| 3.x | 標準ライブラリ | [3-0-core-library-overview.md](spec/3-0-core-library-overview.md) 〜 [3-18-core-system.md](spec/3-18-core-system.md) |
| 4.x | エコシステム仕様（Draft） | [4-0-ecosystem-overview.md](spec/4-0-ecosystem-overview.md) 〜 [4-6-risk-governance.md](spec/4-6-risk-governance.md) |
| 5.x | 公式プラグイン仕様（Draft） | [5-0-official-plugins-overview.md](spec/5-0-official-plugins-overview.md) 〜 [5-7-core-parse-plugin.md](spec/5-7-core-parse-plugin.md) |

> 補足: 章ごとの詳細構成は [`docs/spec/README.md`](spec/README.md) に記載しています。

## 2. 実務ガイド (`docs/guides/`)

ガイドは用途別に以下のカテゴリへ分類しています。詳細は [`docs/guides/README.md`](guides/README.md) を参照してください。

- **開発ワークフロー & ツールチェーン**: [tooling/cli-workflow.md](guides/tooling/cli-workflow.md), [tooling/ci-strategy.md](guides/tooling/ci-strategy.md), [tooling/diagnostic-format.md](guides/tooling/diagnostic-format.md), ほか
- **LSP 連携**: [lsp/lsp-integration.md](guides/lsp/lsp-integration.md), [lsp/lsp-authoring.md](guides/lsp/lsp-authoring.md), ほか
- **コンパイラ / 解析**: [compiler/core-parse-streaming.md](guides/compiler/core-parse-streaming.md), [compiler/llvm-integration-notes.md](guides/compiler/llvm-integration-notes.md), ほか
- **DSL / プラグイン運用**: [dsl/DSL-plugin.md](guides/dsl/DSL-plugin.md), [dsl/plugin-authoring.md](guides/dsl/plugin-authoring.md), [dsl/dsl-first-guide.md](guides/dsl/dsl-first-guide.md), ほか
- **エコシステム & コミュニティ**: [ecosystem/ai-integration.md](guides/ecosystem/ai-integration.md), [ecosystem/manifest-authoring.md](guides/ecosystem/manifest-authoring.md), [ecosystem/community-handbook.md](guides/ecosystem/community-handbook.md), ほか
- **ランタイム / システム連携**: [runtime/runtime-bridges.md](guides/runtime/runtime-bridges.md), [runtime/system-programming-primer.md](guides/runtime/system-programming-primer.md), [runtime/portability.md](guides/runtime/portability.md), ほか
- **FFI / 低レベル**: [ffi/reml-ffi-handbook.md](guides/ffi/reml-ffi-handbook.md), [ffi/reml-bindgen-guide.md](guides/ffi/reml-bindgen-guide.md), [ffi/ffi-build-integration-guide.md](guides/ffi/ffi-build-integration-guide.md), ほか

## 3. 調査ノート (`docs/notes/`)

調査メモや将来計画は [`docs/notes/README.md`](notes/README.md) でカテゴリごとに整理しています。主なドキュメント:

- 言語設計: [reml-design-goals-and-appendix.md](notes/language/reml-design-goals-and-appendix.md), [reml-influence-study.md](notes/language/reml-influence-study.md)
- パーサー: [core-parse-api-evolution.md](notes/parser/core-parse-api-evolution.md), [core-parse-cst-design.md](notes/parser/core-parse-cst-design.md)
- 標準ライブラリ: [core-library-outline.md](notes/stdlib/core-library-outline.md), [core-io-path-gap-log.md](notes/stdlib/core-io-path-gap-log.md)
- バックエンド・クロスコンパイル: [cross-compilation-spec-intro.md](notes/backend/cross-compilation-spec-intro.md), [llvm-spec-status-survey.md](notes/backend/llvm-spec-status-survey.md)
- プロセス・運用: [guides-to-spec-integration-plan.md](notes/process/guides-to-spec-integration-plan.md), [spec-integrity-audit-checklist.md](notes/process/spec-integrity-audit-checklist.md)

## 4. 計画書 (`docs/plans/`)

ブートストラップ実装計画やリポジトリ再編計画を集約しています。

- ブートストラップ計画の総覧: [`docs/plans/bootstrap-roadmap/README.md`](plans/bootstrap-roadmap/README.md)
- フェーズ別詳細: `docs/plans/bootstrap-roadmap/0-x` 〜 `4-x`
- リポジトリ再編計画: [repository-restructure-plan.md](plans/repository-restructure-plan.md)

> 計画書の索引と更新手順は [`docs/plans/README.md`](plans/README.md) にまとめています。

## 5. 参考情報

- サンプル実装は `examples/` に集約し、概要は [examples/README.md](../examples/README.md) を参照してください。
- 実装用ディレクトリ（`compiler/`、`tooling/`）には各フェーズで作業を始める際の README と TODO を配置しています。
- 大規模ドキュメント移行の履歴は [docs-migrations.log](../docs-migrations.log) に記録しています。
