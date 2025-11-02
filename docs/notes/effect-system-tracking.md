# 効果構文 PoC トラッキングメモ

## 目的
- Phase 2-5 で効果構文（`perform` / `do` / `handle ... with handler`）を PoC ステージに留める方針と進捗を横断的に記録する。
- パーサ・型推論・診断・CI 指標の各担当へ引き継ぐ観点を整理し、Phase 2-7 での本実装を円滑化する。
- 既存脚注 `[^effects-syntax-poc-phase25]`（`docs/spec/1-1-syntax.md` ほか）と連動させ、読者が PoC 限定事項を追跡できるようにする。

## Stage 現況（2026-03-27 時点）
| コンポーネント | ステージ | 備考 |
| --- | --- | --- |
| 構文仕様 (Chapter 1/1.5) | Experimental | `-Zalgebraic-effects` 有効時のみ使用可能。脚注 `[^effects-syntax-poc-phase25]` で明示済み。 |
| OCaml パーサ | PoC 設計完了 | `parser.mly` へ挿入する規則を設計。実装は SYNTAX-003 S1 → S2 の計画に従って導入予定。 |
| 型・効果解析 | PoC 設計完了 | `Type_inference_effect` の式対応を SYNTAX-003 S2 で設計。`Σ_before`/`Σ_after` の記録手順を整理済み。 |
| 診断／CI 指標 | 計測計画確定 | `syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` の基準値と PoC サンプル JSON を策定。Phase 2-7 でスクリプト実装・ゴールデン更新を実施予定。 |

## Phase 2-5 S1 パーサ PoC 設計サマリ（2026-03-12）
- `expr_base` に `perform_expr`・`do_expr`・`handle_expr` を追加し、Menhir 優先度は既存の制御構文（`if`／`match` 等）と同列に配置する。これにより `perform Eff.op() + x` が従来の二項演算と同じ優先順位で解釈される。（`parser_design.md` 追加節参照）
- `perform` / `do` は共通の AST バリアント `PerformCall`（仮称）で管理し、`do` は `sugar = DoAlias` として区別する。引数リストは既存の `arg_list_opt` を再利用し、`EffectPath ::= Ident { "." Ident }` を `module_path option * ident` に射影するヘルパを導入する計画。
- `handle` 式は `handle_expr := HANDLE expr WITH handler_literal` と定義し、`handler_literal := HANDLER ident handler_body` でトップレベルの `handler` 宣言と AST 構造を共有する。式位置の `handler` では属性・visiblity が無い点を PoC 制限として明記した。
- `parser.conflicts` は既存 31 件を維持する見込み。`HANDLE`/`PERFORM`/`DO` は他規則と先頭トークンが衝突しないため、新たな shift/reduce は発生しない想定。実装時に `menhir --explain` を実行し、差分が出た場合は `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記する手順を定めた。
- フラグ連携: `parser_run_config.ml` の拡張 (`experimental_effects: bool`) を想定し、`-Zalgebraic-effects` が無効な場合はパーサ段階で `effects.syntax.experimental_disabled` 診断を返すガードを追加する。Typer 側の Stage 判定と同じキーを再利用する計画。
- PoC 実装で未対応とする項目: `resume` を複数回呼び出すハンドラ、`handler` リテラルへの属性付与、`do`/`perform` のミックス構文（`do` は完全なエイリアス扱い）。これらは Phase 2-7 で再評価する。

## Phase 2-5 S2 型・効果解析 PoC サマリ（2026-03-19）
- `Type_inference_effect` の既存 API を確認し、関数単位で Stage を解決する処理のみ実装されていることを把握。PoC では式ノード `TEffectPerform` / `TEffectHandle`（仮称）を Typed AST に追加し、`resolve_expr_profile` が `Σ_before`・`Σ_after`・捕捉タグ一覧を返す設計を採用した。
- PoC の許容範囲は「単一タグ捕捉」「`resume` 1 回」「未宣言タグ禁止」とし、`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-003-proposal.md` に型規則表（perform / handle / handler / @handles）を添付した。`Σ_after = (Σ_before - Σ_handler) ∪ Σ_residual` を Typed AST に保持し、診断拡張へ同値を出力する。
- `compiler/ocaml/tests/test_type_inference.ml` へ効果構文テスト草案コメントを追加。`perform` 成功・`handle` 成功・捕捉漏れ失敗の 3 ケースで診断比較を行う予定を記載し、Phase 2-7 でゴールデン化できるよう準備した。
- CI 指標 `syntax.effect_construct_acceptance` は PoC 期間中 0.0 を許容値、`effects.syntax_poison_rate` は 1.0 を期待値とする。従来の Stage 監査 (`effects.contract.*`) と組み合わせ、PoC では残余効果がゼロになったケースが存在しないことをもって差分を可視化する。

### PoC 記録フォーマット草案
```json
{
  "effect_syntax": {
    "constructs": [
      {
        "kind": "perform",
        "tag": "Console",
        "sigma_before": ["Console"],
        "sigma_after": ["Console"],
        "stage": "Preview",
        "diagnostics": []
      },
      {
        "kind": "handle",
        "tag": "Console",
        "sigma_before": ["Console"],
        "sigma_handler": ["Console"],
        "sigma_after": [],
        "diagnostics": []
      }
    ],
    "metrics": {
      "syntax.effect_construct_acceptance": 0.0,
      "effects.syntax_poison_rate": 1.0
    }
  }
}
```

## Phase 2-5 S3 診断・CI 計測整備（2026-03-27）
- `compiler/ocaml/src/diagnostic.ml`・`parser_diag_state.ml` を調査し、`extensions.effects`／`audit_metadata` に効果構文の構成情報を追加できる構造であることを確認。Info/Hint 指標への影響が無いこと、`Diagnostic.Builder` API で拡張を行えることを検証した。
- CI 集計スクリプト `tooling/ci/collect-iterator-audit-metrics.py` に新指標を追加する設計を整理し、`effect_syntax.constructs` から比率 (`syntax.effect_construct_acceptance` / `effects.syntax_poison_rate`) を計算する案を `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-003-proposal.md` §S3 に記録した。`--require-success` 実行時は 1.0 を必須値とし、逸脱時には CI を失敗させる。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表へ新指標を登録し、PoC 期間の基準値（0.0 / 1.0）と Phase 2-7 でのゲート条件を追記した。`reports/diagnostic-format-regression.md` には CLI/LSP ゴールデン更新時の効果構文チェックリストを追加。
- PoC サンプル JSON を本メモと計画書に添付し、`collect-iterator-audit-metrics.py --section effects` 実装前でもフォーマットを固定できるようにした。サンプルは Phase 2-7 でゴールデン化する際の基準として利用する。

## Phase 2-7 への引き継ぎ TODO
1. `Type_inference_effect` への新 AST バリアント取り込みと `Σ_before` / `Σ_after` 更新規則を追加する（SYNTAX-003 S2 の成果物を参照）。
2. `compiler/ocaml/tests/effect_syntax_tests.ml`（新設予定）で `perform` / `handle` の受理ケースと失敗ケースをゴールデン化し、CI で `syntax.effect_construct_acceptance` を算出する。
3. `reports/diagnostic-format-regression.md` へ効果構文関連の CLI/LSP ゴールデンを追加し、`scripts/validate-diagnostic-json.sh` に `effects.syntax.*` の検証を組み込む。
4. `docs/spec/3-8-core-runtime-capability.md` の Stage テーブルに、効果構文の PoC → 安定化の遷移条件を追記する（脚注 `[^effects-syntax-poc-phase25]` の更新と同期）。

---

作業履歴: 2026-03-12 SYNTAX-003 S1（Parser PoC 設計）で初版作成。2026-03-27 S3（診断・CI 計測整備）で指標計画を追記。更新時は本メモと `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` を同期する。 
