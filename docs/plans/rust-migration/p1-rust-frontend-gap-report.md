# P1 Rust フロントエンド差分調査レポート（2028-02）

本書は Phase P1（フロントエンド移植）で求められる仕様達成度を OCaml 実装と Rust 実装で比較し、未移植項目とフォローアップ作業を整理したものである。`docs/plans/rust-migration/1-0-front-end-transition.md` および `p1-spec-compliance-gap.md` の調査結果を補完し、コード実装箇所と仕様との照合を明文化する。

## 1. 調査方針

- **前提資料**: `docs/spec/0-1-project-purpose.md`, `docs/plans/rust-migration/unified-porting-principles.md`, `docs/plans/rust-migration/overview.md`
- **実装比較**: `compiler/ocaml/src/{parser_driver,parser_expectation,ast,typed_ast,type_inference,diagnostic}.ml` と `compiler/rust/frontend/src/{lexer,token,parser,diagnostic,streaming,typeck}` を中心に比較
- **評価観点**: P1 スコープ（構文解析 / AST & IR / 型推論・制約 / 診断前処理 / Streaming & RunConfig）に対して仕様整合性と dual-write 成果物を確認

## 2. 概要サマリ

| 項目 | 状態 | 主な差分 | 参照 |
| --- | --- | --- | --- |
| Lexer / Parser | 未達 | トークン種別が ASCII 最小集合のみ、Menhir API 相当の `Parser<T>`・`RunConfig` が未移植 | `compiler/rust/frontend/src/token.rs` vs `compiler/ocaml/src/token.ml` |
| AST / IR | 未達 | `ExprKind`/`DeclKind`/`PatternKind` など大半のノードが Rust 版に存在しない。Typed AST JSON も未出力 | `compiler/rust/frontend/src/parser/ast.rs` vs `compiler/ocaml/src/ast.ml` |
| 型推論・制約 | 未達 | Rust `TypecheckDriver` が `SimpleType` で統計のみ出力。HM・制約ソルバ・効果行解析が欠落 | `compiler/rust/frontend/src/typeck/driver.rs` vs `compiler/ocaml/src/type_inference.ml` |
| 診断前処理・JSON | 未達 | `FrontendDiagnostic` に severity/domain/audit が無く、`build_parser_diagnostics` も固定値 | `compiler/rust/frontend/src/diagnostic/mod.rs` vs `compiler/ocaml/src/diagnostic.ml` |
| Streaming & RunConfig | 未達 | `run_stream`/`resume` API が Rust CLI から呼べず、Packrat snapshot や `parser.stream.*` メトリクスを算出できない | `compiler/ocaml/src/parser_driver.ml` vs `compiler/rust/frontend/src/parser/mod.rs` |

## 3. 詳細ギャップ

### 3.1 Lexer / Parser

| ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- |
| トークン網羅性 | `TokenKind` は 30 種弱（ASCII 識別子、`KeywordFn` 等のみ）。Unicode `IDENT` や `UPPER_IDENT`、`var`/`trait`/`handler` など未定義（`compiler/rust/frontend/src/token.rs:7`）。 | `Token.token` は仕様 1-1 §A に沿ってキーワード・演算子・複数基数リテラルを網羅（`compiler/ocaml/src/token.ml:7`）。 | `unicode-ident` 等で XID 判定を導入し、`RunConfig.extensions["lex"]` に応じたプロファイルを Rust Lexer へ追加。 |
| Parser API | `ParserDriver::parse` は `ParsedModule` を返す PoC。`State`/`Reply`/`RunConfig`/`ParseResult` が存在せず、Menhir コンビネータ (`cut`, `attempt`) 未実装（`compiler/rust/frontend/src/parser/mod.rs:26`）。 | OCaml `parser_driver` は `Run_config`／`Core_parse.State` を介し Packrat 状態、`legacy_error`、`span_trace` を `parse_result` へ格納（`compiler/ocaml/src/parser_driver.ml:6`）。 | `docs/spec/2-1-parser-type.md` どおりに `Parser<T>`/`State` 抽象を実装し、`RunConfig`/`ParseResult` を Rust 側へ導入。Menhir 互換コンビネータを `core_parse` 直伝で移植。 |
| `parser_expectation` | Rust `ExpectedTokenCollector` は一部分類のみで、`Not`/`Class`/`TraitBound` のラベル正規化が不足（`compiler/rust/frontend/src/diagnostic/recover.rs:11`）。 | OCaml `parser_expectation.ml` は優先順位・人間可読メッセージ・`recover` 拡張を完全実装（`compiler/ocaml/src/parser_expectation.ml:21`）。 | `parser_expectation` の列挙と整列ロジックを Rust 側へ写経し、`ExpectedToken` 列挙子を仕様 2-5 §B-7 と一致させる。 |

### 3.2 AST / IR

| ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- |
| AST 芝台 | `ast.rs` は `Module/EffectDecl/Function` と 8 種の `Expr` のみ（`compiler/rust/frontend/src/parser/ast.rs:7`）。 | OCaml `Ast` は `expr_kind`, `pattern_kind`, `decl_kind`, `effect_call` 等を全列挙（`compiler/ocaml/src/ast.ml:95`）。 | `1-1-ast-and-ir-alignment.md` の表に沿って全ノードを Rust struct/enum で定義し、JSON ダンプ (`--emit ast-json`) を OCaml と同フォーマットに揃える。 |
| Typed AST / IR | Rust `TypeckArtifacts` は `TypedFunctionSummary` 等の統計 JSON のみで、実際の Typed AST ノードを保持しない（`compiler/rust/frontend/src/bin/poc_frontend.rs:1839`）。 | OCaml `typed_ast.ml` が `typed_expr/typed_decl` を保持し、dual-write で JSON を出力（`compiler/ocaml/src/typed_ast.ml:19`）。 | `TypedAst` モジュールを `crate::semantics::typed` として新設し、`--emit typed-ast` で OCaml の JSON スキーマを再現。 |
| Streaming 状態 | Rust `ParsedModule` は `packrat_stats`/`span_trace` だけ保存し、Packrat cache や `recovered` フラグを返さない（`compiler/rust/frontend/src/parser/mod.rs:26`）。 | OCaml `parse_result` は `packrat_cache`, `recovered`, `farthest_error_offset` 等を保持し CLI/dual-write が利用（`compiler/ocaml/src/parser_driver.ml:27`）。 | Packrat エントリ (`PackratEntry`) のダンプを CLI へ接続し、`parse_result` と同じフィールド集合を Rust でも返却。 |

### 3.3 型推論・制約

| ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- |
| HM 実装 | `TypecheckDriver` は `SimpleType` (Int/Bool/Unknown) を返すのみで、制約生成・一般化・辞書引数が無い（`compiler/rust/frontend/src/typeck/driver.rs:11`）。 | OCaml `type_inference.ml` が Algorithm W + 制約ソルバ + impl レジストリ + 効果解析を実装（`compiler/ocaml/src/type_inference.ml:1`）。 | `types.rs`/`scheme.rs`/`constraint.rs` を分割実装し、`Type_env` と同等の環境を提供。`p1-front-end-checklists.csv` Typed AST 項目を満たす。 |
| 効果行 / Capability | Rust 残余効果検出は `perform` 文字列検索で `TypecheckViolation::residual_leak` を生成する簡易版（`compiler/rust/frontend/src/typeck/driver.rs:232`）。 | OCaml は `Type_inference_effect` と `Effect_profile` で Capability Registry・StageContext を参照（`compiler/ocaml/src/type_inference.ml:80`）。 | `StageContext` と `runtime_capabilities` を `TypecheckDriver` に渡し、Capability Registry との一致判定を Rust でも実装。`effects.contract.*` 診断を JSON へ出力。 |
| dual-write 成果物 | Rust CLI `--emit typed-ast/constraints/typeck-debug` は統計 JSON を作るが AST/constraints 詳細が欠落（`compiler/rust/frontend/src/bin/poc_frontend.rs:1866`）。 | dual-write 手順（`1-3-dual-write-runbook.md`）では OCaml フォーマットと 1:1 の JSON が前提。 | HM 完了後に AST/constraint/export のスキーマを OCaml に合わせ、`reports/dual-write/front-end/w3-type-inference` 形式で保存。 |

### 3.4 診断前処理・JSON

| ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- |
| `Diagnostic` モデル | `FrontendDiagnostic` は `code/message/span/expected` のみで severity/domain/audit/hints を保持しない（`compiler/rust/frontend/src/diagnostic/mod.rs:16`）。 | OCaml `Diagnostic.t` は仕様 3-6 のフィールド（severity/domain/codes/hints/fixits/audit/extensions）を実装（`compiler/ocaml/src/diagnostic.ml:165`）。 | `Diagnostic` 構造体を再定義し、`Diagnostic.Builder` 相当の API を Rust に追加。 |
| JSON エミッタ | `build_parser_diagnostics` が `severity="error"`, `domain="parser"` を固定値で出力し `audit_id` も仮値生成のみ（`compiler/rust/frontend/src/bin/poc_frontend.rs:1192`）。 | OCaml CLI は各診断の severity/domain/audit_metadata を `Diagnostic.Builder` から受け取り JSON 化（`compiler/ocaml/src/parser_driver.ml:68` 等）。 | `Diagnostic` から JSON へ変換する専用モジュールを用意し、`scripts/validate-diagnostic-json.sh` の Schema v2.0.0-draft を満たす。 |
| Recover 拡張 | Rust は streaming recover 用 placeholder のみを挿入し、`Diagnostic.expectation_summary` 由来の `context_note` などを生成しない（`compiler/rust/frontend/src/diagnostic/mod.rs:151`）。 | OCaml `parser_expectation` + `attach_recover_extension` が `expected_tokens` + `message` + `context` を JSON 拡張へ埋め込む（`compiler/ocaml/src/parser_driver.ml:68`）。 | `ExpectedTokensSummary` を JSON へ変換するヘルパーを実装し、`recover_extension_payload` 相当のフィールドを出力。 |

### 3.5 Streaming / RunConfig / dual-write

| ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- |
| Streaming Runner | `StreamFlowState` は checkpoint 数のみ計測し、`run_stream`/`resume`/`Continuation` API が CLI から呼べない（`compiler/rust/frontend/src/streaming/flow.rs:1`）。 | OCaml `Parser_driver.Streaming.run_stream` が `feeder`/`demand_hint`/`resume_hint` を扱い、`recover` 診断を注入（`compiler/ocaml/src/parser_driver.ml:955`）。 | `StreamingRunner` を Rust に実装し、CLI `--streaming` で `run_stream` を呼び出せるよう統合。`docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` の streaming ケースを dual-write 実行。 |
| Packrat メトリクス | Rust `ParsedModule` は `PackratStats` を返すが cache snapshot (`PackratEntry`) を CLI や JSON へ出さない（`compiler/rust/frontend/src/parser/mod.rs:26`）。 | OCaml CLI は `Packrat.dump` を `parse-debug` へ保存し `collect-iterator-audit-metrics.py` が参照（`compiler/ocaml/src/parser_driver.ml:145` + `docs/plans/rust-migration/1-3-dual-write-runbook.md`）。 | Packrat cache の JSON シリアライズを追加し、`reports/dual-write/front-end/*/packrat_cache.json` を出力。 |
| RunConfig 同期 | Rust CLI `RunSettings` は独自フィールドで `parser_run_config` JSON へも落ちない（`compiler/rust/frontend/src/bin/poc_frontend.rs:188` 付近）。 | OCaml `Parser_run_config` が CLI/テスト/Streaming で共有され JSON へ記録（`compiler/ocaml/src/parser_driver.ml:6`）。 | `FrontendConfig`/`RunSettings` を `Run_config` と同構造に再設計し、dual-write レポートに `parser_run_config` を含める。 |

## 4. 推奨フォローアップ

1. **Lexer/Token プロファイル整備**（W4-Parser-Blocker）  
   - `TokenKind` を仕様 1-1 セットに拡張し、`parser_expectation` 等で `ExpectedToken::keyword/token/class` を正しく生成。  
   - `RunConfig.extensions["lex"]` を CLI から Rust Lexer へ伝搬。

2. **Parser ランタイム再構築**（W4-PARSER-API）  
   - `Parser<T>`/`State`/`Reply` 抽象と `ParseResult` フィールドを Rust に導入。  
   - Streaming 状態 (`packrat_cache`, `span_trace`) を CLI/dual-write へ出力。

3. **AST/Typed AST 実装**（W4-AST-IR）  
   - `Ast`/`Typed_ast` の構造を Rust で 1:1 に再現し、`--emit {ast,typed-ast}` の JSON を OCaml と揃える。  
   - `p1-front-end-checklists.csv` AST 項目の完了条件に dual-write Run ID を記録。

4. **HM 型推論 + 効果行**（W4-TYPECK-HM）  
   - `TypecheckDriver` を分割し、Constraint 生成・ソルバ・dict 解決・Stage 監査を Rust へ移植。  
   - `effects.contract.*` 診断と `type_row.*` メトリクスを JSON/LSP へ出力。

5. **診断モデルと JSON エミッタ刷新**（W4-DIAG-V2）  
   - `Diagnostic` 構造・`DiagnosticBuilder` を Rust に導入し、`validate-diagnostic-json.sh` をパスする。  
   - `ExpectedTokenCollector` を OCaml 相当に強化し `recover` 拡張を完全同期。

6. **Streaming Runner 実装**（W4-STREAM-RESUME）  
   - `run_stream`/`resume` を Rust CLI に統合し、`parser.stream.*`/`packrat_snapshot` を dual-write で比較。  
   - `collect-iterator-audit-metrics.py --section streaming` の出力を Rust でも生成できるようにする。

## 5. ノート

- 仕様参照: `docs/spec/1-1-syntax.md`, `1-2-types-Inference.md`, `1-3-effects-safety.md`, `2-1-parser-type.md`, `2-5-error.md`, `2-7-core-parse-streaming.md`, `3-6-core-diagnostics-audit.md`
- 作業ログ: 大きな rename/移動が発生する場合は `docs-migrations.log` を更新すること。
- 本レポートは `p1-spec-compliance-gap.md` の補足資料として扱い、今後の差分調査結果を追記する際はセクション単位で更新する。
