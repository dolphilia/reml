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
| **S4: Phase 2-7 への引き継ぎ準備（週34）** | PoC の成果物と未解決事項を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`・`docs/notes/effect-system-tracking.md` に連携し、Phase 2-7 効果チームが着手できるよう段階表とリスクを整理する。`-Zalgebraic-effects` フラグ運用（CLI/LSP/ビルド）の影響を洗い出す。 | - Phase 2-7 計画書の効果セクション<br>- `docs/notes/dsl-plugin-roadmap.md` の Stage 連携項目<br>- `tooling/ci/` 内の実験フラグ制御スクリプト | - 引き継ぎチェックリスト（[#s4-handover-checklist](#s4-phase-2-7-への引き継ぎ準備2026-04-03))<br>- 2-7 計画へのリンク追加<br>- CLI オプション仕様への TODO |

> 各ステップ終了時には `docs/plans/bootstrap-roadmap/2-5-review-log.md` へ検証ログを追加し、脚注 ID と CI 指標値（達成した場合でも 0.0 → 1.0 の推移を記録）を残す。未完了タスクは `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に転記する。

> Phase 2-5 Week31 更新: S0 を完了し、`docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/3-8-core-runtime-capability.md` に脚注 `[^effects-syntax-poc-phase25]` を追加。`docs/spec/README.md` と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` へも同脚注 ID を登録し、PoC ステージと `-Zalgebraic-effects` 依存を明示した。レビュー記録は `docs/plans/bootstrap-roadmap/2-5-review-log.md` SYNTAX-003 セクションに追記。

### S2 型・効果解析の PoC 接続（2026-03-19）

#### 1. 調査と結論
- `compiler/ocaml/src/type_inference_effect.ml` は Stage 判定と Capability 正規化のみ実装されており、式レベルの `perform` / `handle` ノードを受理する経路が存在しないことを確認した。`Type_inference` 本体も `Ast.expr_kind` に効果系ノードが無い前提で構築されているため、PoC では Typed AST に効果構文ノードを追加しつつ `Type_inference_effect` で `Σ_before` → `Σ_after` の追跡を行う必要がある。
- `docs/spec/1-3-effects-safety.md` §I と Phase 1 の `effect-system-design-note.md` を突き合わせ、PoC では「単一タグ捕捉」「`resume` 1 回まで」「`perform` は宣言済みタグのみ許可」の 3 点に制限すれば `effects.contract.*` 診断と Stage 突合が成立することを整理した。
- `reports/diagnostic-format-regression.md` に登録済みの `effects.contract.mismatch` 例は Stage 判定起因であり、PoC では型推論段階で `Σ_after` を明示的に記録すれば同じ診断インフラを再利用できることを確認した。

#### 2. PoC で通過させる型規則

| 式カテゴリー | 型制約（Typer 側） | 潜在効果集合の処理 | Stage / 診断の扱い |
|---------------|--------------------|--------------------|-------------------|
| `perform E.op(args)` / `do E.op args` | `op : τ_args -> τ_ret` を `effect E` 宣言から解決し、引数型を一括照合する。戻り値は `τ_ret` をそのまま返す。 | `Σ_before ∪ {E}` を `Σ_expr` として保持し、ハンドラ探索が無い場合は `Σ_after = Σ_expr` を Typed AST に保持する。 | `Type_inference_effect.resolve_function_profile` で `E` の Stage を取得し、未定義タグは `effects.contract.unknown_effect` を送出。 |
| `handle expr with handler { effect E.op(x) -> body; return r }` | `expr : τ_in` を推論し、`body : τ_out`、`return r : τ_out` を統一。PoC では単一タグ `E` のみ捕捉し、`resume` は 0〜1 回に制限。 | `Σ_handler = {E}` とし、`Σ_residual = effects(body) ∪ effects(r)` を合成。`Σ_after = (Σ_before - Σ_handler) ∪ Σ_residual` を計算して Typed AST に格納。 | `Σ_after ⊆ allows_effects` を満たさない場合は `effects.contract.residual` を報告。`@handles` 属性があれば Stage 判定と同一フックで検証。 |
| `handler { effect E.op(x, resume k) -> body; return r }`（宣言） | ハンドラ本体に仮引数 `x : τ_arg` と `resume : τ_ret -> τ_out` を供給し、`body` と `return` の戻り値 `τ_out` を統一。PoC では `resume` を 1 回のみ評価、継続型は `τ_ret -> τ_out` 固定。 | `Σ_before` は宣言時点では空集合、`Σ_residual = effects(body) ∪ effects(r)` を記録し、呼び出し側へ伝播する。 | `effect` 宣言に含まれないタグを捕捉した場合は `effects.contract.unhandled_effect`。Stage 要件は `handler` 自身の `@handles` で検証。 |
| `@handles(E)` 属性付き関数 | 関数本体の潜在効果 `Σ_body` と属性列挙 `Σ_handles` を比較。 | `Σ_after = Σ_body - Σ_handles` を計算し、PoC では `Σ_after = ∅` を成立条件とする。 | Stage 監査は既存の `effect_profile` を利用し、`Σ_after ≠ ∅` の場合は `effects.contract.mismatch` を発報。 |

上表は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の効果タスクへ転記予定の PoC スコープ定義として添付した。

#### 3. テスト設計と共有
- `compiler/ocaml/tests/test_type_inference.ml` に「効果構文 PoC テスト草案」セクションを追加し、`perform` が単一タグを追加するケース、`handle` が `Σ_before` からタグを除去できるケース、タグ未捕捉による `effects.contract.residual` の再現ケースの 3 種を列挙した。テストは `Test_support.parse_string` を利用し、失敗時は診断メッセージを文字列比較する方針で記録。
- `streaming_runner_tests.ml` と `reports/diagnostic-format-regression.md` で共有する PoC サンプルは、TypeChecker 側で `Σ_after` を JSON 診断へ書き出す形式とし、`effects.contract.*` の差分を Phase 2-7 でゴールデン化するまでの暫定運用とした。

#### 4. フォローアップ
1. `type_inference_effect.ml` に `resolve_expr_profile`（仮称）を追加し、式単位で `Σ_before` と `Σ_after` を返却する補助関数を Phase 2-7 で実装する。
2. `Typed_ast` に `TEffectPerform` / `TEffectHandle` ノードを追加する設計草案を `effect-system-design-note.md` に追記し、PoC 実装着手前にレビューを受ける。
3. `tooling/ci/collect-iterator-audit-metrics.py` へ PoC 指標 `syntax.effect_construct_acceptance` を渡す際の JSON 例を `docs/notes/effect-system-tracking.md` に追記する。

### S3 診断・CI 計測整備（2026-03-27）

#### 1. 調査と結論
- `compiler/ocaml/src/diagnostic.ml` と `parser_diag_state.ml` を確認し、効果構文の PoC では `extensions.effects` の既存フィールドへ `construct` / `handler_stack` 情報を追加する余地があること、`Diagnostic.Builder.add_extension` による構造化 JSON を再利用すれば追加フィールドを既存テストで検証できることを整理した。`diagnostic.info_hint_ratio` の算出ロジックは Warning 以外を除外していないため、新規 Info/Hint 診断が増えても算出式が変化しない点を確認した。
- CI 指標収集スクリプト `tooling/ci/collect-iterator-audit-metrics.py` を調査し、効果関連の RequiredField 一覧へ `effect_syntax.constructs[*]` を解析する関数を追加するだけで新指標を計算できる構造であることを確認した。既存メトリクス出力の `summary["effects"]` セクションに新しい比率を併記する案を採用し、`--require-success` 時は `syntax.effect_construct_acceptance = 1.0` を期待値とするガードを追加する設計をまとめた。
- 計測基盤文書 `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` を確認し、Phase 2-5 時点で効果構文に関する KPI が未登録であることを特定。S3 で `syntax.effect_construct_acceptance`（構文の受理率）と `effects.syntax_poison_rate`（未捕捉タグがゼロである割合）を追加し、PoC 中はそれぞれ `0.0` / `1.0` を許容値とする運用メモを追記する方針とした。
- `reports/diagnostic-format-regression.md` の差分チェックリストに効果構文の CLI/LSP ゴールデン更新手順が無い点を確認し、PoC サンプルを追加する際に `syntax.effect_construct_acceptance` の JSON 例と CLI テキスト出力の確認項目を新設する計画を立てた。

#### 2. 成果物
- `tooling/ci/` で利用する新規メトリクス用サンプル JSON を策定。PoC では `constructs` 配列に `perform` / `handle` の受理状況を記録し、`metrics` セクションで `syntax.effect_construct_acceptance` と `effects.syntax_poison_rate` を算出するフォーマットとした。

  ```json
  {
    "effect_syntax": {
      "constructs": [
        {
          "kind": "perform",
          "tag": "Console.log",
          "sigma_before": ["Console"],
          "sigma_after": ["Console"],
          "diagnostics": []
        },
        {
          "kind": "handle",
          "tag": "Console.log",
          "sigma_before": ["Console"],
          "sigma_handler": ["Console"],
          "sigma_after": [],
          "diagnostics": ["effects.contract.residual"]
        }
      ],
      "metrics": {
        "syntax.effect_construct_acceptance": 0.0,
        "effects.syntax_poison_rate": 1.0
      }
    }
  }
  ```

- `reports/diagnostic-format-regression.md` に効果構文サンプルを CLI/LSP へ取り込む手順を追記し、ゴールデン更新時に `tools/collect-iterator-audit-metrics.py` の `--section effects` 出力を保存するチェックリストを追加した。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表に新指標を登録し、CI で `--require-success` を実行した際の想定値・逸脱時のフォローアップ手順（`0-4-risk-handling.md` への記録、Phase 2-7 へのエスカレーション）を整理した。
- `docs/notes/effect-system-tracking.md` に診断・CI 計測ステージの更新と S3 サマリを追加し、Phase 2-7 へ提供する計測 TODO（監査ログのゴールデン化、LSP フィクスチャ更新）を明文化した。

#### 3. TODO / フォローアップ
1. Phase 2-7 で `collect-iterator-audit-metrics.py` の `effects` セクションへ新指標を実装し、`--require-success` で 1.0 が必須となるゲート処理を追加する。実装後は CLI/LSP/監査ログすべてで `effect_syntax.metrics` が出力されることを `scripts/validate-diagnostic-json.sh` で確認する。
2. `cli` / `lsp` の診断フィクスチャに効果構文サンプルを追加し、`diagnostic.info_hint_ratio` の値変化を記録するチェックリストを Phase 2-7 に引き継ぐ。PoC 終了時点では Info/Hint の増加が KPI に影響しないかをレビューで確認する必要がある。
3. `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ、効果構文メトリクスを正式運用へ切り替える条件（`syntax.effect_construct_acceptance` ≥ 0.8 を要件とする案）と監査ダッシュボード更新タスクを追加する。

### S4 Phase 2-7 への引き継ぎ準備（2026-04-03）

#### 1. 調査サマリ
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に「効果構文 PoC → 実装」サブタスクを追加し、Phase 2-7 の監査ゲート整備と同じマイルストーンで `syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` を 1.0 へ引き上げる条件を明示した。CLI/LSP/CI それぞれの対応窓口と必要スクリプト（`tooling/ci/collect-iterator-audit-metrics.py`、`scripts/validate-diagnostic-json.sh`）をリンク済み。
- `docs/notes/effect-system-tracking.md` を更新し、PoC 到達条件・残課題・フラグ運用の整理を「S4 引き継ぎパッケージ」として集約。`-Zalgebraic-effects` フラグの CLI/LSP/ビルド制御を三系統で管理するための TODO を追記し、解除条件を一括参照できるようにした。
- Phase 2-7 で参照する関連資料（`docs/notes/dsl-plugin-roadmap.md` Stage チェックリスト、`reports/diagnostic-format-regression.md` のゴールデン更新手順、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` 差分リスト）のリンク先と脚注を本計画書から辿れるよう整備した。

#### 2. 引き継ぎチェックリスト <a id="s4-handover-checklist"></a>
| チェック項目 | 内容 | 引き継ぎ先 |
| --- | --- | --- |
| PoC 到達条件の確認 | `effect-system-tracking.md` のステージ表とサンプル JSON を参照し、S1〜S3 の成果物が揃っているかをレビューする。 | Phase 2-7 効果チームリード |
| `-Zalgebraic-effects` フラグ運用 | CLI（`compiler/ocaml/src/cli/options.ml`）、LSP（`tooling/lsp/tests/client_compat`）、ビルドスクリプトで Experimental フラグを統一。解除条件は 2-7 計画 §3 に追記済み。 | CLI/LSP オーナー |
| CI メトリクス切替 | `collect-iterator-audit-metrics.py --section effects` に新指標を実装し、`--require-success` で 1.0 を必須化。逸脱時は `0-4-risk-handling.md` へ登録する手順を 2-7 計画へ転記。 | CI チーム |
| 脚注・索引更新 | `docs/spec/1-1`・`1-5`・`3-8` に付与した脚注 `[^effects-syntax-poc-phase25]` を Phase 2-7 完了時に解除する条件と連絡経路を確認。 | 仕様エディタ |
| Plugin / Capability 連携 | `docs/notes/dsl-plugin-roadmap.md` のチェックリスト更新と `effects.contract.stage_mismatch` 診断の監査ログ整備を Phase 2-7 の Stage タスクへ紐付け。 | Capability Registry 担当 |

#### 3. TODO / フォローアップ
1. Phase 2-7 で `compiler/ocaml/tests/effect_syntax_tests.ml` を新設し、PoC サンプルをゴールデンとして固定。テスト導入時は `syntax.effect_construct_acceptance` の基準値を 1.0 に更新する。
2. `-Zalgebraic-effects` の最終公開名と CLI ドキュメント（`docs/spec/0-0-overview.md`、`docs/guides/ecosystem/ai-integration.md`）への記載を Phase 2-7 内で確定する。名称決定までは Experimental フラグを維持し、解除判断は Phase 2-7 終盤のレビューにて行う。
3. `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に記載した Stage 遷移条件が満たされた時点で、`docs/notes/effect-system-tracking.md` のステージ表を更新し、脚注 `[^effects-syntax-poc-phase25]` の撤去手順を Phase 2-8 へ通知する。

## 6. 進捗記録（Phase 2-5）
- 2026-03-12: **S1 パーサ PoC 設計完了**。`compiler/ocaml/docs/parser_design.md` §3.3.1 に挿入位置・優先順位・PoC 制限を反映し、`parser_run_config` への実験フラグ導入方針を確定。`compiler/ocaml/docs/effect-system-design-note.md` にモジュール間連携を追記し、`docs/notes/effect-system-tracking.md` を新設して PoC ステージ・引き継ぎ TODO を整理した。レビュー記録は `docs/plans/bootstrap-roadmap/2-5-review-log.md` 2026-03-12 項目を参照。
- 2026-03-19: **S2 型・効果解析 PoC 接続設計完了**。`type_inference_effect` の拡張ポイントと `Σ_before/Σ_after` の記録方針を整理し、PoC で許容する型規則表・テスト草案・CI 連携メモを本計画書および `docs/notes/effect-system-tracking.md` に反映。レビュー記録は `docs/plans/bootstrap-roadmap/2-5-review-log.md` 2026-03-19 項目を参照。
- 2026-03-27: **S3 診断・CI 計測整備完了**。新指標の計測フォーマット・サンプル JSON・ゴールデン更新手順を策定し、`0-3-audit-and-metrics.md`・`reports/diagnostic-format-regression.md`・`docs/notes/effect-system-tracking.md` に同期。レビュー記録は `docs/plans/bootstrap-roadmap/2-5-review-log.md` 2026-03-27 項目を参照。
- 2026-04-03: **S4 Phase 2-7 への引き継ぎ準備完了**。本計画書に引き継ぎチェックリストを追加し、`2-7-deferred-remediation.md`・`effect-system-tracking.md`・`2-5-spec-drift-remediation.md` へ PoC ステージの引き継ぎ条件を同期。`docs/plans/bootstrap-roadmap/2-5-review-log.md` 2026-04-03 項目を参照。

## 残課題
- 効果構文を有効化するフラグ名（`-Zalgebraic-effects` 仮称）と公開ポリシーは Phase 2-7 で確定させる必要がある。S4 で整理したチェックリストに従い、CLI/LSP/ビルドのドキュメント更新タイミングを調整する。  
- `perform` などの構文追加が既存優先順位に与える影響（Menhir の衝突、`parser.conflicts` 更新）を事前にレビューしたい。  
- Phase 2-7 で `collect-iterator-audit-metrics.py` の実装が完了した際、PoC 指標の数値更新と脚注撤去プロセスを `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`・`docs/notes/effect-system-tracking.md` に反映する。

[^effects-syntax-poc-phase25]:
    Phase 2-5 Week31 時点の方針。効果構文は `-Zalgebraic-effects` フラグを必須とする Experimental Stage に留め、正式実装は Phase 2-7 で `parser.mly`・型推論・効果解析を統合した後に進める。紐付く脚注は `docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/3-8-core-runtime-capability.md` に同期済みで、差分ログは `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と `docs/plans/bootstrap-roadmap/2-5-review-log.md` を参照。
