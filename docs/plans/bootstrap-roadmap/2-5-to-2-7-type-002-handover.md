# 2-5 → 2-7 TYPE-002 効果行統合ハンドオーバー

**作成日**: 2026-04-24  
**作成者**: Phase 2-5 Type チーム（TYPE-002）

## 1. 目的とステータス
- Phase 2-5 では効果行統合ポリシーの設計と脚注整備を完了し、実装は Phase 2-7 へ移管する。  
- Step4 までに `effect_row` 統合ドラフト・テスト観点・メトリクス案・脚注ガードを整備済み。Step5 ではリスク登録とゲート条件を確定し、2-7 開始時に参照できるハンドオーバー資料を作成する。  
- Phase 2-7 Sprint C で `RunConfig.extensions["effects"].type_row_mode` の既定値を `"ty-integrated"` へ切り替え、`metadata-only` は互換モードとして維持する。

## 2. 引き継ぎ対象と成果物
| 項目 | 内容 | 参照 |
| --- | --- | --- |
| 設計ドラフト | `TArrow of ty * effect_row * ty` 拡張とデータフロー設計 | `compiler/ocaml/docs/effect-system-design-note.md` §3 |
| テスト観点 | `type_effect_row_*` / `diagnostics.effect_row_stage_consistency` などの追加カテゴリ | `docs/plans/bootstrap-roadmap/2-5-review-log.md#type-002-step4-実装ロードマップとテスト観点2026-04-24` |
| KPI 基準値 | `diagnostics.effect_row_stage_consistency`, `type_effect_row_equivalence`, `effect_row_guard_regressions` | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`（2026-04-24 更新分） |
| 移行ガード | `RunConfig.extensions["effects"].type_row_mode` の運用とロールバック条件 | `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration` |
| ドキュメント更新 | 効果行統合に関する本文・索引の整合（脚注撤去後の状態） | `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/README.md` |
| プロジェクトノート | フォローアップログと PoC 運用メモ | `docs/notes/effect-system-tracking.md`（TYPE-002 セクション） |

## 3. 移行時の Gate 条件
Phase 2-7 で TYPE-002 実装を着手する前に、以下の条件を満たすこと：
1. 設計レビュー  
   - `effect_system_design` レビュー（Type/Effect/Runtime リード参加）を実施し、`TArrow` 拡張のデータ構造と RowVar 対応方針を確認する。  
   - レビュー完了ログを `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記し、`TYPE-002-G1` タグを登録。
2. 仕様・ガードの整合確認  
   - `docs/spec/` 各章で `type_row_mode` の説明と `effect.type_row.*` メタデータが最新実装と一致しているかを確認する。  
   - 変更があった場合は `docs/spec/README.md` と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の参照を更新。
3. テスト基盤の準備  
   - `test_type_inference.ml` と `streaming_runner_tests.ml` に effect row 追跡用の仮テストスケルトン（`SKIP` マーカー付）を配置し、`type_effect_row_*` シリーズを追加できる状態にする。  
   - `tooling/ci/collect-iterator-audit-metrics.py` で KPI を計測するフック（`--section effects` 拡張）を追加していること。
4. Risk Review  
   - リスク ID `TYPE-002-ROW-INTEGRATION` の状態を `Open` に設定し、対応者と期限（2026-10-31）をアサインする。  
   - 週次レビューで KPI 未達や検証ブロッカーが発生した場合のエスカレーション経路を確認する。

## 4. 既知リスクとトラッキング
- **リスク ID**: `TYPE-002-ROW-INTEGRATION`（`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に登録）  
  - 概要: 効果行が型スキームに統合されないまま Phase 3 へ進むと、`@handles`/Stage 契約の検証が実行時依存のままとなり、Self-host CI の判定精度が低下する。  
  - 対応案: Phase 2-7 Sprint B で `generalize` / `instantiate` / `Type_unification` を更新し、Sprint C で Core IR・監査経路を同期。指標 `diagnostics.effect_row_stage_consistency = 1.0` を達成した時点で脚注解除審査を実施する。
- **関連技術的負債**: `compiler/ocaml/docs/technical-debt.md` の H1 (`type_mapping` TODO) と効果プロファイル連携課題を継続監視。RowVar 対応の実装ブロックは Phase 3 へ持ち越さない。

## 5. フォローアップタスク
1. Sprint A（Week35-36）: `effect_row` 型の導入、`types.ml`・`typed_ast.ml`・`effect_analysis` の dual-write を実装。  
2. Sprint B（Week37-38）: `generalize` / `instantiate` / `Type_unification` の効果行比較実装と、`type_effect_row_*` テストを有効化。  
3. Sprint C（Week39-40）: Core IR / LLVM Backend / 診断・監査の効果行伝播、KPI 計測の有効化。  
4. Phase 3 移行前: `type_row_mode = ty-integrated` を既定化し、索引・リスク台帳を更新（2026-12-18 に完了済み）。

## 6. 参照リンク
- `docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md`
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`
- `docs/notes/effect-system-tracking.md`
- `compiler/ocaml/docs/effect-system-design-note.md`

---
Phase 2-7 チームは本ハンドオーバーノートを起点に、Sprint 計画とレビュー資料を整備すること。更新が発生した場合は本書および関係ドキュメントを同時に更新し、差分を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記録する。
