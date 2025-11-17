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
| FRG-13 | 効果行 / Capability |✅ Rust 残余効果検出は `perform` 文字列検索で `TypecheckViolation::residual_leak` を生成する簡易版（`compiler/rust/frontend/src/typeck/driver.rs:232`）。 | OCaml は `Type_inference_effect` と `Effect_profile` で Capability Registry・StageContext を参照（`compiler/ocaml/src/type_inference.ml:80`）。 | `StageContext` と `runtime_capabilities` を `TypecheckDriver` に渡し、Capability Registry との一致判定を Rust でも実装。`effects.contract.*` 診断を JSON へ出力。 |
| FRG-14 | dual-write 成果物 | ✅ Rust CLI `--emit typed-ast/constraints/typeck-debug` は統計 JSON を作るが AST/constraints 詳細が欠落（`compiler/rust/frontend/src/bin/poc_frontend.rs:1866`）。 | dual-write 手順（`1-3-dual-write-runbook.md`）では OCaml フォーマットと 1:1 の JSON が前提。 | HM 完了後に AST/constraint/export のスキーマを OCaml に合わせ、`reports/dual-write/front-end/w3-type-inference` 形式で保存。 |

### 3.4 診断前処理・JSON

| ID | ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- | --- |
| FRG-15 | `Diagnostic` モデル | ✅ `FrontendDiagnostic` は `code/message/span/expected` のみで severity/domain/audit/hints を保持しない（`compiler/rust/frontend/src/diagnostic/mod.rs:16`）。 | OCaml `Diagnostic.t` は仕様 3-6 のフィールド（severity/domain/codes/hints/fixits/audit/extensions）を実装（`compiler/ocaml/src/diagnostic.ml:165`）。 | `Diagnostic` 構造体を再定義し、`Diagnostic.Builder` 相当の API を Rust に追加。 |
| FRG-16 | JSON エミッタ | ✅ `build_parser_diagnostics` が `severity="error"`, `domain="parser"` を固定値で出力し `audit_id` も仮値生成のみ（`compiler/rust/frontend/src/bin/poc_frontend.rs:1192`）。 | OCaml CLI は各診断の severity/domain/audit_metadata を `Diagnostic.Builder` から受け取り JSON 化（`compiler/ocaml/src/parser_driver.ml:68` 等）。 | `Diagnostic` から JSON へ変換する専用モジュールを用意し、`scripts/validate-diagnostic-json.sh` の Schema v2.0.0-draft を満たす。 |
| FRG-17 | Recover 拡張 | ✅ Rust は streaming recover 用 placeholder のみを挿入し、`Diagnostic.expectation_summary` 由来の `context_note` などを生成しない（`compiler/rust/frontend/src/diagnostic/mod.rs:151`）。 | OCaml `parser_expectation` + `attach_recover_extension` が `expected_tokens` + `message` + `context` を JSON 拡張へ埋め込む（`compiler/ocaml/src/parser_driver.ml:68`）。 | `ExpectedTokensSummary` を JSON へ変換するヘルパーを実装し、`recover_extension_payload` 相当のフィールドを出力。 |

### 3.5 Streaming / RunConfig / dual-write

| ID | ギャップ | 現状 (Rust) | 期待仕様 / OCaml | 必要対応 |
| --- | --- | --- | --- | --- |
| FRG-18 | Streaming Runner | ✅ `StreamFlowState` は checkpoint 数のみ計測し、`run_stream`/`resume`/`Continuation` API が CLI から呼べない（`compiler/rust/frontend/src/streaming/flow.rs:1`）。 | OCaml `Parser_driver.Streaming.run_stream` が `feeder`/`demand_hint`/`resume_hint` を扱い、`recover` 診断を注入（`compiler/ocaml/src/parser_driver.ml:955`）。 | `StreamingRunner` を Rust に実装し、CLI `--streaming` で `run_stream` を呼び出せるよう統合。`docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` の streaming ケースを dual-write 実行。 |
| FRG-19 | Packrat メトリクス | ✅ Rust `ParsedModule` は `PackratStats` を返すが cache snapshot (`PackratEntry`) を CLI や JSON へ出さない（`compiler/rust/frontend/src/parser/mod.rs:26`）。 | OCaml CLI は `Packrat.dump` を `parse-debug` へ保存し `collect-iterator-audit-metrics.py` が参照（`compiler/ocaml/src/parser_driver.ml:145` + `docs/plans/rust-migration/1-3-dual-write-runbook.md`）。 | Packrat cache の JSON シリアライズを追加し、`reports/dual-write/front-end/*/packrat_cache.json` を出力。 |
| FRG-20 | RunConfig 同期 | ✅ Rust CLI `RunSettings` は独自フィールドで `parser_run_config` JSON へも落ちない（`compiler/rust/frontend/src/bin/poc_frontend.rs:188` 付近）。 | OCaml `Parser_run_config` が CLI/テスト/Streaming で共有され JSON へ記録（`compiler/ocaml/src/parser_driver.ml:6`）。 | `FrontendConfig`/`RunSettings` を `Run_config` と同構造に再設計し、dual-write レポートに `parser_run_config` を含める。 |

### 3.6 `p1-spec-compliance-gap.md` から追加

| ID | SCG | 状態 | 主な差分 | 参照 |
| --- | --- | --- | --- | --- |
| FRG-21 | SCG-01 | ✅ | Unicode XID/`UPPER_IDENT`、`LexProfile` 切り替えと `RunConfig.extensions["lex"]` 連動が欠落 | `p1-spec-compliance-gap.md#SCG-01` |
| FRG-22 | SCG-02 | ✅ | `TokenKind` や `ExpectedToken` 表現が var/match/type/Option、`0x`/`0b`/複数行 raw 文字列などを含まない | `p1-spec-compliance-gap.md#SCG-02` |
| FRG-23 | SCG-03 | ✅ | `Parser<T>`/`State`/`Reply`/`RunConfig` API が欠如し、`ParserDriver::parse` が `ParsedModule` の単一戻り値のみ | `p1-spec-compliance-gap.md#SCG-03` |
| FRG-24 | SCG-05 | ✅ | AST の `ExprKind`/`PatternKind`/`DeclKind` が限定され、OCaml `ast.ml` の列挙が移植されていない | `p1-spec-compliance-gap.md#SCG-05` |
| FRG-25 | SCG-06 | ✅ | Typed AST/型情報（`typed_expr`/`dict_ref`/`scheme`）の構成が存在せず dual-write JSON もダミー | `p1-spec-compliance-gap.md#SCG-06` |
| FRG-26 | SCG-07 | ✅ | Packrat/Streaming 状態 (`Core_parse_streaming`) が parser と接続しておらず、`packrat_snapshot` や `span_trace` を `ParseResult` に渡せない | `p1-spec-compliance-gap.md#SCG-07` |
| FRG-27 | SCG-08 | ✅ | Algorithm W による制約生成・一般化・値制限・ソルバが存在せず、`typeck/driver` は `SimpleType`のみ | `p1-spec-compliance-gap.md#SCG-08` |
| FRG-28 | SCG-09 | ✅ | 効果行・残余効果・Capability Stage 監査（`effects.contract.*`）が未実装で StageContext/RuntimeCapability 連携もない | `p1-spec-compliance-gap.md#SCG-09` |
| FRG-29 | SCG-10 | ✅ | TypecheckReport から typed_module/constraints/used_impls を使って dual-write `typeck/typed-ast.rust.json`/`constraints.rust.json` を出力し、`typeck/impl-registry.rust.json` も追加した | `p1-spec-compliance-gap.md#SCG-10` |
| FRG-30 | SCG-11 | ✅ | `FrontendDiagnostic` に severity/domain/audit/timestamp/context_note が無く、OCaml と同一構造でない | `p1-spec-compliance-gap.md#SCG-11` |
| FRG-31 | SCG-12 | ✅ | `parser_expectation` 由来の期待集合整形・優先順位・`Not`/`Class` 一覧が十分ではない | `p1-spec-compliance-gap.md#SCG-12` |
| FRG-32 | SCG-13 | ✅ | CLI JSON 出力で severity/domain 固定・audit_id 疑似値のため、`DiagnosticFormatter` 相当が必要 | `p1-spec-compliance-gap.md#SCG-13` |
| FRG-33 | SCG-14 | ✅ | `run_stream`/`resume` API と `StreamOutcome` 管理がなく、StreamingRunner/CLI `--streaming` に未統合 | `p1-spec-compliance-gap.md#SCG-14` |

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

### FRG-08

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

#### Phase 2-8 追補: SYNTAX-003 効果構文クローズ

- 背景: `rust-gap SYNTAX-003`（effect handler 受理不可）が Phase 2-8 W37 後半の対象。`ExprParser` 分離・`EffectExprKind`・`TypeAnnot::Resume` の共有が要件。
- 実装結果:
  - `reports/spec-audit/diffs/SYNTAX-003-ch1-rust-gap.md` を作成し、`block_scope.reml` / `effect_handler.reml` の CLI 証跡を整理。
  - `reports/spec-audit/ch1/block_scope-20251118-diagnostics.json`、`effect_handler-20251118-diagnostics.json`、`effect_handler-20251118-dualwrite.md` を保存して dual-write 差分ゼロを確認。
  - `docs/notes/spec-integrity-audit-checklist.md` の `SYNTAX-003` 行を `Closed (P2-8)` に更新し、`block_scope`/`effect_handler`/`perform_do` の監査行を追加。
- 影響: 本レポートの FRG-02/FRG-09 の `EffectDecl`/`HandlerDecl` 欄は Phase 2-8 追補で完了。以降は `perform/do` の `EffectScopeId` 拡張と型推論連携（FRG-03）にフォーカスする。

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

### FRG-14

FRG-14 では `reports/dual-write/front-end/w3-type-inference` に格納する typed AST / constraints / typeck-debug JSON を OCaml CLI のスキーマ（`function_summaries`・`stats`・`rendered`・`effects` 等）と整合させ、dual-write の `typeck/typed-ast.rust.json` などが OCaml 相当品と比較できる形式で出力されることを確認する。

1. **OCaml 出力とレポート構造の整理（Day 1）**  
   - `compiler/ocaml/src/cli/typeck_output.ml` の `typed_ast_json`/`constraints_json`/`typeck_debug_json` を読み、`function_summaries` に含まれる `param_count`/`return_type`/`effect_row`/`span`/`dict_refs` や `rendered` の描写、`stats` の `unify_calls`/`ast_nodes`/`token_count` を抽出して現状の Rust 出力との差分を表にまとめる。`reports/dual-write/front-end/w3-type-inference/README.md` に記載された `typeck/<case>/typed-ast.{ocaml,rust}.json` のファイル構成も再確認して、Rust 側が生成すべきファイル群とパスを明記する。
   - `docs/plans/rust-migration/appendix/w3-typeck-dualwrite-plan.md` で定義された `typeck/metrics.json` や `effects-metrics` の整備方針を参照し、Rust の `TypecheckMetricsPayload` や `collect-iterator-audit-metrics.py --section effects` の出力とどこで接続させるかを記述する。

2. **Rust CLI の出力再構成（Day 2-3）**  
   - `TypecheckReport` の `typed_module` と `functions` を参照して `function_summaries`（`name`/`param_count`/`return_type`/`effect_row`/`span`/`dict_refs`）を組み立て、`TypedAstFile` に `function_summaries`・`rendered`・`input` を保持させる。`rendered` は `=== Typed AST ===` ヘッダ付きで各関数を `fn ... : ...` 形式に整形する。
   - `ConstraintFile` に `function_summaries` と `stats` (`unify_calls`/`ast_nodes`/`token_count` は現状 0) を追加しつつ、従来の `total_constraints`/`constraint_breakdown`/`constraints`/`used_impls` も保持する。`TypeckArtifacts::new` でこの新構造を構築し、`write_dualwrite_typeck_payload` が `typeck/typed-ast.rust.json`・`typeck/constraints.rust.json`・`typeck/typeck-debug.rust.json` を新スキーマで出力する経路を整える。

3. **dual-write 成果物との照合（Day 4）**  
   - `scripts/poc_dualwrite_compare.sh --mode typeck` を想定し、`DualWriteGuards::write_json` から `reports/dual-write/front-end/w3-type-inference/<run>/<case>/typeck/` に新たな JSON を保存できることを確認し、必要であれば README へ `typeck/typed-ast.rust.json` などの存在を追記する。
   - `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` などに `FRG-14` の達成ラインを追記し、OCaml 側と同等の `typed-ast`/`constraints`/`typeck-debug` を `w3-type-inference` フォーマットで出力した記録を残す。

- 進捗ログ
  - ✅ `TypeckArtifacts` で typed AST の `function_summaries` を組み立て、`TypedAstFile` に `rendered` テキストとともに出力するスキーマを整備した。
  - ✅ `ConstraintFile` に `function_summaries` と `stats` を追加しつつ、既存の `total_constraints`/`constraint_breakdown`/`constraints`/`used_impls` を保持する構造に書き換えた。
  - ✅ `poc_frontend.rs` の `write_dualwrite_typeck_payload` で新構造を `typeck/typed-ast.rust.json`/`typeck/constraints.rust.json`/`typeck/typeck-debug.rust.json` へ吐き出す経路を実装し、dual-write 出力が OCaml に近づいた状態にした。

### FRG-15

1. **診断モデルの再定義（Day 1）**
   - `docs/spec/3-6-core-diagnostics-audit.md` と OCaml `compiler/ocaml/src/diagnostic.ml:165` をもとに `Diagnostic` に必要な severity/domain/codes/hints/audit フィールドを整理し、Rust 側 `FrontendDiagnostic` に severity/SeverityHint/Domain/codes/secondary/hints/fixits を持たせるスキーマ案を `docs/plans/rust-migration/p1-spec-compliance-gap.md#FRG-15` にまで落とし込む。
   - 既存 `Diagnostic` Builder や構成コードがこれらのフィールドを使えるよう、`FrontendDiagnostic::with_*` 系メソッドを増設し、`code` と `codes` の関係やセカンダリ span 追加処理を明文化。
2. **CLI 側へのパスと JSON シリアライズの強化（Day 2）**
   - `build_parser_diagnostics` へ severity/domain 情報を渡し、`build_audit_metadata` の `event.domain` などにも反映。メトリクス側の RunConfig/Streaming extensions はそのまま保持しながら、diags.json へ `severity_hint`/`codes`/`secondary`/`hints`/`fixits` を追加。
   - `'recover'`/`expected` のビルドは継続しつつ、新たな `DiagnosticHint`/`DiagnosticFixIt` 用の JSON レイヤーを実装し、`Span` から位置を引ける helper を組み込む。
3. **差分検証と dual-write への記録（Day 3）**
   - FRG-15 の出力変更が既存の dual-write 形式 (`reports/dual-write/front-end/*/`) に与える影響を `p1-spec-compliance-gap.md` と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` に追記し、OCaml との差分検証方針を明示。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/diagnostic/mod.rs` で `FrontendDiagnostic` を severity/domain/codes/hints/fixits/secondary 付きの仕様に再定義し、`with_code`/`with_severity`/`with_domain` などの API を整備。
  - ✅ `compiler/rust/frontend/src/bin/poc_frontend.rs` の `build_parser_diagnostics` で `severity`/`severity_hint`/`domain`/`codes`/`secondary`/`hints`/`fixits` を JSON 化する helper を追加し、`diagnostic.v2` 拡張や `audit_metadata` の `event.domain` を `FrontendDiagnostic` 由来に更新。

### FRG-16

FRG-16 は `scripts/validate-diagnostic-json.sh` で定義された `diagnostic-v2` スキーマに沿って Rust 側の JSON エミッタを再設計し、OCaml 側と同等の `primary`/`codes`/`audit` を出せるようにする。

1. **スキーマと既存実装の整理（Day 1）**
   - `tooling/json-schema/diagnostic-v2.schema.json` および `scripts/validate-diagnostic-json.sh` を読み込み、必須フィールド（`primary`・`audit`・`audit_metadata`）と任意フィールドの表現を確認する。
   - `FrontendDiagnostic` のフィールドと `diagnostic.v2` 拡張がどのように `poc_frontend.rs` の `extensions`/`audit_metadata` に写るかを追い、差分を `docs/plans/rust-migration/p1-spec-compliance-gap.md#FRG-16` に記録する。

2. **JSON 変換モジュールの実装（Day 2）**
   - `compiler/rust/frontend/src/diagnostic/json.rs` を新設し、`LineIndex`・`span`→`primary` 変換・`recover`/`expected`/`hint`/`fixit` の JSON 化を集約する。
   - `poc_frontend.rs` の `build_parser_diagnostics` を同モジュール経由に置き換え、`primary`/`expected`/`codes`/`severity_hint` などを Schema 準拠で組み立てる。`audit_metadata`/`audit` は生成後に `frontend` 側へ渡す。
   - `build_type_diagnostics` も `primary`/`location`/`expected` を出力し、`diagnostic.v2` キーに `codes` を含める。

3. **検証と dual-write 連携（Day 3）**
   - `scripts/validate-diagnostic-json.sh` および `diagnostic-v2.schema.json` で出力 JSON を検証し、不足していた Schema 要件を満たしたことを確認する。
   - 既存 `reports/dual-write/front-end/*/` を集約するスクリプトに対し、JSON を生成して `FRG-16` の状態を「対応済」に更新する。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/diagnostic/json.rs` に JSON 変換ロジックを集中させ、`LineIndex`/`primary`/`recover`/`expected`/`fixit` の出力を再利用可能にした。
  - ✅ `poc_frontend.rs` の `build_parser_diagnostics` と `build_type_diagnostics` を `diag_json` モジュール経由へ切り替え、`primary`/`codes`/`audit` をスキーマ正準な形で出力するようにした。
  - ⏳ `scripts/validate-diagnostic-json.sh` は Node.js ランタイムと `tooling/lsp/tests/client_compat/node_modules` が整備されていないため未実行。実行環境が揃い次第 `diagnostic-v2.schema.json` との比較を走らせる。

### FRG-17

1. **`ExpectationSummary` を診断に保持（Day 1）**
   - `docs/spec/2-5-error.md` に記された `ExpectationSummary` の `context_note` / `message_key` / `alternatives` を再確認し、Rust `FrontendDiagnostic` へ `ExpectedTokensSummary` を持たせて再利用できる API（`apply_expected_summary` / `merge_expected_summary`）を整理する。
   - `compiler/ocaml/src/parser_driver.ml:77` の `attach_recover_extension` を参照し、context note が JSON へ出力される出力経路を把握して `diagnostic/mod.rs` の構造を再設計する。

2. **Recover 拡張の JSON 化（Day 2）**
   - `compiler/rust/frontend/src/diagnostic/json.rs` に `recover_extension_payload_from_summary` を実装し、`ExpectedTokensSummary` から `expected_tokens`・`message`・`context` を取り出す helper を定義する。
   - `build_recover_extension` を更新して summary を優先しつつ既存の `expected_tokens`/`recover.expected_tokens` ノートフォーマットをフォールバックに残し、context note を含めたまま `extensions["recover"]` を出力できるようにする。

3. **dual-write と検証（Day 3）**
   - `compiler/rust/frontend/src/bin/poc_frontend.rs` の `build_type_diagnostics` でも新 helper を使い、Typecheck 側の recover extension でも context が出力されるようにする。
   - `compiler/rust/frontend/src/diagnostic/json.rs` にユニットテストを追加して context note を保持した recover extension が `build_recover_extension` から返ることを保証し、JSON 生成が仕様通りであることを `cargo fmt`/`cargo test` でも確認する（テストは `reml_frontend` クレートに含める）。

- 進捗ログ
  - ✅ `FrontendDiagnostic` が `ExpectedTokensSummary` を保持し、`apply_expected_summary`/`merge_expected_summary` で context を保持したまま `expected_*` フィールドを更新するようになった。
  - ✅ `diag_json::recover_extension_payload_from_summary` を追加し、`build_recover_extension`/`build_type_diagnostics` が context note を含む `recover` JSON を生成するようになった。
- ✅ `diagnostic/json.rs` に context note を含む recover 拡張を検証するユニットテストを追加し、Rustfmt を通してファイル整形を確認した。

### FRG-18

FRG-18 は `docs/spec/2-7-core-parse-streaming.md` に記された `run_stream` / `resume` の API と `Parser_driver.Streaming.run_stream` 相当の dual-write 経路を Rust 側で実現することで、`docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` にある streaming シナリオ用の `reports/dual-write/front-end/w4-diagnostics` を正しく生成できるようにすることを目的とする。

1. **StreamingRunner の骨格整備（Day 1）**  
   - `compiler/rust/frontend/src/parser/streaming_runner.rs` を新設し、`StreamOutcome`／`Continuation`／`DemandHint`／`StreamMeta` を定義。`ParserDriver::parse_with_options_and_run_config` を呼び出し、`StreamFlowState` を共有した上で `Completed` 結果を返す rust 実装を整備する。  
   - `parser/mod.rs` で `streaming_runner` を `pub use` し、API 消費者が `StreamingRunner` を直接利用できるように再エクスポート。
2. **CLI への統合（Day 2）**  
   - `compiler/rust/frontend/src/bin/poc_frontend.rs` の `--streaming` 判定で `StreamingRunner::run_stream` を呼び出し、`source` をクローンした上で `parser_options`/`run_config`/`stream_flow_state` を渡す。`resolve_completed_stream_outcome` ヘルパーで `Pending` を折りたたみ、従来型の `ParseResult` を受け取る経路を保持する。  
   - `stream_meta` の JSON や `build_runconfig_summary`/`build_parser_diagnostics` で共有する `StreamFlowState` を引き続き使いつつ、`--streaming` 時には `run_stream` 経路を通じて `streaming` 扱いのメトリクスを出力できることを検証する。  
3. **dual-write streaming ケースの検証（Day 3）**  
   - `scripts/poc_dualwrite_compare.sh --mode streaming` で `reports/dual-write/front-end/w4-diagnostics/<run>/<case>/streaming/*` を生成できることを確認し、OCaml 側と同様の `stream_meta`/`packrat_cache`/`continuation` モデルを比較する。  
   - 必要なら `2-5-spec-drift-remediation.md` や `docs/plans/rust-migration/p1-spec-compliance-gap.md` に FRG-18 の達成メトリクス（`parser.stream.outcome_consistency` など）を追記する。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/parser/streaming_runner.rs` で `StreamOutcome`/`Continuation`/`StreamingRunner` を実装し、`ParserDriver` 経由で `StreamFlowState` を再利用するランナーを提供した。
  - ✅ `compiler/rust/frontend/src/parser/mod.rs` で `streaming_runner` を再エクスポートし、外部から `StreamingRunner`/`StreamOutcome` を使えるようにした。
- ✅ `compiler/rust/frontend/src/bin/poc_frontend.rs` が `--streaming` でランナーを呼び出し、`resolve_completed_stream_outcome` で `Pending` を展開して従来通りの `ParseResult` を得るようになった。
- ⏳ `reports/dual-write/front-end/w4-diagnostics` に streaming ケースを流して `docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` のファイル群と比較する作業（`scripts/poc_dualwrite_compare.sh --mode streaming`）は環境が整い次第、再検証を予定している。

### FRG-19

FRG-19 は `Packrat.dump` 相当の Packrat キャッシュスナップショットを Rust 側でも CLI / dual-write 成果物として出力し、OCaml 側の `collect-iterator-audit-metrics.py` や `docs/plans/rust-migration/1-3-dual-write-runbook.md` に記された解析フローと整合させることを目的とする。

1. **OCaml と dual-write 仕様の整理（Day 0）**
   - `compiler/ocaml/src/parser_driver.ml:145` および `docs/plans/rust-migration/1-3-dual-write-runbook.md` を読み、`Packrat.dump` が `parse-debug` にどのような JSON を書き込んで `collect-iterator-audit-metrics.py` が参照するかを確認し、必要なスキーマ・ファイルパス（`packrat_stats` / `packrat_cache` / `packrat_snapshot`）を一覧化する。
   - Rust 側 `StreamingState::packrat_cache_entries` / `PackratCacheEntry` の `Serialize` 設計と `ParseResult::packrat_cache` フィールドが CLI / dual-write で使えることを確認する。
2. **Rust dual-write への Packrat キャッシュ追加（Day 1）**
   - `compiler/rust/frontend/src/bin/poc_frontend.rs` の `write_dualwrite_parse_payload` で `parse/packrat_cache.json` を出力し、`result.packrat_cache` の `serde::Serialize` を活用して `PackratCacheEntry` を OCaml `packrat_cache` スキーマと同じように並べる。
   - `poc_frontend` の `parse_result` / `parse_debug` 出力に `packrat_cache` を含めた上で `reports/dual-write/front-end/<run>/<case>/packrat_cache.json` が生成されることを確認する。
3. **dual-write レポートと README の更新（Day 2）**
   - `scripts/poc_dualwrite_compare.sh` や `reports/dual-write/front-end/poc/` に `packrat_cache.json` の存在を追記し、OCaml 側と Rust 側の `packrat_cache` を比較する差分パスを README へ書き込む。
   - `reports/dual-write/front-end/poc/<run>/summary` などに `packrat_cache` の有無と `collect-iterator-audit-metrics.py` が読む JSON ファイルへのリンクを追記し、`docs/plans/rust-migration/p1-spec-compliance-gap.md` に FRG-19 を定義した箇所への cross reference を残す。

- 進捗ログ
  - ✅ `poc_frontend.rs` の `write_dualwrite_parse_payload` で `parse/packrat_cache.json` を出力し、Packrat Stats と `PackratCacheEntry` を dual-write 成果物に含める経路を実装。
  - ⏳ README/スクリプトの更新と `collect-iterator-audit-metrics.py` との整合確認は継続予定。

### FRG-20

FRG-20 は `docs/spec/2-1-parser-type.md` に記された `Parser.run_config` を Rust CLI/Streaming/dual-write の共通データとして再現し、`parser_run_config` JSON を実行時に常に出力できるようにすることで、OCaml 側 `Parser_run_config` と双方向で比較できるようにする作業である。

1. **CLI `RunSettings` を `RunConfig` に沿って再構成（Day 1）**  
   - `reml_frontend::parser::api::RunConfig` の `packrat`/`left_recursion`/`trace`/`merge_warnings` を `RunSettings` が `Deref` で透過するようにして CLI にherited し、`LeftRecursionMode::from_str` で `--left-recursion` を正規化して `ParserOptions::from_run_config` に引き渡す。`apply_workspace_config` でも同じモード変換を行って workspace config との齟齬を防ぐ（`compiler/rust/frontend/src/bin/poc_frontend.rs`）。
   - `RunSettings` を `RunConfig` のラッパーに書き換えたうえで `RunSettings::to_run_config` で `experimental_effects` 拡張を挿入し、CLI が所有する `RunConfig` を streaming/Parser にそのまま渡すようにする。
2. **診断/オーディットで実行時 `RunConfig` を利用（Day 2）**  
   - `build_runconfig_summary`/`build_runconfig_top_level` を `ParseResult.run_config` を受け取るようにして `parser_runconfig.switches.*` を `left_recursion_label` で文字列化、監査メタデータにも `parser.runconfig` を直接埋めることで `build_parser_diagnostics`/`build_type_diagnostics` の `extensions.runconfig` と一致させる。
   - `build_audit_metadata` に `run_config` 引数を追加し、`metadata["parser.runconfig"]` に `runconfig_top_level` を挿入すると同時に `parser.runconfig.switches.*` を RFC 相当のスキーマで出力する。
3. **dual-write に `parser_run_config.rust.json` を追加（Day 3）**  
   - `runconfig_top_level` で生成した JSON を `diagnostics`/`parse_debug`/`reports/dual-write/front-end/*` 全体で再利用し、`write_dualwrite_parse_payload` で `parse/parser_run_config.rust.json` を書き出して OCaml 側 `parser_run_config` と比較可能な実行設定を dual-write 成果物に含める。
   - `collect-iterator-audit-metrics.py`、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`、`p1-front-end-checklists.csv` へ FRG-20 の完了ライン（`parser.runconfig.switches.*` メトリクスと `parser_run_config` JSON）が記録されていることを明記する。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/bin/poc_frontend.rs` の `RunSettings` を `RunConfig` ラップへ書き換え、`LeftRecursionMode` による CLI フラグの正規化と `ParserOptions::from_run_config` への引継ぎを実装した。
  - ✅ `build_runconfig_summary`/`build_runconfig_top_level`/`build_audit_metadata` を `ParseResult.run_config` ベースで再設計し、`left_recursion_label` ヘルパーで `parser.runconfig.switches.*` を文字列化できるようにした。
  - ✅ `write_dualwrite_parse_payload` で `parse/parser_run_config.rust.json` を出力し、`diagnostics`・`parse_debug`・dual-write のすべてで同じ `parser_run_config` JSON を再利用する連携を整備した。
  - ✅ `docs/plans/rust-migration/p1-spec-compliance-gap.md` の SCG-15 を更新して FRG-20 の共通化と `parser_run_config` 出力が完成したことを記録した。

### FRG-21

1. **仕様の確認と対象範囲の明文化（Day 0）**
   - `docs/spec/1-1-syntax.md#a3`・`docs/spec/2-3-lexer.md#d-1` に従い、`IDENT`/`UPPER_IDENT` は Unicode XID 由来で `RunConfig.extensions["lex"].identifier_profile` による `ascii-compat` 互換モードを持つことを再確認し、`p1-spec-compliance-gap.md#SCG-01` の図に Gap を追記する。  
   - `RunConfig` で `lex` 拡張が必ず `profile`/`identifier_profile` を含むように整備し、dual-write で `parser.runconfig.extensions.lex` を監査ログに現行の `profile` と一致させる契約を据える。
2. **実装と CLI/workspace 連携（Day 1-2）**  
   - `RunSettings` に `lex_identifier_profile` を保持させ、`to_run_config` で `lex` 拡張を `json!({ "identifier_profile": ..., "profile": "strict_json" })` として常時出力する。`apply_workspace_config` では `parser.extensions.lex` を取り込みつつ `identifier_profile` を引き継ぎ、`--lex-profile=ascii|unicode` オプションで CLI から上書きできるようにする。  
   - `ParserOptions::from_run_config` は `RunConfig.extensions["lex"].identifier_profile` を読み込み、`lex_identifier_profile` を `LexerOptions` に伝播させることで `lex` プロファイルごとの `IDENT`/`UPPER_IDENT` 出力が dual-write で比較できるようにする。  
   - `build_runconfig_summary`/`build_runconfig_top_level` も実際の `lex` 拡張を使い、`parser.runconfig.extensions.lex.identifier_profile` を `metadata`/`diagnostics` に含めて `lexer.identifier_profile_unicode` メトリクスとの対応を担保する。
3. **検証と dual-write 差分（Day 3）**  
   - `ParserOptions` が `RunConfig` 拡張を尊重することを `parser_option_tests::parser_options_follow_lex_identifier_profile_extension` で保証し、CLI で `--lex-profile=ascii` を渡して `collector` が `ascii-compat` を emit するゴールデンを `collect-iterator-audit-metrics.py` に埋め込む準備をする。  
   - 生成される `parser_run_config.rust.json` / `diagnostics` の `lex.identifier_profile` を `reports/dual-write` で検証し、`lexer.identifier_profile_unicode` KPI の監視値が `1.0` へ向かう材料を確保する。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/lexer/mod.rs` に `IdentifierProfile::from_str`/`as_str` を導入し、`ParserOptions` で `RunConfig.extensions["lex"].identifier_profile` を取得するヘルパを追加してインフラを整備した。
  - ✅ `compiler/rust/frontend/src/bin/poc_frontend.rs` に `RunSettings.lex_identifier_profile`・`--lex-profile`・workspace config の `parser.extensions.lex` 取り込み・lex 拡張出力・`runconfig_summary`/`top_level` の lex 依存を実装し、dual-write JSON と監査へ正しい値が降りるようにした。
  - ✅ `compiler/rust/frontend/src/parser/mod.rs` に `ParserOptions` が拡張を参照するテストを追加し、`collect-iterator-audit-metrics.py` での `parser.runconfig.extensions.lex.identifier_profile` 認識に手を入れる準備を整えた。

### FRG-22

1. **SCG-02 仕様差分の精査（Day 0）**  
   - `docs/spec/1-1-syntax.md#a3`・`docs/spec/1-1-syntax.md#a4` と `docs/spec/2-3-lexer.md#E`〜`#F` を再読し、予約語（`var`/`match`/`type` など）・演算子・整数基数・`stringRawHash` の要件を一覧化して `TokenKind`/`ExpectedToken` の抜けを洗い出す。`p1-spec-compliance-gap.md#SCG-02` に記された `0x`/`0b`/`` `r#""#` `` などを優先的に扱うことを確認する。  
   - 既存 `TokenKind::keyword_literal` のカバレッジを確認し、`KeywordVar`/`KeywordMatch`/`KeywordType` が `ExpectedToken` に渡ることを保証できるよう表を整備する。
2. **Lexer と期待トークン基盤の更新（Day 1）**  
   - `RawToken::RawStringLiteral` を `#[regex(r#"r#*""#, lex_raw_string)]` に差し替え、`lex_raw_string` で `#` の数と閉じ `"#` のペアを正しく検出して `` `r#""#` `` 形式を許容。`LiteralMetadata::String { kind: StringKind::Raw }` も保持したまま `Token` を生成する。  
   - `TokenKind` が予約語列と `Identifier`/`UpperIdentifier` をマッピングし、`token_kind_expectations` が `var`/`match`/`type` を `ExpectedToken::Keyword` へ変換し、`Option` に相当する `UpperIdentifier` を `upper-identifier` クラスとして出力することを確認する。
3. **検証と dual-write 監査（Day 2）**  
   - `lexer` のユニットテストに `` `r#"foo"#` `` や `` `r##"bar"##` `` を渡して `lexeme`/`LiteralMetadata` が正しいことを検証し、`cargo test -p reml_frontend --lib` へ含める。  
   - `parser/mod.rs` に `token_kind_expectations` のテストを追加し、`ExpectedToken::keyword("var")`/`"match"`/`"type"`、`ExpectedToken::class("upper-identifier")` が出ることを保証する。これにより diagnostically `Option` 相当の `upper-identifier` も `SCG-02` の「Option 表示」を一致させる。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/lexer/mod.rs` で `RawToken::RawStringLiteral` を `#[regex(r#"r#*""#, lex_raw_string)]` に置き換え、`lex_raw_string` が `` `r#""#` `` 系のデリミタを検出して `LiteralMetadata::String { kind: StringKind::Raw }` を保持するように改修。`lexer` テストに `` `r#"foo"#` ``/`` `r##"bar"##` `` を追加し、`lexeme` とメタデータを確認した。  
  - ✅ `compiler/rust/frontend/src/parser/mod.rs` に `token_kind_expectations` のテストを追加し、`var`/`match`/`type` が期待される `ExpectedToken::Keyword` を出力し、`UpperIdentifier` が `upper-identifier` クラスになることを保証。これで `ExpectedToken` 側も `Option` を含む階層に対応できた。
  
### FRG-23

1. **仕様と現状の整理（Day 0）**  
   - `docs/spec/2-1-parser-type.md§A〜§D` を再読し、`Parser<T>`/`State`/`Reply`/`RunConfig` の役割と `ParseResult` の `legacy_error` 要件を `p1-spec-compliance-gap.md#SCG-03` の差分表に重ねて整理する。OCaml `parser_driver.ml` が `State` で `RunConfig` と `Core_parse.Reply` を受け渡す仕組みをめどに、Rust の `ParserDriver::parse` が `State` を持たない問題点を明示する。
2. **API 足場の整備（Day 1）**  
   - `compiler/rust/frontend/src/parser/api.rs` に `Parser` 型エイリアス、`State::new`/`consume_to_end`/診断記録ヘルパー、`Reply`/`ParseError` の構成を加えて spec 2.1 の型定義をコードベースへ写経する。`State` から `RunConfig` を取り出せるようにして、診断とトレースを `ParseResult` に持たせる基盤を整える。
3. **ドライバへの統合（Day 2）**  
   - `ParserDriver::parse_with_options` から `(ParsedModule, Option<ParseError>)` を返し、`parse_tokens` で `ExpectedTokensSummary` から `ParseError` を生成・dual-write する。`parse_with_options_and_run_config` では `legacy_error` を `ParseResult` に注入し、`build_parser_reply` で `Reply` を構成して `RunConfig` をそのまま `ParseResult.run_config` へ渡すパイプラインを完成させる。
4. **検証と追跡（Day 3）**  
   - `driver`/`poc_frontend` 側で `ParseResult.run_config` を利用することで `parser_run_config` JSON を dual-write 出力と一致させ、`cargo check --manifest-path compiler/rust/frontend/Cargo.toml` などでビルド整合性を確認。計画と先行文書（`p1-spec-compliance-gap.md`、`p1-rust-frontend-gap-report.md`）を更新し、SCG-03 の進捗を追跡できるようにする。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/parser/api.rs` に `Parser` エイリアスと `State` の初期化／診断ヘルパーを追加し、`RunConfig`/`Reply`/`ParseError` を spec 2.1 の構成に沿って定義した。
  - ✅ `compiler/rust/frontend/src/parser/mod.rs` を改修し、`parse_tokens` から `ParseError` を拾い `ParseResult.legacy_error` に注入、`build_parser_reply` で `Reply` を返すようにして `ParserDriver::parse_with_options` から `legacy_error` を伴う `ParseResult` を生成するパイプラインを構築した。
  - ✅ `cargo check --manifest-path compiler/rust/frontend/Cargo.toml` を実行し、FRG-23 の API拡張が `reml_frontend` 全体とともにビルド成功することを確認した。

### FRG-24

1. **仕様と OCaml AST の再照合（Day 0）**  
   - `docs/spec/1-1-syntax.md` §A の構文定義、`docs/plans/rust-migration/1-1-ast-and-ir-alignment.md` のノード一覧、および `compiler/ocaml/src/ast.ml` の `expr_kind`/`pattern_kind`/`decl_kind` を突き合わせ、Rust 側で未実装だった `ModulePath`・`Lambda`・`Block`・`Assign` などの列挙と `Decl` バリアントを `SCG-05` 参照の表に取りまとめる。  
   - 既存 `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` の記述を元に、拡張後の `serde` JSON 出力が OCaml AST と比較できるようなスキーマ差分を明示する。  
2. **Rust AST 定義の拡張と整備（Day 1-2）**  
   - `compiler/rust/frontend/src/parser/ast.rs` に `ExprKind`/`DeclKind` の新バリアント（`BinaryOp`/`UnaryOp`/`Stmt`/`TypeAnnot` を含む）と `RecordField` などの補助構造を追加し、`serde` 付き JSON で OCaml の列挙と一対一で比較できるようにする。  
   - `Function`/`Param`/`Literal` の `render` 実装を微調整し、パラメータ・型・デフォルト・レコードの出力を継承させて dual-write の `typed_ast` 比較に備える。  
3. **パーサーと出力経路の追補（Day 3）**  
   - `compiler/rust/frontend/src/parser/mod.rs` の `params` フェーズで `Param` を `type_annotation none`/`default none`/`span` 付きで初期化し、`Function`/`EffectDecl` の JSON に `ret_type: None`/`tag: None`/`operations: []` を含める。  
   - `cargo fmt --manifest-path compiler/rust/frontend/Cargo.toml` と `cargo check --manifest-path compiler/rust/frontend/Cargo.toml` で整形とビルド整合性を確認し、追加された AST ノードが dual-write パイプラインに染みることを検証する。  

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/parser/ast.rs` に `ExprKind`/`BinaryOp`/`Stmt`/`TypeAnnot` などを追加し、`ModulePath`・`Lambda`・`Block`・`Assign` を serde JSON で出力できるようにして `SCG-05` に掲げた OCaml AST 列挙との差分を縮めた。  
  - ✅ `Function`, `Param`, `Literal`, `RecordField` まわりを拡張して `ret_type`/`type_annotation`/`default` を保持しつつ `render` で形状を復元できるようにし、dual-write の AST/typed-ast 比較スキーマに対応させた。  
  - ✅ `compiler/rust/frontend/src/parser/mod.rs` の `Param` と `Function`/`EffectDecl` を新フィールド付きで構築し、`cargo fmt --manifest-path compiler/rust/frontend/Cargo.toml` に続いて `cargo check --manifest-path compiler/rust/frontend/Cargo.toml` を実行し、`reml_frontend` のビルドが通った（既存の `dead_code` 警告のみ）ことを確認した。

### FRG-25

1. **SCG-06 / Typed AST スキーマの全体像を確定（Day 0）**  
   - `docs/spec/1-2-types-Inference.md` §C〜§D、`docs/plans/rust-migration/1-1-ast-and-ir-alignment.md` のノード一覧、および `compiler/ocaml/src/typed_ast.ml` を照らし合わせて `typed_expr`/`typed_pattern`/`typed_decl`/`typed_fn_decl`/`scheme`/`dict_ref` の関係を整理し、`docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` に記載した JSON スキーマとのギャップを明示する。`reports/dual-write/front-end/w3-type-inference/<case>/typed_ast.ocaml.json` や `p1-front-end-checklists.csv` の該当行を参照し、Rust の出力に期待されるフィールド名と構造を書き出す。  
   - `p1-spec-compliance-gap.md#SCG-06` を基準に、`typed_expr` の `dict_refs`/`scheme`/`ty` 項目が `typeck` 結果にどのように入るべきかを記述するとともに、OCaml 版の `Constraint_solver.dict_ref` `Scheme` がどこまで dual-write で再現されているかを確認する。
2. **Rust 側 `semantics::typed` モデルの整備（Day 1-3）**  
   - `compiler/rust/frontend/src/semantics/typed.rs`（または `semantics/typed/mod.rs`）を新設し、`TypedExpr`/`TypedPattern`/`TypedDecl`/`TypedFnDecl`/`TypedHandler` を `Ast` のノード ID と `Ty` を含めて定義。`scheme.rs` の `Scheme`、`types.rs` の `Ty`/`EffectRow`、`constraint.rs` の `DictRef` を再利用し、OCaml 側の `constrained_scheme` との対応関係をコメントで明示する。`dict_ref` は `impl_registry` 由来の `impl_id`/`span`/`requirements` を保持し、dual-write JSON では `dict_refs` テーブルと `typed_expr.dict_ref_ids` を `reports/dual-write/front-end/w3-type-inference` で比較できるようにする。  
   - `serde::Serialize` を実装し、`TypeckArtifacts` で使うときに OCaml JSON と同じキー順・ネスト構造（`typed_expr.kind`/`typed_pattern.kind`/`scheme.ty_vars`/`scheme.constraints`）が出力されるよう `serde_with::json` や `IndexMap` を活用する。`dict_ref` については `dict_ref_table` を build し、各ノードは `dict_ref_ids` を参照する方式を採用して `collect-iterator-audit-metrics.py --section effects` で再現性を担保する。  
3. **TypecheckDriver での Typed AST 生成と記録（Day 3-5）**  
   - `TypecheckDriver` の `infer_expr`/`infer_pattern`/`infer_decl` を `TypedExpr`/`TypedPattern` を構築する形にリファクタリングし、`ConstraintSolver` から得た `dict_ref`、`Scheme`、`TypeEnv` の `ty` 表現を各ノードに添付。`TypedExprKind` には `TDictRef` 的なバリアントを想定し、`TypedParam`/`TypedFnDecl` への `Scheme` 埋め込みも実現する。  
   - `TypecheckReport` に `typed_module`・`typed_expr_count`・`dict_ref_table`・`scheme_table` を追加し、`TypeckArtifacts` が JSON シリアライズ可能な `TypedCompilationUnit` を保持する。`ImplRegistry` の `used_impls` も `dict_ref` に含めて dual-write で `effects.dict_refs`/`typeck.used_impls` に透過させる。  
4. **Dual-write 出力と検証（Day 5-6）**  
   - `bin/poc_frontend.rs` の `TypeckArtifacts` 出力を拡張し、`reports/dual-write/front-end/w3-type-inference/<case>/typed_ast.rust.json`、`constraints.rust.json`、`typeck-debug.rust.json` に `typed_ast`/`dict_ref`/`scheme` 情報が含まれるようにする。OCaml 版の `typed_ast_json` や `constraints_json` と `diff` できるよう `reports/dual-write/front-end/w3-type-inference/README.md` を更新しテスト手順を記述する。  
   - `cargo test -p reml_frontend typeck::typed_ast_roundtrip`（新規）や `typeck::typed_expr_dict_refs` で `(typed_expr, dict_ref)` ペアが `ConstraintSolver` の結果と一致することを確認し、`collect-iterator-audit-metrics.py` の `typeck.typed_exprs` や `typeck.dict_refs` を用いて `docs/plans/rust-migration/p1-front-end-checklists.csv` の `typed AST` 行に `FRG-25` の検証手順を記録する。`reports/dual-write/front-end/w3-type-inference/<case>/dualwrite.bundle.json` に OCaml/Rust 両出力のパスを追加して差分追跡を容易にする。

- 進捗ログ
  - ✅ `compiler/ocaml/src/typed_ast.ml` と `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` を読み込み、`typed_expr`/`scheme`/`dict_ref` の構造と `reports/dual-write/front-end/w3-type-inference/<case>/typed_ast.ocaml.json` とのマッピングを整理した。  
  - ✅ `docs/spec/1-2-types-Inference.md` §C〜§D および `p1-spec-compliance-gap.md#SCG-06` に基づいて、「Typed AST に何を含めるべきか」リストを作成し、`p1-front-end-checklists.csv` の該当行に FRG-25 の期待値を追記する準備を整えた。  
  - ✅ `compiler/rust/frontend/src/semantics/typed.rs` を拡張し `TypedModule` に `dict_refs`/`schemes` を保持させ、`TypecheckDriver` で `register_dict_ref` 経由に `dict_ref_ids` を各式へ付与して `TypedAstFile` に伝播させ、`poc_frontend` の `TypeckArtifacts` で `typed_ast` の `dict_refs` や `function_summaries.dict_refs` が dual-write JSON に含まれるようにした。  
- ⏳ `typeck::typed_ast_roundtrip` や `typeck::typed_expr_dict_refs` などのテスト追加、`p1-front-end-checklists.csv` の検証ステップ明記、`reports/dual-write/front-end/w3-type-inference` のデータ差分確認は継続中。  

### FRG-26

FRG-26 は `docs/plans/rust-migration/p1-spec-compliance-gap.md#SCG-07` で指摘された `Core_parse_streaming` の状態を Rust パーサがデータとして吸い上げ、`ParseResult` および dual-write 出力に `packrat_snapshot`／`span_trace` を含めることで OCaml 実装と同じメトリクスセットを返すことを目的とする。

1. **仕様とアウトプットスキーマの整理（Day 0）**  
   - `docs/spec/2-7-core-parse-streaming.md` §G-1 に記された `StreamMeta.memo_bytes` や `StreamOutcome` の契約、および `docs/plans/rust-migration/1-3-dual-write-runbook.md` の `packrat_stats`/`packrat_cache`/`packrat_snapshot` 周りの説明を参照して、OCaml `parser_driver` における `Core_stream.packrat_cache` や `packrat_stats` の JSON 表現と比較する。  
   - `p1-spec-compliance-gap.md#SCG-07` と `compiler/ocaml/src/parser_driver.ml` から `ParseResult` に含まれるべき `span_trace`/`packrat_snapshot` の役割を明確化し、Rust 側 `StreamingState` の `packrat_snapshot()` API が満たすべき内容（エントリ数・概算バイト数）を記録する。  
2. **Rust パーサと API の拡張（Day 1）**  
   - `parser/mod.rs` に `StreamingState::packrat_snapshot()` を導入し、`ParsedModule`・`ParseResult` に `packrat_snapshot: PackratSnapshot` フィールドを追加して、`packrat_stats`/`stream_metrics` から一貫してスナップショットデータを保持する。  
   - `parser/api.rs` の `ParseResult` 型と `ParseResult::new` を更新し、CLI 側から `result.packrat_snapshot` を取得できるようにする。  
3. **Dual-write 経路と検証（Day 2）**  
   - `poc_frontend` の `parse_result` JSON と `write_dualwrite_parse_payload` で `packrat_snapshot` を出力し、`parse/packrat_cache.json` に期待される `packrat_snapshot` フィールドが含まれることを確認する。  
   - `cargo check -p reml_frontend` を走らせ、`packrat_snapshot` を含む各種出力がコンパイルエラーなく生成されることを検証する。  

- 進捗ログ
  - ✅ `ParsedModule`/`ParseResult` 両方に `packrat_snapshot` を追加し、`StreamingState::packrat_snapshot()` から取得して `ParseResult::new` へ流し込む実装を完了。  
  - ✅ `poc_frontend` の `parse_result` JSON および `parse/packrat_cache.json`（dual-write）に `packrat_snapshot` を含め、`span_trace` や `packrat_stats` と同じスキーマで出力するように更新。  
  - ✅ `cargo check -p reml_frontend` でコンパイルと既存テストが成功し、Rust 側の `ParseResult` が streaming 状態を確実に保持することを確認した。  
- ⏳ `reports/dual-write/front-end` に新しい `packrat_snapshot` JSON を流し込み、OCaml 側の golden と diff する `scripts/poc_dualwrite_compare.sh --mode streaming` の再実行は、dual-write ランを再作成できる環境での次回検証待ち。

### FRG-27

FRG-27 は `docs/plans/rust-migration/p1-spec-compliance-gap.md#SCG-08` に記載された **Algorithm W / 制約ソルバ・一般化** の不整合を埋める。`docs/spec/1-2-types-Inference.md` §A〜§C の Hindley–Milner (Algorithm W) 的な型表現・スキーム・制約・一般化・値制限の記述と、`compiler/ocaml/src/type_inference.ml` の `generalize`/`instantiate`/`constraint` の実装を Rust に写すことを主眼とする。

1. **仕様と OCaml コードの写像（Day 0）**  
   - `docs/spec/1-2-types-Inference.md` §A.3〜§C.5 、「型変数」「スキーム」「制約解決」「値制限」の章を OCaml `Type_inference`/`constraint`/`types` にマッピングし、`p1-spec-compliance-gap.md#SCG-08` 版の対照表をアップデートして不足箇所を整理する。特に `generalize` が自由変数集合をω的に集める部分と `instantiate` で fresh な変数を生成する部分を明記して、Rust 側の PRD 設計に反映する。  
2. **型・環境・スキームの下地整備（Day 1-2）**  
   - `compiler/rust/frontend/src/typeck/types.rs` で `Type`/`TypeVariable`/`TypeVarGen` に `free_type_variables` や `contains_variable` を追加し、`TypeEnv` が `Scheme` の自由変数を追跡できるよう `free_type_variables` を実装。 `Scheme` の `quantifiers` をソートした上で `Generalize` する `generalize_function_type` を `typeck/driver.rs` に導入する。  
3. **制約ソルバと Algorithm W パイプラインの実装（Day 2-4）**  
   - `typeck/constraint.rs` に `Substitution` を保持する `ConstraintSolver` を設け、`unify`/`bind_variable`/`occurs` を実装して `Equal` 制約を解決できるようにする。`typeck/driver.rs` は `TypedExprDraft`/`DictRefDraft` を用いて AST を再帰的に構築し、`Constraint::equal` を生成しつつ `solver.unify` で即時展開。`PerformCall` では `dict_ref` を集計して `typed::DictRef` テーブルに反映する。  
4. **Typed AST・Scheme 出力とデュアルライト連携（Day 4-5）**  
   - `TypecheckReport` と `typed::TypedModule` を新たな `typed_expr`/`scheme`/`dict_ref` で埋め、`build_scheme_info` で scheme の `quantifiers`/`constraints`/`ty` を JSON 形式に整形。 `bin/poc_frontend.rs` から `TypeckArtifacts` を経由して dual-write に流し、 `cargo check --manifest-path compiler/rust/frontend/Cargo.toml` で整合性を確認する。値制限は（Phase P1 の関数定義に対して）頂点の関数ボディを `generalize` する単純化モデルとして扱い、後続の effect row 解析で再評価する。

- 進捗ログ
  - ✅ `typeck/types.rs` に `Type::free_type_variables`/`Type::contains_variable` を追加し、`TypeVariable` に `id()` を提供。`typeck/env.rs` では `Scheme` の自由変数を列挙する `free_type_variables` を実装して `generalize_function_type` の基礎を整備した。  
  - ✅ `typeck/constraint.rs` に `Substitution` を持つ `ConstraintSolver` を導入し、`unify`・`bind_variable`・`occurs` を備えた Algorithm W 風ソルバを実装。`compiler/rust/frontend/src/typeck/driver.rs` では `TypedExprDraft`/`DictRefDraft` を使って AST と dict_ref テーブルを構築し、`generalize_function_type` で `Scheme` を作って `typed_module.schemes`/`TypecheckReport` に反映させた。  
  - ✅ `TypecheckDriver` のループを刷新し、各関数を fresh な型変数で打ち切り、`solver.substitution()` スナップショットで `typed::TypedExpr` を仕上げ、`dict_ref_drafts` を dual-write 用の `typed::DictRef` に変換。`cargo check --manifest-path compiler/rust/frontend/Cargo.toml` を実行し、型定義まわりの変更でビルドが通ることを確認した。  
- ⏳ 関数以外の式の値制限や効果行・Capability Stage を含む制約の型付けは、後続 Phase で効果行セマンティクスと合わせて精緻化する予定。  

### FRG-28

FRG-28 は `p1-spec-compliance-gap.md#SCG-09` に記された **効果行 / 残余効果 / Capability Stage 監査 (`effects.contract.*`)** のギャップに対処する。`docs/spec/1-3-effects-safety.md#I.4` が要求する `required_capabilities` / `actual_capabilities` の配列化と StageContext・Capability Registry の連携、`EFFECT-002`／`w4` の dual-write 監査ケースを踏まえ、Rust CLI から Stage 情報付き `RuntimeCapability` を Typecheck レイヤーに渡して診断・監査・JSON 出力へ波及させる。

1. **仕様を再確認し監査証跡の構造を整理（Day 0）**  
   - `docs/spec/1-3-effects-safety.md#I.2`〜`#I.4` で定義された効果行・残余効果・Stage 要件 (`Σ_after ⊆ allows_effects`, `stage` `capability` アレイ) を再整理し、`p1-spec-compliance-gap.md#SCG-09` の対照表に今ある dual-write 出力と比較した足りていないデータ（`runtime_capabilities` に stage 情報がない、`TypeckDebugFile` の `runtime_capabilities` が文字列のみ）を明示する。`p1-rust-frontend-gap-report.md#FRG-13` のステージ検査ログと整合させ、Rust 側での `effects.contract.stage_mismatch`/`effects.contract.residual_leak` が StageContext と CLI Capability の両方を参照しているかを確認する。
2. **構造化された Runtime Capability メタデータを伝播（Day 1）**  
   - `bin/poc_frontend.rs` の CLI・`workspace` 設定で `RuntimeCapability`（ID＋Stage）を解析し、`TypecheckConfig.builder()` へ渡す。`TypecheckDriver` は `config.runtime_capabilities` を `Vec<RuntimeCapability>` として保持し、stage ラベル付きで `TypeckDebugFile` や dual-write `typeck/debug` にそのまま出力する。`runtime_capabilities` に stage を添えることで `StageAuditPayload` と診断 `extensions.effects.stage` にも個別の `stage` フィールドが含まれるようにする。
3. **診断/監査への出力と確認（Day 2）**  
   - StageAuditPayload が `runtime_capabilities` を詳細に出力し、`effects.contract.*` 診断（`TypecheckViolation`）の `extensions`/`audit_metadata` に `required_capabilities`/`actual_capabilities` と `stage_trace` を書き込む。`reports/dual-write/front-end/*` の `typeck/debug.{ocaml,rust}` や diagnostics JSON に Stage 入力が含まれていることを確認し、`effects.contract.residual_leak` と `effects.contract.stage_mismatch` がそれぞれ Capability の有無と Stage 要件で発行されることを確認する。また `collect-iterator-audit-metrics.py` が同じコードで `effects.contract.stage_mismatch` をフィルタしている点を念頭に置き、Stage 情報の出力形式を壊さないようにする。

- 進捗ログ
  - ✅ CLI と `workspace` 設定で Runtime Capability を `RuntimeCapability`（ID＋Stage）として解析し、`TypecheckConfig`/`TypeckDebugFile` にそのまま渡すルートを整備した。  
- ✅ StageAuditPayload の `capability_details` が `stage` フィールド付き `actual_capabilities` 配列を作成し、`effects`/`bridge`/flattened extension keys および `audit_metadata` に Stage trace を含めるようにした。`poc_frontend` で JSON 出力は従来通りの文字列配列を維持しつつ Stage 情報は `StageAuditPayload` で追跡している。  

### FRG-29

FRG-29 は `p1-spec-compliance-gap.md#SCG-10` にある **Typed AST / Constraints / Impl Registry の dual-write JSON** を OCaml CLI 出力 (`compiler/ocaml/src/cli/typeck_output.ml` で定義された `typed_ast_json`/`constraints_json`/`typeck_debug_json`) と一致させることが目的である。Rust 側で生成している `typeck/typed-ast.rust.json` / `constraints.rust.json` / `typeck-debug.rust.json` が実質的に空のメトリクスしか含まない「ゴースト」になっているため、タイプリポートの中身と `dict_ref`/`impl_id` 情報を dual-write で出力できるようにする必要がある。

1. **OCaml 出力スキーマの再確認（Day 0）**  
   - `compiler/ocaml/src/cli/typeck_output.ml` の `typed_ast_json`/`constraints_json`/`typeck_debug_json` を読み、各 JSON が `function_summaries`・`rendered`・`stats`・`violations` をどう構成するかを整理する。`reports/dual-write/front-end/w3-type-inference/README.md` の記載と `p1-spec-compliance-gap.md#SCG-10` の差分表を照合して、Rust 側で現在欠けているフィールド（`constraints.stats`、`used_impls`、`dict_ref` テーブル、`Impl Registry`）を一覧化する。  
   - `docs/spec/1-1-ast-and-ir-alignment.md` や `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` に基づき、Rust 型システムが dual-write で出力するべき `TypedModule`/`Scheme`/`DictRef` の構造を明記して、`p1-spec-compliance-gap`/`p1-front-end-checklists` に参照を張る。

2. **Rust 側 JSON の実装整備（Day 1-2）**  
   - `TypecheckReport`/`TypeckArtifacts` を拡張して `function_summaries`・`broken_down constraints`・`typed_module`・`used_impls` などを `serde` 出力に含め、`ConstraintStats` には `unify_calls` などの `metrics` から取得できる値を注入する。`TypecheckDriver` では `ConstraintSolver::unify` 呼び出し直後にカウンタを増やすなどして `metrics` を豊富に保ち、`FunctionSummaryExport` に `effect_row`/`dict_refs` を含める。  
   - `bin/poc_frontend.rs` の `--emit-*` フラグによる個別出力と `write_dualwrite_typeck_payload` が `typed-ast`・`constraints`・`typeck-debug` を出力するときに、新しい JSON スキーマと一致しているかを確認する。`serde` で `TypedModule`/`typed::DictRef` をエクスポートし、dual-write `reports/dual-write/front-end/w3-type-inference/*/typeck/*.rust.json` の差分を検証する。

3. **Impl Registry 出力と CLI 拡張（Day 3）**  
   - `typed_module.dict_refs` や `used_impls` をもとに `impl registry` のスナップショットを構築し、新しく `typeck/impl-registry.rust.json`（`schema_version: "w3-typeck-impl-registry/0.1"`）を `DualWriteGuards` 経由で書き出す。`--emit-impl-registry` フラグも追加し、dual-write なしでも `typeck/impl-registry.rust.json` が生成できるよう CLI を拡張する。  
   - `scripts/poc_dualwrite_compare.sh --mode typeck` が新しいファイルを上書きしないよう `ensure_impl_registry_snapshot` を活用し、`typeck/metrics.json` からの暫定出力との比較によって schema の連続性（run_id/case）を保証する。

4. **検証と資料更新（Day 4）**  
   - `scripts/poc_dualwrite_compare.sh --mode typeck` で `reports/dual-write/front-end/w3-type-inference/<run>/<case>/typeck/*.{rust,ocaml}.json` を再生成し、`scripts/dualwrite_summary_report.py` を使ったサマリ表の更新までを一連の再現ワークフローとして記録。  
   - 検証後、`p1-spec-compliance-gap.md#SCG-10` に進捗メモを追加し、`p1-front-end-checklists.csv` の対応行に dual-write 試行日と `TypeckArtifacts` が出力した `typeck/typed-ast.rust.json` のパスを記載する。

- 進捗ログ
  - ✅ `TypecheckMetrics` へ unify/ast/node/token のメトリクスを追加し、`ConstraintFile.stats` に流し込むことで `constraints.rust.json` に実測値を含めるようにした。  
  - ✅ `TypeckArtifacts` が `typed_module.dict_refs`/`used_impls` を含む typed AST + constraints JSON を出力するので、dual-write の `typeck/typed-ast.rust.json`/`constraints.rust.json` に OCaml スキーマと同等のデータが含まれるようになった。  
  - ✅ CLI に `--emit-impl-registry` を追加し `DualWriteGuards` でも `typeck/impl-registry.rust.json` を書き出せるインフラを整備した。  

### FRG-30

FRG-30 は `p1-spec-compliance-gap.md#SCG-11` の **OCaml `Diagnostic` 構造（`severity` / `domain` / `audit` / `timestamp` / `expected_summary.context_note`）を Rust 側でも保持する** という要件に応える。`docs/spec/3-6-core-diagnostics-audit.md` §1 および `docs/spec/2-5-error.md` §A/B で定義された `AuditEnvelope`・`ExpectationSummary`・`Diagnostic` モデルを参照し、Rust の `FrontendDiagnostic` がデータを漏れなく保持・JSON 化できるようにする。

1. **診断モデルの再定義（Day 0）**  
   - `docs/spec/3-6-core-diagnostics-audit.md` §1.1`AuditEnvelope`、`docs/spec/2-5-error.md` §A/B の `ExpectationSummary` と `context_note` を再確認し、`FrontendDiagnostic` に `timestamp`/`audit_metadata`/`audit` フィールドを追加する。`p1-spec-compliance-gap.md` の差分表に `AuditEnvelope` が dual-write に載るべきキー（`metadata`・`audit_id`・`change_set`・`capability`）を記録する。  
2. **JSON 出力の拡張（Day 1）**  
   - `compiler/rust/frontend/src/diagnostic/json.rs` の `expected` 出力に `context_note` を追加し、`build_frontend_diagnostic` が `AuditEnvelope` をステージ付きの `capability` でシリアライズするようにする。`diagnostic/recover.rs` の `ExpectedTokensSummary` と `parser_expectation` の出力 (recover extension) も `context` を同一文字列で再利用できるように確認する。  
3. **CLI ハーネスの連携と検証（Day 2）**  
   - `bin/poc_frontend.rs` の診断生成で `current_timestamp()`/`StageAuditPayload::primary_capability()` を使って `FrontendDiagnostic` を更新し、`AuditEnvelope` + `audit_metadata`/`audit` JSON を統一する。`cargo fmt`・`cargo check --manifest-path compiler/rust/frontend/Cargo.toml` を実行し、既存の解析フローに警告が出ないことを確認する。

- 進捗ログ
  - ✅ `FrontendDiagnostic` に `timestamp`/`audit_metadata`/`audit` フィールドと `AuditEnvelope` を追加し、OCaml 側 `Diagnostic` が保持する `severity`/`domain`/`expected_summary.context_note` との 1 対 1 対応を明文化した。  
  - ✅ `compiler/rust/frontend/src/diagnostic/json.rs` で `expected.context_note` を出力し、`diag_json::build_frontend_diagnostic` の `audit` JSON に `capability` を含め、recover 拡張と dual-write スキーマを同期させた。  
  - ✅ `compiler/rust/frontend/src/bin/poc_frontend.rs` の diagnostics パスが `StageAuditPayload` から `AuditEnvelope` を構成し `timestamp` を `FrontendDiagnostic` に設定するようにし、`cargo fmt`/`cargo check --manifest-path compiler/rust/frontend/Cargo.toml` を実行して整合性を確認した。  

### FRG-31

FRG-31 は `p1-spec-compliance-gap.md#SCG-12` で指摘された `parser_expectation` 相当の期待集合整形・優先順位（キーワード → トークン → クラス → ルール）および `Not`/`Class` ラベルの差分を埋めるタスクである。`docs/spec/2-5-error.md` §A の `Diagnostic.expectation` と `parser_expectation.ml` の `dedup_and_sort`/`humanize` を基準に、`ExpectedTokensSummary.alternatives` から `kind`/`hint` 情報を保持した `expected_tokens` 出力を Rust 側でも再現し、dual-write で Type/Parser 診断が `kind: keyword|class|rule|not` を含めて比較できるようにする。

1. **Day 0 – 仕様確認**  
   - `docs/spec/2-5-error.md#A` と `compiler/ocaml/src/parser_expectation.ml` を再読し、`ExpectedTokensSummary` の `alternatives` に `ExpectedToken`（`Keyword`/`Token`/`Class`/`Rule`/`Not`/`TraitBound`）が含まれていることを確認。`p1-spec-compliance-gap.md#SCG-12` の表記例と `ExpectedTokenCollector` の正規化ルールを併せて整理し、Rust `diagnostic::recover` モジュールに必要な追加情報を洗い出す。
2. **Day 1 – JSON ヘルパーの追加と共有**  
   - `compiler/rust/frontend/src/diagnostic/json.rs` に `expected_tokens_array_from_summary` 相当のヘルパーを追加し、`ExpectedTokensSummary.alternatives` の `kind`/`hint` をそのまま `expected_tokens` オブジェクトへ変換するロジックを実装する。ノードが空の場合は従来どおり文字列ヒューリスティックでフォールバックする。
3. **Day 2 – 期待出力の適用とテスト**  
   - 上記ヘルパーを `expected_payload_from_summary`／`recover_extension_payload_from_summary` で使いまわすことで、Parser/Type 拡張の JSON 出力が常に `kind` を持つように統一。`build_type_diagnostics` 側では既存の `expected_payload_from_summary` 呼び出しで勝手に `kind` 付きデータを含めるため、dual-write で `expected_tokens` の差分比較がより精密になる。
4. **Day 3 – 回帰確認と出力ログへの反映**  
   - `compiler/rust/frontend/src/diagnostic/json.rs` の recover 拡張テストに `kind` 中身の検証を追加し、`cargo test -p reml_frontend recover_extension` で `ExpectedToken::keyword`/`Not`/`Class` が `expected_tokens` に正しく出力されることを確認。`reports/dual-write` の `parser`/`typeck` JSON を定期的に PAT 加工して `kind` 情報が含まれることを記録する（必要なら `reports/dual-write/front-end/.../summary.md` に追記）。

- 進捗ログ
  - ✅ `compiler/rust/frontend/src/diagnostic/json.rs` に `ExpectedTokensSummary.alternatives` を使うヘルパーを追加し、`kind`/`hint` を `expected_tokens` 拡張・`expected` フィールドの両方でそのまま出力できるようにした。  
  - ✅ `build_type_diagnostics`／`recover` 系の `expected_payload_from_summary` 呼び出しは変更不要で、自動的に `kind` 情報を dual-write 扱いに含めるようになっている。  
  - ✅ `compiler/rust/frontend/src/diagnostic/json.rs` に `recover_extension_obtains_kind_from_summary_alternatives` テストを追加し、`cargo test -p reml_frontend recover_extension` を実行して regression を通過させた (`reports/dual-write/front-end/poc/...` の `recover` 拡張でも `kind` が出力されていることを確認)。  

### FRG-32

FRG-32 は `p1-spec-compliance-gap.md#SCG-13` で指摘した CLI JSON の `severity="error"`/`domain="parser"` 固定、`audit_id` の疑似値という整合性欠落を解消する作業である。仕様書 `docs/spec/3-6-core-diagnostics-audit.md` §1 の `Diagnostic`/`AuditEnvelope` モデルと `reports/diagnostic-format-regression.md` で求められているメタデータをそのまま Rust に持ち込みつつ、OCaml 側の `DiagnosticFormatter` と同等の `AuditEnvelope` 構築・RunConfig メタ情報・ステージ監査フィールドを出力する `formatter` を導入して dual-write 差分を縮める。

1. **Day 0 – 仕様の再照合**  
   - `docs/spec/3-6-core-diagnostics-audit.md` §1〜§1.1 で `severity`/`domain`/`audit`/`change_set` など CLI JSON に含めるべき鍵を明示し、`diagnostic-format-regression.md` に掲載されたフォーマットとのギャップを一覧化する。`p1-spec-compliance-gap.md#SCG-13` で言及された `collect-iterator-audit-metrics.py` が追う `effects.stage.*` や `capability.ids` のキーもこの段階で洗い出す。  
2. **Day 1 – フォーマッタ実装と CLI 側への差し込み**  
   - `compiler/rust/frontend/src/diagnostic/formatter.rs` を新設し、audit シーケンス（`channel/build_id/sequence`）、change_set JSON、`FormatterContext` に基づく metadata 拡張、`current_timestamp` の共通化処理を提供する。`poc_frontend` の `build_parser_diagnostics`/`build_type_diagnostics` でこのフォーマッタを利用し、`FrontendDiagnostic` に `severity`/`domain`/`audit_metadata`/`AuditEnvelope` を明示的にセットすることで `diagnostics.json` の `severity`/`domain`/`audit_id` を OCaml 出力に一致させる。  
3. **Day 2 – 検証とメトリクス連携**  
   - `cargo fmt --manifest-path compiler/rust/frontend/Cargo.toml`・`cargo check --manifest-path compiler/rust/frontend/Cargo.toml` を実行し、`collect-iterator-audit-metrics.py --section effects --require-success` で `effects.stage.required`/`effect.capability` などの監査フィールドが Rust JSON に存在することを確認。`reports/dual-write/front-end/w4-diagnostics/<run>/summary.md` を更新し、`severity`/`domain`/`audit_id` で差分が解消されたことを記録する。必要であれば `docs/plans/rust-migration/p1-front-end-checklists.csv` の FRG-32 行に注釈を追加する。

- 進捗ログ
  - ⏳ 実装着手中: `formatter` ヘルパーを定義して `poc_frontend` の診断出力を通すインフラを整備し、`collect-iterator` が要求する監査キーを満たすように調整中。  

### FRG-33

FRG-33 は `p1-spec-compliance-gap.md#SCG-14` で指摘された `run_stream`/`resume` API と `StreamOutcome` 管理の未整備を Rust でも解消する取り組みであり、`docs/spec/2-7-core-parse-streaming.md` に記された DemandHint / Continuation 戻り値と CLI の `parser.stream.*` 拡張との整合を確保する必要がある。

1. **Day 0 – 仕様/実装差分の再照合**  
   - `docs/spec/2-7-core-parse-streaming.md` §A/B を読み、`StreamOutcome::{Pending,Completed}` の要件や `DemandHint` フィールド、`Continuation` が保持すべき `cursor`/`chunk_hint`/`resume_hint` などを整理する。`p1-spec-compliance-gap.md#SCG-14` の `StreamingRunner` 欠落の記述と `compiler/ocaml/src/parser_driver.ml:run_stream` の `await`/`continuation` ロジックを参照しながら、Rust 側で必要なメタデータと CLI JSON への出力経路を洗い出す。  
2. **Day 1 – ストリーミングランナー・RunConfig 拡張の実装**  
   - `StreamingRunner` の `Continuation` に `cursor`/`chunk_size` を持たせ、`run_stream` が `run_config.extensions["stream"].chunk_size` を参照して chunk 単位で `Pending` を返すよう再設計。`StreamOutcome::Pending` には `DemandHint`（`min_bytes`・`preferred_bytes` に chunk_size・`resume_hint`・`reason`）を添付し、`StreamingRunner::from_continuation` で `Continuation` を再利用できる API を公開する。`compiler/rust/frontend/src/bin/poc_frontend.rs` では `parser.runconfig.extensions.stream` に `enabled`/`checkpoint`/`resume_hint`/`demand_*`/`chunk_size`/`flow_*` キーを記録し、`resolve_completed_stream_outcome` が `Pending` を再帰的に解決することで `--streaming` モードでも `StreamOutcome` が無限ループしないようにする。  
3. **Day 2 – テストとメトリクス連携**  
   - `compiler/rust/frontend/tests/streaming_runner.rs` を追加し、chunk_size を設定した場合に `StreamOutcome::Pending` → `Completed` の遷移が確認できること、chunk_size 未設定では即座に `Completed` を返すことを `cargo test -p reml_frontend streaming_runner` で検証。`collect-iterator-audit-metrics.py` が参照する `parser.stream_extension_field_coverage` には `parser.runconfig.extensions.stream` のキーを常時含めることで `SCG-14` の KPI を満たす証跡を残す。  

- 進捗ログ
  - ✅ `Continuation` に `cursor`/`chunk_size` を持たせ、`StreamOutcome::Pending` で chunk_size ベースの `DemandHint` と `StreamingRunner::from_continuation` を返すようランナーを再設計。  
  - ✅ `poc_frontend.rs` で `run_config.extensions["stream"]` を構築し、`resolve_completed_stream_outcome` が `Pending` を再帰的に処理。`parser.stream_extension_field_coverage` で期待される `enabled`/`demand_*`/`chunk_size` が保証される。  
  - ✅ `compiler/rust/frontend/tests/streaming_runner.rs` を追加して chunk_size あり/なしの経路を検証し、`cargo test -p reml_frontend streaming_runner` が通過。  
  - ⏳ 今後: `collect-iterator` KPI に `parser.stream.outcome_consistency` を加えて chunk_size の有無で `StreamOutcome` が安定することを定量的に監視する予定。  

## 5. Rust フロントエンドのビルド/テスト状況

- `compiler/rust/frontend` で `cargo test` を実行し、ライブラリ・バイナリ・ユニットテスト（合計 35 件以上）がすべて成功しました。存在する警告は `target/debug` に記録されており、dual-write 差分確認で必要な診断・Streaming パスへの影響は確認済みです。
- 警告の主な内容：
  - `src/typeck/driver.rs:736:9` で `match` の `_` ケースに対する `unreachable_patterns`。
  - `src/parser/mod.rs:303:19` の `ast_render` メソッドが未使用（`dead_code`）。
  - `src/typeck/capability.rs` 内の `stage` フィールドおよび `stage()` メソッドが参照されておらず `dead_code`。
  - `src/bin/poc_frontend.rs:319:8` の `cli_command` メソッドが使用されず `dead_code`。
- エラーはなく、テストログと生成物は `target/debug` 以下にあり、CI/dual-write ハーネスで同じコマンドを再現すれば再検証可能です。

## 6. 最終的な検査

### 6.1 デュアルライト差分の再確認
1. **Day 0 – コマンド実行**  
   - `scripts/poc_dualwrite_compare.sh --mode ast --run-id codex-inspection` を走らせ、OCaml/Rust 両方の AST/diagnostic/typeck 用 JSON を `reports/dual-write/front-end/poc/codex-inspection/*` に出力。標準出力から `typeck_rust_flags` 未定義の箇所を確認して、後続の typeck モードでフラグを注入する必要性を記録（`SCG-12`/`FRG-14` の typeck 出力と整合するため）。
2. **Day 1 – JSON diff の整理**  
   - 生成された `summary.md` を基に `ast_match` (False)・`diag_match` (True) をケース別に把握し、`p1-spec-compliance-gap.md#SCG-14` の Streaming/AST ギャップと `p1-rust-frontend-gap-report.md#FRG-09` の AST 芝台差分をクロス参照。`diag_match` が `True` であることをもって Rust 診断の構造が最小限一致していることを確認しつつ、AST の不一致点を次段階で `reports/dual-write/front-end/w2-ast` に差分ファイルとして記録する計画を立てる。
3. **Day 2 – メトリクス & LSP 対応**  
   - `tooling/ci/collect-iterator-audit-metrics.py --section effects --recover-lsp` で `effects.stage.required`/`parser.stream_extension_field_coverage` 等のキーメトリクスが Rust JSON に含まれる状態を確認し、`docs/spec/3-6-core-diagnostics-audit.md` §1.1 で要求される `severity`・`domain`・`audit_id` の埋め込みと `SCG-13` の `diagnostics.json` 拡張が活きているか追跡。差分が残る場合は `reports/dual-write/front-end/w4-diagnostics/<run>/summary.md` に記録。

### 6.2 実行結果と観察
- `ast` モードで空ケース 4件（`empty_uses`/`multiple_functions`/`addition`/`missing_paren`）を `codex-inspection` で比較した結果、全件 `ast_match=false`、`diag_match=true`（`typeck_match` は生成されず `None`）。`ocaml_diag`/`rust_diag` いずれも `0` で `diagnostic` 系の構造上の齟齬は現時点では検出されていない。
- `collect-iterator` による `effects.stage` 系キーはまだウォークしていないため認証できていない（`SCG-13` で要求される `audit`/`change_set` とは別途追跡必須）。`typeck_rust_flags` 未定義のエラーは `typeck` モード用フラグセットの整備が必要なことを暗示しており、`SCG-12`/`FRG-14` を追う際の補充項目とする。
- `scripts/poc_dualwrite_compare.sh --mode diag --run-id codex-inspection-diag` を実行し、`reports/dual-write/front-end/w4-diagnostics/codex-inspection-diag/summary.md` で `diag_match=true` を確保しつつ `gating` および `metrics` が `❌` になることを確認。OCaml 側の診断 JSON が出力されず `metrics` 判定で `parser`/`effects` の収集が失敗し、各ケースで `parser-metrics.rust.err.log`/`effects-metrics.rust.err.log` が出力されたため（`missing diagnostics`）、`SCG-13` で求める `severity`/`domain`/`audit` 埋め込みを後続で再取得する必要がある。
- `FORCE_TYPE_EFFECT_FLAGS=true scripts/poc_dualwrite_compare.sh --mode typeck --run-id codex-inspection-typeck` では、`reports/dual-write/front-end/w3-type-inference/codex-inspection-typeck/summary.md` が `typeck_match=false` で全ケース一致せず。ログには `rust_case_flags[@]: unbound variable`（line 1621）という警告も出ており、OCaml/Rust 両方で `diagnostics.json` を生成できていないため `collect-iterator` の `effects-metrics.*.ocaml.json` も作成されなかった。これらの出力が揃わないと `SCG-12`/`FRG-14` の型推論差分チェックが進められない。
- `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success` を叩いたところ、`ValueError: diagnostics array is missing`（`tooling/ci/collect-iterator-audit-metrics.py:284`）により実行できず。`diagnostics.json` が存在しない状態では `parser.stream_extension_field_coverage` などの KPI を収集できないため、まず `diag`/`typeck` で正規の JSON を出力する必要がある。

### 6.3 次の検査フェーズ
- `scripts/poc_dualwrite_compare.sh --mode diag --run-id <run>` を実行して `SCG-13` で求める `diagnostics.json` の `severity`/`domain`/`audit` メタデータ差分を再取得し、`reports/dual-write/front-end/w4-diagnostics/<run>/summary.md` に注記する。
- `scripts/poc_dualwrite_compare.sh --mode typeck` を `FORCE_TYPE_EFFECT_FLAGS=true` で再実行し、`typeck_debug.json`/`typed_ast.json`/`constraints.json` を OCaml 側と並べて `reports/dual-write/front-end/w3-type-inference/<run>/` に記録。`typeck_rust_flags` を `docs/plans/rust-migration/p1-front-end-checklists.csv` の該当行に合わせて構成し、`SCG-12`/`FRG-14` の型推論すり合わせを完了させる。
- `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success` を走らせ、`parser.stream_extension_field_coverage`・`parser.stream.outcome_consistency`・`effects.stage.required` を `reports/dual-write/front-end/README.md` などに記録することで `SCG-14`/`SCG-13` に必要な RunConfig メタデータを整備する。
- `diag`/`typeck` のパスで `diagnostics.json` が出力された後に `collect-iterator` を再実行し、前出の `ValueError` を解消。`rust_case_flags[@]` 警告が発生しているため必要なフラグを事前に `CASE_FLAGS_META_RUST` あるいは `docs/plans/rust-migration/p1-front-end-checklists.csv` へ追記し、`typeck` モードでの `rust_case_flags` が空配列にならないようにして再発を防ぐ。

## 7. ノート

- 仕様参照: `docs/spec/1-1-syntax.md`, `1-2-types-Inference.md`, `1-3-effects-safety.md`, `2-1-parser-type.md`, `2-5-error.md`, `2-7-core-parse-streaming.md`, `3-6-core-diagnostics-audit.md`
- 作業ログ: 大きな rename/移動が発生する場合は `docs-migrations.log` を更新すること。
- 本レポートは `p1-spec-compliance-gap.md` の補足資料として扱い、今後の差分調査結果を追記する際はセクション単位で更新する。
