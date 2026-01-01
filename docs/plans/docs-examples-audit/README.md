# ドキュメント Reml コード検証計画

`docs/` 配下の Reml コードブロックを `.reml` として抽出し、検証可能な資産として `examples/docs-examples/` に集約する計画書群。

## 目的
- 仕様・ガイド・ノートの Reml コードが実装と整合し、再現可能であることを保証する。
- コードブロックと `.reml` の 1:1 対応を確立し、監査ログとの参照を一貫させる。

## 参照優先度
1. `docs/spec/`
2. `docs/guides/`
3. `docs/notes/` と `docs/plans/`

## 収録ファイル
- [0-0-overview.md](0-0-overview.md)
- [0-1-workflow.md](0-1-workflow.md)
- [1-0-validation-plan.md](1-0-validation-plan.md)
- [1-1-spec-code-block-inventory.md](1-1-spec-code-block-inventory.md)
- [1-2-spec-sample-fix-plan.md](1-2-spec-sample-fix-plan.md)
- [1-2-spec-sample-fix-targets.md](1-2-spec-sample-fix-targets.md)
- [1-2-impl-gap-backend-runtime-plan-20251224.md](1-2-impl-gap-backend-runtime-plan-20251224.md)
- [1-2-impl-gap-backend-runtime-plan-20251224-2.md](1-2-impl-gap-backend-runtime-plan-20251224-2.md)
- [1-7-backend-runtime-type-decl-layout-plan-20251227.md](1-7-backend-runtime-type-decl-layout-plan-20251227.md)
- [2-0-stdlib-plugin-migration-plan.md](2-0-stdlib-plugin-migration-plan.md)
- [2-1-stdlib-plugin-migration-impl-plan.md](2-1-stdlib-plugin-migration-impl-plan.md)

## 配置ポリシー（概要）
- `.reml` は `examples/docs-examples/<kind>/<doc-path>/` に配置する。
- `<kind>` は `spec` / `guides` / `notes` / `plans` を使用する。
- 既存サンプルの移設やリンク修正は `docs-migrations.log` に記録する。

## 関連資料
- `docs/spec/0-3-code-style-guide.md`
- `docs/spec/1-1-syntax.md`
- `docs/guides/compiler/core-parse-streaming.md`
- `docs/plans/repository-restructure-plan.md`
