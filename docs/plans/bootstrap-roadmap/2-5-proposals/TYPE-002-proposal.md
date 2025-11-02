# TYPE-002 効果行統合ポリシー計画

## 1. 背景と症状
- 仕様は関数型に効果集合を含める（`A -> B ! {io, panic}`）と定義し、行多相や残余効果計算を規定している（docs/spec/1-2-types-Inference.md:155-169, docs/spec/1-3-effects-safety.md:236-303）。  
- 現行型表現 `ty` は `TArrow` のみで効果情報を保持せず（compiler/ocaml/src/types.ml:48-58）、実際の効果は `typed_fn_decl.tfn_effect_profile` に別管理される（compiler/ocaml/src/type_inference.ml:2380-2404）。  
- この乖離により、型比較や `@handles` 契約では効果集合を参照できず、仕様上の「型と効果が対で扱われる」という前提が崩れている。

## 2. Before / After
### Before
- 効果解析は `Effect_analysis` で行うが、型スキームは効果集合を持たないため `let` 多相や値制限で効果差分をチェックできない。  
- ドキュメント上は効果行が型の一部と説明されているが、実装では診断メタデータ扱いであり、自動整合（`Σ_after` 等）が不可能。

### After
- Phase 2-5 では仕様に脚注を追加し「OCaml 実装は効果行を型スキームに統合する準備中」と明記。  
- 効果集合を `ty` へ統合する設計案を作成し、`compiler/ocaml/docs/effect-system-design-note.md` に `TArrow` 拡張（`TArrow of ty * effect_row * ty` など）のドラフトを追記。  
- 実装ロードマップを Phase 2-7 効果チームと共有し、効果行統合の段階的導入（診断 → 型表現 → 行多相）を調整する。

## 3. 影響範囲と検証
- **型比較**: 効果を考慮した型等価・部分順序の仕様を整理し、`Type_unification` テストを追加（`compiler/ocaml/tests/test_type_inference.ml` に `type_effect_row_*` 系ケースを新設し、`types.ml:48` で導入する `TArrow` 拡張を厳密に検証）。  
- **残余効果**: EFFECT-002 / EFFECT-003 の実装と連動し、効果集合を型内で扱えるか PoC を実施（`Effect_analysis` → `Type_inference_effect` → `generalize`/`instantiate` の各経路で残余効果・Stage 情報が消失しないことを `compiler/ocaml/tests/streaming_runner_tests.ml` と監査ゴールデンで追跡）。  
- **ドキュメント**: Chapter 1/3 の効果行説明に実装ステージを明記し、読者が差分状態を把握できるよう脚注を追加（`docs/spec/1-2-types-Inference.md` §A.2、`docs/spec/1-3-effects-safety.md` §4.2、`docs/spec/3-6-core-diagnostics-audit.md` §5 に脚注と参照を配置）。  
- **設計ノート**: `compiler/ocaml/docs/effect-system-design-note.md` に `effect_row` のデータ構造比較（リスト/ビットセット/マップ）の評価結果を追記し、仕様更新時の根拠を残す。`docs/notes/effect-system-tracking.md` に調査ログと PoC 実験条件を記録。

## 4. フォローアップ
- 効果行を型へ組み込む際、`generalize` / `instantiate` を更新する必要があるため、Phase 2-7 の型クラスチームへ事前連絡する。  
- 型表現の変更に伴う Core IR や LLVM バックエンドへの影響を調査し、行多相を導入する際の性能評価計画を立てる。  
- 仕様側脚注を解除する時期と、typeclass 差分（TYPE-003）との整合を Phase 3 手前で再評価する。
- `docs/notes/effect-system-tracking.md` に行多相導入ロードマップを追記し、型チームと効果チームで共有するチェックポイントを明記する。
- **タイミング**: Phase 2-5 では設計検討と脚注整備を完了し、実装は Phase 2-7 の効果システム統合スプリント開始時に着手、必要に応じて Phase 3 序盤まで延長する。

## 5. 実施ステップ
1. **Step1 現状棚卸と差分タグ付け（Week32 Day1-3 / 担当: Type チーム） — ✅ 完了（2026-04-10）**  
   - **実施内容**: `docs/spec/1-2-types-Inference.md:155-210` と `docs/spec/1-3-effects-safety.md:236-303` を再読し、`A -> B ! Σ` 前提と `Σ_after = (Σ_before - Σ_handler) ∪ Σ_residual` の整合を確認。`compiler/ocaml/src/types.ml:48-72`、`compiler/ocaml/src/type_inference.ml:2691-2734`、`compiler/ocaml/src/typed_ast.ml:167-175` を棚卸し、効果集合が `typed_fn_decl.tfn_effect_profile` に分離管理されている現状を整理した。  
   - **成果物**: `docs/plans/bootstrap-roadmap/2-5-review-log.md#type-002-step1-効果行統合棚卸2026-04-10` に差分ログを追加し、タグ `TYPE-002-S1` を発行。`docs/notes/effect-system-tracking.md` に「Phase 2-5 TYPE-002 Step1 効果行統合棚卸（2026-04-10）」節を追記し、Step2/Step3 への入力（`effect_row` データ構造比較、脚注追加候補）をまとめた。  
   - **フォローアップ**: Step2 で `compiler/ocaml/docs/effect-system-design-note.md` の型表現ドラフトを更新し、Step3 で Chapter 1-2 / 1-3 へ脚注を追加する。

2. **Step2 型表現拡張案の起草と評価（Week32 Day4-5 / 担当: Type + Effect） — ✅ 完了（2026-04-18）**  
   - **調査**: `Effect_analysis.collect_from_fn_body`（compiler/ocaml/src/type_inference.ml:292）で返却されるタグ列と `typed_fn_decl.tfn_effect_profile` の保持形式を突合し、`effect_row` へ移行する際の互換性を検証。`string list`・`StringSet.t`・`row_var` の 3 案について、集合演算コスト・診断表示順序・行多相拡張の観点で評価した。  
   - **成果物**: `compiler/ocaml/docs/effect-system-design-note.md` に新設した「## 3. 型表現統合ドラフト（TYPE-002 Step2, 2026-04-18）」で、候補データ構造比較表・`TArrow of ty * effect_row * ty` 案の API 差分・影響モジュール一覧・データフロー図を整理。`effect_row` を表示用配列と正規化集合を組み合わせた二層構造で扱う方針を暫定採用とした。  
   - **フォローアップ**: Phase 2-7 で `generalize` / `instantiate` / `solve_trait_constraints` の RowVar 対応を実装できるよう、`docs/plans/bootstrap-roadmap/2-5-review-log.md#type-002-step2-型表現統合ドラフト2026-04-18` に検証観点とテスト案を記録。Step3 では仕様脚注と移行ガードの設計に着手する。

3. **Step3 仕様脚注と移行ガード設計（Week33 Day1-2 / 担当: Docs） — ✅ 完了（2026-04-22）**  
   - **調査**: `docs/spec/1-2-types-Inference.md` §A.2 / §C.6、`docs/spec/1-3-effects-safety.md` §A・§I、`docs/spec/3-6-core-diagnostics-audit.md` §2.4.2 を突合し、効果行を型スキームへ統合していない現行実装との差異とガード要件（`type_row_mode`）を洗い出した。  
   - **作業**: 各仕様書に脚注 `[^type-row-metadata-phase25]` を追加し、Phase 2-5 の暫定運用（診断メタデータ保持と `RunConfig.extensions["effects"].type_row_mode = "metadata-only"` ガード）を明文化。`effects.type_row.integration_blocked` 診断と `effect.type_row.*` 監査キーを定義し、索引用 `docs/spec/README.md`・`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と連動させた。  
   - **成果物**: `docs/plans/bootstrap-roadmap/2-5-review-log.md#type-002-step3-効果行脚注と移行ガード2026-04-22` にレビュー記録を追加し、`TYPE-002-S3` タグを発行。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration` へ解除条件（`TArrow` 拡張・`diagnostics.effect_row_stage_consistency` KPI・レビュー承認）を引き継いだ。

4. **Step4 実装ロードマップとテスト観点の確定（Week33 Day3-5 / 担当: Type + QA） — ✅ 完了（2026-04-24）**  
   - **調査**: `compiler/ocaml/src/type_inference.ml:2684-2740` と `compiler/ocaml/src/constraint_solver.ml:121-332` を再確認し、`generalize` / `instantiate` / `solve_trait_constraints` が `TArrow` 再構築時に一切の効果情報を参照していないことを特定。`compiler/ocaml/tests/test_type_inference.ml:612-910` と `compiler/ocaml/tests/streaming_runner_tests.ml:210-398` の既存ケースを棚卸しし、効果行統合後に追加すべきテストカテゴリ（等価判定・残余効果保持・監査整合）を洗い出した。`tooling/ci/collect-iterator-audit-metrics.py:148-362` の指標収集経路を確認し、効果行 KPI を集計する拡張ポイントを特定。  
   - **実施内容**: Phase 2-7 の 3 スプリント構成（Sprint A: 型表現と `effect_row` 基盤、Sprint B: `generalize`/`instantiate`/`Type_unification` 拡張と診断出力、Sprint C: Core IR・監査伝播と Windows/macOS クロスチェック）を決定し、各スプリントで触るモジュールと依存関係を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration` に反映。`RunConfig.extensions["effects"].type_row_mode` の段階的切替手順（`metadata-only` → `dual-write` → `ty-integrated`）とロールバック条件を定義し、CLI/LSP/CI それぞれでのガードを文書化した。  
   - **成果物**: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に新規 KPI `diagnostics.effect_row_stage_consistency` / `type_effect_row_equivalence` / `effect_row_guard_regressions` を追加し、指標の収集タイミングと `collect-iterator-audit-metrics.py` での検証方法を記録。`docs/notes/effect-system-tracking.md` へ Step4 ログを追記し、効果行統合ロードマップとテスト観点を Phase 2-7 へ引き継ぐチェックポイントを整理。`docs/plans/bootstrap-roadmap/2-5-review-log.md#type-002-step4-実装ロードマップとテスト観点2026-04-24` にレビュー記録を追加し、タグ `TYPE-002-S4` を発行した。  
   - **フォローアップ**: Sprint A 着手前に `effect_row` 実装ブランチへ `types.ml` / `typed_ast.ml` の dual-write 実験を用意し、`TYPE-002 Step5` ハンドオーバーで Windows CI 監査プロファイルの追加確認を行う。`effect_row` 行多相（`RowVar`) の保留事項は Phase 3 での評価対象として `docs/notes/effect-system-tracking.md#phase-3-以降の検討` に移管。

5. **Step5 ハンドオーバー準備とリスク登録（Week34 Day1 / 担当: PM） — ✅ 完了（2026-04-24）**  
   - **調査**: `compiler/ocaml/docs/technical-debt.md` の H1（type_mapping TODO）と効果プロファイル項目を再確認し、行統合の遅延が Phase 3 Self-host 判定へ与える影響を整理。  
   - **作業**: Phase 2-7 効果チーム向けハンドオーバーノート [`2-5-to-2-7-type-002-handover.md`](../2-5-to-2-7-type-002-handover.md) を作成し、Gate 条件（設計レビュー完了、脚注・ガード整合、テスト基盤準備、リスクレビュー）を定義。`docs/plans/bootstrap-roadmap/2-4-to-2-5-handover.md` に追記して導線を確保。  
   - **成果物**: `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスク ID `TYPE-002-ROW-INTEGRATION` を登録し、期限 2026-10-31・状態 Open で追跡開始。`docs/plans/bootstrap-roadmap/2-5-review-log.md#type-002-step5-ハンドオーバー準備とリスク登録2026-04-24` にレビュー記録（タグ `TYPE-002-S5`）を追加し、Phase 2-6 週次レビューで報告可能な状態にした。

## 6. 残課題
- 効果行を `ty` に含める際の表現形式（リスト / 集合 / 位置付きタグ）をどこまで詳細化するか、型推論チームの合意が必要。  
- 行多相の完全導入をどのフェーズで行うか（Phase 3 へ繰越すか）を PM と相談したい。
