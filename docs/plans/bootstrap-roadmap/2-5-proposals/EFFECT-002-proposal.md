# EFFECT-002 効果操作 PoC 明確化計画

## 1. 背景と症状
- 仕様では効果操作とハンドラ適用の残余効果計算を定義し（docs/spec/1-3-effects-safety.md:200-268）、`Σ_after` などの計算式を提示している。  
- 実装は `effect` / `handler` を AST に保持するものの、型推論や効果解析で `perform`・`handle` を処理しておらず、`Σ_before`/`Σ_after` の検証ができない。  
- 効果操作が利用できないため、効果 PoC（Phase 2-2）で用意したサンプルが OCaml 実装で失敗し、残余効果契約や Capability 検査の整合を確認できない。

## 2. Before / After
### Before
- `Effect_analysis` は `panic` 等の基本タグのみを収集し、`perform` / `handle` による効果移動を扱わない。  
- `Type_inference` 側で `handler` 宣言を型付けする処理が未実装で、`THandlerDecl` はプレースホルダのまま。  
- 仕様との乖離を示す脚注がなく、読者は効果操作が利用可能と誤解する恐れがある。

### After
- Chapter 1 に「Phase 2 時点では効果操作は PoC ステージ、および `-Zalgebraic-effects` 有効時に限定」と脚注を追加し、実装差分を明示。  
- 効果操作の PoC 実装方針をまとめ（`perform_expr` / `handle_expr` の解析、残余効果計算、`@handles` 検査）、Phase 2-2 / Phase 2-7 効果チームへレビューを依頼する。  
- PoC 完了まで `Effect_analysis` に `perform` / `handle` のモック処理を追加し、`Σ_before` 記録だけでも可能にする。

## 3. 影響範囲と検証
- **型推論**: `compiler/ocaml/src/type_inference_effect.ml` と `compiler/ocaml/src/constraint_solver.ml` に `perform` / `handle` の型規則と `Σ_before → Σ_after` の写像を追加し、`EffectConstraintTable` で残余効果を追跡する。`compiler/ocaml/tests/test_type_inference.ml` を PoC ケースで拡張し、辞書モードとモノモルフィゼーションの両方で `effects.contract.*` 診断が一致することを確認する。  
- **診断**: `compiler/ocaml/src/diagnostic.ml`・`compiler/ocaml/src/diagnostic_serialization.ml` を拡張し、`effects.contract.mismatch` と `effects.syntax.experimental_disabled` の PoC ケースを `reports/diagnostic-format-regression.md` に追加する。`scripts/validate-diagnostic-json.sh` を通じて CLI/LSP/監査 JSON に `Σ_before`・`Σ_after`・捕捉効果が埋まるか検証する。  
- **ドキュメント**: `docs/spec/1-3-effects-safety.md` と `docs/spec/3-8-core-runtime-capability.md` に PoC ステージの注記と進捗リンクを付与し、`[^effects-syntax-poc-phase25]` の撤去条件を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に同期する。  
- **実装テスト**: `compiler/ocaml/tests/effect_handler_poc_tests.ml` を新設し、`perform`/`handle` の残余効果履歴が `Σ_before`・`Σ_after` に反映されるか CI で検証する。併せて `tooling/ci/collect-iterator-audit-metrics.py` に `syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` の PoC 基準値を登録する。

## 4. フォローアップ
- 効果操作の本格実装は `EFFECT-003`（Capability 多重処理）と密接に関係するため、タスクを一体管理する。  
- Phase 3 の self-host 移植前に、効果 PoC（ハンドラ 1st クラス）を完成させるマイルストーンを設定し、`0-3-audit-and-metrics.md` へ記録する。  
- `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` に PoC ステージの成果物と検証項目を追記する。
- `docs/notes/effect-system-tracking.md` へ PoC の進捗ログと試験ケース一覧を記録し、Phase 2-7・Phase 3 でのフォローアップを容易にする。
- **タイミング**: 設計と PoC 条件の整理は Phase 2-5 の後半までに完了し、実装着手は Phase 2-7 の効果チームキックオフに合わせて開始する。

## 5. 実施ステップ
1. **Step1 スコープ確定と差分棚卸（Week32 Day1-2） — ⏳ 着手前**  
   - **調査**: `docs/spec/1-1-syntax.md` §J、`docs/spec/1-3-effects-safety.md` §I、`docs/spec/3-8-core-runtime-capability.md`、`docs/notes/effect-system-tracking.md` を読み、PoC で許容する構文・Stage を整理する。`compiler/ocaml/src/parser.mly`・`compiler/ocaml/src/ast.ml`・`compiler/ocaml/src/typed_ast.ml`・`compiler/ocaml/src/type_inference.ml` を確認し、`perform` / `handle` ノードが未実装であることを棚卸する。  
   - **実施項目**: `docs/plans/bootstrap-roadmap/2-5-review-log.md` に棚卸結果と既存設計メモ（`compiler/ocaml/docs/effect-system-design-note.md` / `docs/notes/effect-system-tracking.md`）の参照を追記し、`-Zalgebraic-effects` ガードと PoC 限定事項を Phase 2-7 へ引き継ぐ前提条件として整理する。  
   - **成果物**: 棚卸メモ、`docs/notes/effect-system-tracking.md` の更新（PoC スコープ表とメトリクス基準値の確定）。

2. **Step2 AST / Parser PoC 対応（Week32 Day2-4） — ⏳ 着手前**  
   - **調査**: `parser_design.md` の効果構文セクション、`compiler/ocaml/src/parser_run_config.ml`、`tooling/cli` 配下のフラグ定義を再確認し、Menhir 生成物 (`parser.automaton` / `parser.conflicts`) の現状を把握する。  
   - **実施項目**: `compiler/ocaml/src/ast.ml`・`compiler/ocaml/src/typed_ast.ml` に `PerformCall` / `HandleExpr`（仮称）を追加し、`compiler/ocaml/src/parser.mly`・`parser_actions.ml`・`parser_expectation.ml` を更新する。`parser_run_config.ml` に `experimental_effects: bool` を追加して `-Zalgebraic-effects` 無効時は即時エラーを返す。`menhir --explain compiler/ocaml/src/parser.mly` を実行し、コンフリクト差分を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記録する。  
   - **成果物**: Parser PoC 実装、`compiler/ocaml/tests/golden/parser/effects/*.golden`（新設）と CLI エラーメッセージのゴールデン、`parser_expectation.mli` の拡張案。

3. **Step3 Typer / 効果解析 PoC（Week32 Day4-6） — ⏳ 着手前**  
   - **調査**: `compiler/ocaml/src/effect_profile.ml`、`compiler/ocaml/src/type_inference_effect.ml`、`compiler/ocaml/src/effect_analysis.ml`、`compiler/ocaml/src/constraint_solver.ml` の既存フローを確認し、`EffectConstraintTable` と Stage 判定の接続点を洗い出す。  
   - **実施項目**: Typed AST に `TEffectPerform` / `TEffectHandle`（仮称）を導入し、`Effect_analysis.collect_expr` で `Σ_before` を収集する。`type_inference_effect.ml` に `perform` / `handle` の型規則と `Σ_after = (Σ_before - Σ_handler) ∪ Σ_residual` の更新ロジックを追加し、`constraint_solver.ml` が `Σ_handler`・`Σ_residual` を診断へ伝播できるようにする。  
   - **成果物**: `compiler/ocaml/tests/effect_handler_poc_tests.ml` の新規作成、`compiler/ocaml/tests/test_type_inference.ml` への PoC ケース追加、`reports/diagnostic-format-regression.md` への追記草案。

4. **Step4 診断・CI 計測整備（Week33 Day1-2） — ⏳ 着手前**  
   - **調査**: `compiler/ocaml/src/diagnostic.ml`・`compiler/ocaml/src/diagnostic_serialization.ml`・`compiler/ocaml/src/main.ml`、`tooling/ci/collect-iterator-audit-metrics.py`、`scripts/validate-diagnostic-json.sh` を分析し、効果構文の残余効果データを拡張フィールドに流す経路を確認する。  
   - **実施項目**: `Diagnostic.extensions["effects"]` と `AuditEnvelope.metadata` に `perform` / `handle` の `Σ_before`・`Σ_handler`・`Σ_after` を記録し、CLI/LSP/監査のゴールデンを更新する。CI 指標 `syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` を PoC 期間は (0.0 / 1.0) に設定し、`tooling/ci/collect-iterator-audit-metrics.py --require-success` で監視できるようスクリプトを拡張する。  
   - **成果物**: 新規ゴールデン一式、`reports/diagnostic-format-regression.md` の更新、`0-3-audit-and-metrics.md` への指標登録。

5. **Step5 ドキュメント整合とハンドオーバー（Week33 Day2-3） — ⏳ 着手前**  
   - **調査**: `docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/1-3-effects-safety.md`・`docs/spec/3-8-core-runtime-capability.md` の脚注・表を確認し、PoC 注記の挿入先と撤去条件を整理する。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`、`docs/notes/effect-system-tracking.md`、`0-4-risk-handling.md` を参照し、引き継ぎ TODO とリスク登録フォームを揃える。  
   - **実施項目**: 仕様書へ PoC 脚注と参照リンクを追記し、`README.md`・`docs/spec/0-0-overview.md` の導線を更新する。Phase 2-7 へ残課題を転記し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分リストを同期する。  
   - **成果物**: 更新済み仕様・索引・計画書、`docs/notes/effect-system-tracking.md` の最終ログ、`0-4-risk-handling.md` への残課題登録。

## 6. 残課題
- PoC が対象とする効果構文の範囲（`perform` のみか、`resume`/`rethrow` まで含むか）を効果チームと確認したい。  
- PoC をどのリリースチャネルで公開するか、運用上の方針（experimental フラグの命名）を Phase 2-7 と協議する必要がある。
