# ERR-001 期待集合出力整備提案

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
- **テスト**: `parser_driver_tests.ml`（PARSER-001 で追加予定）に期待集合検証ケースを追加し、代表的な構文エラーで `expected` が埋まるか確認。  
- **監査**: `0-3-audit-and-metrics.md` に `parser.expected_summary_presence` を追加し、CI で期待集合が欠落していないかを監視。  
- **CLI/LSP**: `reports/diagnostic-format-regression.md` の JSON フィクスチャを更新し、`scripts/validate-diagnostic-json.sh` が期待集合を検証するよう拡張。
- **実装**: `compiler/ocaml/src/parser_driver.ml` と `compiler/ocaml/src/parser_expectation.ml`（新設）へユニットテストを追加し、Menhir が生成する期待集合が `ExpectationSummary` へ正しく写像されるかをスナップショットで検証する。

## 4. フォローアップ
- `ParseResult` シム（PARSER-001）と連携し、`DiagState` に保持した最遠エラー位置から期待集合を取得する実装計画をまとめる。  
- 仕様書の脚注で「OCaml 実装は期待集合導入中」と明記し、実装完了時に脚注を削除する。  
- `docs/guides/core-parse-streaming.md` に期待集合がストリーミングモードでも利用可能である旨を追記する。
- `docs/guides/plugin-authoring.md` に期待集合 API の利用例を追加し、外部 DSL が CLI/LSP と同じ情報を取得できるようにする。
- **タイミング**: PARSER-001 のシム構築と並行して Phase 2-5 前半に対応し、Phase 2-5 中盤の CLI/LSP ゴールデン更新までに完了させる。

## 確認事項
- Menhir の期待集合から `Expectation` 列挙へ写像する際の粒度（記号／規則／否定等）を Parser チームと調整する必要がある。  
- 期待集合が大量になる場合の扱い（上限件数や優先順位）を CLI/LSP チームと合意したい。
