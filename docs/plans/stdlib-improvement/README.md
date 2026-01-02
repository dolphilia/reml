# 標準ライブラリ改善計画（DSL開発者体験）

ステータス: ドラフト（初版）

`docs/notes/stdlib/stdlib-improvement-proposal.md` を起点に、DSL のライフサイクル（テスト・CLI・整形・ドキュメント・IDE 支援）を標準ライブラリで支えるための計画群を整理する。
`docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` を再開する前に、標準ライブラリ側の欠落を補い、回帰の対象と計測指標を拡張する。

- 設計指針: `docs/spec/0-1-project-purpose.md`
- 参照仕様: `docs/spec/3-0-core-library-overview.md`, `docs/spec/3-1-core-prelude-iteration.md`, `docs/spec/3-2-core-collections.md`, `docs/spec/3-3-core-text-unicode.md`, `docs/spec/3-5-core-io-path.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-10-core-env.md`
- 関連計画: `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md`

## 文書一覧
- [0-0-overview.md](0-0-overview.md): 背景・目的・成功条件・方針
- [0-1-workstream-tracking.md](0-1-workstream-tracking.md): ワークストリームと成果物の追跡ルール
- [1-0-core-test-plan.md](1-0-core-test-plan.md): `Core.Test`（統合/ゴールデン/ファジング）計画
- [1-1-core-cli-plan.md](1-1-core-cli-plan.md): `Core.Cli`（宣言的 CLI）計画
- [1-2-core-text-pretty-plan.md](1-2-core-text-pretty-plan.md): `Core.Text.Pretty`（プリティプリンタ）計画
- [1-3-core-doc-plan.md](1-3-core-doc-plan.md): `Core.Doc`（ドキュメント生成）計画
- [1-4-core-lsp-plan.md](1-4-core-lsp-plan.md): `Core.Lsp`（LSP ツールキット）計画
- [2-0-bootstrap-integration.md](2-0-bootstrap-integration.md): Phase4/5 との接合点

## 運用ルール
- 新規モジュールを追加する場合は `docs/spec/3-0-core-library-overview.md` へ概要を追記する。
- 用語追加や名称変更がある場合は `docs/spec/0-2-glossary.md` を更新する。
- 仕様差分や判断理由は脚注または TODO として根拠ファイルをリンクし、`docs/notes/` に判断記録を残す。
- Phase 4 回帰と関連するシナリオや診断キーは `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` へ登録する。

## 変更履歴
- 初版ドラフトを作成。
