# SYNTAX-003 効果構文の実装ステージ明確化提案

## 1. 背景と症状
- Chapter 1 では `effect` 宣言、`perform` / `do` 呼び出し、`handle ... with handler` 構文を定義し、Formal BNF でも同様の規則を記載している（docs/spec/1-1-syntax.md:180-226, docs/spec/1-5-formal-grammar-bnf.md）。  
- 現行 `parser.mly` には `PERFORM` / `HANDLE` に対応する生成規則が存在せず、OCaml 実装で効果構文を受理できない。`handler` 宣言は定義済みだが、式位置での `handle` / `perform` を解析する経路が欠落している。  
- 効果構文を利用するサンプル（効果 PoC）やガイドが実装で再現できず、効果システムの PoC 進行と仕様整合が取れていない。

## 2. Before / After
### Before
- `effect` 宣言はトークンのみ定義済みで具体的な構文規則が未実装。  
- `perform` / `handle` を含むソースは構文エラーとなり、EFFECT-002 / EFFECT-003 などの差分評価が進められない。

### After
- 効果構文を「PoC ステージ」と位置付け、仕様本文と Formal BNF に「Phase 2 では `-Zalgebraic-effects` で有効化する暫定機能」と脚注追加。  
- `parser.mly` に `perform_expr` / `handle_expr` 規則を追加する計画を立て、Phase 2-2 / Phase 2-7 効果チームへ実装タスクを連携。  
- OCaml 実装が PoC の範囲内で効果構文を受理できるようになるまで、仕様側に暫定制限を明記して差分を可視化する。

## 3. 影響範囲と検証
- **構文テスト**: `compiler/ocaml/tests/effect_syntax_tests.ml`（新設）で `perform` / `handle` のサンプルを追加し、PoC 実装が解析できるか確認。  
- **効果解析**: EFFECT-002 / EFFECT-003 と連携し、ハンドラ適用後の残余効果計算が可能になるか検証。  
- **ドキュメント**: Chapter 1 と 3 の関係箇所、および `docs/spec/1-5-formal-grammar-bnf.md` に PoC 脚注を追記し、整合性を維持。
- **メトリクス**: `0-3-audit-and-metrics.md` に `syntax.effect_construct_acceptance` を追加し、PoC 期間中は Experimental として計測、正式導入時に PASS 判定へ更新する。

## 4. フォローアップ
- 効果構文を実装する際は `Type_inference_effect` との統合が必須であり、Phase 2-2 の効果整合計画と同じレビュー体制を取る。  
- PoC 実装が完成した段階で脚注を解除し、Phase 3 の self-host 計画書へ対応状況を反映する。  
- CLI / LSP の効果診断（`effects.contract.*`）が効果構文出力と整合するよう、`reports/diagnostic-format-regression.md` のテスト更新を予定する。
- `docs/notes/effect-system-tracking.md` に構文受理状況と `-Zalgebraic-effects` フラグの運用メモを残し、PoC と正式導入の境界条件を共有する。

## 確認事項
- 効果構文を有効化するフラグ名（`-Zalgebraic-effects` など）と公開ポリシーを Phase 2-7 と調整する必要がある。  
- `perform` などの構文追加が既存優先順位に与える影響（Menhir の衝突、`parser.conflicts` 更新）を事前にレビューしたい。
