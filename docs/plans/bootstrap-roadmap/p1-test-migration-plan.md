# P1 Rust フロントエンドのテスト移植計画

## 1. 背景と目的
- `docs/plans/rust-migration/p1-rust-frontend-gap-report.md` で整理した P1 の未達項目は一通り完了し（Lexer/Parser の `FRG-06`〜`FRG-08`、型推論 `FRG-12`〜`FRG-14`、診断/Streaming 周りの `FRG-04`/`FRG-05` など）、Rust 実装が Dual-write の基盤を持つことを前提としている。
- `docs/plans/rust-migration/1-0-front-end-transition.md` と `p1-spec-compliance-gap.md` の完了条件を満たすために、OCaml フロントエンドで保持していたテスト資産を Rust 側に移植し、仕様と挙動の差分監査を継続的に実行したい。
- 本計画は `compiler/ocaml/tests/` のうち Rust フロントエンドで再現可能なテストを分類し、移植実施手順と検証ストリームを整理することで、Phase P1 のテストカバレッジを Rust に移す下支えとする。

## 2. Rust 実装に既に存在するテスト資産
- `compiler/rust/frontend/tests/lexer_token_coverage.rs`、`typeck_hindley_milner.rs`、`streaming_runner.rs` の追加で、Lexer/Parser、型推論（Constraint/診断含む）、Streaming の基本的な経路は既にユニットテストを通過している。また `compiler/rust/frontend/src/diagnostic/json.rs` の recover 拡張テスト (`recover_extension_obtains_kind_from_summary_alternatives`) も存在するため、Dual-write の JSON 生成部分への追加も検証済みである。
- これらのテストには `docs/plans/rust-migration/p1-front-end-checklists.csv` の該当項目（Lexer/Parser、Typed AST、制約ソルバ、Diagnostics）が参照されており、`reports/dual-write/front-end/w3-*` や `w4-*` の出力と連携できるハーネスが整備済みだと見なせる。

## 3. 移植対象候補一覧
### 3.1 Lexer／Parser 系
| ID | テスト | 依存対象 | Rust 移植の理由・補足 |
| --- | --- | --- | --- |
| TPM-LEX-01 | `core_parse_lex_tests.ml` | `lexer`/`token` API | ✅ `FRG-06` に沿ってトークン網羅性が確認済みのため、UTF/Escape/コメントを前提としたトークン列比較を `lexer_token_coverage` のように `cargo test` へ組み込み可能。<br>Dual-write で `collect-iterator-audit-metrics.py` の `lexer.identifier_profile_*` を再利用する。 |
| TPM-LEX-02 | `test_lexer.ml` / `unicode_ident_tests.ml` | 同上 + 識別子正規化 | ✅ Rust 側が Unicode 識別子・ASCII プロファイルを持つため、実際の文字列パターンを `tests/lexer.rs` として再実装。 |
| TPM-LEX-03 | `packrat_tests.ml` / `test_parser.ml` / `test_parser_driver.ml` | ✅ `parser_driver.ml` の RunConfig/State | `FRG-07` の `RunConfig`/`Parser<T>` 達成をもとに `parser_driver` と `ParseResult` の Rust 版を `rust` CLI で叩き、ゴールデン AST（`--emit-ast`）を比較。 |
| TPM-LEX-04 | `test_parser_expectation.ml` / `test_parse_result_state.ml` | `parser_expectation` | ✅ `FRG-08` で `ExpectedTokenCollector` を Enhancement したため、期待候補の正規化/空集合補正を再現できる。 |

### 3.2 型推論・制約周り
| ID | テスト | 依存対象 | コメント |
| --- | --- | --- | --- |
| TPM-TYPE-01 | `test_type_inference.ml` | `type_inference.ml` + `constraint.ml` + `constraint_solver.ml` | ✅ `p1-spec-compliance-gap` の `FRG-12` で要求された Constraint/TyEnv/Scheme が Rust へ揃っているため、型推論ケースの JSON（Typed AST + Constraint + Typecheck Report）を `scripts/poc_dualwrite_compare.sh --mode typeck` で再現し、`reports/dual-write/front-end/w3-type-inference/<case>` に出力。 |
| TPM-TYPE-02 | `test_constraint_solver.ml` / `test_type_errors.ml` / `test_let_polymorphism.ml` | 同上 | ✅ それぞれ `Constraint` の解決、型エラー検出、スコープ・一般化の振る舞いを確認するため、Rust 側の `ConstraintSolver` と `TypecheckDriver` が出す `TypecheckReport`/`Diagnostic` に対して JSON で差分を記録。 |
| TPM-TYPE-03 | `test_cli_callconv_snapshot.ml` / `test_ffi_contract.ml` | CLI (`StageContext`/`runtime_capabilities`) + `effect` | ✅ CLI 連携が `p1-spec-compliance-gap#SCG-09` や `FRG-13` で整理されているため、Rust CLI へ同じフラグを渡し `--emit-typed-ast`/`--emit-effects` を出力、`reports/dual-write/front-end/w4-diagnostics` に `effects.contract.stage_mismatch` などを記録。 |

### 3.3 診断・Streaming
| ID | テスト | 依存対象 | 理由 |
| --- | --- | --- | --- |
| TPM-DIAG-01 | `test_cli_diagnostics.ml` | `diagnostic.ml` + `Collect-iterator` metrics | ✅ `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`/`2-7-deferred-remediation.md` で扱った診断 JSON スキーマと `scripts/validate-diagnostic-json.sh` の整備を Rust でも実行し、`diagnostics.*` のメトリクスを `collect-iterator-audit-metrics.py` で評価。 |
| TPM-DIAG-02 | `streaming_runner_tests.ml` | `parser_driver.ml` + `Core_parse_streaming` | ✅ `FRG-05` を踏まえて Rust 版 `StreamingRunner` に `chunk_size` や `recover` 拡張があるため、Pending/Completed コードパスを再現し、`recover` の `expected_tokens` を JSON で検証するテストを `streaming_runner.rs` に移行。 |
| TPM-DIAG-03 | `effect_analysis_tests.ml` / `effect_handler_poc_tests.ml` | `type_inference_effect.ml`/`impl_registry` | ✅ Stage/Capability レポートが整備されている現在、diagnostics/metrics の `effects.contract.*` での確認が可能。 |

## 4. 移植アプローチ
1. **対象テストの翻訳設計**  
   - OCaml の `compiler/ocaml/tests/*.ml` を参照し、同じ記述（入力 ReML スニペット・期待 JSON）の Rust テストへ抽出する。同じ文字列は `compiler/ocaml/tests/golden/` で管理されており `scripts/poc_dualwrite_compare.sh` の `--case-origin ocaml-tests` モードで Rust CLI を呼び出せるデータとしてまとめる。  
   - `docs/plans/rust-migration/appendix/type-inference-ocaml-inventory.md` や `docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md` に記載されたテストメタ情報を再利用し、分類・`#flags`/`#tests` を `reml` ムービングに転記。  
2. **Rust ハーネスと CLI の拡張**  
   - `scripts/poc_dualwrite_compare.sh` の `--mode typeck|diag` に新しく移植対象ケースを登録し、`reports/dual-write/front-end/w3-*`/`w4-*` へ JSON を書き出す。  
   - ツリーの `compiler/rust/frontend/src/bin/poc_frontend.rs` で `--emit-parsed`/`--emit-typed-ast`/`--emit-constraints`/`--emit-diagnostics` などのフラグを OCaml CLI と一致させる。既存の `typeck/debug` 出力を `reports/dual-write` に入れ込むのと同じ形式で記録。  
3. **Rust テスト追加**  
   - 各カテゴリごとに `compiler/rust/frontend/tests/{parser, lexer, typeck, diagnostics, streaming}` のモジュールを設け、OCaml テストの期待値（文字列・JSON）と同じ名前で `#[test]` を追加。  
   - JSON 比較では `serde_json::from_str` → `Value::as_object` で `span`/`stage`/`dict_refs` などを厳密一致させ、`reports/dual-write` の `.json` と `compiler/ocaml/tests/golden/` との diff を `scripts/poc_dualwrite_compare.sh` で生成。  
4. **メトリクス/監査**  
   - `collect-iterator-audit-metrics.py --section lexer,typeck,effects` を Rust 実装でも実行し、`effects.impl_resolve.delta` や `lexer.identifier_profile_*` の `±0.5` ルールを `docs/plans/rust-migration/p1-front-end-checklists.csv` へ記録。  
   - `scripts/validate-diagnostic-json.sh` に Rust 実装向けサブコマンド (`--frontend rust`) を追加し、`diagnostic-v2.schema.json` との比較を `reports/dual-write/w4-diagnostics` に保管。  
5. **フォローアップと記録**  
   - 移植したテストは `docs-migrations.log` に「P1 テスト移植」のエントリを追加し、対象ファイル・双方向実行結果・未対応項目を簡潔に記録する。  
   - 移植不能またはバックエンド依存のテスト（例: `test_cli_llvm.reml`, `test_runtime_integration.sh`）は `docs/plans/rust-migration/p1-spec-compliance-gap.md` で `SCG-xx` を追記し、Phase P1 以降に deferred する旨を明示する。

## 5. 進捗管理と次のアクション
- 最初の段階で `ledger` 対応ケース（`core_parse_lex_tests.ml` など）を Rust フロントエンドの `compiler/rust/frontend/tests/parser/` に移して `cargo test` を通し、`reports/dual-write/front-end/w1-parser` に JSON を保存しない問題を検知する。  
- Type inference/diagnostics のケースは `scripts/poc_dualwrite_compare.sh --mode typeck`/`--mode diag` を実行し、`reports/dual-write/front-end/w3-type-inference/<case>`/`w4-diagnostics/<case>` に `typed-ast.{ocaml,rust}.json`、`constraints.{ocaml,rust}.json`、`diagnostics.*.json` を溜める。  
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ `W4` 系の進捗を随時追記し、未完了の `ffi_dispatch_async` 等は `p1-spec-compliance-gap` とのリンクを維持する。  
- 次は `compiler/rust/frontend/tests` 以下に最初の 6 個のテストを書き起こし、Dual-write JSON のフォーマット差分を `reports/dual-write/front-end/w3-typing/` に収録する。変更により `README.md` 等への参照パスが変わる場合は `docs/plans/bootstrap-roadmap/SUMMARY.md` も更新する。

## 6. 具体的な計画

### TPM-LEX-01

1. **調査・前提整理（1日）**  
   1. `compiler/ocaml/tests/core_parse_lex_tests.ml` の各ケース（UTF-8、エスケープ、コメント、マルチラインリテラルなど）の入力 ReML スニペットと期待トークン列を一覧化し、`docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md` と `docs/plans/rust-migration/p1-front-end-checklists.csv` に記載されている要件（`lexer.identifier_profile_*` など）と照合します。  
   2. Rust 側で `lexer_token_coverage` に再現できるトークン列の構造（`TokenKind`/`Lexer` ルール）を把握し、`scripts/poc_dualwrite_compare.sh --mode lexer` で OCaml 由来のケースを CLI に渡せるよう入力フォーマットを定義します。
2. **テストハーネス設計（0.5日）**  
   1. 新規モジュール `compiler/rust/frontend/tests/lexer/core_parse.rs` を作り、`core_parse_lex_tests.ml` 相当の `#[test]` を `reml` の文字列リテラル＋期待トークン列で定義。テストは `lexer::tokenize` → `serde_json::to_string_pretty` で得られたトークン配列を `compiler/ocaml/tests/golden/core_parse_lex_tests.tokens.json` と比較する。  
   2. `scripts/poc_dualwrite_compare.sh --mode lexer` に `--case-origin ocaml-core-parse` を追加し、生成 JSON を `reports/dual-write/front-end/w1-lexer/core_parse_lex_tests/` に書き出して `collect-iterator-audit-metrics.py` で `lexer.identifier_profile_*` を計測するフックを呼び出す。
3. **実装・移植（1日）**  
   1. Rust 側の `lexer_token_coverage.rs` に `core_parse` 系ケースを `#[test_case(...)]` で統合し、`TokenProfile` が `lexer.identifier_profile_*` に近い分布を維持していることを `assert!` で検証。  
   2. `compiler/rust/frontend/src/bin/poc_frontend.rs` に `--emit-tokens` フラグを追加し、`lexer::tokenize` と `serde_json` で同名ファイルを出力するようにすることで、`scripts/poc_dualwrite_compare.sh` でも同じ golden データを確認できる。
4. **検証・監査（0.5日）**  
   1. `cargo test --test lexer_core_parse` を実行し、`reports/dual-write/front-end/w1-lexer/core_parse_lex_tests/tokens.{ocaml,rust}.json` に同一のトークン列（`TokenKind`/`span`）が出力されることを確認。  
   2. `collect-iterator-audit-metrics.py --section lexer --case core_parse_lex_tests` でメトリクスを取り（±0.5 ルール）、結果を `docs/plans/rust-migration/p1-front-end-checklists.csv` に書き込む。  
5. **記録・フォローアップ（0.25日）**  
   1. `docs-migrations.log` に「TPM-LEX-01: core_parse_lex_tests.lexer トークン移植」として成果・差分・未再現ケースを記録し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の診断 JSON 進捗欄にも言及。  
   2. 未移植の corner case（`CPP-style` コメント、Zero-width space など）が残っている場合は `docs/plans/rust-migration/p1-spec-compliance-gap.md` に `SCG-xx` として再分類し、将来の deferred リストに追加。

#### TPM-LEX-01 進捗

- Rust 側で `core_parse_lex_tests.ml` 相当のトークン列を `compiler/ocaml/tests/golden/core_parse_lex_tests.tokens.json` にゴールデン化し、`compiler/rust/frontend/tests/lexer/core_parse.rs` で `lexer::lex_source_with_options` 結果を厳密一致させることで `cargo test --test lexer_core_parse` だけで移植済みケースの差分が検知できるようになった。  
- `poc_frontend` に `--emit-tokens` フラグを追加し、Dual-write のトークン JSON を `reports/dual-write/front-end/w1-lexer/core_parse_lex_tests/` 以下に書き出して `collect-iterator-audit-metrics.py` の `lexer.identifier_profile_*` 指標に流し込むパイプラインを確立。`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` にもこのワークフローを追記した。  
- 実装済み: `scripts/poc_dualwrite_compare.sh --mode lexer` に `TPM-LEX-01` ケースを登録し、`docs/plans/bootstrap-roadmap/p1-test-migration-lexer-cases.txt` で `compiler/ocaml/tests/golden/core_parse_lex_tests.tokens.json` を読み込んだトークン列を Rust 出力（`reports/dual-write/front-end/w1-lexer/<run>`）と比較して `tokens.{ocaml,rust}.json`/`tokens.diff.json` を記録、OCaml/Rust の診断 JSON を `collect-iterator-audit-metrics.py --section lexer --case core_parse_lex_tests` へ与えて `lexer-metrics.{ocaml,rust}.json` を生成し、`lexer.identifier_profile_unicode`/`lexer.identifier_profile_ascii` の ±0.5 ルールを追跡する運用を確立した。

### TPM-LEX-02

1. **調査・前提整理（1日）**
   1. `compiler/ocaml/tests/test_lexer.ml` の識別子・Unicode 検証（`test_identifiers`〜`test_lexer_errors`）と `compiler/ocaml/tests/unicode_ident_tests.ml` の承認ケースを読み、`docs/spec/1-1-syntax.md §A.3` / `docs/spec/1-4-test-unicode-model.md` の要件と照らし合わせて必要なコードポイント群（ひらがな、カタカナ、大文字ラテン、ギリシャ・キリル・ハングル、合成文字）および ASCII 互換拒否メッセージ（`U+89E3`, `profile=ascii-compat`）を整理します。
   2. `unicode_identifiers.reml` フィクスチャと `REML_ENABLE_UNICODE_TESTS` の制御フローを確認し、Rust 側で `IdentifierProfile::AsciiCompat`／`IdentifierProfile::Unicode` を切り替える `--identifier-profile` フラグや環境変数の扱いを定義します。`docs/plans/rust-migration/p1-front-end-checklists.csv` の `lexer.identifier_profile_*` 項目に対応するメトリクスを再現するための出力フォーマット（`serde_json` で `TokenKind`・`span` を記録）も明文化します。
2. **テストハーネス設計（0.5日）**
   1. `compiler/rust/frontend/tests/lexer/identifier.rs` モジュールを追加し、OCaml の `test_identifiers`/`unicode_ident_tests` で使われるラベルごとに `#[test_case]` を定義。`IdentifierProfile::AsciiCompat` と `IdentifierProfile::Unicode` で期待トークン・正規化文字列・`span` を `serde_json` 出力や `Token::to_string` で確認できるようにし、`compiler/ocaml/tests/golden/` に相当する Rust 用ゴールデン JSON（`identifiers.{ocaml,rust}.json`）を扱えるようにします。
   2. `scripts/poc_dualwrite_compare.sh --mode lexer` に `--case-origin ocaml-identifier` を追加し、ASCII プロファイル拒否ケースと Unicode 合格ケースの両方を `reports/dual-write/front-end/w1-lexer/identifier/` 以下へ吐き出す。`collect-iterator-audit-metrics.py --section lexer` が `identifier_profile_ascii`/`identifier_profile_unicode` に記録する差分を `docs/plans/rust-migration/p1-front-end-checklists.csv` にリンクします。
3. **実装・移植（1日）**
   1. Rust 側で ASCII 用のエラーメッセージが `test_lexer.ml` で期待される文字列（`U+89E3` を含み `profile=ascii-compat` を出力）および `span` 値と一致することを `#[test]` で検証し、`IdentifierProfile::AsciiCompat` から `capture_lexer_error` 風の `Result<Token, LexerError>` を生成するユーティリティを作る。
   2. Unicode プロファイルでは `Unicode` 文字も `IDENT`/`UPPER_IDENT` として受理され、正規化（`café`）や補助ケース（ゼロ幅結合子）も `TokenKind` 文字列と `Token::lexeme()` を比較して得られるようにする。`compiler/rust/frontend/src/bin/poc_frontend.rs` に `--identifier-profile`＋`--emit-tokens` を追加して Dual-write JSON を `reports/dual-write/front-end/w1-lexer/identifier/{ocaml,rust}` に並べます。
4. **検証・監査（0.5日）**
   1. `cargo test --test lexer_identifier` で ASCII/Unicode ケースを通し、`reports/dual-write/front-end/w1-lexer/identifier/tokens.{ocaml,rust}.json` に `TokenKind`・`span` が一致することを確認。Unicode スキップフラグ（`REML_ENABLE_UNICODE_TESTS=0`）が動作することを `cargo test --test lexer_identifier --no-unicode` で確かめます。
   2. `collect-iterator-audit-metrics.py --section lexer --case identifier` を実行し、ASCII 拒否時の `identifier_profile_ascii` と Unicode 受理時の `identifier_profile_unicode` が ±0.5 以内に収まるよう `docs/plans/rust-migration/p1-front-end-checklists.csv` に追記します。`scripts/validate-diagnostic-json.sh` の `--frontend rust --case identifier` を使ってメッセージスキーマの互換性を検証。
5. **記録・フォローアップ（0.25日）**
   1. `docs-migrations.log` に「TPM-LEX-02: test_lexer/unicode_ident_tests の識別子プロファイル移植」の記録を書き、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の診断 JSON 更新欄で ASCII/Unicode リジェクト／アクセプトを明記する。
   2. Unicode 観点でまだ再現できない項目（例: `ZERO WIDTH` 文字の `span` や `CPP-style` コメントを挟んだ際の `profile` 表示）が残る場合は `docs/plans/rust-migration/p1-spec-compliance-gap.md` で `SCG-xx` にまとめ、Phase P2 以降の deferred リストに誘導する。

#### TPM-LEX-02 進捗

- `test_lexer.ml` および `unicode_ident_tests.ml` のケースを読み込み、各ラベルが期待する `TokenKind`・`span`・エラーメッセージを整理。`unicode_identifiers.reml` の入力と `REML_ENABLE_UNICODE_TESTS` のフローを押さえた上で、Rust の `IdentifierProfile` 切り替えが同等の挙動を提供するような `scripts/poc_dualwrite_compare.sh` のパラメータ設計を確定した。
- `docs/spec/1-1-syntax.md §A.3`/`docs/spec/1-4-test-unicode-model.md` および `docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md` と突き合わせながら、ASCII 互換拒否メッセージの文言 (`U+89E3`, `profile=ascii-compat`) と Unicode 受理の正規化ケース（`café` やゼロ幅結合子）を `docs/plans/bootstrap-roadmap/p1-test-migration-plan.md` に列挙し、Dual-write JSON のメトリクス収集と `docs/plans/rust-migration/p1-front-end-checklists.csv` への記録方針を明文化した。
- Rust 側に `lexer/identifier.rs` テストを追加し、ASCII プロファイルの拒否メッセージ（コードポイント/プロファイル/Span）と Unicode プロファイルの正規化/ゼロ幅結合子の動作を検証するケースを定義。Normalization に `unicode-normalization` を導入、ASCII 拒否時の `FrontendErrorKind::UnexpectedStructure` と `TokenKind::Unknown` を使って `push_ascii_error` をリファクタリングした。
- `compiler/ocaml/tests/golden/identifier_lex_tests.tokens.json` を作成して `poc_frontend --emit-tokens` の出力を集約し、`docs/plans/bootstrap-roadmap/p1-test-migration-lexer-cases.txt` に `TPM-LEX-02` エントリを追加することで `scripts/poc_dualwrite_compare.sh --mode lexer` から `reports/dual-write/front-end/w1-lexer/identifier` に JSON/metrics を蓄積できるようにした。

### TPM-LEX-03

1. **調査・前提整理（1日）**  
   1. `compiler/ocaml/tests/packrat_tests.ml`/`test_parser.ml`/`test_parser_driver.ml` と `parser_driver.ml` を読み、`Parser_driver.run_string`/`ParseResult` がどのような `diagnostics` 期待情報・`packrat_stats`・`span_trace` を保持しているか、`docs/spec/1-5-formal-grammar-bnf.md` に定められる期待候補の構造（`Diagnostic.expectation_summary`）と照らし合わせて整理。  
   2. `Parser_run_config`/`RunConfig` の制御点（`packrat` フラグ、`left_recursion` など）と、`packrat_tests.ml` が検証している `Core_parse` 由来メタデータ・Packrat メトリクス出力の関係を `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` で定義された監査項目と結び付けて記録。

2. **テストハーネス設計（0.5日）**  
   1. `compiler/rust/frontend/tests/parser/packrat.rs` を追加し、OCaml 版と同様のサンプル入力を `ParserDriver::parse_with_options_and_run_config` で処理して `ParseResult` の `packrat_stats` や `diagnostics` を `serde_json` でダンプし、Golden AST（`reports/dual-write/front-end/ocaml/packrat/<case>/ast.json`）と比較できる構成を設計。  
   2. `scripts/poc_dualwrite_compare.sh --mode ast` の既存ケース定義に `TPM-LEX-03` エントリを追加し、OCaml 側の `packrat_tests` 出力（`compiler/ocaml/tests/golden/packrat_*.json` など）と Rust 側の `reports/dual-write/front-end/w2-ast-alignment/<run>/` 以下の `parse_result.{ocaml,rust}.json` を並列に収集するパラメータ設計を決める。

3. **実装・移植（1日）**  
   1. Rust の `parser` テストハーネスで `ParserDriver` 実行結果から `diagnostics` の `recover`/`expected_tokens` 拡張や `packrat_stats` を `serde_json` で保存し、`compiler/ocaml/tests/packrat_tests.ml` が期待する `parser_id`/`namespace`/`origin` を `PARSER_NAMESPACE`/`PARSER_NAME` にそろえた JSON を生成。  
   2. `poc_frontend` に `--emit-ast` 出力と `write_dualwrite_parse_payload` を `scripts/poc_dualwrite_compare.sh` の `TPM-LEX-03` ケースにフックして、日本語の Golden AST/diagnostic メタデータを `reports/dual-write/front-end/w2-ast-alignment/<run>/<case>` に `ast.{ocaml,rust}.json`・`parse_result.{ocaml,rust}.json` で残す。  
   3. `Parser_run_config` の `packrat`/`streaming`/`left_recursion` のフラグを CLI `--run-config` で再現するため、`docs/plans/rust-migration/p1-front-end-checklists.csv` に必要な `run_config` パターンを追記。

4. **検証・監査（0.5日）**  
   1. `cargo test --test parser_packrat` を実行し、`reports/dual-write/front-end/w2-ast-alignment/<run>/packrat` 以下の `parse_result.{ocaml,rust}.json` が同一 `diagnostics.expected` の候補や `packrat_stats` 比、`value`/`span` 形状を保持することを確認。  
   2. `scripts/poc_dualwrite_compare.sh --mode ast --cases docs/plans/bootstrap-roadmap/p1-test-migration-parser-cases.txt` を用い、OCaml/Rust の AST 比較レポート（`ast.diff.json`）と `collect-iterator-audit-metrics.py --section parser --case packrat` で得たメトリクスを `docs/plans/rust-migration/p1-front-end-checklists.csv` に書き込み、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` に親和性がある監査結果を記録。

5. **記録・フォローアップ（0.25日）**  
   1. `docs-migrations.log` に「TPM-LEX-03: packrat/test_parser/test_parser_driver の AST/ParseResult 移植」として結果を記録し、既存の `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` のテスト/診断整合欄にも言及。  
   2. `TPM-LEX-03` で再現できなかった期待候補（`left_recursion` シナリオや Packrat キャッシュ不足）を `docs/plans/rust-migration/p1-spec-compliance-gap.md` に `SCG-xx` で収束させ、Phase P2 以降の補遺計画へつなげる。

#### TPM-LEX-03 進捗

- `compiler/ocaml/tests/packrat_tests.ml`/`test_parser.ml`/`test_parser_driver.ml` を読み、`Parser_driver.parse_string`/`run_string` が出力する `ParseResult` の `diagnostics` `packrat_stats` `span_trace` の構造と `diagnostic.expected` のサマリ候補を把握し、`Parser_run_config` のフラグを `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の監査指標に対応付けた。  
- `parser_driver.ml` で `read_core_rule_metadata` `packrat_cache` `packrat_stats` を保持する箇所を確認し、Rust 側 `poc_frontend` の `--emit-ast`/`write_dualwrite_parse_payload` 出力に対応するよう `reports/dual-write/front-end/w2-ast-alignment` の構成を想定した。  
- `scripts/poc_dualwrite_compare.sh --mode ast` の既存フローを踏まえ、OCaml/Rust の `ast.{ocaml,rust}.json`、`parse_result.{ocaml,rust}.json` を `reports/dual-write/front-end` に保存しつつ `collect-iterator-audit-metrics.py --section parser` へ渡す運用案を plan に落とし込んだ。
- `compiler/rust/frontend/tests/parser.rs` および `tests/parser/packrat.rs` に `ParserDriver` の Packrat 統計・`diagnostics.expected_summary` を検証する統合テストを実装し、`cargo test --test parser` で `packrat_stats` クエリ/ヒット、`span_trace`、`packrat_cache`、期待候補の代替表現をカバーするエントリを追加した。

### TPM-LEX-04

1. **調査・要件整理（1日）**  
   1. `compiler/ocaml/tests/test_parser_expectation.ml` と `test_parse_result_state.ml` を読み込み、`Parser_expectation` が `Diagnostic.expected` を補完するために提供する `Keyword`/`Token`/`Class`/`Rule`/`Not`/`Custom` の振る舞いと優先度/空集合補正ロジックを `docs/spec/1-1-syntax.md §A.3`／`docs/spec/1-4-test-unicode-model.md` のトークン分類と対照し、Rust 側の `token_kind_expectations`/`ExpectedTokenCollector`—`FrontendDiagnostic` の連携で再現すべき期待候補の出力要件をまとめる。  
   2. `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` および `2-7-deferred-remediation.md` が提示する診断 JSON メトリクス（`parser.expected_summary_presence`，`parser.expected_tokens` など）と `docs/plans/rust-migration/1-0-front-end-transition.md#expected-summary` の `ExpectedTokenCollector` 拡張を再確認し、Rust パーサが `legacy_error`/`diagnostics.expected_summary` に留めるべき情報と、OCaml パッケージが期待する `farthest_error_offset`・`legacy_error.expected` の構造を整理する。

2. **テストハーネス設計（0.5日）**  
   1. `compiler/rust/frontend/src/parser/mod.rs` の `expectation_tests` に `ExpectedTokenCollector`＋`FrontendDiagnostic::apply_expected_summary` を使って空集合補正後に `解析継続トークン` プレースホルダと `parse.expected.empty` キーが設定されることを確認するユニットテストを追加し、OCaml `ensure_minimum_alternatives` 相当の契約を担保する。  
   2. 同ファイルに `parse_result_tests` モジュールを新設し、`ParserDriver::parse("fn broken( ->")` や `ParserDriver::parse("fn @@@")` を実行して `value`/`diagnostics`/`farthest_error_offset`/`legacy_error.expected`/`diagnostics.expected_summary` の期待値を検証するテストケースを定義する。これにより `test_parse_result_state.ml` の `run_string` 期待値が Rust ビルドでもカバーされる。

3. **実装・移植（1日）**  
   1. 上記テストを `parser/mod.rs` の `#[cfg(test)]` 内に実装し、`token_kind_expectations` を利用した `Keyword`/`Identifier` の分類と `ExpectedTokenCollector` の `summarize()` 出力順が `Humanize` の `ここで…` メッセージと整合することを検証する。  
   2. `ParserDriver::parse` の結果から `ParseResult` を観察し、`legacy_error` が `ExpectedToken` を必ず持ち、`diagnostics.first().expected_summary` が `has_alternatives()` を満たすことを `cargo test --test parser` で確認できるようにすることで `test_parse_result_state` の `legacy_expected` 期待値を満たす。

4. **検証・監査（0.5日）**  
   1. `cargo test --test parser` を実行し、`parser.rs`（Packrat 統計）に加えて期待候補/Legacy 期待値テストもグリーンとなることを確認してから、`reports/dual-write/front-end/w4-diagnostics` 向けの `parser-metrics` スキーマに `expected_summary` が出力されることを `collect-iterator-audit-metrics.py` の `parser.expected_summary_presence` 条件と合わせて再点検する。  
   2. 新規テストで依存する `ExpectedTokenCollector` の `humanize` 文字列が日本語表現（`ここで...`）を出力することと、`FrontEndDiagnostic::apply_expected_summary` により `expected_tokens` が空のとき `EXPECTED_PLACEHOLDER_TOKEN` へフォールバックする挙動を確認する。

5. **記録・フォローアップ（0.25日）**  
   1. `docs-migrations.log` に「TPM-LEX-04: parser期待候補/ParseResult 状態テストの Rust 移植」として差分と進捗を記録し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の `parser.expected_summary` 監査欄と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の `legacy_error` 追跡欄へ「Rust 側テストを追加」旨を追記する。  
   2. `docs/plans/rust-migration/1-0-front-end-transition.md` の expected-summary セクションに、`ParserDriver::parse` 実行例を `reports/dual-write/front-end/w4-diagnostics/` の `parser-metrics.rust.json` から得た結果と合わせて既存の `diags/expected_summary_presence` 条件へリンクする注記を追記する（必要に応じて P2 以降の補遺へ引き渡す）。

#### TPM-LEX-04 進捗

- `compiler/ocaml/tests/test_parser_expectation.ml` と `test_parse_result_state.ml` を精査し、OCaml の `Diagnostic.expected` に期待される `ExpectedToken` の優先順位・空集合補正・`legacy_error` との関係を抽出した。  
- `compiler/rust/frontend/src/parser/mod.rs` の `expectation_tests` に `ExpectedTokenCollector` 経由で空集合補正後の `解析継続トークン` プレースホルダを確認するユニットテストを追加し、`parse_result_tests` モジュールを新設して `ParserDriver::parse` の失敗結果で `farthest_error_offset`/`legacy_error.expected`/`diagnostics.expected_summary` を検証するテスト群を実装した。  
- `cargo test --test parser` を実行し、新規テストを含むパーサモジュール全体がグリーンであることを確認した。  
- `docs-migrations.log` に本作業の記録（TPM-LEX-04）を追加し、関連する `docs/plans/bootstrap-roadmap/{2-5-spec-drift-remediation.md,2-7-deferred-remediation.md}` と `docs/plans/rust-migration/1-0-front-end-transition.md` への追記を今後検討する旨をコメントとして残した。  

### TPM-TYPE-01

1. **調査・前提整理（1日）**  
   1. `compiler/ocaml/tests/test_type_inference.ml` と `docs/plans/rust-migration/appendix/type-inference-ocaml-inventory.md` を横断し、パターンマッチ・型パラメータ・スキームがどのように `TypecheckReport` の `typed_module`/`constraints`/`violations` に対応するかを整理する。特に `p1-spec-compliance-gap.md` の `FRG-12` が要求する Typed AST/Constraint/Scheme の JSON スキーマ構造と照合する。  
   2. `docs/plans/bootstrap-roadmap/p1-test-migration-plan.md` と `docs/plans/rust-migration/p1-front-end-checklists.csv` に必要な Dual-write case 名と期待メトリクス（`constraints_total`・`typed_functions`・`violations`）を記載し、`reports/dual-write/front-end/w3-type-inference/<case>` に保存する JSON 出力のプロパティを定義する。  
2. **ハーネス設計（0.5日）**  
   1. `scripts/poc_dualwrite_compare.sh --mode typeck` に `--cases docs/plans/bootstrap-roadmap/p1-test-migration-typeck-cases.txt` を追加し、`parser_driver` で再現できる ReML ソースのタプル型・コンストレイントテストを case 化して `reports/dual-write/front-end/w3-type-inference/<run>/<case>/` に `typed-ast.{ocaml,rust}.json`・`constraints.{ocaml,rust}.json` を出力する。  
   2. `compiler/rust/frontend/src/bin/poc_frontend.rs` に `--emit-typed-ast`／`--emit-constraints` を整備し、`TypecheckDriver` の `typed_module`/`constraints` を `serde_json` で書き出す処理を `typeck` モードに追加する。  
3. **実装（1日）**  
   1. `compiler/rust/frontend/tests/typeck_*` 下に `typeck_inference_report.rs` などのテストモジュールを作り、OCaml 側の `test_type_inference.ml` に対応する Typed AST（関数・パラメータ・body）・制約・スキームを Rust でも生成できることを `serde_json` で検証する。  
   2. 上記テストで生成した `TypecheckReport` を `serde_json::to_value` して `typed_module.functions`・`constraints`・`metrics` のキーが揃うことを確認する。これにより `reports/dual-write/front-end/w3-type-inference` へ出力する JSON のスキーマと配列サイズの前提が担保される。  
   3. `scripts/poc_dualwrite_compare.sh --mode typeck` で OCaml/Rust の `typed-ast` と `constraints` を比較して `reports/dual-write/front-end/w3-type-inference/<run>` に保存する処理をアサートするテストを `collect-iterator-audit-metrics.py --section typeck --case <case>` でトリガー可能にする。  
4. **検証・監査（0.5日）**  
   1. `cargo test --test typeck_inference_report` と `scripts/poc_dualwrite_compare.sh --mode typeck --cases docs/plans/bootstrap-roadmap/p1-test-migration-typeck-cases.txt` を走らせ、`reports/dual-write/front-end/w3-type-inference/<run>/<case>` に `typed-ast.{ocaml,rust}.json`・`constraints.{ocaml,rust}.json`・`typeck-debug.*` を溜める。  
   2. `collect-iterator-audit-metrics.py --section typeck --case <case>` で `constraints_total`/`typed_functions`/`violations` の `±0.5` ルールをチェックし、`docs/plans/rust-migration/p1-front-end-checklists.csv` に書き込んで `reports/dual-write` の JSON との差分を監査。  
5. **記録・フォローアップ（0.25日）**  
   1. `docs-migrations.log` と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の `typeck` セクションを更新し、Rust 側テストの `TypecheckReport` JSON と差分監査の出力パスを記録する。  
   2. `docs/plans/rust-migration/1-0-front-end-transition.md` の `FRG-12`・`FRG-14` で `typed-ast` 出力例を追記し、スキーム・制約・診断の Dual-write 用 CLI フラグ（`--emit-typed-ast`/`--emit-constraints`）を明記する。  

#### TPM-TYPE-01 進捗

- `compiler/ocaml/tests/test_type_inference.ml` を `docs/plans/rust-migration/appendix/type-inference-ocaml-inventory.md` と突き合わせて必要なパターン/制約を洗い出し、`p1-spec-compliance-gap.md` の `FRG-12` に対応する Typed AST/Constraint 出力の要件を整理した。  
- `compiler/rust/frontend/tests/typeck_inference_report.rs` を追加し、`TypecheckReport` の `typed_module.functions`・`constraints`・`metrics` が `serde_json::to_value` で直列化可能であること、パラメータが同一の型変数で共有されること、`constraints` に `Equal` 制約が含まれることを検証するテストを実装した。  
- `cargo test --test typeck_inference_report` を実行して新規テストが通ることを確認し、`TypecheckReport` の JSON 出力が `reports/dual-write/front-end/w3-type-inference` で想定されるスキーマ構造を満たすことと、`typed_functions`/`constraints_total` メトリクスが記録されることを確認した。  
- `docs-migrations.log` に「TPM-TYPE-01: `TypecheckReport` JSON/Constraint テストの Rust 移植」エントリを追加し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と `docs/plans/rust-migration/1-0-front-end-transition.md` への追記を今後検討する旨をコメントとして残した。  

### TPM-TYPE-02

1. **調査・前提整理（1日）**  
   1. `compiler/ocaml/tests/test_constraint_solver.ml`/`test_type_errors.ml`/`test_let_polymorphism.ml` を読み込み、`FRG-12`〜`FRG-14` で求められる Constraint/TyEnv/Scheme 出力、`docs/spec/1-2-types-Inference.md` や `docs/spec/3-6-core-diagnostics-audit.md` でのエラーコード・診断メトリクス要件（`E7001`〜`E7021`、`effects.contract.*`）と照合する。  
   2. `docs/plans/rust-migration/unified-porting-principles.md` の優先順位（振る舞い→設計→実装）と `docs/plans/rust-migration/p1-spec-compliance-gap.md` の `TPM-TYPE-02` 要件を踏まえ、OCaml テストが評価する ConstraintSolver の振る舞い（プリミティブ/複合型/再帰/エラー）と診断メッセージの品質基準を洗い出す。  
   3. `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`/`2-7-deferred-remediation.md` に記載した診断 JSON スキーマと `collect-iterator-audit-metrics.py` の `diagnostics.*` メトリクス項目を参照しつつ、Rust 版で比較すべき `TypecheckReport`・`Diagnostic` のキーを整理し `docs/plans/rust-migration/p1-front-end-checklists.csv` に追記する。  
2. **テストハーネス設計（0.5日）**  
   1. `scripts/poc_dualwrite_compare.sh --mode typeck` の既存フローに ConstraintSolver/TypeError ケースを追加し、`docs/plans/bootstrap-roadmap/p1-test-migration-typeck-cases.txt` と同様に `docs/plans/bootstrap-roadmap/p1-test-migration-constraint-cases.txt` を作成して `reports/dual-write/front-end/w3-type-inference/<case>` に `constraints.{ocaml,rust}.json`・`diagnostics.{ocaml,rust}.json` を格納。  
   2. `collect-iterator-audit-metrics.py` の `--section typeck` に `constraint_errors`/`let_polymorphism` カテゴリを付加し、`metrics` に `violations`・`constraint_graph_cycles` などの比較対象を定義して `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と同期する。  
3. **実装（1日）**  
   1. `compiler/rust/frontend/src/typeck/constraint.rs`/`typeck/driver.rs` の `ConstraintSolver` 実装を補完し、`ConstraintSolver::solve` が複合制約・OccursCheck・ミスマッチを検出するように `tests/constraint_solver.rs` などのユニットテストを追加する。OCaml 側のサンプル（Eq<Tuple>・Eq<Option>・再帰）を Rust でも再現し、`serde_json` 出力で `constraint`/`substitutions` を `reports/dual-write/front-end/w3-type-inference/<run>/constraints.{ocaml,rust}.json` に残す。  
   2. `compiler/rust/frontend/tests/typeck_error.rs` を新設し、`TypecheckDriver` による `E7001`〜`E7021` の診断コード・メッセージ・notes の整合性と `LET` 多相・`TraitConstraintFailure` の扱いを `diagnostics.{rust}` で `diagnostic_summary`/`expected_tokens` を JSON に出力、`scripts/poc_dualwrite_compare.sh --mode typeck` で OCaml の `diagnostics.{ocaml}` と比較。（`docs/spec/3-6-core-diagnostics-audit.md` の診断指標に言及）  
   3. `compiler/rust/frontend/src/diagnostic/json.rs` に `TypecheckReport` 拡張を追加し、`Diagnostic` の `code`/`notes`/`secondary` を `serde` で出力することで `collect-iterator-audit-metrics.py` が期待するスキーマに一致させる。  
4. **検証・監査（0.5日）**  
   1. `cargo test --test constraint_solver`/`cargo test --test typeck_error` を実行し、`reports/dual-write/front-end/w3-type-inference/<run>` に JSON が生成されることを確認したあと、`scripts/poc_dualwrite_compare.sh --mode typeck --cases docs/plans/bootstrap-roadmap/p1-test-migration-constraint-cases.txt` で OCaml/Rust の `diagnostics`/`constraints` に差分がないことを `collect-iterator-audit-metrics.py --section typeck --case <case>` でチェック。  
   2. `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` に `constraint_errors` セクションを追記し、`reports/dual-write/front-end/w3-type-inference` の出力や `collect-iterator` メトリクス（`constraint_graph_cycles`・`violations`）を記録。  
5. **記録・フォローアップ（0.25日）**  
   1. `docs-migrations.log` に「TPM-TYPE-02: ConstraintSolver/type error JSON の dual-write 移植」エントリを追加し、`docs/plans/rust-migration/1-0-front-end-transition.md` および `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` に `--emit-diagnostics`/`--emit-type-errors` フラグと出力フォーマットを追記する。  
   2. `p1-spec-compliance-gap.md` の `SCG-xx` で残り課題（例えば `TraitConstraintFailure` のメトリクスや `let` 多相の generalization）を記録し、Phase P2 以降での改善トラッキングにリンク。  

#### TPM-TYPE-02 進捗

- `compiler/ocaml/tests/test_constraint_solver.ml`/`test_type_errors.ml`/`test_let_polymorphism.ml` を精読し、ConstraintSolver のプリミティブ・複合・再帰・エラーのケースと `E7001`〜`E7021` の診断要件を一覧化し `docs/plans/bootstrap-roadmap/p1-test-migration-typeck-cases.txt` に追加する項目の素案を整理。  
- `docs/spec/1-2-types-Inference.md` と `docs/spec/3-6-core-diagnostics-audit.md` を参照しつつ、`collect-iterator-audit-metrics.py` の `typeck` セクションで比較すべき `violations`/`constraint_graph_cycles`/`diagnostic.secondary` のメトリクスを選定し、`docs/plans/rust-migration/p1-front-end-checklists.csv` へ記入すべき観測ポイントを記録。  
- `docs/plans/rust-migration/unified-porting-principles.md` の「振る舞いが最優先」や `p1-spec-compliance-gap.md` の `FRG-12`/`FRG-13` を踏まえ、Rust 側の `TypecheckReport` JSON で `typed_functions`・`constraints_total` だけでなく `TypecheckError` の `code`/`notes` まで含めるべきことを確認した。  

### TPM-TYPE-03

1. **調査・前提整理（1日）**  
   1. `compiler/ocaml/tests/test_cli_callconv_snapshot.ml` / `test_ffi_contract.ml` を精読し、IR ゴールデン・診断 JSON・監査 `audit.metadata` の構造、`effects.contract.stage_mismatch` などの診断コードと `StageContext`/`RuntimeCapability` に係る監査タグを抽出したうえで `p1-spec-compliance-gap.md#SCG-09` に掲げた残余効果監査の要件と照合する。  
   2. `docs/spec/3-8-core-runtime-capability.md` / `docs/spec/1-3-effects-safety.md` を参照し、`effects.contract.*` メトリクスが何を担保すべきか（Stage フェーズ・Capability レジストリの照合・呼出し規約/効果行の整合性）を整理し、Dual-write で比較すべき JSON キーを `docs/plans/rust-migration/p1-front-end-checklists.csv` に追記する。  

2. **テストハーネス設計（0.5日）**  
   1. `scripts/poc_dualwrite_compare.sh --mode diag` に TPM-TYPE-03 用ケース群（`cli-callconv`、`ffi-contract`）を登録するため、`docs/plans/bootstrap-roadmap/p1-test-migration-ffi-cases.txt` を作成し、`#flags`/`#metrics-case` で `--emit-effects`・`--runtime-capabilities` などの CLI フラグと `effects.contract.stage_mismatch` メトリクスラベルを指定する。  
   2. `reports/dual-write/front-end/w4-diagnostics/effects-contract/<case>` 配下へ `diagnostics.{ocaml,rust}.json`・`audit.{ocaml,rust}.jsonl` を書き出すルールを定め、`collect-iterator-audit-metrics.py --section diag --metrics-case effects-contract` から `effects.contract.stage_mismatch` / `effects.contract.capability_missing` / `effects.contract.ownership` の差分を収集するパイプラインを決める。  

3. **実装（1日）**  
   1. Rust 側に `compiler/rust/frontend/tests/cli/ffi_effects.rs`（仮称）を追加し、`poc_frontend` に `--emit-typed-ast`・`--emit-effects`・`--emit-diagnostics` を渡して OCaml の `cli-callconv`/`ffi-contract` サンプルと同一 IR・診断・監査 JSON を生成し、`effects.contract.stage_mismatch` を含む `Diagnostic`／`Audit` 出力のキー・metadata を比較する。StageContext の run-time フェーズと `RuntimeCapability` の `capability.target`/`capability.stage` を JSON に含めるビヘイビアも確認する。  
   2. `compiler/rust/frontend/src/diagnostic/json.rs` を拡張し、`Cli.Json_formatter` 経由で `FfiContractViolation` の `StageContext` と `RuntimeCapability` を `audit.metadata["effects.contract.*"]` へ埋め込み、OCaml の `Ffi_contract` 監査と schema マッチする出力にする。`scripts/validate-diagnostic-json.sh --frontend rust` を使った schema 検証も想定する。  
   3. `scripts/poc_dualwrite_compare.sh` に `TPM-TYPE-03` 用の `--case-origin`/`--flags` を用意し、`FORCE_TYPE_EFFECT_FLAGS` 環境変数で `--runtime-capabilities` を強制することで `effects`/`ffi` ケースの出力検査を自動化する。Dual-write の `reports/dual-write/front-end/w4-diagnostics/effects-contract/<run>` に `diff.json` も残す。  

4. **検証・監査（0.5日）**  
   1. `cargo test --test cli_ffi_effects`（仮称）や `scripts/poc_dualwrite_compare.sh --mode diag --cases docs/plans/bootstrap-roadmap/p1-test-migration-ffi-cases.txt` を実行し、`reports/dual-write/front-end/w4-diagnostics` に `diagnostics.{ocaml,rust}`・`audit.{ocaml,rust}`・`effects.contract.*.json` を生成したことを確認する。  
   2. `collect-iterator-audit-metrics.py --section diag --metrics-case effects-contract` で `effects.contract.stage_mismatch`/`effects.contract.capability_missing`/`effects.contract.ownership` の差分が許容範囲内（`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` で定義したスキーマ）であることを記録し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#effects-change-log` に該当差分と展開済みケースを記載する。  
   3. `scripts/validate-diagnostic-json.sh --frontend rust --schema diagnostics-v2.schema.json` で JSON schema に則った `Diagnostic`/`Audit` 出力を照合し、`reports/dual-write/front-end/w4-diagnostics/effects-contract/<case>/schema.{ocaml,rust}.log` を保存する。  

5. **記録・フォローアップ（0.25日）**  
   1. `docs-migrations.log` に「TPM-TYPE-03: CLI FFI 効果/契約の dual-write 検証」エントリを追加し、`docs/plans/rust-migration/1-3-dual-write-runbook.md` / `p1-spec-compliance-gap.md#SCG-09` に `--emit-effects`/`StageContext`・`RuntimeCapability` フラグと `effects.contract.*` 出力形式を追記する。  
   2. `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の診断進捗表に TPM-TYPE-03 を追記し、残存する `effects.contract.stage_mismatch` の差分を `p1-spec-compliance-gap.md` の `SCG` 列で `Deferred` に分類したうえで次フェーズの追跡ノートを `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にリンクする。  

#### TPM-TYPE-03 進捗

- `docs/plans/bootstrap-roadmap/p1-test-migration-ffi-cases.txt` を作成し、`cli-callconv`/`ffi-contract` 両ケースに `#origin`/`#metrics-case`/`#flags` を記述して `poc_dualwrite_compare.sh --mode diag` から `effects-contract` ラベル付きで実行可能な構成を整備した。  
- `scripts/poc_dualwrite_compare.sh` を拡張し、ケース定義で `case_origin`/`case_metrics_label` を保持、`case_metrics_label` を `collect-iterator-audit-metrics.py` の `--case`/`--metrics-case` に渡すとともに `FORCE_TYPE_EFFECT_FLAGS` で `effects.contract.*` 出力を強制し、summary.json にメタデータを刻むことで `effects-contract` 系ケースの差分を明示できるようにした。  
- `tooling/ci/collect-iterator-audit-metrics.py` に `--section diag`/`--metrics-case` を導入し、`effects-contract` ケースでは `effects.contract.stage_mismatch`/`capability_missing`/`ownership` をカウントする新規メトリクスを追加して gating に組み込み、`poc_dualwrite_compare.sh` から `diag-metrics.{frontend}.json` が生成されるフローを確立した。  

### TPM-DIAG-01

1. **調査・前提整理（1日）**  
   1. `compiler/ocaml/tests/test_cli_diagnostics.ml` のケース群（`StageContext` による PhaseRack、`Diagnostic` の `aux`/`secondary`、`effects.contract.*` メタデータ）と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` および `2-7-deferred-remediation.md` に掲載された JSON スキーマ（`diagnostics.*.json`、`audit.metadata`、`metrics`）を読み、Rust 側で再現すべき `Diagnostic`/`Audit` の構造と許容差（メトリクスの ±0.5 ルール、`collect-iterator` の `code`/`notes`/`secondary` カウント）を明文化する。  
   2. `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` と `docs/spec/3-6-core-diagnostics-audit.md` を参照し、`scripts/validate-diagnostic-json.sh` で使用する `diagnostics-v2.schema.json`（`diagnostics.*.schema`）や `collect-iterator-audit-metrics.py --section diag` が求めるメトリクスラベル（`diagnostics.count`、`diagnostics.expected_summary_presence`、`diagnostics.kind_summary`）をドキュメント化し、既存の `docs/plans/rust-migration/p1-front-end-checklists.csv` に対応する欄を設ける。  
2. **テストハーネス設計（0.5日）**  
   1. Dual-write 診断比較用に `docs/plans/bootstrap-roadmap/p1-test-migration-diagnostic-cases.txt` を案定義し、`scripts/poc_dualwrite_compare.sh --mode diag` で `test_cli_diagnostics.ml` の CLI フラグ（`--show-stage-context`、`--effects-summary`、`--runtime-capabilities`）と `--emit-diagnostics`/`--emit-audit` を同時に渡せるようにする。  
   2. `reports/dual-write/front-end/w4-diagnostics/cli_diagnostics/<case>` 以下に `diagnostics.{ocaml,rust}.json`、`audit.{ocaml,rust}.json`、`metrics.{ocaml,rust}.json`、`diff.json` を書き出し、`collect-iterator-audit-metrics.py --section diag --metrics-case cli_diagnostics` で `diagnostics.expected_summary`/`diagnostics.count`/`effects.contract.*` を比較できる構成を整理する。  
3. **実装（1日）**  
   1. `compiler/rust/frontend/tests/diagnostics/cli_diagnostics.rs` を新設し、OCaml の `test_cli_diagnostics.ml` に記載された CLI ケース（`stage_callconv`、`ffi_effects`、`diagnostic_notes`）を `poc_frontend` に渡して `diagnostics`/`audit` を `serde_json` で生成、`collect-iterator` に送るテストを追加する。  
   2. `compiler/rust/frontend/src/bin/poc_frontend.rs` に `--emit-diagnostics`/`--emit-audit`/`--emit-effects` を整理し、`collect-iterator` の `DiagnosticMetric`/`AuditMetric` に合わせたフィールド（`code`/`notes`/`secondary`/`metrics`）を JSON に含めるよう `diagnostic/json.rs` を拡張する。  
   3. `scripts/poc_dualwrite_compare.sh` の `diag` モードで `case_metrics_label` と `case_origin` を扱えるようにし、`FORCE_DIAGNOSTIC_FLAGS=1` を使って `--runtime-capabilities` `--effects-summary` を強制、`reports/dual-write/front-end/w4-diagnostics/cli_diagnostics/<run>/diff.json` を残す自動化を追加する。  
4. **検証・監査（0.5日）**  
   1. `cargo test --test cli_diagnostics` と `scripts/poc_dualwrite_compare.sh --mode diag --cases docs/plans/bootstrap-roadmap/p1-test-migration-diagnostic-cases.txt` を実行し、`reports/dual-write/front-end/w4-diagnostics/cli_diagnostics/{ocaml,rust}` に `diagnostics`/`audit`/`metrics` を吐き出す運用を確認。  
   2. `collect-iterator-audit-metrics.py --section diag --metrics-case cli_diagnostics` で `diagnostics.expected_summary_presence`/`diagnostics.count`/`effects.contract.stage_mismatch` を ±0.5 ルールで監査し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#diagnostic-audit` のテーブルに結果を追記する。  
   3. `scripts/validate-diagnostic-json.sh --frontend rust --schema diagnostics-v2.schema.json` を回し、`reports/dual-write/front-end/w4-diagnostics/cli_diagnostics/schema.{ocaml,rust}.log` を残して JSON schema 準拠を担保する。  
5. **記録・フォローアップ（0.25日）**  
   1. `docs-migrations.log` に「TPM-DIAG-01: CLI 診断 JSON の dual-write 移植」エントリを追記し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の診断差分表と `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` の Diagnostic フラグ一覧を更新する旨を記載。  
   2. `p1-spec-compliance-gap.md#SCG-xx` に残る診断コード（例: `E8050` - StageContext の不一致）を deferred に分類し、Phase P2 以降のアクションリンクを `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に張る。  

#### TPM-DIAG-01 進捗

- `compiler/ocaml/tests/test_cli_diagnostics.ml` を読み、CLI (`StageContext`/`RuntimeCapability`/`effects.contract.*`) で出力される `diagnostics`/`audit`/`metrics` の構造と `collect-iterator-audit-metrics.py --section diag` に渡すべきラベル（`diagnostics.count`/`diagnostics.expected_summary_presence`/`effects.contract.stage_mismatch`）を整理した。  
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の診断 JSON スキーマ節と `2-7-deferred-remediation.md` の `effects-contract` ロードマップを参照し、Rust 側にも `diagnostics-v2.schema.json` を `scripts/validate-diagnostic-json.sh` で検証すること、`reports/dual-write/front-end/w4-diagnostics/cli_diagnostics/<case>` に `schema.{ocaml,rust}.log` を残す運用を追記する方針を固めた。  
- 必要 CLI/metrics/Golden JSON の組み合わせを `docs/plans/bootstrap-roadmap/p1-test-migration-plan.md` に記載し、`docs/plans/rust-migration/p1-front-end-checklists.csv` の `collect-iterator` 担当列へ `diag.expected_summary_presence`/`diag.effects.contract.*` を追記する案を立てて、Dual-write の監査ゴールと比較項目を明確にした。  
- `docs/plans/bootstrap-roadmap/p1-test-migration-diagnostic-cases.txt` に `stage_callconv`/`ffi_effects`/`diagnostic_notes` ケースを `#metrics-case: cli_diagnostics` のもとで整理し、`poc_dualwrite_compare.sh --mode diag` で両フロントエンドに `--emit-diagnostics`/`--emit-audit` を併用することで `diagnostics`・`audit`・`diag-metrics` を `reports/dual-write/front-end/w4-diagnostics/cli_diagnostics/<case>` に残せるようにした。  
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と `2-7-deferred-remediation.md` に TPM-DIAG-01 の監査フローと `collect-iterator-audit-metrics.py --section diag --metrics-case cli_diagnostics` の追跡欄を追加し、`docs/plans/rust-migration/1-2-diagnostic-compatibility.md` の比較手順にも CLI diag ケースの記録を参照する旨を追記した。

### TPM-DIAG-02

1. **調査・前提整理（1日）**  
   1. `compiler/ocaml/tests/streaming_runner_tests.ml` と `Parser_driver.Streaming`/`Core_parse_streaming` 実装を読み、`test_streaming_matches_batch`（Completed 対応）と `test_pending_resume_flow`（Pending → resume → Completed）で検証している `StreamOutcome`・`ContinuationMeta`・`Audit`・`Diagnostic.Extensions.recover_expected_tokens` の粒度と `parser_driver.ml` 側の `expected_tokens` 出力を把握する。 `docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md`、`docs/plans/rust-migration/p1-front-end-checklists.csv`、および `docs/plans/rust-migration/unified-porting-principles.md` を参照し、Dual-write では振る舞いの再現が最優先であることを再確認する。  
   2. `docs/spec/2-7-core-parse-streaming.md` と `docs/guides/compiler/core-parse-streaming.md` から `StreamOutcome`/`DemandHint`/`ContinuationMeta.expected_tokens` の仕様と `docs/spec/3-6-core-diagnostics-audit.md` の診断 JSON schema を照合し、`p1-spec-compliance-gap.md#FRG-05` で再掲した Streaming Recover 要件に沿って `expected_tokens` と `recover` の `Diagnostic`/`Audit` フィールドを比較する設計を固める。  
2. **テストハーネス設計（0.5日）**  
   1. `docs/plans/bootstrap-roadmap/p1-test-migration-streaming-cases.txt` を作成し、ストリーミングの代表ケース（Completed の `streaming_matches_batch` / Pending の `pending_resume_flow`）を `#metrics-case: streaming_runner` で整理。`poc_dualwrite_compare.sh --mode diag` に `--emit-expected-tokens streaming_runner` を渡して `reports/dual-write/front-end/w4-diagnostics/streaming_runner/<case>` へ `expected_tokens.{ocaml,rust,diff}.json` を書き出すルールを決める。  
   2. `tooling/ci/collect-iterator-audit-metrics.py --section streaming` で `parser.stream.outcome_consistency`、`parser.stream.backpressure_sync`、`parser.stream.bridge_backpressure_diagnostics`、`parser.stream.demandhint_coverage`、`parser.stream.flow.auto_coverage`、`ExpectedTokenCollector.streaming` といったメトリクスを取得し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` のストリーミング監査欄に差分とスキーマ（`docs/spec/3-6-core-diagnostics-audit.md`）を記録できる運用を定義する。  
3. **実装（1日）**  
   1. `compiler/rust/frontend/tests/streaming_runner.rs` に `chunk_size`/`recover` を明示したケースを追加し、`StreamOutcome::Pending` で `continuation.meta.expected_tokens` の候補数や `pending.audit_events`、`pending.meta.backpressure_policy` が `streaming_runner_tests.ml` と一致することを `assert_eq!` で検証する。  
   2. `compiler/rust/frontend/src/bin/poc_frontend.rs` の `--streaming` パスを強化し、`stream_meta` に `StreamOutcome` の `last_reason`/`resume_count`/`expected_tokens` を含め、`diagnostic/json.rs` で `Diagnostic.Extensions.recover_expected_tokens` を JSON に含めるようにして `collect-iterator` の `ExpectedTokenCollector.streaming` と `recover.expected_tokens` を満たす。`docs/spec/2-7` の `ContinuationMeta.expected_tokens` 出力形式と CLI の `--stream-checkpoint`/`--stream-demand-*` フラグを同期させる。  
   3. `scripts/poc_dualwrite_compare.sh --mode diag` を `streaming_runner` ケースにも使えるよう `--emit-expected-tokens` オプションを CLI から渡し、`reports/dual-write/front-end/w4-diagnostics/streaming_runner/<run>/diff.json` に `expected_tokens`/`diagnostics`/`audit` の差分を残せるようにする（CLI フラグは `--streaming --stream-demand-min-bytes 4 --stream-demand-preferred-bytes 8 --stream-flow-max-lag 8192`）。  
4. **検証・監査（0.5日）**  
   1. `cargo test --test streaming_runner` および `scripts/poc_dualwrite_compare.sh --mode diag --cases docs/plans/bootstrap-roadmap/p1-test-migration-streaming-cases.txt --emit-expected-tokens streaming_runner` を実行し、`reports/dual-write/front-end/w4-diagnostics/streaming_runner/{ocaml,rust}` に `diagnostics`/`audit`/`expected_tokens`/`metrics` を出力する。  
   2. `collect-iterator-audit-metrics.py --section streaming --case streaming_runner` で `parser.stream.*` 系と `ExpectedTokenCollector.streaming` を ±0.5 ルールで比較し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#streaming-audit` に結果を追記する。  
   3. `scripts/validate-diagnostic-json.sh --frontend rust --schema diagnostics-v2.schema.json` を回し、`reports/dual-write/front-end/w4-diagnostics/streaming_runner/<case>/schema.{ocaml,rust}.log` を残して JSON schema 準拠を担保する。  
5. **記録・フォローアップ（0.25日）**  
   1. `docs-migrations.log` に「TPM-DIAG-02: StreamingRunner recover/expected-tokens dual-write」エントリを追加するとともに、`docs/plans/rust-migration/p1-spec-compliance-gap.md#FRG-05` と `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` に `streaming.expected_tokens`/`recover.extensions` フラグを追記する。  
   2. `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` のストリーミング監査行に `collect-iterator-audit-metrics.py --section streaming --case streaming_runner` の結果と `expected_tokens` の `reports/dual-write/front-end/w4-diagnostics/streaming_runner/<case>` 出力を記録し、残る `backpressure`/`resume` 差分を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#streaming-recover` へ移行する。  

#### TPM-DIAG-02 進捗

- `docs/spec/0-1-project-purpose.md` / `docs/plans/rust-migration/overview.md` / `docs/plans/rust-migration/unified-porting-principles.md` を再掲して Phase1 の「振る舞い最優先」方針を確認しつつ、`compiler/ocaml/tests/streaming_runner_tests.ml` と `Parser_driver.Streaming` 側の `expected_tokens`/`continue` ロジック、`docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md`、`docs/plans/rust-migration/p1-front-end-checklists.csv` で `streaming` ケース配置を把握した。  
- `docs/spec/2-7-core-parse-streaming.md` と `docs/guides/compiler/core-parse-streaming.md` から `StreamOutcome`・`ContinuationMeta`・`DemandHint` の仕様を参照し、`FRG-05` の recover/expected token 出力が `docs/spec/3-6-core-diagnostics-audit.md` の JSON schema に則っているかを整理した。  
- `docs/plans/bootstrap-roadmap/p1-test-migration-streaming-cases.txt` を作成し、`compiler/ocaml/tests/golden/streaming/streaming_matches_batch.reml` / `pending_resume_flow.reml` を新設して `poc_dualwrite_compare.sh --mode diag` から `streaming_runner` ケースを `--emit-expected-tokens streaming_runner` で実行できるようにした。 `collect-iterator-audit-metrics.py` の `--section streaming` で `parser.stream.*`・`ExpectedTokenCollector.streaming` を取得する仕様を既存コードから洗い出し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#streaming-audit` で追跡する候補指標を整理した。

### TPM-DIAG-03

1. **調査・前提整理（1日）**
   1. `compiler/ocaml/tests/effect_analysis_tests.ml` / `compiler/ocaml/tests/effect_handler_poc_tests.ml` を精読し、`mut`/`io`/`ffi` タグ収集と `perform`/`handle` による残余効果、`StageContext`・`RuntimeCapability` を含む `Diagnostic`/`Audit` のゴールデータを抽出して `p1-spec-compliance-gap.md#SCG-09` に掲げた残余効果監査要件と突き合わせる。
   2. `docs/spec/1-3-effects-safety.md`・`docs/spec/3-6-core-diagnostics-audit.md`・`docs/spec/3-8-core-runtime-capability.md` を参照して `effects.contract.*` メトリクスが担保すべき Stage フェーズ／Capability レジストリの照合と `@allows_effects`/`effect` 宣言との整合性を整理し、`docs/plans/rust-migration/p1-front-end-checklists.csv` の `collect-iterator` 担当列に比較対象キーを追記する。

2. **テストハーネス設計（0.5日）**
   1. `docs/plans/bootstrap-roadmap/p1-test-migration-effect-cases.txt` を新設し、`scripts/poc_dualwrite_compare.sh --mode diag` に `effects-contract` ラベル付きのケース群（`effect_analysis`、`effect_handler_poc`）と `#flags`/`#metrics-case`/`#origin TPM-DIAG-03` を定義して `--emit-effects`・`--runtime-capabilities`・`--show-stage-context` などの CLI フラグを一元管理する。
   2. `reports/dual-write/front-end/w4-diagnostics/effects-contract/<case>` 下に `diagnostics.{ocaml,rust}.json`・`audit.{ocaml,rust}.jsonl`・`effects.contract.*.json`・`diff.json` を書き出すフォルダ構造を決め、`collect-iterator-audit-metrics.py --section diag --metrics-case effects-contract` で `effects.contract.stage_mismatch`/`capability_missing`/`ownership` を収集できるパイプラインを設計する。

3. **実装（1日）**
   1. Rust 側に `compiler/rust/frontend/tests/diagnostics/effect_analysis.rs` と `effect_handler_poc.rs` を追加し、`EffectAnalysis` 相当の `mut`/`io`/`ffi` タグ検出と `perform`/`handle` 構文の残余タグ・`Console` 相当の `effect` を `assert_eq!` で確認するケースを実装する。
   2. `poc_frontend` の `--emit-effects`/`--emit-diagnostics`/`--emit-audit` を整理し、`diagnostic/json.rs` で `StageContext`/`RuntimeCapability` を `audit.metadata["effects.contract.*"]` に含め、Rust CLI 出力が `effect_handler_poc_tests.ml` の期待と一致するよう `FfiContractViolation` の `capability.stage`/`capability.target` も記録する。
   3. `scripts/poc_dualwrite_compare.sh` の `diag` モードに `effects-contract` ケース（`case_origin`/`case_metrics_label`）と `FORCE_EFFECT_FLAGS` 環境変数を追加し、`collect-iterator-audit-metrics.py` への `metadata` ラベルの伝達と `reports/dual-write/front-end/w4-diagnostics/effects-contract/<run>/diff.json` 出力を自動化する。

4. **検証・監査（0.5日）**
   1. `cargo test --test effect_analysis`・`cargo test --test effect_handler_poc` と `scripts/poc_dualwrite_compare.sh --mode diag --cases docs/plans/bootstrap-roadmap/p1-test-migration-effect-cases.txt` を実行し、`reports/dual-write/front-end/w4-diagnostics/effects-contract/{ocaml,rust}` に `diagnostics`/`audit`/`effects.contract.*` を出力する。
   2. `collect-iterator-audit-metrics.py --section diag --metrics-case effects-contract` で `effects.contract.stage_mismatch`/`capability_missing`/`ownership` の `±0.5` ルールを確認し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と `2-7-deferred-remediation.md#effects-change-log` に監査結果を追記する。
   3. `scripts/validate-diagnostic-json.sh --frontend rust --schema diagnostics-v2.schema.json` で `Diagnostic`/`Audit`/`effects.contract.*` のスキーマ検証を `reports/dual-write/front-end/w4-diagnostics/effects-contract/<case>/schema.{ocaml,rust}.log` へ残す。

5. **記録・フォローアップ（0.25日）**
   1. `docs-migrations.log` に「TPM-DIAG-03: 効果タグと FFI handler の dual-write 監査」エントリを追加し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の `effects-change-log` や `docs/plans/rust-migration/1-3-dual-write-runbook.md` に `--emit-effects`/`StageContext` の出力と `collect-iterator` 指標を追記する。
   2. `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の診断監査表に `effects-contract` 行を加え、`p1-spec-compliance-gap.md#SCG-09` で現状の `effects.contract.stage_mismatch` 差分を `Deferred` に分類し `docs/plans/rust-migration/unified-porting-principles.md` の「振る舞い最優先」方針と合わせて次フェーズのフォローにリンクする。

#### TPM-DIAG-03 進捗

- `compiler/ocaml/tests/effect_analysis_tests.ml` / `effect_handler_poc_tests.ml` を読み、`mut`/`io`/`ffi` タグ検出と `perform`/`handle` の残余タグのチェックポイント、`StageContext`・`RuntimeCapability` を含む `Diagnostic`/`Audit` JSON の構造を整理した。
- `docs/spec/1-3-effects-safety.md`・`docs/spec/3-6-core-diagnostics-audit.md`・`docs/spec/3-8-core-runtime-capability.md`・`docs/plans/rust-migration/p1-spec-compliance-gap.md#SCG-09` を参照し、`effects.contract.*` の監査レベルと `CapabilityRegistry` の Stage 情報を `p1-front-end-checklists.csv` の `collect-iterator` 担当列に追加すべき比較キーとして洗い出した。
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`/`2-7-deferred-remediation.md` の診断差分表と `scripts/validate-diagnostic-json.sh` の schema 検証フローを見直し、`collect-iterator-audit-metrics.py --section diag` に `effects-contract` ケースを渡す運用、`p1-test-migration-effect-cases.txt` に `#metrics-case: effects-contract` などの metadata を含める設計案をまとめた。
