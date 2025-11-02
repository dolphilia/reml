# 効果構文 PoC トラッキングメモ

## 目的
- Phase 2-5 で効果構文（`perform` / `do` / `handle ... with handler`）を PoC ステージに留める方針と進捗を横断的に記録する。
- パーサ・型推論・診断・CI 指標の各担当へ引き継ぐ観点を整理し、Phase 2-7 での本実装を円滑化する。
- 既存脚注 `[^effects-syntax-poc-phase25]`（`docs/spec/1-1-syntax.md` ほか）と連動させ、読者が PoC 限定事項を追跡できるようにする。

## Stage 現況（2026-03-12 時点）
| コンポーネント | ステージ | 備考 |
| --- | --- | --- |
| 構文仕様 (Chapter 1/1.5) | Experimental | `-Zalgebraic-effects` 有効時のみ使用可能。脚注 `[^effects-syntax-poc-phase25]` で明示済み。 |
| OCaml パーサ | PoC 設計完了 | `parser.mly` へ挿入する規則を設計。実装は SYNTAX-003 S1 → S2 の計画に従って導入予定。 |
| 型・効果解析 | 未着手 | `Type_inference_effect` への接続は SYNTAX-003 S2 で設計予定。 |
| 診断／CI 指標 | 未着手 | `syntax.effect_construct_acceptance` 指標は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追加予定。 |

## Phase 2-5 S1 パーサ PoC 設計サマリ（2026-03-12）
- `expr_base` に `perform_expr`・`do_expr`・`handle_expr` を追加し、Menhir 優先度は既存の制御構文（`if`／`match` 等）と同列に配置する。これにより `perform Eff.op() + x` が従来の二項演算と同じ優先順位で解釈される。（`parser_design.md` 追加節参照）
- `perform` / `do` は共通の AST バリアント `PerformCall`（仮称）で管理し、`do` は `sugar = DoAlias` として区別する。引数リストは既存の `arg_list_opt` を再利用し、`EffectPath ::= Ident { "." Ident }` を `module_path option * ident` に射影するヘルパを導入する計画。
- `handle` 式は `handle_expr := HANDLE expr WITH handler_literal` と定義し、`handler_literal := HANDLER ident handler_body` でトップレベルの `handler` 宣言と AST 構造を共有する。式位置の `handler` では属性・visiblity が無い点を PoC 制限として明記した。
- `parser.conflicts` は既存 31 件を維持する見込み。`HANDLE`/`PERFORM`/`DO` は他規則と先頭トークンが衝突しないため、新たな shift/reduce は発生しない想定。実装時に `menhir --explain` を実行し、差分が出た場合は `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記する手順を定めた。
- フラグ連携: `parser_run_config.ml` の拡張 (`experimental_effects: bool`) を想定し、`-Zalgebraic-effects` が無効な場合はパーサ段階で `effects.syntax.experimental_disabled` 診断を返すガードを追加する。Typer 側の Stage 判定と同じキーを再利用する計画。
- PoC 実装で未対応とする項目: `resume` を複数回呼び出すハンドラ、`handler` リテラルへの属性付与、`do`/`perform` のミックス構文（`do` は完全なエイリアス扱い）。これらは Phase 2-7 で再評価する。

## Phase 2-7 への引き継ぎ TODO
1. `Type_inference_effect` への新 AST バリアント取り込みと `Σ_before` / `Σ_after` 更新規則を追加する（SYNTAX-003 S2 の成果物を参照）。
2. `compiler/ocaml/tests/effect_syntax_tests.ml`（新設予定）で `perform` / `handle` の受理ケースと失敗ケースをゴールデン化し、CI で `syntax.effect_construct_acceptance` を算出する。
3. `reports/diagnostic-format-regression.md` へ効果構文関連の CLI/LSP ゴールデンを追加し、`scripts/validate-diagnostic-json.sh` に `effects.syntax.*` の検証を組み込む。
4. `docs/spec/3-8-core-runtime-capability.md` の Stage テーブルに、効果構文の PoC → 安定化の遷移条件を追記する（脚注 `[^effects-syntax-poc-phase25]` の更新と同期）。

---

作業履歴: 2026-03-12 SYNTAX-003 S1（Parser PoC 設計）で初版作成。更新時は本メモと `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` を同期する。 
