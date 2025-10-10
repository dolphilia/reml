# Reml プロジェクト概要

[![Bootstrap Linux CI](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-linux.yml/badge.svg)](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-linux.yml)

Reml (Readable & Expressive Meta Language) はパーサーコンビネーターと静的保証に重点を置いた言語設計プロジェクトです。本リポジトリは仕様、設計ガイド、ブートストラップ実装計画、サンプル実装を集約し、言語実装とエコシステム整備を進めるための中枢ドキュメントとして機能します。

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
- `docs-migrations.log`: 大規模ドキュメント移行の履歴
- `AGENTS.md` / `CLAUDE.md`: AI エージェント向け作業ガイド

## ドキュメントへの導線

- 仕様書・ガイド・調査ノートの全体索引: [`docs/README.md`](docs/README.md)
- ブートストラップ計画の統合マップ: [`docs/plans/bootstrap-roadmap/README.md`](docs/plans/bootstrap-roadmap/README.md)
- リポジトリ再編計画書: [`docs/plans/repository-restructure-plan.md`](docs/plans/repository-restructure-plan.md)
- 仕様書の差分履歴や横断的メモ: `docs/notes/` 配下の各ノートを参照

## 実装ロードマップの要点

- **Phase 1 (OCaml ブートストラップ)**: パーサー/型推論/IR/LLVM/最小ランタイム/CLI/CI を揃える
- **Phase 2 (仕様安定化)**: 型クラス・効果タグ・診断メタデータ・Windows 対応を正式化
- **Phase 3 (Self-Host 移行)**: Reml 自身でコンパイラを構築し、標準ライブラリ API を完成
- **Phase 4 (リリース体制)**: マルチターゲット CI・署名・配布パイプライン・サポートポリシーを整備

詳細タスクや依存関係は [`docs/plans/bootstrap-roadmap/`](docs/plans/bootstrap-roadmap/) 以下を参照してください。

## サンプル実装

- [代数的効果サンプルセット](examples/algebraic-effects/README.md)
- [言語実装比較ミニ言語集](examples/language-impl-comparison/README.md)

## コントリビューションのヒント

1. 仕様変更・ガイド更新時は `docs/spec/` および関連ノートの整合性を確認し、必要に応じて `docs-migrations.log` を更新
2. 実装タスクを着手する場合は `compiler/`, `runtime/`, `tooling/` の README を確認し、対応する計画書 (`docs/plans/...`) と同期
3. サンプルの追加・更新時は `examples/README.md` と関連仕様からのリンクを整備
4. 大規模なディレクトリ移動やリファクタリングを行う場合は [`docs/plans/repository-restructure-plan.md`](docs/plans/repository-restructure-plan.md) のフェーズ区分に従う

## ライセンスとクレジット

Reml プロジェクトに関する利用条件やクレジット情報は今後 `docs/` 配下に集約予定です。暫定的な運用ポリシーは各仕様書・計画書内のライセンス欄を参照してください。
