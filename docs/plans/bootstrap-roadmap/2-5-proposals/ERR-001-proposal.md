# ERR-001 期待集合出力整備計画

## 1. 背景と症状
- 仕様は `ParseError.expected` と `ExpectationSummary` を用いて期待集合を返すと定義しており（docs/spec/2-5-error.md:1-160）、CLI/LSP/監査が期待値を提示できることを前提にしている。  
- 現行 OCaml 実装は `Diagnostic.of_parser_error` 呼び出し時に `expected = []` を固定しており（compiler/ocaml/src/parser_driver.ml:10-38）、Menhir が提供する期待集合を収集していない。  
- 期待値が欠落しているため `effects.contract.*` や `recover` の診断品質が仕様下限を満たさず、`reports/diagnostic-format-regression.md` の差分比較でも空集合となる。

## 2. Before / After
### Before
- Menhir のチェックポイントから期待集合を取り出さず、全ての構文エラーが「構文エラー: 入力を解釈できません」など汎用メッセージのみで報告される。  
- `ExpectationSummary` や `Diagnostic.extensions["parser"]` に有用な情報が入らず、IDE や CLI が修正候補を提示できない。

### After
- `Parser.MenhirInterpreter.expected`（Menhir API）を利用して期待集合を取得し、`Expectation` 列挙へ写像するシムを実装。  
- `ParseResult` 経由で最遠エラーの `ExpectationSummary` を構築し、`Diagnostic.expected` と `extensions["parse"].expected_overview` に反映する。  
- CLI/LSP のゴールデンを更新し、仕様通りの期待集合が表示されることを確認する。

## 3. 影響範囲と検証
- **テスト**: `compiler/ocaml/tests/test_parser_expectation.ml`（新設）と `compiler/ocaml/tests/parser_driver_tests.ml` に期待集合の写像・重複縮約・最遠エラー更新を確認するケースを追加し、Menhir 自動生成コードを含む `Parser.MenhirInterpreter` 更新時にリグレッションが検出できるようにする。  
- **監査**: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `parser.expected_summary_presence` と `parser.expected_tokens_per_error` を登録し、`tooling/ci/collect-iterator-audit-metrics.py` にしきい値チェックを追加して Phase 2-8 の監査ダッシュボード (`reports/audit/dashboard/`) と連携する。  
- **CLI/LSP**: `reports/diagnostic-format-regression.md` の差分手順に「期待集合の JSON フィールド検証」を追記し、`scripts/validate-diagnostic-json.sh` / `tooling/lsp/tests/client_compat/fixtures/` のフィクスチャを更新して `expected_summary.alternatives` が空でないことを CI で確認する。CLI テキストゴールデン（`compiler/ocaml/tests/golden/diagnostics/*.golden`）にも期待集合表示例を追加する。  
- **実装**: `compiler/ocaml/src/parser_driver.ml:62-138` のエラーハンドリング分岐で Menhir 期待集合を回収し `Diagnostic.of_parser_error` へ渡す経路を整備する。併せて `compiler/ocaml/src/parser_diag_state.ml:9-48` で最遠スナップショットに `ExpectationSummary` を保持し、`compiler/ocaml/src/diagnostic.ml:70-140` に期待集合をセットするヘルパを追加する。期待値マッピングは `compiler/ocaml/src/parser_expectation.ml` / `.mli`（新設）で実装し、`menhir --list-errors` の出力と突合したマッピング表を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に添付する。

## 4. フォローアップ
- `ParseResult` シム（PARSER-001）と連携し、`DiagState` に保持した最遠エラー位置から期待集合を取得する実装計画をまとめる。  
- 仕様書の脚注で「OCaml 実装は期待集合導入中」と明記し、実装完了時に脚注を削除する。  
- `docs/guides/core-parse-streaming.md` に期待集合がストリーミングモードでも利用可能である旨を追記する。
- `docs/guides/plugin-authoring.md` に期待集合 API の利用例を追加し、外部 DSL が CLI/LSP と同じ情報を取得できるようにする。
- **タイミング**: PARSER-001 のシム構築と並行して Phase 2-5 前半に対応し、Phase 2-5 中盤の CLI/LSP ゴールデン更新までに完了させる。

## 5. 実装ステップ

### S1. Menhir 期待集合 API の棚卸し（Week31 Day1）
- **目的**: `Parser.MenhirInterpreter.expected` の出力と `docs/spec/2-5-error.md` の `Expectation` 整合を事前に確認し、写像方針と優先順位を確定する。
- **調査**:
  - `menhir --list-errors compiler/ocaml/src/parser.mly` を実行し、`[%on]` ラベルや `error` トークンの有無を含む期待集合パターンを抽出する。
  - `Parser.MenhirInterpreter` が返す `Terminal` / `Nonterminal` / `EOF` の型を確認し、`MenhirLib.EngineTypes.element` の表現を把握する。
  - `compiler/ocaml/src/token.ml` の予約語・演算子リストを抜き出して分類表を作成し、`Token` と `Keyword` の判定基準を明文化する。
- **タスク**:
  - 期待集合のカテゴリ（記号・キーワード・規則・否定・クラス）ごとに写像ルール案を整理し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に初期草案として記録する。
  - `Parser_diag_state` の最遠スナップショット仕様（`compiler/ocaml/src/parser_diag_state.ml:9-48`）と突合し、保存形式への制約を明文化する。
- **成果物/完了条件**: `review-log` に「ERR-001/S1 調査メモ」を追加し、写像ルール表と `menhir --list-errors` サマリが共有されていること。

### S2. 期待集合マッピング層の実装（Week31 Day2-3）
- **目的**: Menhir の期待集合を `Diagnostic.expectation` / `ExpectationSummary` へ変換する専用モジュールを新設し、変換ロジックを単体テスト可能にする。
- **調査**:
  - `Diagnostic.expectation` 列挙（`compiler/ocaml/src/diagnostic.ml:70-110`）の既存利用箇所を洗い出し、追加の `Expectation` が必要か確認する。
  - `docs/spec/2-5-error.md` §A の `ExpectationSummary` 例を再確認し、`message_key`・`locale_args` の既定値・テンプレート設計指針をまとめる。
- **タスク**:
  - `compiler/ocaml/src/parser_expectation.ml` / `.mli` を新設し、Menhir の `Terminal` → `Token | Keyword | Rule | Class | Not | Custom`、`Nonterminal` → `Rule` などの写像関数と `ExpectationSummary` ビルダを提供する。
  - 同モジュール用のユニットテスト `compiler/ocaml/tests/test_parser_expectation.ml` を追加し、代表 6 ケース（キーワード、演算子、EOF、規則、否定、文字クラス）をスナップショット化する。
  - 期待集合が 0 件の場合のフォールバック（`message_key = Some "parse.expected.empty"` など）を定義し、仕様との矛盾を避ける。
- **成果物/完了条件**: 新モジュールとテストが `dune runtest` に組み込まれ、`ExpectationSummary` の JSON 表現がスナップショットで確認できる状態。

### S3. パーサドライバと診断状態への組込み（Week31 Day3-4）
- **目的**: `parser_driver` のエラーハンドリング経路で期待集合を計算し、`Diagnostic` へ確実に伝播させる。
- **調査**:
  - `compiler/ocaml/src/parser_driver.ml:62-138` の `I.HandlingError` 分岐と `process_parser_error` / `process_rejected_error` の呼び出し経路を精査する。
  - `parser_diag_state` の `farthest_snapshot` 更新ロジックを確認し、既存の `expected` 集約が期待集合導入と競合しないことを確認する。
- **タスク**:
  - `process_parser_error` / `process_rejected_error` に `Parser_expectation.collect ~checkpoint` の結果を渡し、`Diagnostic.of_parser_error` が `expected` を受け取れるよう `~expected` 引数の生成を差し替える。
  - `Parser_diag_state.record_diagnostic` に `ExpectationSummary` を保存するフィールドを追加し、`Diagnostic.expected` が `None` のままになる経路を排除する。
  - `parser_driver_tests.ml` に「最遠エラーで期待集合が縮約される」「複数回復後も最遠位置の `expected` が保持される」ケースを追加し、`PARSER-001` の `ParseResult` シムと協調動作を確認する。
- **成果物/完了条件**: `Parser.run` で発生した構文エラーが `Diagnostic.expected` に期待集合を含み、`parse_result.legacy_error.expected` に反映されることをテストで証明。

### S4. ゴールデンと CI 監視の整備（Week31 Day4-5）
- **目的**: 期待集合が CLI/LSP/監査経路で可視化・検証される状態を構築し、回 regressions を CI で検知する。
- **調査**:
  - `reports/diagnostic-format-regression.md` と `scripts/validate-diagnostic-json.sh` の現行チェック項目を再確認し、期待集合フィールド追加に伴う差分手順を明文化する。
  - `tooling/ci/collect-iterator-audit-metrics.py` の既存メトリクスを把握し、新指標導入時の出力形式を決定する。
- **タスク**:
  - CLI ゴールデン（`compiler/ocaml/tests/golden/diagnostics/*.golden`）と LSP フィクスチャ（`tooling/lsp/tests/client_compat/fixtures/*.json`）へ期待集合フィールドを追加し、`scripts/validate-diagnostic-json.sh` が `expected_summary.alternatives|length > 0` を検証するよう更新する。
  - `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に新指標を追記し、CI で 0 件検出時に失敗させるロジックを `collect-iterator-audit-metrics.py` へ追加する。
  - `reports/audit/dashboard/` のテンプレートを更新し、期待集合関連の KPI（例: `expected_tokens_per_error`）を可視化する。
- **成果物/完了条件**: GitHub Actions（`diagnostic-json` / `audit-dashboard`）が期待集合欠落を自動検知し、ローカルでも `scripts/validate-diagnostic-json.sh` が新フィールドを検証できることが確認できる。

### S5. ドキュメントと共有タスク（Week31 End）
- **目的**: 実装完了後の仕様・ガイド・ログ更新を一括で整理し、他チームとの連携事項を明確化する。
- **調査**:
  - `docs/spec/2-5-error.md` と `docs/spec/3-6-core-diagnostics-audit.md` に期待集合関連の脚注や TODO が残っていないか確認する。
  - `docs/guides/core-parse-streaming.md` / `docs/guides/plugin-authoring.md` に追加すべき API 説明を洗い出す。
- **タスク**:
  - 仕様書に「2025-Phase2.5 で期待集合が OCaml 実装へ導入済み」とする脚注を追加し、旧脚注（導入中）を削除する。
  - `docs/plans/bootstrap-roadmap/2-5-review-log.md` に S1〜S4 の結果と検証ログ（`menhir --list-errors` 実行結果、CLI/LSP スナップショットリンク、CI 証跡）を追記する。
  - `docs/notes/spec-integrity-audit-checklist.md`（Phase 2-8 で参照予定）へ期待集合監視項目を追加するための草案メモを残す。
- **成果物/完了条件**: 関連ドキュメントの更新とレビュー記録が完了し、Phase 2-7 以降のチームが期待集合の状態を追跡できること。

## 6. 残課題
- Menhir の期待集合から `Expectation` 列挙へ写像する際の粒度（記号／規則／否定等）を Parser チームと調整する必要がある。  
- 期待集合が大量になる場合の扱い（上限件数や優先順位）を CLI/LSP チームと合意したい。
