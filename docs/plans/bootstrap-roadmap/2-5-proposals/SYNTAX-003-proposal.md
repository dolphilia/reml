# SYNTAX-003 効果構文の実装ステージ明確化計画

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
- **構文テスト**: `compiler/ocaml/tests/effect_syntax_tests.ml`（新設）に `perform` / `handle` / `do` を組み合わせた最小例・入れ子例・`resume` 付きハンドラを追加し、`make test_parser` と `menhir --list-errors compiler/ocaml/src/parser.mly` の衝突結果を確認する。  
- **効果解析**: EFFECT-002 / EFFECT-003 と連携し、`Type_inference_effect`・`effect_analysis.ml` の `Σ_before` / `Σ_after` 記録が PoC でも追跡できるかを `compiler/ocaml/tests/test_type_inference.ml`・`compiler/ocaml/tests/streaming_runner_tests.ml` を用いて検証。  
- **ドキュメント**: Chapter 1（docs/spec/1-1-syntax.md §B.5）・Chapter 1.5（docs/spec/1-5-formal-grammar-bnf.md）・Chapter 3.8（docs/spec/3-8-core-runtime-capability.md）へ PoC 脚注を同期し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分リストから逆引きできるよう脚注 ID を登録する。  
- **メトリクス**: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` を追記し、`tooling/ci/collect-iterator-audit-metrics.py` で Experimental 値（PoC 期間は 0.0 許容、正式導入で 1.0 必須）を集計する。

## 4. フォローアップ
- 効果構文を実装する際は `Type_inference_effect` との統合が必須であり、Phase 2-2 の効果整合計画と同じレビュー体制を取る。  
- PoC 実装が完成した段階で脚注を解除し、Phase 3 の self-host 計画書へ対応状況を反映する。  
- CLI / LSP の効果診断（`effects.contract.*`）が効果構文出力と整合するよう、`reports/diagnostic-format-regression.md` のテスト更新を予定する。
- `docs/notes/effect-system-tracking.md` に構文受理状況と `-Zalgebraic-effects` フラグの運用メモを残し、PoC と正式導入の境界条件を共有する。
- **タイミング**: Phase 2-5 では早期に脚注・PoC 設計を整備し、効果構文の実装と公開は EFFECT-002 と同期して Phase 2-7 の効果チーム着手時に実行する。

## 5. 実施ステップと調査計画（Phase 2-5 内）

| ステップ | 目的と完了条件 | 主な調査項目 | 成果物 |
|----------|----------------|--------------|--------|
| **S0: ステージ定義の再確認（週31）** | Phase 2-5 時点で効果構文が PoC に留まることを仕様・計画書に明示し、`-Zalgebraic-effects` を Stage 判定に紐付ける。`docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/3-8-core-runtime-capability.md` に暫定脚注を追加し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分リストへ ID を登録済みであること。 | - Chapter 1 B.5 の既存脚注と Stage テーブルの整合確認<br>- `docs/plans/bootstrap-roadmap/2-5-review-log.md` の SYNTAX 系エントリに脚注 ID を追加済みか確認<br>- `docs/spec/README.md` の索引導線をレビュー | - 本計画書の「S0」節記録<br>- 仕様脚注 ID（仮: `[^effects-syntax-poc-phase25]`）<br>- 2-5 差分リスト更新メモ |
| **S1: パーサ PoC 設計（週31-32）** | `parser.mly` に `perform_expr` / `handle_expr` の挿入位置と優先順位を設計し、Menhir 衝突と `parser.conflicts` の増減を調査。`parser_design.md` へ解析結果をフィードバックし、PoC で許容する構文制限（例: `resume` の未実装扱い）を明文化する。 | - `parser.mly` の式優先順位表と `HandleExpr` 付近の `%prec` 指定<br>- `compiler/ocaml/docs/parser_design.md` の効果構文欄<br>- `effect-system-design-note.md` の AST ノード構成 | - `parser.conflicts` の更新案と差分コメント<br>- `docs/notes/effect-system-tracking.md` に PoC 仕様メモ<br>- `EFFECT-002` 共有用の parser PoC TODO |
| **S2: 型・効果解析の PoC 接続（週33）** | `Type_inference_effect` が `perform` / `handle` を受理できる最低限のハンドラ規則と `Σ_before` 記録を導入する設計案をまとめる。`test_type_inference.ml` の PoC ケースで失敗位置と診断を可視化し、`EFFECT-002` へ同期。 | - `compiler/ocaml/src/type_inference_effect.ml`（仮）と `effect_analysis.ml` の現状把握<br>- `docs/spec/1-3-effects-safety.md` §G～I の規則<br>- `reports/diagnostic-format-regression.md` に登録済みの `effects.contract.*` ケース | - PoC で通過させる型規則の表（本計画書添付）<br>- `compiler/ocaml/tests/test_type_inference.ml` の新規セクション草案<br>- `docs/plans/bootstrap-roadmap/2-5-review-log.md` への経過記録 |
| **S3: 診断・CI 計測整備（週33-34）** | テキスト診断と JSON 監査に効果構文関連のキーを追加する計画を立案。`tooling/ci/collect-iterator-audit-metrics.py` に `syntax.effect_construct_acceptance` を追加するための入力仕様とエビデンスを整理し、`reports/diagnostic-format-regression.md` のゴールデン改修方針をまとめる。 | - `compiler/ocaml/src/diagnostic.ml`・`parser_diag_state.ml` の拡張ポイント<br>- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 体系<br>- `DIAG-002` / `DIAG-003` 計画とのリンケージ | - CI 指標追加用の YAML/JSON サンプル<br>- CLI/LSP ゴールデン更新手順書（下書き）<br>- `diagnostic.info_hint_ratio` との整合確認メモ |
| **S4: Phase 2-7 への引き継ぎ準備（週34）** | PoC の成果物と未解決事項を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`・`docs/notes/effect-system-tracking.md` に連携し、Phase 2-7 効果チームが着手できるよう段階表とリスクを整理する。`-Zalgebraic-effects` フラグ運用（CLI/LSP/ビルド）の影響を洗い出す。 | - Phase 2-7 計画書の効果セクション<br>- `docs/notes/dsl-plugin-roadmap.md` の Stage 連携項目<br>- `tooling/ci/` 内の実験フラグ制御スクリプト | - 引き継ぎチェックリスト（本計画書貼付）<br>- 2-7 計画へのリンク追加<br>- CLI オプション仕様への TODO |

> 各ステップ終了時には `docs/plans/bootstrap-roadmap/2-5-review-log.md` へ検証ログを追加し、脚注 ID と CI 指標値（達成した場合でも 0.0 → 1.0 の推移を記録）を残す。未完了タスクは `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に転記する。

> Phase 2-5 Week31 更新: S0 を完了し、`docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/3-8-core-runtime-capability.md` に脚注 `[^effects-syntax-poc-phase25]` を追加。`docs/spec/README.md` と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` へも同脚注 ID を登録し、PoC ステージと `-Zalgebraic-effects` 依存を明示した。レビュー記録は `docs/plans/bootstrap-roadmap/2-5-review-log.md` SYNTAX-003 セクションに追記。

## 6. 進捗記録（Phase 2-5）
- 2026-03-12: **S1 パーサ PoC 設計完了**。`compiler/ocaml/docs/parser_design.md` §3.3.1 に挿入位置・優先順位・PoC 制限を反映し、`parser_run_config` への実験フラグ導入方針を確定。`compiler/ocaml/docs/effect-system-design-note.md` にモジュール間連携を追記し、`docs/notes/effect-system-tracking.md` を新設して PoC ステージ・引き継ぎ TODO を整理した。レビュー記録は `docs/plans/bootstrap-roadmap/2-5-review-log.md` 2026-03-12 項目を参照。

## 残課題
- 効果構文を有効化するフラグ名（`-Zalgebraic-effects` など）と公開ポリシーを Phase 2-7 と調整する必要がある。  
- `perform` などの構文追加が既存優先順位に与える影響（Menhir の衝突、`parser.conflicts` 更新）を事前にレビューしたい。  
- Phase 2-7 へ渡す引き継ぎ資料（PoC 到達条件・CI 設定・脚注削除手順）を `S4` 完了時までに確定させる。

[^effects-syntax-poc-phase25]:
    Phase 2-5 Week31 時点の方針。効果構文は `-Zalgebraic-effects` フラグを必須とする Experimental Stage に留め、正式実装は Phase 2-7 で `parser.mly`・型推論・効果解析を統合した後に進める。紐付く脚注は `docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/3-8-core-runtime-capability.md` に同期済みで、差分ログは `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と `docs/plans/bootstrap-roadmap/2-5-review-log.md` を参照。
