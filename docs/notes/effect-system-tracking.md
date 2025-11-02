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

## Phase 2-5 Step1 スコープ確定メモ（2026-04-08）

### 調査サマリ
- 仕様側では `perform` / `handle` を Experimental ステージと定め、残余効果計算 `Σ_after = (Σ_before - Σ_handler) ∪ Σ_residual` と `@handles` 契約を必須としている（docs/spec/1-3-effects-safety.md:240, docs/spec/1-3-effects-safety.md:244）。`Σ_after = ∅` を満たすハンドラは純粋値として扱えるため、PoC ではこのパターンを再現できるテストを優先する。
- 効果構文は Capability Registry と Stage 契約が結び付いており、`-Zalgebraic-effects` が無効な環境では公開しないことが要求される（docs/spec/3-8-core-runtime-capability.md:1）。
- 設計メモには式バリアント名と `parser_run_config.experimental_effects` 拡張案が記録されているが、現状の実装は反映前である（compiler/ocaml/docs/effect-system-design-note.md:200）。

### PoC スコープ表
| 区分 | 仕様で許容される構文・契約 | Phase 2-5 PoC での取り扱い | 備考 |
| --- | --- | --- | --- |
| 効果発火 | `perform Effect.op(args)` / `do Effect.op(args)`（Stage=Experimental、`Σ_before` にタグ追加） | 構文・AST・型付けすべて未実装。Step2 で AST/Parser を PoC 追加する。 | Parser トークンは定義済みだが規則が未導入。 |
| ハンドラ | `handle expr with handler` / `handler name { case .. }`（`Σ_handler` 捕捉と `Σ_residual` 合成） | 未実装。PoC では単一タグ捕捉と `resume` 1 回までをサポート対象に想定。 | Stage と Capability の検証は既存 API を流用。 |
| 契約属性 | `@handles`, `@requires_capability(stage=...)`, `allows_effects` | 属性解析は既存機構を再利用。`Σ_after ⊆ allows_effects` の検証ロジックが欠落。 | 脚注 `[^effects-syntax-poc-phase25]` を維持。 |
| 診断 | `effects.contract.mismatch`, `effects.syntax.experimental_disabled` | 現状は実験機能無効のモック診断のみ。Step4 で JSON 拡張とメトリクス送出を整備。 | `Diagnostic.extensions["effects"]` は配列対応済み。 |
| CI 指標 | `syntax.effect_construct_acceptance`, `effects.syntax_poison_rate` | 基準値 0.0 / 1.0 を棚卸しで確定。実値集計は Phase 2-7 が担当。 | `tooling/ci/collect-iterator-audit-metrics.py` へ追加予定。 |

### 実装差分棚卸
- AST/Typed AST に `Perform` / `Handle` ノードが存在せず、式バリアントは Phase 1 時点の構成に留まっている（compiler/ocaml/src/ast.ml:95, compiler/ocaml/src/typed_ast.ml:118）。
- パーサは `PERFORM` / `HANDLE` トークンを列挙しているものの、`primary_expr`/`postfix_expr` に対応規則が無く効果構文を受理できない（compiler/ocaml/src/parser.mly:668, compiler/ocaml/src/parser.mly:1200）。
- `parser_run_config` には `experimental_effects` フラグが存在せず、CLI の `-Zalgebraic-effects` を無効化した際にエラーへ反映できない（compiler/ocaml/src/parser_run_config.ml:76）。
- 効果解析は関数呼び出しベースのタグ収集に限定され、`perform`/`handle` の残余効果計算が未定義（compiler/ocaml/src/type_inference.ml:180）。
- `Type_inference_effect` は Stage 判定と Capability 解決に特化しており、式レベルの `Σ_before` / `Σ_after` を受け取る API が無い（compiler/ocaml/src/type_inference_effect.ml:1）。

### メトリクス基準値
- `syntax.effect_construct_acceptance`: Phase 2-5 では 0.0 を維持し、実装未着手であることを数値で明示する。
- `effects.syntax_poison_rate`: 1.0 を基準とし、捕捉に失敗する構文しか存在しない現状を示す。
- 追加指標 `effects.contract.residual_snapshot`（仮称）を Step4 で仕様化し、`Σ_before` / `Σ_after` の差分を監査ログへ反映する。

### 引き継ぎメモ
- Step2 は AST/Parser の PoC 実装と Menhir コンフリクト差分の記録を最優先とする。
- Step3 以降は本棚卸しで確定したスコープとメトリクスを前提に進め、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の H-O1/H-O4 と連動して監査整合を維持する。

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
  "diagnostics": [
    {
      "extensions": {
        "effects": {
          "sigma": {
            "before": ["console"],
            "handler": ["console"],
            "residual": ["console"],
            "after": ["console"]
          },
          "constructs": [
            {
              "kind": "perform",
              "tag": "Console",
              "span": { "start": { "line": 4, "column": 3 }, "end": { "line": 4, "column": 25 } },
              "handled_by": null,
              "diagnostics": []
            },
            {
              "kind": "handle",
              "tag": "Console",
              "span": { "start": { "line": 6, "column": 1 }, "end": { "line": 10, "column": 4 } },
              "handled_by": "app.handler.console",
              "diagnostics": []
            }
          ]
        }
      },
      "audit": {
        "metadata": {
          "effect": {
            "sigma": {
              "before": ["console"],
              "handler": ["console"],
              "residual": ["console"],
              "after": ["console"]
            },
            "syntax": {
              "constructs": {
                "total": 2,
                "accepted": 0,
                "poisoned": 2,
                "residual_tags": ["console"]
              }
            }
          }
        }
      }
    }
  ]
}
```

## Phase 2-5 S3 診断・CI 計測整備（2026-03-27）
- `compiler/ocaml/src/diagnostic.ml`・`parser_diag_state.ml` を調査し、`extensions.effects`／`audit_metadata` に効果構文の構成情報を追加できる構造であることを確認。Info/Hint 指標への影響が無いこと、`Diagnostic.Builder` API で拡張を行えることを検証した。2026-04-18 追記: `sigma.before` / `sigma.handler` / `sigma.residual` / `sigma.after` および `constructs` 配列を追加し、`set_extension` と `set_audit_metadata` のキーをドット区切りで揃える設計を確定。
- CI 集計スクリプト `tooling/ci/collect-iterator-audit-metrics.py` に新指標を追加する設計を整理し、`effect_syntax.constructs` から比率 (`syntax.effect_construct_acceptance` / `effects.syntax_poison_rate`) を計算する案を `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-003-proposal.md` §S3 に記録した。2026-04-18 追記: `iter_effect_constructs`（仮称）で `constructs` を巡回し、`Σ_after = ∅` 判定と `effects.contract.*` 診断を突き合わせる実装メモを追加。`--require-success` 実行時の閾値は PoC 期間 (0.0 / 1.0)、本実装 (1.0 / 0.0) の二段階運用とする。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表へ新指標を登録し、PoC 期間の基準値（0.0 / 1.0）と Phase 2-7 でのゲート条件を追記した。`reports/diagnostic-format-regression.md` には CLI/LSP ゴールデン更新時の効果構文チェックリストを追加し、効果専用フィクスチャを参照する手順を整備。
- PoC サンプル JSON を本メモと計画書に添付し、`collect-iterator-audit-metrics.py --section effects` 実装前でもフォーマットを固定できるようにした。サンプルは Phase 2-7 でゴールデン化する際の基準として利用する。

### 2026-04-18: σ 記録と KPI 詳細
| 出力先 | キー | 内容 | 備考 |
| --- | --- | --- | --- |
| `extensions.effects.sigma.before` | 配列 | `perform` / `do` 解析前に観測した潜在効果集合 `Σ_before` | タグ名は小文字へ正規化 |
| `extensions.effects.sigma.handler` | 配列 | ハンドラ宣言が捕捉する集合 `Σ_handler` | `@handles` 属性と同期 |
| `extensions.effects.sigma.residual` | 配列 | ハンドラ本体で追加されたタグ `Σ_residual` | `effect.contract.residual_snapshot` と連携 |
| `extensions.effects.sigma.after` | 配列 | ハンドラ適用後の残余集合 `Σ_after` | 空集合か否かで accept/poison を判定 |
| `extensions.effects.constructs` | 配列 | 各 `perform` / `handle` の詳細 (`kind`・`tag`・`span`・`handled_by`・`diagnostics`) | CI 集計とデバッグに利用 |
| `audit.metadata.effect.sigma.*` | 配列 | `sigma` 各集合のミラー | 監査ダッシュボードで差分確認 |
| `audit.metadata.effect.syntax.constructs.total` | 整数 | 効果構文の総数 | `total > 0` で指標算出 |
| `audit.metadata.effect.syntax.constructs.accepted` | 整数 | `Σ_after = ∅` の件数 | PoC 期間は 0 を想定 |
| `audit.metadata.effect.syntax.constructs.poisoned` | 整数 | `Σ_after ≠ ∅` の件数 | PoC 期間は total と同値 |
| `audit.metadata.effect.syntax.constructs.residual_tags` | 配列 | 捕捉できなかったタグ一覧 | 監査ダッシュボード表示用 |

- KPI は `syntax.effect_construct_acceptance = accepted / total`、`effects.syntax_poison_rate = poisoned / total`。PoC（Phase 2-5）は (0.0 / 1.0) を許容値、本実装（Phase 2-7 以降）は (1.0 / 0.0) を要求値とし、`collect-iterator-audit-metrics.py --require-success` が閾値逸脱時に CI を失敗させる。
- `scripts/validate-diagnostic-json.sh` へ効果構文用バリデータを追加し、`extensions.effects.sigma` の配列構造、`constructs[*].kind` の列挙（`perform`/`handle`）、`Σ_after` と `effect.syntax.constructs.accepted` の一致を検証する。検証対象ゴールデンは `compiler/ocaml/tests/golden/diagnostics/effect-handler-poc.json.golden`（仮称）で管理する。

## Phase 2-5 S4 引き継ぎパッケージ（2026-04-03）
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に効果構文 PoC 移行タスクを追加し、監査ゲート（`syntax.effect_construct_acceptance` / `effects.syntax_poison_rate`）を 1.0 へ引き上げる条件とエスカレーション経路（`0-4-risk-handling.md`）を同期した。
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分リストへ S4 完了メモを追記し、脚注 `[^effects-syntax-poc-phase25]` の撤去条件と Stage 遷移シナリオ（PoC → Preview → Stable）を明文化した。
- CLI/LSP/ビルドの実験フラグ運用を棚卸しし、`-Zalgebraic-effects` 仮称の扱いを Phase 2-7 で統一するための TODO を整理。CLI は `compiler/ocaml/src/cli/options.ml`、LSP は `tooling/lsp/tests/client_compat/fixtures/`, ビルドは `scripts/` 配下の CI スクリプトを管理対象と定義した。
- プラグイン連携の監査ポイントを `docs/notes/dsl-plugin-roadmap.md` に転記し、Capability Stage とハンドラ捕捉条件を Phase 2-7 の Stage レビューに統合する手順を記した。

## Phase 2-7 への引き継ぎ TODO
| ID | タスク | 成功条件 | 参照 |
| --- | --- | --- | --- |
| H-O1 | `Type_inference_effect` に `TEffectPerform` / `TEffectHandle`（仮称）を実装し、`Σ_before` / `Σ_after` の計算を導入する。 | PoC サンプルで残余効果が正しく除去され、`effects.contract.residual` が期待通り発火する。 | SYNTAX-003 S2 セクション／Phase 2-7 計画 §3 |
| H-O2 | `compiler/ocaml/tests/effect_syntax_tests.ml` を新設し、`perform` / `handle` の成功・失敗パターンをゴールデン化する。 | CI で `syntax.effect_construct_acceptance = 1.0`、`effects.syntax_poison_rate = 0.0` を達成し、`--require-success` が通過する。 | SYNTAX-003 S3／`collect-iterator-audit-metrics.py` 実装メモ |
| H-O3 | `-Zalgebraic-effects` フラグを CLI・LSP・CI で一貫制御し、公開名称とドキュメント更新手順を確定する。 | CLI/LSP/CI すべてでフラグの既定値・警告文が一致し、仕様書（0-0/1-1/1-5/3-8）から脚注撤去が可能。 | SYNTAX-003 S4 チェックリスト／Phase 2-7 計画 §2 |
| H-O4 | `docs/notes/dsl-plugin-roadmap.md` と Stage テーブルを更新し、効果ハンドラ登録と Capability Stage の整合を監査できるようにする。 | `effects.contract.stage_mismatch` / `bridge.stage.*` 診断が効果構文を通じて再現でき、監査ログに Stage 情報が揃う。 | SYNTAX-003 S4／`docs/spec/3-8-core-runtime-capability.md` |
| H-O5 | 脚注 `[^effects-syntax-poc-phase25]` の撤去条件を満たした時点で仕様・索引を更新し、Phase 2-8 へ報告する。 | Stage = Stable に昇格した週のレビューで脚注撤去を承認し、`2-5-spec-drift-remediation.md` へ完了記録を残す。 | SYNTAX-003 S4／`2-5-spec-drift-remediation.md` 差分リスト |

---

作業履歴: 2026-03-12 SYNTAX-003 S1（Parser PoC 設計）で初版作成。2026-03-27 S3（診断・CI 計測整備）で指標計画を追記。2026-04-03 S4（Phase 2-7 引き継ぎ準備）でハンドオーバー チェックリストとフラグ運用メモを整備。2026-04-12 S2（AST/Parser PoC 実装）で構文ノード・RunConfig フラグ・診断ガードを実装済みとして追記。更新時は本メモと `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` を同期する。 
