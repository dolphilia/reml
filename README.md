# Reml プロジェクト概要

[![Bootstrap Linux CI](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-linux.yml/badge.svg)](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-linux.yml)
[![Bootstrap macOS CI](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-macos.yml/badge.svg)](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-macos.yml)

Reml (Readable & Expressive Meta Language) はパーサーコンビネーターと静的保証に重点を置いたプログラミング言語です。本リポジトリは仕様、設計ガイド、ブートストラップ実装計画、サンプル実装を集約し、言語実装とエコシステム整備を進めるための中枢として機能します。

## ディレクトリ構成（再編後）

- `docs/`: 仕様書・ガイド・調査ノート・計画書を集約したアーカイブ
  - `docs/spec/`: 章番号付き Reml 公式仕様
  - `docs/guides/`: ツールチェーンや DSL 運用ガイド
  - `docs/notes/`: 調査メモと将来計画
  - `docs/plans/`: ブートストラップ実装計画・ロードマップ
- `compiler/`: Phase 1 (OCaml ブートストラップ) 〜 Phase 3 (セルフホスト) を受け止める実装領域
- `runtime/`: 最小ランタイムと Capability 拡張の実装領域
- `tooling/`: CLI・CI・リリース・LSP など開発ツール資産
- `examples/`: 仕様や計画書と連動したサンプル実装・比較資料
- `reports/`: CI/ローカルの監査ログと計測レポート。`reports/audit/index.json`・`summary.md`・`history/*.jsonl.gz`・`failed/<build-id>/` などの永続成果物を格納する。
- `docs-migrations.log`: 大規模ドキュメント移行の履歴
- `AGENTS.md` / `CLAUDE.md`: AI エージェント向け作業ガイド
- GitHub Actions `Rust Frontend CI (config-lint)` が `remlc config lint --manifest examples/core_config/reml.toml --schema examples/core_config/cli/schema.json --format json` を実行し、`lint-report.json` と `examples/core_config/cli/diff.expected.json` の整合性を毎回検証する。

## ドキュメントへの導線

- 仕様書・ガイド・調査ノートの全体索引: [`docs/README.md`](docs/README.md)
- ブートストラップ計画の統合マップ: [`docs/plans/bootstrap-roadmap/README.md`](docs/plans/bootstrap-roadmap/README.md)
- リポジトリ再編計画書: [`docs/plans/repository-restructure-plan.md`](docs/plans/repository-restructure-plan.md)
- 仕様書の差分履歴や横断的メモ: `docs/notes/` 配下の各ノートを参照
- AI/LSP 向け診断ローカライズキー一覧: [`docs/guides/ecosystem/ai-integration.md#51-lsp-診断ローカライズキー対応表`](docs/guides/ecosystem/ai-integration.md#51-lsp-%E8%A8%BA%E6%96%AD%E3%83%AD%E3%83%BC%E3%82%AB%E3%83%A9%E3%82%A4%E3%82%BA%E3%82%AD%E3%83%BC%E5%AF%BE%E5%BF%9C%E8%A1%A8)
- 3.6 Core Diagnostics & Audit の最新仕様とゴールデン: [`docs/spec/3-6-core-diagnostics-audit.md`](docs/spec/3-6-core-diagnostics-audit.md)、`examples/core_diagnostics/*.expected.*`（`tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` で再生成）
- 3.7 Core Config & Data の API/マニフェスト例: [`docs/spec/3-7-core-config-data.md`](docs/spec/3-7-core-config-data.md)、`examples/core_config/README.md`（`cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin remlc -- manifest dump --manifest examples/core_config/reml.toml` で JSON を確認）
- Core Parse コンビネーター抽出の進捗: [`docs/spec/2-2-core-combinator.md`](docs/spec/2-2-core-combinator.md) 脚注および [`docs/notes/parser/core-parse-api-evolution.md`](docs/notes/parser/core-parse-api-evolution.md) Phase 2-5 Step6 を参照
- Unicode 識別子の暫定対応状況: [`docs/spec/1-1-syntax.md`](docs/spec/1-1-syntax.md)・[`docs/spec/1-5-formal-grammar-bnf.md`](docs/spec/1-5-formal-grammar-bnf.md) の脚注と [`docs/spec/0-2-glossary.md`](docs/spec/0-2-glossary.md) の「Unicode 識別子プロファイル（暫定）」を参照（Phase 2-7 `lexer-unicode` タスクで本実装予定）
- Core.Dsl パラダイムキットの仕様: [`docs/spec/3-16-core-dsl-paradigm-kits.md`](docs/spec/3-16-core-dsl-paradigm-kits.md) に Object/Gc/Actor/Vm の最小 API を整理
- W3 型推論 dual-write の成果物と CLI オプション: [`reports/dual-write/front-end/w3-type-inference/README.md`](reports/dual-write/front-end/w3-type-inference/README.md) に `--dualwrite-root` の運用ルールと `remlc --frontend {ocaml,rust} --emit typeck-debug <dir>` を含む実行手順をまとめています。`scripts/poc_dualwrite_compare.sh --mode typeck --dualwrite-root reports/dual-write/front-end/w3-type-inference --run-id <label>` を利用し、Typed AST/Constraint/Impl Registry/Effects メトリクスの差分ログを取得してください。

## 実装ロードマップの要点

- **Phase 1 (OCaml ブートストラップ)**: パーサー/型推論/IR/LLVM/最小ランタイム/CLI/CI を揃える
- **Phase 2 (仕様安定化)**: 型クラス・効果タグ・診断メタデータ・Windows 対応を正式化
- **Phase 3 (Self-Host 移行)**: Reml 自身でコンパイラを構築し、標準ライブラリ API を完成
- **Phase 4 (リリース体制)**: マルチターゲット CI・署名・配布パイプライン・サポートポリシーを整備

詳細タスクや依存関係は [`docs/plans/bootstrap-roadmap/`](docs/plans/bootstrap-roadmap/) 以下を参照してください。

## サンプル実装

- [代数的効果サンプルセット](examples/algebraic-effects/README.md)
- [言語実装比較ミニ言語集](examples/language-impl-comparison/README.md)
- [Core.Collections 統合サンプル](examples/core-collections/README.md)
- [Core.Text & Unicode サンプル](examples/core-text/README.md)
- [Core Config & Manifest サンプル](examples/core_config/README.md)
- [Core Diagnostics & Audit サンプル](examples/core_diagnostics/README.md)
- [Core.Native Intrinsics サンプル](examples/native/README.md)

## コントリビューションのヒント

1. 仕様変更・ガイド更新時は `docs/spec/` および関連ノートの整合性を確認し、必要に応じて `docs-migrations.log` を更新
2. 実装タスクを着手する場合は `compiler/`, `runtime/`, `tooling/` の README を確認し、対応する計画書 (`docs/plans/...`) と同期
3. サンプルの追加・更新時は `examples/README.md` と関連仕様からのリンクを整備
4. 大規模なディレクトリ移動やリファクタリングを行う場合は [`docs/plans/repository-restructure-plan.md`](docs/plans/repository-restructure-plan.md) のフェーズ区分に従う
5. CLI の監査ログ圧縮 (`reports/audit/history/*.jsonl.gz`) は `camlzip` に依存するため、開発環境では `opam install . --deps-only --with-test` を実行して依存関係を揃える（`reml_ocaml.opam` に統合済み）。

## ライセンスとクレジット

Reml プロジェクトに関する利用条件やクレジット情報は今後 `docs/` 配下に集約予定です。暫定的な運用ポリシーは各仕様書・計画書内のライセンス欄を参照してください。
