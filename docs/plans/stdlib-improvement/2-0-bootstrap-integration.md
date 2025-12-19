# 2.0 ブートストラップ計画との統合

## 目的
- Phase 4 の回帰計画（spec_core）に標準ライブラリ拡張の検証ポイントを統合する。
- Phase 5 以降のセルフホスト段階で DSL 支援機能が欠落しないよう、依存関係と順序を明示する。

## 接続ポイント
- 参照元: `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md`
- 参照先: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`

## 統合方針
1. **Core.Test** を最優先で設計し、Phase 4 の回帰シナリオにスナップショット/診断チェックを追加する。
2. **Core.Cli** を次点で整備し、DSL CLI の挙動を回帰シナリオとして登録する。
3. **Core.Text.Pretty** はフォーマッタ/コード生成のシナリオとして Phase 4 後半へ追加する。
4. **Core.Lsp/Core.Doc** は Phase 5 以降のセルフホストと同期し、最小仕様の確定後に回帰対象へ追加する。

## 依存関係
- `Core.Test` は `Core.Diagnostics` の整合（`docs/spec/3-6-core-diagnostics-audit.md`）を前提とする。
- `Core.Cli` は `Core.Env` と CLI 診断の統一を前提とする。
- `Core.Lsp` は `Core.Parse` のストリーミング/キャンセル方針と整合が必要。

## 成果物
- Phase 4 シナリオへの追加方針
- 仕様更新の順序と依存関係メモ
