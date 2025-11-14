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
| TPM-LEX-03 | `packrat_tests.ml` / `test_parser.ml` / `test_parser_driver.ml` | `parser_driver.ml` の RunConfig/State | `FRG-07` の `RunConfig`/`Parser<T>` 達成をもとに `parser_driver` と `ParseResult` の Rust 版を `rust` CLI で叩き、ゴールデン AST（`--emit-ast`）を比較。 |
| TPM-LEX-04 | `test_parser_expectation.ml` / `test_parse_result_state.ml` | `parser_expectation` | `FRG-08` で `ExpectedTokenCollector` を Enhancement したため、期待候補の正規化/空集合補正を再現できる。 |

### 3.2 型推論・制約周り
| ID | テスト | 依存対象 | コメント |
| --- | --- | --- | --- |
| TPM-TYPE-01 | `test_type_inference.ml` | `type_inference.ml` + `constraint.ml` + `constraint_solver.ml` | `p1-spec-compliance-gap` の `FRG-12` で要求された Constraint/TyEnv/Scheme が Rust へ揃っているため、型推論ケースの JSON（Typed AST + Constraint + Typecheck Report）を `scripts/poc_dualwrite_compare.sh --mode typeck` で再現し、`reports/dual-write/front-end/w3-type-inference/<case>` に出力。 |
| TPM-TYPE-02 | `test_constraint_solver.ml` / `test_type_errors.ml` / `test_let_polymorphism.ml` | 同上 | それぞれ `Constraint` の解決、型エラー検出、スコープ・一般化の振る舞いを確認するため、Rust 側の `ConstraintSolver` と `TypecheckDriver` が出す `TypecheckReport`/`Diagnostic` に対して JSON で差分を記録。 |
| TPM-TYPE-03 | `test_cli_callconv_snapshot.ml` / `test_ffi_contract.ml` | CLI (`StageContext`/`runtime_capabilities`) + `effect` | CLI 連携が `p1-spec-compliance-gap#SCG-09` や `FRG-13` で整理されているため、Rust CLI へ同じフラグを渡し `--emit-typed-ast`/`--emit-effects` を出力、`reports/dual-write/front-end/w4-diagnostics` に `effects.contract.stage_mismatch` などを記録。 |

### 3.3 診断・Streaming
| ID | テスト | 依存対象 | 理由 |
| --- | --- | --- | --- |
| TPM-DIAG-01 | `test_cli_diagnostics.ml` | `diagnostic.ml` + `Collect-iterator` metrics | `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`/`2-7-deferred-remediation.md` で扱った診断 JSON スキーマと `scripts/validate-diagnostic-json.sh` の整備を Rust でも実行し、`diagnostics.*` のメトリクスを `collect-iterator-audit-metrics.py` で評価。 |
| TPM-DIAG-02 | `streaming_runner_tests.ml` | `parser_driver.ml` + `Core_parse_streaming` | `FRG-05` を踏まえて Rust 版 `StreamingRunner` に `chunk_size` や `recover` 拡張があるため、Pending/Completed コードパスを再現し、`recover` の `expected_tokens` を JSON で検証するテストを `streaming_runner.rs` に移行。 |
| TPM-DIAG-03 | `effect_analysis_tests.ml` / `effect_handler_poc_tests.ml` | `type_inference_effect.ml`/`impl_registry` | Stage/Capability レポートが整備されている現在、diagnostics/metrics の `effects.contract.*` での確認が可能。 |

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
