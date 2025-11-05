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

## Phase 2-5 Step5 ドキュメント整合（2026-04-20）
- Chapter 1（`docs/spec/1-1-syntax.md`・`docs/spec/1-3-effects-safety.md`・`docs/spec/1-5-formal-grammar-bnf.md`）へ `Σ_before`/`Σ_after` 記録と PoC 指標 (`syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`) の参照脚注を追加し、Step4 で合意した JSON/監査設計と整合させた。
- 概要 (`docs/spec/0-0-overview.md`) と索引 (`docs/spec/README.md`) を更新し、効果構文が Experimental Stage であることと KPI 参照ルート（`docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-002-proposal.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`）を明示した。
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`・`docs/plans/bootstrap-roadmap/2-5-review-log.md` に Step5 完了メモを記録し、残課題を Phase 2-7 ハンドラ実装へ転送。`0-4-risk-handling.md` には Stage 昇格遅延リスク（ID: EFFECT-POC-Stage）を登録済み。

## Phase 2-7 への引き継ぎ TODO
| ID | タスク | 成功条件 | 参照 |
| --- | --- | --- | --- |
| H-O1 | `Type_inference_effect` に `TEffectPerform` / `TEffectHandle`（仮称）を実装し、`Σ_before` / `Σ_after` の計算を導入する。 | PoC サンプルで残余効果が正しく除去され、`effects.contract.residual` が期待通り発火する。 | SYNTAX-003 S2 セクション／Phase 2-7 計画 §3 |
| H-O2 | `compiler/ocaml/tests/effect_syntax_tests.ml` を新設し、`perform` / `handle` の成功・失敗パターンをゴールデン化する。 | CI で `syntax.effect_construct_acceptance = 1.0`、`effects.syntax_poison_rate = 0.0` を達成し、`--require-success` が通過する。 | SYNTAX-003 S3／`collect-iterator-audit-metrics.py` 実装メモ |
| H-O3 | `-Zalgebraic-effects` フラグを CLI・LSP・CI で一貫制御し、公開名称とドキュメント更新手順を確定する。 | CLI/LSP/CI すべてでフラグの既定値・警告文が一致し、仕様書（0-0/1-1/1-5/3-8）から脚注撤去が可能。 | SYNTAX-003 S4 チェックリスト／Phase 2-7 計画 §2 |
| H-O4 | `docs/notes/dsl-plugin-roadmap.md` と Stage テーブルを更新し、効果ハンドラ登録と Capability Stage の整合を監査できるようにする。 | `effects.contract.stage_mismatch` / `bridge.stage.*` 診断が効果構文を通じて再現でき、監査ログに Stage 情報が揃う。 | SYNTAX-003 S4／`docs/spec/3-8-core-runtime-capability.md` |
| H-O5 | 脚注 `[^effects-syntax-poc-phase25]` の撤去条件を満たした時点で仕様・索引を更新し、Phase 2-8 へ報告する。 | Stage = Stable に昇格した週のレビューで脚注撤去を承認し、`2-5-spec-drift-remediation.md` へ完了記録を残す。 | SYNTAX-003 S4／`2-5-spec-drift-remediation.md` 差分リスト |

### 2026-12-07 H-O3 フラグ運用棚卸
- `compiler/ocaml/src/cli/options.ml` に `-Zalgebraic-effects` / `--experimental-effects` フラグが未定義であり、`Options.to_run_config` も `Parser_run_config.set_experimental_effects` を呼び出していないことを確認。CLI 実行時に `Parser_flags.set_experimental_effects_enabled` が常に `false` で終わるため、PoC 構文はテスト専用の `Run_config` からしか利用できない状態。
- LSP 実装では `tooling/lsp/tests/client_compat/fixtures/` の RunConfig JSON に `experimental_effects` キーが存在せず、`diagnostic_transport.ml` も CLI と同じフラグを解釈していない。LSP 経由で PoC を行うと `effects.syntax.experimental_disabled` が発火し続けるため、Phase 2-7 で交渉メッセージへフラグを追加する必要がある。
- CI/スクリプト側では `scripts/validate-diagnostic-json.sh`・`tooling/ci/collect-iterator-audit-metrics.py` がフラグを渡しておらず、効果構文ゴールデンや KPI 計測を手動で回す際にオプション指定漏れが発生し得る。PoC ゴールデン生成スクリプトを新設し、CI では環境変数 `REML_ENABLE_EFFECT_POC`（仮称）で統一的に有効化する案を検討。
- ドキュメントは脚注 `[^effects-syntax-poc-phase25]` の記述が最新だが、CLI/LSP 操作ガイドにフラグの導線が無いため、Stage 昇格判定時にユーザー操作が再現できないリスクが残る。`docs/guides/cli-workflow.md` と `docs/notes/dsl-plugin-roadmap.md` にフラグ利用手順を追加するタスクを `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §8.2 へ追記した。
- 次アクション: Phase 2-7 Sprint A で CLI → LSP → CI → ドキュメントの順に対応するワークフロー案を確定し、完了後は H-O3 の成功条件（CLI/LSP/CI の文言・既定値統一、脚注撤去準備）をレビューでチェックする。

### 2026-12-12 H-O1〜H-O5 進捗レビュー
- **H-O1 (完了)**: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §8.1 で PoC 実装の統合が完了し、`Type_inference_effect` に `TEffectPerform` / `TEffectHandle` を導入して `Σ_before` / `Σ_after` の差分が診断へ伝播することを確認済み。`effects.contract.residual` のエラーパスも `effect_syntax_tests.ml` で再現できるため、Stage レビューでは residual 判定を重点確認項目から外した。
- **H-O2 (完了)**: `compiler/ocaml/tests/effect_syntax_tests.ml` のゴールデンと `collect-iterator-audit-metrics.py --section effects --require-success` のゲートが稼働しており、CI で `syntax.effect_construct_acceptance = 1.0` / `effects.syntax_poison_rate = 0.0` を維持している。計測結果は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` 2026-12-12 追記に記録した。
- **H-O3 (対応中)**: フラグ運用は未整備のままのため、CLI/LSP/CI への伝播タスクを Phase 2-7 Sprint A backlog に固定した。完了までは効果構文レビューで `experimental_effects` を手動設定する暫定手順を維持する。
- **H-O4 (未着手)**: `docs/notes/dsl-plugin-roadmap.md` と Stage テーブルは Phase 2-5 時点から更新が無い。効果ハンドラ監査のサンプルと `bridge.stage.*` 診断の連携を Phase 2-7 Sprint B のレビュー議題に追加し、Stage ミスマッチ検証を整備する。
- **H-O5 (未達)**: 脚注 `[^effects-syntax-poc-phase25]` の撤去条件（Stage = Stable、フラグ導線整備、CI 監査の 1.0 維持）は揃っていない。H-O3/H-O4 の完了後に再レビューし、脚注運用の棚卸し結果を `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` 2026-12-12 追記へ反映する。
- **フォローアップ**: 週次レビューで本メモを参照し、H-O3〜H-O5 の進捗更新を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §8.3 の進捗欄と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の記録に同期させる。

---

## Phase 2-5 TYPE-002 Step1 効果行統合棚卸（2026-04-10）

### 調査サマリ
- 仕様は関数シグネチャに効果行を組み込み、`fn A -> B ! {io, panic}` の形式で型と効果を一体管理することを前提としている（docs/spec/1-2-types-Inference.md:158, docs/spec/1-3-effects-safety.md:244）。残余効果 `Σ_after` の検査と `@handles` 契約はこの前提に基づく。
- OCaml 実装の型表現 `ty` は `TArrow of ty * ty` のままで効果情報を保持せず（compiler/ocaml/src/types.ml:48）、効果集合は `typed_fn_decl.tfn_effect_profile` に分離管理されている（compiler/ocaml/src/typed_ast.ml:167）。`Type_inference` でも `fn_ty` 組み立て時に効果を落としており（compiler/ocaml/src/type_inference.ml:2691）、プロファイルは補助メタデータとして別フィールドに記録される（compiler/ocaml/src/type_inference.ml:2712）。
- `Effect_analysis.collect_from_fn_body` で収集したタグは診断・監査向けの `profile` へ統合されるが、型スキーム `scheme.body` へは伝播せず、`generalize` / `instantiate` の経路では効果差分を比較できない（compiler/ocaml/src/type_inference.ml:2698）。

### 乖離ポイントと影響
- **型等価・ジェネリクス**: `TArrow` に効果行が無いため、`∀ε. τ ! ε` のような行多相スキームを表現できず、`@handles` や `@pure` が期待する「型と効果の同時比較」が不可能。`TYPE-002` の目的は `ty` へ効果行（候補: `effect_row`) を追加し、`generalize` / `instantiate` / `Type_unification` で効果集合を処理できる状態へ移行すること。
- **契約検査の一貫性**: 現状は `typed_fn_decl.tfn_effect_profile` を参照して `effects.contract.*` 診断を出しているが、型アノテーション (`fn foo() -> Result<_, _> ! {panic}`) との比較が `type_inference` 内で行えない。`TYPE-002-S1` では `docs/spec/1-2-types-Inference.md` と `docs/spec/1-3-effects-safety.md` に脚注（予定）を追加し、「Phase 2-5 時点では効果行が型スキームへ統合されていない」ことを明示する必要がある。
- **Stage/Capability 連携**: `record_effect_profile` が Capability Stage の整合に利用されるが（compiler/ocaml/src/type_inference.ml:2723）、型情報として残らないため、`StageRequirement` の推論や `RuntimeBridge` との突き合わせを型段階で実行できない。行統合後は `Stage` と効果タグを型システムの比較対象に含める計画。

### フォローアップ（Step2 以降への入力）
1. `compiler/ocaml/docs/effect-system-design-note.md` に効果行統合案（`TArrow of ty * effect_row * ty`）をドラフト化し、`effect_row` のデータ構造比較（リスト/ビットセット/RowVar）を記録する。
2. `docs/plans/bootstrap-roadmap/2-5-review-log.md` に `TYPE-002-S1` エントリを追加し、脚注追加候補と検証観点（`generalize` / `instantiate` / `Type_unification` / `Effect_analysis`) をタグ付きで追跡する。
3. Phase 2-7 着手条件として、`generalize` / `instantiate` / `solve_trait_constraints` への改修順序とテスト観点（`type_effect_row_*` シリーズ、`diagnostics.effect_row_stage_consistency` 新設案）を Step4 で確定させる。

更新時は本メモと `docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md` を同期し、脚注追加・実装スケジュールの差分が発生した場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に TODO を登録する。

作業履歴: 2026-03-12 SYNTAX-003 S1（Parser PoC 設計）で初版作成。2026-03-27 S3（診断・CI 計測整備）で指標計画を追記。2026-04-03 S4（Phase 2-7 引き継ぎ準備）でハンドオーバー チェックリストとフラグ運用メモを整備。2026-04-12 S2（AST/Parser PoC 実装）で構文ノード・RunConfig フラグ・診断ガードを実装済みとして追記。更新時は本メモと `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` を同期する。 


## Phase 2-5 TYPE-002 Step2 型表現統合ドラフト（2026-04-18）

### サマリ
- `Effect_analysis.collect_from_fn_body` のタグ列と `typed_fn_decl.tfn_effect_profile` の転記フローを再点検し、効果タグ正規化（`normalize_effect_name`）を型統合後も共有できることを確認。
- `effect_row` 候補（`string list` / `StringSet.t` / `row_var`）を比較し、集合演算と表示順序を両立させるため「表示用配列 + 正規化集合」の二層構造を暫定採用。`row_var` は Phase 2-7 で `Constraint_solver` 拡張と同時導入する前提で `None` 固定とした。
- 設計ノート `compiler/ocaml/docs/effect-system-design-note.md` に `TArrow of ty * effect_row * ty` 案・影響モジュール一覧・`Effect_analysis → effect_row → 型スキーム → 診断／監査` のデータフロー図を追記し、`TYPE-002-S2` 差分タグをレビュー記録へ登録。

### フォローアップ TODO
1. `Constraint_solver` への RowVar 対応を Phase 2-7 `TYPE-002` 実装タスクへ連携するため、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に「effect_row 行多相 PoC」を追加（Step4 設計で反映予定）。
2. `diagnostics.effect_row_stage_consistency` KPI の JSON キー案（`extensions.effects.row.declared`, `...residual`, `audit.metadata.effect.row.canonical`）を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ草案登録し、Step4 で正式指標として追加する。
3. Chapter 1-2 / 1-3 へ追加する脚注案を整備し、「Phase 2-5 は診断メタデータ運用」「Phase 2-7 で型統合予定」「解除条件＝RowVar 対応と KPI 達成」を明記するテンプレートを Step3 作業のインプットとして準備する。

### 参考リンク
- 設計ノート: `compiler/ocaml/docs/effect-system-design-note.md`「## 3. 型表現統合ドラフト（TYPE-002 Step2, 2026-04-18）」
- レビュー記録: `docs/plans/bootstrap-roadmap/2-5-review-log.md#type-002-step2-型表現統合ドラフト2026-04-18`
- 計画書: `docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md` Step2、`docs/plans/bootstrap-roadmap/2-5-proposals/README.md`（脚注 `[^type-002-row-design]`）

## Phase 2-5 TYPE-002 Step4 実装ロードマップ確定（2026-04-24）

### サマリ
- Phase 2-7 を 3 スプリント（Sprint A: `types.ml` / `typed_ast.ml` での `effect_row` 統合と dual-write 実験、Sprint B: `generalize` / `instantiate` / `Type_unification` / `constraint_solver.ml` の拡張と診断同時出力、Sprint C: `core_ir/desugar_fn.ml`・`runtime/effect_registry.ml`・Windows/macOS 監査ラインの検証）に分割し、各スプリントの完了条件と依存関係を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration` に記録した。  
- `RunConfig.extensions["effects"].type_row_mode` の移行シナリオを「`metadata-only` → `dual-write` → `ty-integrated`」と定義し、ガード診断・監査メタデータ・ロールバック条件を整理。dual-write 期間は `typed_fn_decl.tfn_effect_profile` と新 `effect_row` の両出力を CI で比較する手順を確立した。  
- 新規 KPI `diagnostics.effect_row_stage_consistency` / `type_effect_row_equivalence` / `effect_row_guard_regressions` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に登録し、`tooling/ci/collect-iterator-audit-metrics.py` の実装差分（`--section effects` 拡張、`require_success` での 1.0 強制）を計画した。

### テスト & KPI 計画
- `compiler/ocaml/tests/test_type_inference.ml` に `type_effect_row_equivalence_*` シナリオ（宣言順差異・残余効果差分・`@handles` 照合）を追加し、`StringSet` 正規化と CLI 表示順保持を両立させるゴールデンテンプレートを準備する。  
- `compiler/ocaml/tests/streaming_runner_tests.ml` へ `streaming_effect_row_stage_consistency` を新設し、`effect_row` 統合後も監査ログが `effect.type_row.canonical` を保持するか検証。  
- `reports/diagnostic-format-regression.md` に効果行差分セクションを追加し、dual-write 期間中は CLI/LSP/監査出力が完全一致するかレビューする。  
- CI では `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success --section effects` を追加し、`diagnostics.effect_row_stage_consistency = 1.0`・`type_effect_row_equivalence = 1.0`・`effect_row_guard_regressions = 0` をゲート条件とする。

### TODO / フォローアップ
1. Sprint A 着手前に `effect_row` dual-write ブランチを作成し、`metadata-only` モードと比較できる診断スナップショットを収集する。  
2. Windows/macOS CI の監査ゴールデンへ効果行フィールドを追加し、`collect-iterator-audit-metrics.py` の `platform` 列が `effect_row_guard_regressions` を監視できるよう更新する（Phase 2-7 Sprint C）。  
3. RowVar（行多相）対応は Phase 3 へ移管し、Step5 ハンドオーバーで評価メモと API 予約値の扱いを確認する。  
4. `docs/plans/bootstrap-roadmap/2-4-to-2-5-handover.md` の後継欄へ TYPE-002 Step5 用チェックリストを追加し、効果チームへの引き継ぎ内容（dual-write 成果物、CI 指標、監査テンプレート）を整理する。

### 参考リンク
- `docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md` Step4
- `docs/plans/bootstrap-roadmap/2-5-review-log.md#type-002-step4-実装ロードマップとテスト観点2026-04-24`
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`（新規 KPI）
- `tooling/ci/collect-iterator-audit-metrics.py`（指標集計ポイント）

## Phase 2-5 TYPE-002 Step5 ハンドオーバー準備とリスク登録（2026-04-24）

### サマリ
- `docs/plans/bootstrap-roadmap/2-5-to-2-7-type-002-handover.md` を新設し、Phase 2-7 Sprint A/B/C の到達条件、Gate 条件（設計レビュー、脚注整合、テスト基盤、リスクレビュー）、およびロールバック手順を整理。  
- `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスク ID `TYPE-002-ROW-INTEGRATION`（期限 2026-10-31, 状態 Open）を登録し、EFFECT 系リスク（`EFFECT-POC-Stage`）との依存関係をコメントで明示。  
- KPI `diagnostics.effect_row_stage_consistency` / `type_effect_row_equivalence` / `effect_row_guard_regressions` を Phase 2-7 開始時から追跡できるよう、`tooling/ci/collect-iterator-audit-metrics.py` の拡張点（`--section effects`）と CI ランナー（Linux/Windows/macOS）の導線を点検。

### フォローアップ
1. Phase 2-7 キックオフ前に `effect_system_design` レビュー（タグ `TYPE-002-G1`）を開催し、`TArrow` 拡張ドラフトと RowVar 先送り方針を承認する。  
2. `type_row_mode` を `dual-write` へ切り替える前に、CI で `collect-iterator-audit-metrics.py --section effects` が動作し、`effect_row_guard_regressions = 0` を維持できることを検証する。  
3. KPI が基準値（1.0 / 1.0 / 0.0）を満たした時点で脚注 `[^type-row-metadata-phase25]` 撤去案とリスククローズ条件をまとめ、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` とリスク台帳を更新する。  
4. RowVar 実装は Phase 3 で判断するため、Phase 2-7 では `Open row_var` を予約値として維持し、API から外部露出しないようレビュー時に確認する。

## Phase 2-5 TYPE-002 Step3 仕様脚注と移行ガード（2026-04-22）

### サマリ
- `docs/spec/1-2-types-Inference.md` §A.2 / §C.6、`docs/spec/1-3-effects-safety.md` §A / §I、`docs/spec/3-6-core-diagnostics-audit.md` §2.4.2 に脚注 `[^type-row-metadata-phase25]` を追加し、Phase 2-5 の暫定運用（効果行は診断メタデータとして保持、`type_row_mode = "metadata-only"` 固定）を明文化。
- `RunConfig.extensions["effects"].type_row_mode` のモード定義を整理し、`ty-integrated` 要求時に `effects.type_row.integration_blocked` を発行するポリシーと `effect.type_row.*` 監査キー（`requested_mode` / `available_mode` / `guard_stage`）を仕様へ登録。
- `docs/spec/README.md`・`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`・`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に TYPE-002 脚注と解除条件を導線として追加し、索引と後続フェーズ計画の同期を完了。

### 主要アウトプット
1. 脚注 `[^type-row-metadata-phase25]` の本文（Phase 2-5 暫定・解除条件・参照先）を 1-2 / 1-3 / 3-6 / spec README / 2-5 Spec Drift 計画に展開。
2. `effects.type_row.integration_blocked` 診断テンプレートと監査キー `effect.type_row.requested_mode` / `available_mode` / `guard_stage` を `docs/spec/3-6-core-diagnostics-audit.md` に追加。
3. `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration` に Phase 2-7 での実装タスク（`TArrow` 拡張、KPI、脚注撤去）と検証条件を登録。

### フォローアップ
1. Phase 2-7 で `ty` 拡張と KPI 実装が完了したら、`type_row_mode` の既定を `"ty-integrated"` へ更新し、`metadata-only` はレガシーモードとして CLI オプションに残す。
2. `diagnostics.effect_row_stage_consistency` の算出実装（`collect-iterator-audit-metrics.py`）と `type_effect_row_*` テストを Step4 設計へ取り込み、Phase 2-7 Sprint 開始時の受け入れ条件へ登録。
3. 脚注撤去時に備えて、`docs/spec/1-2-types-Inference.md` / `1-3-effects-safety.md` / `3-6-core-diagnostics-audit.md` の該当節にチェックリスト（手動確認項目）を付与する案を Step4 で検討。

### 参考リンク
- 仕様差分計画: `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`（`TYPE-002` 項の 2026-04-22 更新）
- 設計ノート: `compiler/ocaml/docs/effect-system-design-note.md`「## 3. 型表現統合ドラフト」
- レビュー記録: `docs/plans/bootstrap-roadmap/2-5-review-log.md#type-002-step3-効果行脚注と移行ガード2026-04-22`
- フォローアップ計画: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration`

[^type-row-metadata-phase25]: Phase 2-5 `TYPE-002 Step3`（2026-04-22 完了）で追加。効果行を `ty` へ統合する前段として `RunConfig.extensions["effects"].type_row_mode = "metadata-only"` とし、`effects.type_row.integration_blocked` 診断・`effect.type_row.*` 監査キーで移行試行を記録する。解除条件は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration` を参照。
