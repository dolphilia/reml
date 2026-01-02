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
- `docs/notes/effects/effect-system-tracking.md` へ PoC の進捗ログと試験ケース一覧を記録し、Phase 2-7・Phase 3 でのフォローアップを容易にする。
- **タイミング**: 設計と PoC 条件の整理は Phase 2-5 の後半までに完了し、実装着手は Phase 2-7 の効果チームキックオフに合わせて開始する。

## 5. 実施ステップ
1. **Step1 スコープ確定と差分棚卸（Week32 Day1-2） — ✅ 完了（2026-04-08）**  
   - **実施内容**: 仕様側の残余効果計算・`@handles` 契約・Stage 条件を整理し、PoC がカバーすべき構文と Capability ガードを確定。`compiler/ocaml/src/parser.mly`・`compiler/ocaml/src/ast.ml`・`compiler/ocaml/src/typed_ast.ml`・`compiler/ocaml/src/type_inference.ml` を確認して `perform` / `handle` 系が未実装であることを棚卸し、差分を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記録した。  
   - **成果物**: `docs/notes/effects/effect-system-tracking.md` に PoC スコープ表・メトリクス基準値・実装差分を追記し、Phase 2-7 への引き継ぎ条件を整理。`docs/plans/bootstrap-roadmap/2-5-review-log.md` に Step1 棚卸エントリを追加。  
   - **フォローアップ**: Step2 以降は棚卸結果を前提に AST/Parser PoC を実装し、Menhir 差分・CI ガード導入の検証結果を追記する。

2. **Step2 AST / Parser PoC 対応（Week32 Day2-4） — ✅ 完了（2026-04-12）**  
   - **実施内容**: `compiler/ocaml/src/ast.ml` と `compiler/ocaml/src/typed_ast.ml` に効果構文ノード（`PerformCall` / `Handle`、`TEffectPerform` / `TEffectHandle` ほか補助型）を追加し、`ast_printer.ml` のデバッグ出力を更新。`compiler/ocaml/src/parser.mly` へ `perform` / `do` / `handle ... with` 規則と `effect_target` ヘルパを導入し、`-Zalgebraic-effects` 無効時は `Experimental_effects_disabled` 例外で拒否するガードを実装した。RunConfig に `experimental_effects: bool` フィールドと `set_experimental_effects` API を追加し、`parser_driver.ml` でフラグを伝搬させ `effects.syntax.experimental_disabled` 診断を発火させる。  
   - **成果物**: 効果構文 PoC を受理する Parser 実装、`compiler/ocaml/tests/test_parser.ml` に experimental フラグ有無のユニットテストを追加、`docs/notes/effects/effect-system-tracking.md`・`docs/plans/bootstrap-roadmap/2-5-review-log.md` 更新用メモ。  
   - **フォローアップ**: `menhir --explain` によるコンフリクト差分確認と CLI/監査ゴールデン整備は Step4 以降に持ち越し。CLI から `-Zalgebraic-effects` を受け取って RunConfig を切り替える経路を追加する。

3. **Step3 Typer / 効果解析 PoC（Week32 Day4-6） — ✅ 完了（2026-04-15）**  
   - `compiler/ocaml/src/type_inference.ml` に `PerformCall` / `Handle` 分岐を追加し、typed handler ノード（`THandle`）の最小限 PoC 推論と `Effect_analysis` でのタグ収集を実装。返り値は PoC として `()` を採用し、残余効果診断へ流れるタグに `Console` → `console` の正規化を適用。  
   - 効果解析ワークフローを検証する専用テスト `compiler/ocaml/tests/effect_handler_poc_tests.ml` を追加し、`perform` で `console` タグが残余集合へ反映されること、`handle` 式が Typed AST 上で `THandle` として構築されることを確認（CI 実行は `dune build tests/effect_handler_poc_tests.exe` が既存 `ast.ml` の再帰型前提に依存するため要ローカル確認）。  
   - **既知の制約**: `Σ_handler` の算出と `Σ_after` の差集合は Stage 2-7 で再設計予定。現 PoC では `handle` 捕捉タグの控除を行わず、効果ハンドラ内部の残余タグ収集のみサポート。戻り値型推論は `()` 固定の暫定実装。フォローアップを Step5 で `docs/notes/effects/effect-system-tracking.md` に記録。

4. **Step4 診断・CI 計測整備（Week33 Day1-2） — ✅ 完了（2026-04-18）**  
   - **実施内容（診断拡張）**: `compiler/ocaml/src/diagnostic.ml` と `compiler/ocaml/src/diagnostic_serialization.ml` を再確認し、`extensions["effects"]` に `sigma.before` / `sigma.handler` / `sigma.residual` / `sigma.after` と `constructs` 配列（`kind`・`tag`・`span`・`handled_by`・`diagnostics`）を持たせる設計を確定。`AuditEnvelope.metadata` では `effect.sigma.*` および `effect.syntax.constructs.*`（`total`・`accepted`・`poisoned`・`residual_tags`）を同期させ、既存の Stage フィールドと同じキー命名（ドット区切り）で保存する方針を策定した。  
     `Σ_after = (Σ_before - Σ_handler) ∪ Σ_residual` を JSON 上で再現するため、Typed AST 側で収集した残余効果を `sigma.residual` に格納し、`effect.contract.residual_snapshot` 診断と連携する設計メモを作成。  
   - **実施内容（CI 指標）**: `tooling/ci/collect-iterator-audit-metrics.py` の効果セクションを調査し、`syntax.effect_construct_acceptance = accepted / total`、`effects.syntax_poison_rate = poisoned / total` を算出するルーチンと `--require-success` 時の許容値（PoC 期間は 0.0 / 1.0、正式運用で 1.0 / 0.0）を定義。集計対象を `effect.syntax.constructs` と `audit.metadata["effect.sigma.*"]` から抽出する `iter_effect_constructs`（仮称）を追加する案を整理し、Stage 監査との整合条件をドキュメント化した。  
   - **実施内容（検証フロー）**: `scripts/validate-diagnostic-json.sh` の Node/Python 検証に効果構文のスキーマ検証を差し込む手順を整理。`extensions.effects.sigma` と `audit.metadata.effect.sigma.*` の存在チェック、`constructs` 配列の型検証、`Σ_after = ∅` 判定を `effects.contract.residual_snapshot` に伝搬させるための Python 補助関数を洗い出し、PoC 用フィクスチャ（`compiler/ocaml/tests/golden/diagnostics/effect-handler-poc.json.golden` 仮称）を作成して基準データを固定した。  
   - **成果物**: `docs/plans/bootstrap-roadmap/2-5-review-log.md` に Step4 調査記録を追加し、CI/診断の観測ポイントとキー対応表を掲載。`docs/notes/effects/effect-system-tracking.md` へ `effect.syntax.constructs` の JSON 仕様・メトリクス算出式・PoC フィクスチャ案を追記。`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分リストに `Σ` 記録と KPI 移管条件を追記し、Phase 2-7 での実装タスクとリンクさせた。  
   - **フォローアップ**: `EFFECT-003` および Phase 2-7 へ `diagnostic.ml` 拡張・CI ゴールデン更新・ダッシュボード実装を移譲（`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に登録済み）。CLI/LSP ゴールデン生成スクリプトへ `effect.syntax.constructs` を含める改修、`reports/diagnostic-format-regression.md` のチェックリスト更新、`effect.contract.residual_snapshot` のテストケース整備を Step5 以降で実施する。

5. **Step5 ドキュメント整合とハンドオーバー（Week33 Day2-3） — ✅ 完了（2026-04-20）**  
   - **実施内容**: Chapter 1（`docs/spec/1-1-syntax.md`・`docs/spec/1-3-effects-safety.md`・`docs/spec/1-5-formal-grammar-bnf.md`）に `Σ_before`/`Σ_after` 記録と PoC KPI (`syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`) の参照脚注を追加し、PoC 運用が Phase 2-5 `EFFECT-002 Step4` 仕様と `docs/notes/effects/effect-system-tracking.md` のハンドオーバー条件へ準拠することを明記した。索引（`docs/spec/0-0-overview.md`, `docs/spec/README.md`）を更新し、効果構文が Experimental ステージである旨と指標連携を読者が追跡できるようにした。  
   - **連携更新**: `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の `EFFECT-002` 差分項へ Step5 完了メモを追記し、`docs/notes/effects/effect-system-tracking.md` に Step5 ログと KPI 参照表を追加。`docs/plans/bootstrap-roadmap/2-5-review-log.md` にドキュメント整合完了エントリを記録し、残課題を Phase 2-7 計画と `0-4-risk-handling.md` へ転記した。  
   - **成果物**: 更新済み仕様・索引・計画書・ノート、PoC KPI の参照ルート、`0-4-risk-handling.md` に登録した「効果構文 Stage 遷移遅延」リスク（ID: EFFECT-POC-Stage）。

## 6. 残課題
- PoC が対象とする効果構文の範囲（`perform` のみか、`resume`/`rethrow` まで含むか）を効果チームと確認したい。  
- PoC をどのリリースチャネルで公開するか、運用上の方針（experimental フラグの命名）を Phase 2-7 と協議する必要がある。
