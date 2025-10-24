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
- **型推論**: `Type_inference_effect` へ `perform` / `handle` の型規則を追加し、`Σ_after` 計算が仕様通りかテストする。  
- **診断**: `effects.contract.mismatch` の PoC ケースを `reports/diagnostic-format-regression.md` に追加し、PoC でも差分を把握できるようにする。  
- **ドキュメント**: `docs/spec/1-3-effects-safety.md` と `docs/spec/3-8-core-runtime-capability.md` に PoC ステージの注記と進捗リンクを付与。
- **実装テスト**: `compiler/ocaml/tests/effect_handler_poc_tests.ml` を新設し、`perform`/`handle` の組み合わせと残余効果履歴が `Σ_before`・`Σ_after` に反映されるか CI で検証する。

## 4. フォローアップ
- 効果操作の本格実装は `EFFECT-003`（Capability 多重処理）と密接に関係するため、タスクを一体管理する。  
- Phase 3 の self-host 移植前に、効果 PoC（ハンドラ 1st クラス）を完成させるマイルストーンを設定し、`0-3-audit-and-metrics.md` へ記録する。  
- `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` に PoC ステージの成果物と検証項目を追記する。
- `docs/notes/effect-system-tracking.md`（未作成なら新規）へ PoC の進捗ログと試験ケース一覧を記録し、Phase 2-7・Phase 3 でのフォローアップを容易にする。
- **タイミング**: 設計と PoC 条件の整理は Phase 2-5 の後半までに完了し、実装着手は Phase 2-7 の効果チームキックオフに合わせて開始する。

## 残課題
- PoC が対象とする効果構文の範囲（`perform` のみか、`resume`/`rethrow` まで含むか）を効果チームと確認したい。  
- PoC をどのリリースチャネルで公開するか、運用上の方針（experimental フラグの命名）を Phase 2-7 と協議する必要がある。
