# P1 Rust フロントエンド差分調査レポート（2028-02）

本書は Phase P1（フロントエンド移植）で求められる仕様達成度を OCaml 実装と Rust 実装で比較し、未移植項目とフォローアップ作業を整理したものである。`docs/plans/rust-migration/1-0-front-end-transition.md` および `p1-spec-compliance-gap.md` の調査結果を補完し、コード実装箇所と仕様との照合を明文化する。

## 1. 調査方針

- **前提資料**: `docs/spec/0-1-project-purpose.md`, `docs/plans/rust-migration/unified-porting-principles.md`, `docs/plans/rust-migration/overview.md`
- **実装比較**: `compiler/ocaml/src/{parser_driver,parser_expectation,ast,typed_ast,type_inference,diagnostic}.ml` と `compiler/rust/frontend/src/{lexer,token,parser,diagnostic,streaming,typeck}` を中心に比較
- **評価観点**: P1 スコープ（構文解析 / AST & IR / 型推論・制約 / 診断前処理 / Streaming & RunConfig）に対して仕様整合性と dual-write 成果物を確認

## 2. 概要サマリ

以下の表では ID（`FRG-XX`）を付与し、未達項目を横断管理できるようにした。

| ID | 項目 | 状態 | 主な差分 | 参照 |
| --- | --- | --- | --- | --- |
| FRG-01 | Lexer / Parser | 未達 | トークン種別が ASCII 最小集合のみ、Menhir API 相当の `Parser<T>`・`RunConfig` が未移植 | `compiler/rust/frontend/src/token.rs` vs `compiler/ocaml/src/token.ml` |
| FRG-02 | AST / IR | 未達 | `ExprKind`/`DeclKind`/`PatternKind` など大半のノードが Rust 版に存在しない。Typed AST JSON も未出力 | `compiler/rust/frontend/src/parser/ast.rs` vs `compiler/ocaml/src/ast.ml` |
| FRG-03 | 型推論・制約 | 未達 | Rust `TypecheckDriver` が `SimpleType` で統計のみ出力。HM・制約ソルバ・効果行解析が欠落 | `compiler/rust/frontend/src/typeck/driver.rs` vs `compiler/ocaml/src/type_inference.ml` |
| FRG-04 | 診断前処理・JSON | 未達 | `FrontendDiagnostic` に severity/domain/audit が無く、`build_parser_diagnostics` も固定値 | `compiler/rust/frontend/src/diagnostic/mod.rs` vs `compiler/ocaml/src/diagnostic.ml` |
| FRG-05 | Streaming & RunConfig | 未達 | `run_stream`/`resume` API が Rust CLI から呼べず、Packrat snapshot や `parser.stream.*` メトリクスを算出できない | `compiler/ocaml/src/parser_driver.ml` vs `compiler/rust/frontend/src/parser/mod.rs` |

## 3. 詳細ギャップ

### 3.1 Lexer / Parser

| ID | ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- | --- |
| FRG-06 | トークン網羅性 | ✅ `token.rs`/`lexer/mod.rs` を刷新し 38+ キーワード／演算子 26 種／`IdentifierProfile` 実装済み。CLI からは常に `Unicode` を指定中で、RunConfig 伝播と dual-write メトリクス更新が未着手。 | `Token.token` は仕様 1-1 §A に沿ってキーワード・演算子・複数基数リテラルを網羅（`compiler/ocaml/src/token.ml:7`）。 | RunConfig/LSP/CLI で `identifier_profile` を切替・記録し、`collect-iterator-audit-metrics.py` の `lexer.identifier_profile_*` を再測定。ASCII 互換モードでの CLI/LSP ゴールデン実行を追加。 |
| FRG-07 | Parser API | ✅ `ParserDriver::parse` は `ParsedModule` を返す PoC。`State`/`Reply`/`RunConfig`/`ParseResult` が存在せず、Menhir コンビネータ (`cut`, `attempt`) 未実装（`compiler/rust/frontend/src/parser/mod.rs:26`）。 | OCaml `parser_driver` は `Run_config`／`Core_parse.State` を介し Packrat 状態、`legacy_error`、`span_trace` を `parse_result` へ格納（`compiler/ocaml/src/parser_driver.ml:6`）。 | `docs/spec/2-1-parser-type.md` どおりに `Parser<T>`/`State` 抽象を実装し、`RunConfig`/`ParseResult` を Rust 側へ導入。Menhir 互換コンビネータを `core_parse` 直伝で移植。 |
| FRG-08 | `parser_expectation` | ✅ Rust `ExpectedTokenCollector` は一部分類のみで、`Not`/`Class`/`TraitBound` のラベル正規化が不足（`compiler/rust/frontend/src/diagnostic/recover.rs:11`）。 | OCaml `parser_expectation.ml` は優先順位・人間可読メッセージ・`recover` 拡張を完全実装（`compiler/ocaml/src/parser_expectation.ml:21`）。 | `parser_expectation` の列挙と整列ロジックを Rust 側へ写経し、`ExpectedToken` 列挙子を仕様 2-5 §B-7 と一致させる。 |

### 3.2 AST / IR

| ID | ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- | --- |
| FRG-09 | AST 芝台 | ✅ `ast.rs` は `Module/EffectDecl/Function` と 8 種の `Expr` のみ（`compiler/rust/frontend/src/parser/ast.rs:7`）。 | OCaml `Ast` は `expr_kind`, `pattern_kind`, `decl_kind`, `effect_call` 等を全列挙（`compiler/ocaml/src/ast.ml:95`）。 | `1-1-ast-and-ir-alignment.md` の表に沿って全ノードを Rust struct/enum で定義し、JSON ダンプ (`--emit ast-json`) を OCaml と同フォーマットに揃える。 |
| FRG-10 | Typed AST / IR | ✅ Rust `TypeckArtifacts` は `TypedFunctionSummary` 等の統計 JSON のみで、実際の Typed AST ノードを保持しない（`compiler/rust/frontend/src/bin/poc_frontend.rs:1839`）。 | OCaml `typed_ast.ml` が `typed_expr/typed_decl` を保持し、dual-write で JSON を出力（`compiler/ocaml/src/typed_ast.ml:19`）。 | `TypedAst` モジュールを `crate::semantics::typed` として新設し、`--emit typed-ast` で OCaml の JSON スキーマを再現。 |
| FRG-11 | Streaming 状態 | ✅ Rust `ParsedModule` は `packrat_stats`/`span_trace` だけ保存し、Packrat cache や `recovered` フラグを返さない（`compiler/rust/frontend/src/parser/mod.rs:26`）。 | OCaml `parse_result` は `packrat_cache`, `recovered`, `farthest_error_offset` 等を保持し CLI/dual-write が利用（`compiler/ocaml/src/parser_driver.ml:27`）。 | Packrat エントリ (`PackratEntry`) のダンプを CLI へ接続し、`parse_result` と同じフィールド集合を Rust でも返却。 |

### 3.3 型推論・制約

| ID | ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- | --- |
| FRG-12 | HM 実装 | ✅ `TypecheckDriver` は `SimpleType` (Int/Bool/Unknown) を返すのみで、制約生成・一般化・辞書引数が無い（`compiler/rust/frontend/src/typeck/driver.rs:11`）。 | OCaml `type_inference.ml` が Algorithm W + 制約ソルバ + impl レジストリ + 効果解析を実装（`compiler/ocaml/src/type_inference.ml:1`）。 | `types.rs`/`scheme.rs`/`constraint.rs` を分割実装し、`Type_env` と同等の環境を提供。`p1-front-end-checklists.csv` Typed AST 項目を満たす。 |
| FRG-13 | 効果行 / Capability | Rust 残余効果検出は `perform` 文字列検索で `TypecheckViolation::residual_leak` を生成する簡易版（`compiler/rust/frontend/src/typeck/driver.rs:232`）。 | OCaml は `Type_inference_effect` と `Effect_profile` で Capability Registry・StageContext を参照（`compiler/ocaml/src/type_inference.ml:80`）。 | `StageContext` と `runtime_capabilities` を `TypecheckDriver` に渡し、Capability Registry との一致判定を Rust でも実装。`effects.contract.*` 診断を JSON へ出力。 |
| FRG-14 | dual-write 成果物 | Rust CLI `--emit typed-ast/constraints/typeck-debug` は統計 JSON を作るが AST/constraints 詳細が欠落（`compiler/rust/frontend/src/bin/poc_frontend.rs:1866`）。 | dual-write 手順（`1-3-dual-write-runbook.md`）では OCaml フォーマットと 1:1 の JSON が前提。 | HM 完了後に AST/constraint/export のスキーマを OCaml に合わせ、`reports/dual-write/front-end/w3-type-inference` 形式で保存。 |

### 3.4 診断前処理・JSON

| ID | ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- | --- |
| FRG-15 | `Diagnostic` モデル | `FrontendDiagnostic` は `code/message/span/expected` のみで severity/domain/audit/hints を保持しない（`compiler/rust/frontend/src/diagnostic/mod.rs:16`）。 | OCaml `Diagnostic.t` は仕様 3-6 のフィールド（severity/domain/codes/hints/fixits/audit/extensions）を実装（`compiler/ocaml/src/diagnostic.ml:165`）。 | `Diagnostic` 構造体を再定義し、`Diagnostic.Builder` 相当の API を Rust に追加。 |
| FRG-16 | JSON エミッタ | `build_parser_diagnostics` が `severity="error"`, `domain="parser"` を固定値で出力し `audit_id` も仮値生成のみ（`compiler/rust/frontend/src/bin/poc_frontend.rs:1192`）。 | OCaml CLI は各診断の severity/domain/audit_metadata を `Diagnostic.Builder` から受け取り JSON 化（`compiler/ocaml/src/parser_driver.ml:68` 等）。 | `Diagnostic` から JSON へ変換する専用モジュールを用意し、`scripts/validate-diagnostic-json.sh` の Schema v2.0.0-draft を満たす。 |
| FRG-17 | Recover 拡張 | Rust は streaming recover 用 placeholder のみを挿入し、`Diagnostic.expectation_summary` 由来の `context_note` などを生成しない（`compiler/rust/frontend/src/diagnostic/mod.rs:151`）。 | OCaml `parser_expectation` + `attach_recover_extension` が `expected_tokens` + `message` + `context` を JSON 拡張へ埋め込む（`compiler/ocaml/src/parser_driver.ml:68`）。 | `ExpectedTokensSummary` を JSON へ変換するヘルパーを実装し、`recover_extension_payload` 相当のフィールドを出力。 |

### 3.5 Streaming / RunConfig / dual-write

| ID | ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- | --- |
| FRG-18 | Streaming Runner | `StreamFlowState` は checkpoint 数のみ計測し、`run_stream`/`resume`/`Continuation` API が CLI から呼べない（`compiler/rust/frontend/src/streaming/flow.rs:1`）。 | OCaml `Parser_driver.Streaming.run_stream` が `feeder`/`demand_hint`/`resume_hint` を扱い、`recover` 診断を注入（`compiler/ocaml/src/parser_driver.ml:955`）。 | `StreamingRunner` を Rust に実装し、CLI `--streaming` で `run_stream` を呼び出せるよう統合。`docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` の streaming ケースを dual-write 実行。 |
| FRG-19 | Packrat メトリクス | Rust `ParsedModule` は `PackratStats` を返すが cache snapshot (`PackratEntry`) を CLI や JSON へ出さない（`compiler/rust/frontend/src/parser/mod.rs:26`）。 | OCaml CLI は `Packrat.dump` を `parse-debug` へ保存し `collect-iterator-audit-metrics.py` が参照（`compiler/ocaml/src/parser_driver.ml:145` + `docs/plans/rust-migration/1-3-dual-write-runbook.md`）。 | Packrat cache の JSON シリアライズを追加し、`reports/dual-write/front-end/*/packrat_cache.json` を出力。 |
| FRG-20 | RunConfig 同期 | Rust CLI `RunSettings` は独自フィールドで `parser_run_config` JSON へも落ちない（`compiler/rust/frontend/src/bin/poc_frontend.rs:188` 付近）。 | OCaml `Parser_run_config` が CLI/テスト/Streaming で共有され JSON へ記録（`compiler/ocaml/src/parser_driver.ml:6`）。 | `FrontendConfig`/`RunSettings` を `Run_config` と同構造に再設計し、dual-write レポートに `parser_run_config` を含める。 |

## 4. 具体的な計画

### FRG-06

1. **仕様ソースの統合表を確定（Day 1）**  
   - `docs/spec/1-1-syntax.md#A.3` と `compiler/ocaml/src/token.ml` を突き合わせ、キーワード 38 種・将来予約語 2 種・演算子 26 種・リテラル 4 系列（INT/FLOAT/CHAR/STRING）を Rust で再現するマッピング表を `docs/plans/rust-migration/p1-spec-compliance-gap.md#FRG-06` に追記する。  
   - 既存 `TokenKind` との diff をコメントで宣言し、Rust 実装が何を未実装だったのかを可視化する。これにより `p1-front-end-checklists.csv` の「Lexer coverage」列を更新できる。
2. **Lexer 実装の再設計（Day 2-3）**  
   - `unicode-ident` クレートを導入し、`IdentifierProfile::{Unicode, AsciiCompat}` を Rust 側に実装する。`ParserOptions`（暫定 RunConfig）に `lex_identifier_profile` を追加し、今後 CLI/LSP から `extensions.lex.identifier_profile` を渡せる経路を確保する。  
   - `lexer::RawToken` を再生成し、全キーワードを `#[token(...)]` で網羅、演算子は優先順位を付けてロングトークン（`~>`, `=>`, `:=` 等）から検出する。`INT` は基数接頭辞（`0b/0o/0x`）と `Ast.int_base` の区別を維持し、`Token` 側で `lexeme` とあわせて `NumericBase` を保持する。  
   - Unicode 識別子は `unicode_ident::is_xid_start`/`is_xid_continue` を使って `IDENT`/`UPPER_IDENT` を分岐し、ASCII 互換モードでは `[A-Za-z_]` の範囲に絞る。
3. **検証と CI 連携（Day 4）**  
   - `compiler/rust/frontend/tests/lexer_token_coverage.rs`（新規）で ① Unicode 識別子、② 主要キーワード、③ 代表演算子、④ 基数別整数リテラルをフィクスチャ化し、`cargo test -p reml_frontend lexer_token_coverage` を CI で回す。  
   - `p1-rust-frontend-gap-report.md` の FRG-06 状態を「対応中」に更新し、`reports/dual-write/front-end/w4-diagnostics/*` の Lexer メトリクスから `lexer.identifier_profile_unicode=1.0` を再測定して `collect-iterator-audit-metrics.py` に記録する。

- 進捗ログ
  - ✅ `docs/plans/rust-migration/p1-spec-compliance-gap.md` に仕様トークン表を追記し、Rust `TokenKind` の不足を明文化（Day 1 完了）。
  - ✅ `compiler/rust/frontend/src/token.rs` と `src/lexer/mod.rs` を刷新し、38+ キーワード・演算子 26 種・`IdentifierProfile::{Unicode,AsciiCompat}` を実装。`ParserOptions.lex_identifier_profile` と CLI (`poc_frontend.rs`) まで配線済み。
  - ✅ `compiler/rust/frontend/tests/lexer_token_coverage.rs` を追加し `cargo test --test lexer_token_coverage` で Unicode/ASCII プロファイル・基数・文字列種別を検証。
  - ⏳ RunConfig/LSP からの `identifier_profile` 伝播、および dual-write メトリクス (`lexer.identifier_profile_unicode`) の再測定は未完。CLI/LSP 設定取得の導線を整備した上で、`collect-iterator-audit-metrics.py` で値を反映させる。

### FRG-07

FRG-07 は `docs/spec/2-1-parser-type.md` に記された `Parser<T>` / `State` / `RunConfig` / `ParseResult` を Rust 側へ導入し、OCaml 側 `parser_driver` に倣った dual-write API を実現することが目的である。今回の対応では以下のように段階を踏み、構造と実装の両面を整備した。

1. `compiler/rust/frontend/src/parser/api.rs` を追加し、RunConfig の拡張扱い（`with_extension` など）や `ParseResult` のメタデータを定義。`LeftRecursionMode` などの仕様型を `indexmap` + `serde_json::Value` で保持し、将来的な CLI/LSP 連携に備えた拡張基盤を整備した。
2. `ParserDriver` を `ParseResult<Module>` を返すようリファクタリングし、`ParserOptions::from_run_config` + `parse_with_options_and_run_config` で既存のトークン解析ループと streaming 状態を `RunConfig` に紐付け。テストでは新 API を用いて AST の検証・診断の比較を維持しつつ、`parse_result_from_module` で従来の `ParsedModule` から変換できるようにした。
3. `compiler/rust/frontend/src/bin/poc_frontend.rs` 側に `RunSettings::to_run_config` を実装し、CLI フラグから RunConfig を構築。その RunConfig を `ParserDriver` へ渡すことで、diagnostic JSON や streaming メトリクスを `ParseResult` で受け取り、`packrat_stats`/`stream_meta` などの出力をそのまま継続できるようにした。

これらのステップを実施したことで、FRG-07 の計画は実装済みとなり、今後の RunConfig 拡張や Menhir 互換コンビネータ導入に向けて必要な型基盤と CLI パスが整ったと言える。

## FRG-08

FRG-08（`parser_expectation`）では、`docs/spec/2-5-error.md#b-7` に則った期待集合の優先順位・テンプレート・フォールバックを Rust 版 Recover が再現することを目指す。

1. **期待の分類と整列**
   - `compiler/ocaml/src/parser_expectation.ml` の `Keyword`/`Token`/`Class` 等の列挙、`priority`/`raw_label`/`quoted_label` を参考に `ExpectedToken` のバリアントを網羅しつつ、`ExpectedTokenCollector` が OCaml と同じ順序・重複除去・humanize を遂行できるようにする。
   - `recover` モジュールの `ExpectedTokensSummary` が `parse.expected` の `message_key`/`locale_args`/`humanized` を仕様 2-5 §B-7 と整合させるべく、欠損時のフォールバック文字列や streaming 用 placeholder も含めた検証を行う。

2. **トークン期待値の正規化**
   - `compiler/rust/frontend/src/parser/mod.rs` における `build_expected_summary` が `TokenKind` から `identifier`/`upper-identifier`/`integer-literal` など OCaml と同一のラベルを出力し、`expected_tokens` の JSON 表現・CLI 表示ともに両実装で一致する。
   - 既存の `expression_expected_tokens` や streaming 再開処理で使うプレースホルダも上記 `ExpectedToken` を再利用し、`collect-iterator-audit-metrics.py --section streaming` の `ExpectedTokenCollector.streaming` との比較で `expected_tokens_match` を満たす。

3. **将来対応と検証**
   - `ExpectedTokenCollector` に `Not`/`TypeExpected`/`TraitBound` などのビルダーを追加し、将来的に `Expectation::not` や `TraitBound` を捕捉する際にもラベルが正規化された出力になるよう拡張する。
   - `cargo fmt` を含む `compiler/rust/frontend` の Rustfmt/テストを走らせ、変更後 `expected_tokens` 出力が人間可読な `quoted_label`（例:「ここで `)` または identifier が必要です」）になっていることを確認する。

上記の計画はすでに実施済みで、`compiler/rust/frontend/src/diagnostic/recover.rs` では `ExpectedToken` のバリアントを拡張し `ExpectedTokenCollector::humanize` を仕様通りに整備、`compiler/rust/frontend/src/parser/mod.rs` では `TokenKind` から英語ラベルを返すように改修済みであるため、FRG-08 については OCaml 実装に倣った `expected_tokens` サマリを出力できる準備が整ったと考えている。

### FRG-09

1. **AST 定義の一覧化（Day 1）**  
   - `docs/plans/rust-migration/1-1-ast-and-ir-alignment.md` の `expr_kind`/`pattern_kind` を `compiler/ocaml/src/ast.ml` と突き合わせ、必要な `Ident`/`Literal`/`Pattern`/`Decl` を Rust 型として明文化する。  
   - `ExprKind` や `Literal` には `kind` タグ付きの serde 表現を与えて Dual-write で JSON を比較できるように整える。
2. **パーサと簡易型推論の同期（Day 2）**  
   - `module_parser` が `Ident` を経由して各ノードを組み立て、`Expr::if_else`/`Expr::perform` などで `ExprKind` に該当する AST を生成する。  
   - `compiler/rust/frontend/src/typeck/driver.rs` が `ExprKind` を走査し、既存 `SimpleType` ルールと residual leak 検出を維持しつつ AST 構造の変更に追従する。
3. **AST JSON 出力と dual-write（Day 3）**  
   - `compiler/rust/frontend/src/bin/poc_frontend.rs` に `--emit-ast` オプションを追加し、`ParseResult` に含まれる `Module` を `serde_json` で出力する。  
   - dual-write 出力にも `parse/ast.rust.json` を追加し、OCaml AST JSON と比較する際の参照先を確保する。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/parser/ast.rs` を `Ident`/`Pattern`/`ExprKind` を含む構造に再構成し、JSON 仕様に合わせた `render`/`span` を新設した。
  - ✅ `compiler/rust/frontend/src/parser/mod.rs` と `compiler/rust/frontend/src/typeck/driver.rs` を AST の新構造に合わせて更新し、`ExprKind` に基づく再帰的走査を維持した。
  - ✅ `compiler/rust/frontend/src/bin/poc_frontend.rs` に `--emit-ast`/dual-write AST JSON 出力を追加し、`parse/ast.rust.json` を出力できるパスを確保した。
  - ✅ `cargo fmt --manifest-path compiler/rust/frontend/Cargo.toml` を実行し、Rust ソースの整形を完了した。

### FRG-10

1. **Typed AST データモデルの整理（Day 1）**
   - `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` の `TypedExpr`/`TypedDecl`/`TyId` セクションを参照し、Rust 側に新設する `crate::semantics::typed` モジュールで再現すべきフィールド・JSON スキーマを表でまとめる。
   - `compiler/rust/frontend/src/parser/ast.rs` の `ExprKind`/`PatternKind` を同一の variant 名で `TypedExprKind`/`TypedPatternKind` に写し、各ノードに `ty` ラベルが付いた serde 表現を決定して dual-write 比較に備える。

2. **TypecheckDriver との連携による Typed AST 生成（Day 2-3）**
   - `infer_expr`/`infer_function` を拡張して、`SimpleType` のスキーマを `TypedExpr` ノードツリーに埋め込みながら再帰的な型ラベルを構築し、関数ごとの `TypedFunction` を生成する。
   - 生成結果を `TypecheckReport` に `TypedModule` として保持し、`TypeckArtifacts::typed_ast` から `serde::Serialize` 対応の Typed AST JSON を再取得できるようにする。

3. **CLI・dual-write・検証（Day 4）**
   - `--emit-typed-ast` と dual-write `typeck/typed-ast.rust.json` に `TypedModule` を出力し、`reports/dual-write/front-end/w3-type-inference/<case>/typed-ast.rust.json` で OCaml 側と構造・`ty` ラベルが一致しているかを差分確認できるようにする。
   - 差分が残った場合には `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` のスキーマ記載を更新し、検証結果を同ファイルか `reports/dual-write/front-end/w3-type-inference/README.md` に記録する。

- 進捗ログ
  - ✅ TypecheckDriver の `infer_expr` を把握しつつ `TypedExpr` 架構の草案を `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` に反映。
  - ✅ `crate::semantics::typed` モジュール定義と `TypecheckReport` 拡張の実装を進行中。
  - ✅ `compiler/rust/frontend/src/typeck/{types,scheme,constraint}.rs` を追加し、`TypeEnv`/`Scheme`/`Constraint` のデータモデルと `ConstraintSolver` を整備。`typeck::driver::TypecheckDriver` を新構造に書き換えて型エンジンの土台を切り替えた。
  - ✅ `p1-spec-compliance-gap.md#FRG-12` にこれらのモジュールと `TypeEnv` の再構成を追記し、`p1-front-end-checklists.csv` の Type inference 項目とのリンクを通じてステータス管理を明示化した。

### FRG-11

1. `compiler/rust/frontend/src/streaming/mod.rs` に `PackratCacheEntry` を定義し、`StreamingState` から `packrat_cache` を `Vec<PackratCacheEntry>` で取り出すヘルパーを追加する。`PackratEntry` の `TokenSample`/`ExpectationSummary` を serde 対応のまま保持し、OCaml の `Parser_expectation.Packrat.t` 相当の JSON スキーマを満たす。
2. `compiler/rust/frontend/src/parser/mod.rs` と `compiler/rust/frontend/src/parser/api.rs` で `ParsedModule`/`ParseResult` に `packrat_cache`・`recovered`・`farthest_error_offset` を付与し、`StreamingRecoverController` で recoverable なエラーを記録したタイミングでフラグを立てる。`docs/spec/2-1-parser-type.md` に記載された `ParseResult` フィールド群と一致する構造にして、dual-write の `parse_result` 比較に備える。
3. `compiler/rust/frontend/src/bin/poc_frontend.rs` の `parse_debug`/`diagnostic` 出力に新フィールドを含め、`reports/dual-write/front-end/<case>/rust.parse-debug.json` で OCaml 側と同じスキーマで `packrat_cache` や `farthest_error_offset` を比べ、`run_config`/`stream_meta` との整合性チェックを完了する。

### FRG-12

1. **HM 基盤仕様の写経と整理（Day 1）**
   - `docs/spec/1-2-types-Inference.md` と `compiler/ocaml/src/type_inference.ml`（特に `Algorithm W` 相当の `infer_module`, `infer_expr`, `infer_constraint`）をルートに、Rust で再実装すべき型・スキーマ・制約・環境の一覧表を `docs/plans/rust-migration/p1-spec-compliance-gap.md#FRG-12` に追記する。`Type_env`, `Scheme`, `Constraint` の役割と `Type_env.lookup`/`insert` の振る舞いを比較し、差分と移植方針を明示する。
   - `compiler/rust/frontend/src/typeck/driver.rs` の `SimpleType` や `infer_simple_type` を参照しつつ、`TyVar` 生成、`unify`、`generalize`/`instantiate`、`dict` 生成のポイントを抽出し、構造的再利用案（`TypeEnv` 構造体、`Substitution` 構造体）を決定する。
   - ✅ Day1 実績: `docs/plans/rust-migration/p1-spec-compliance-gap.md#FRG-12` に TypeEnv/Scheme/Constraint/Driver の対比表を追加し、`p1-front-end-checklists.csv` の “Type inference” 項で FRG-12 参照を明示してステータス管理の起点とした。

2. **HM コアモジュールの実装（Day 2-3）**
   - `compiler/rust/frontend/src/typeck/types.rs` を新設し、`Type`（`Var`, `Builtin`, `App`, `Arrow` など）、`TypeKind`、`TypeVariable`、`CapabilityContext` を定義。`serde`/`Display` 実装を整え、dual-write JSON で OCaml `Type_expr` 相当を出力できるようにする。
   - `compiler/rust/frontend/src/typeck/scheme.rs` では `Scheme`（`quantifiers`, `constraints`）と `instantiate`/`generalize`を実装し、`constraints` を `indexmap::IndexMap<Name, Type>` で保持することで `Type_env` に組み込みやすくする。
   - `compiler/rust/frontend/src/typeck/constraint.rs` では `Constraint`（`Equal`, `HasCapability`, `ImplBound`）と `Substitution`/`ConstraintSolver` を定義し、`Substitution::apply_unwrap`/`merge` および `ConstraintSolver::solve` を `TypecheckDriver` から呼び出せるようにする。`impl` レジストリ（`compiler/ocaml/src/type_inference.ml` の `Registry` 相当）を `crate::frontend::impl_registry` で保持し、`Type_env` が `ConstraintSolver` へ参照を渡せるようにする。

3. **TypecheckDriver の再構成（Day 4-5）**
   - `TypecheckDriver::infer_function`/`infer_expr`/`infer_pattern` を `Constraint` を蓄積する形に書き換え、`InferContext` で `TypeEnv` と `Substitution` を共有。`Tuple`/`Function`/`Effect` のパターンごとに適切な初期型を割り当て、`infer_expr` が `Constraint` を返す API を通じて `ConstraintSolver` に渡す。
   - `TypecheckReport` を拡張して `constraints`/`typed_module` を保持し、`TypeckArtifacts::typed_ast`/`TypeckArtifacts::constraints` を `serde::Serialize` で dual-write 可能にする。`StageContext`/`runtime_capabilities` を `TypeEnv` に注入し、`resolver` が `Capability` の check（`effects.contract.*` 診断）を `Constraint` として挿入できるようにする。
   - 辞書引数（`impl`）と residual leak 判定を `Dict` 構造体で追跡し、`TypecheckReport` に `used_impls` として記録。これにより `FRG-13` で Capability Registry と整合性を取るための土台を整える。
   - ✅ Rust 側で `TypeEnv`/`Scheme`/`Constraint` のデータ構造を先行実装し、`TypecheckDriver` を `infer_expr` が `Type` を返すように書き換えて `Vec<Constraint>` を形成、`ConstraintSolver` に渡す流れを確立。`TypedExpr` は型ラベルを保持し、`TypecheckReport` は `TypeckArtifacts::typed_ast` と同期できるようにした。

4. **検証・dual-write・ドキュメント（Day 6）**
   - `compiler/rust/frontend/tests/typeck/hindley_milner.rs` などを追加し、ポリモーフィズム、`let` 限界、タプルの法則、パターンマッチでの束縛を `cargo test -p reml_frontend typeck::hindley_milner` で検証。`reports/dual-write/front-end/w3-type-inference/<case>/constraints.rust.json` に solver 出力を残し、OCaml 出力と差分確認。
 - `p1-front-end-checklists.csv` の “Type inference” 項目を `FRG-12` にリンクさせ、ステータスや検証手順を `docs/plans/rust-migration/p1-spec-compliance-gap.md` の `FRG-12` 参照欄に反映。
 - 計画の運用ログは `docs/plans/rust-migration/p1-rust-frontend-gap-report.md` の該当セクションに追記し、進捗と発見した仕様差異（例: `Capability` が `ty_env` で必要なため `docs/spec/1-3-effects-safety.md` を追記）を記録する。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/typeck/driver.rs` で関数ごとの `Constraint` を集約して `TypecheckReport` に `constraints`/`used_impls` を持たせ、`compiler/rust/frontend/src/bin/poc_frontend.rs` の `typeck/constraints.rust.json` にそのまま双方向出力できるようにした。
  - ✅ `compiler/rust/frontend/tests/typeck_hindley_milner.rs` を追加し、`Constraint::Equal` の収集と `ConditionLiteralBool` 診断のトリガーが `TypecheckReport` に反映されることを `cargo test -p reml_frontend hindley_milner` で確認した。
  - ✅ `docs/plans/rust-migration/p1-spec-compliance-gap.md#FRG-12` と `docs/plans/rust-migration/p1-front-end-checklists.csv` の Type inference 項目に `FRG-12` 参照と `reports/dual-write/front-end/w3-type-inference/<case>/constraints.{ocaml,rust}.json` との比較手順を追記した。
- ✅ `docs/spec/1-3-effects-safety.md` を FRG-12 の検証スコープに明示的にリンクさせ、能力チェック時の `StageContext`/`Capability` 追加が必要であることを記録している。

### FRG-13

1. **OCaml の効果ステージ検証の再確認（Day 0）**  
   - `compiler/ocaml/src/type_inference_effect.ml` や `runtime_capability_resolver.ml` が `StageContext` と `CapabilityRegistry` から `effects.contract.*` 診断を出している仕組みを追い、`docs/spec/1-3-effects-safety.md` §I に記された `Σ_after ⊆ allows_effects`／ステージの照合の要件を整理する。Stage の順序（stable < beta < experimental）や CLI `--effect-stage-(runtime|capability)` オプションとのインタフェースを明文化する。
2. **Rust 側の Capability Registry モジュールの設計（Day 1）**  
   - `typeck/capability.rs` を作成し、`perform` で出現する効果名→Capability ID/Stage のマッピングと CLI から渡される `runtime_capabilities`（`id[@stage]` 形式）を正規化するロジックを実装する。`typeck/env.rs` に StageRequirement の比較・ラベル化 API を追加し、`StageContext` が要求するステージと検出された Capability のステージを比較できるようにする。
3. **診断生成と dual-write 出力の統合（Day 2-3）**  
   - `TypecheckDriver` に `EffectUsage` の収集と Stage 要件チェックを追加し、`TypecheckViolation` に `StageMismatch` も加えて `effects.contract.stage_mismatch` / `effects.contract.residual_leak` を JSON に出力する。`poc_frontend.rs` の StageAuditPayload も `StageRequirement::label` を使うよう更新し、dual-write の `typeck/debug` JSON にステージ情報を焼き込む。
4. **検証とログ化（Day 4）**  
   - `cargo test -p reml_frontend typeck::hindley_milner` などで `perform` を含むケースが残余効果として検出されること、ステージ要求を下回ると `effects.contract.stage_mismatch` が出力されることを確認し、`reports/dual-write/front-end/w4-diagnostics` の metadata に Stage 情報を追記する。`p1-spec-compliance-gap.md#SCG-09` も併せて更新し、ドキュメントから仕様ギャップと対応状況が参照できる状態にする。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/typeck/capability.rs` を新設し、`perform` で得られる効果名を Capability ID/Stage にマッピングしつつ CLI `runtime_capabilities`（`id` または `id@stage`）を正規化するヘルパを実装。
  - ✅ `compiler/rust/frontend/src/typeck/env.rs` に StageRequirement の比較・ラベル化 API を追加し、`poc_frontend.rs` の `stage_requirement_label` も新 API を呼ぶようにして StageAuditPayload の文字列が仕様に一致するようにした。
  - ✅ `compiler/rust/frontend/src/typeck/driver.rs` を更新して `EffectUsage` を収集し、StageContext/runtime capabilities との照合で `TypecheckViolation::stage_mismatch` / `residual_leak` を生成。dual-write `typeck/debug` と diagnostics JSON には `effects.contract.*` が含まれている。

## 5. ノート

- 仕様参照: `docs/spec/1-1-syntax.md`, `1-2-types-Inference.md`, `1-3-effects-safety.md`, `2-1-parser-type.md`, `2-5-error.md`, `2-7-core-parse-streaming.md`, `3-6-core-diagnostics-audit.md`
- 作業ログ: 大きな rename/移動が発生する場合は `docs-migrations.log` を更新すること。
- 本レポートは `p1-spec-compliance-gap.md` の補足資料として扱い、今後の差分調査結果を追記する際はセクション単位で更新する。
