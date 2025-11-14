# P1 フロントエンド仕様未達リスト（2028-02 現状）

Phase P1（フロントエンド移植）の達成条件を Reml 仕様に照らして再確認したところ、現行 Rust 実装では未実装／不完全な領域が複数残っている。本書では `docs/plans/rust-migration/1-0-front-end-transition.md` が規定する 4 つの対象範囲（構文解析・AST/IR・型推論・診断前処理）と `docs/plans/rust-migration/p1-front-end-checklists.csv` の達成要件に基づき、優先度順にギャップと対応方針を整理する。

各表には ID（`SCG-XX`）を付与し、未達項目の参照とチケット連携を容易にした。

## 0. 参照資料

- 仕様: `docs/spec/1-1-syntax.md`, `1-2-types-Inference.md`, `1-3-effects-safety.md`, `2-1-parser-type.md`, `2-5-error.md`, `2-7-core-parse-streaming.md`, `3-6-core-diagnostics-audit.md`
- 計画: `docs/plans/rust-migration/1-0-front-end-transition.md`, `1-1-ast-and-ir-alignment.md`, `1-2-diagnostic-compatibility.md`, `1-3-dual-write-runbook.md`
- 実装: `compiler/rust/frontend/src/{lexer,parser,diagnostic,streaming,typeck}` および CLI `compiler/rust/frontend/src/bin/poc_frontend.rs`

## 1. FRG-12: HM 基盤仕様の写経と整理

`docs/spec/1-2-types-Inference.md` と `compiler/ocaml/src/type_inference.ml` を軸に、OCaml 側で動作している Algorithm W 系の型・スキーム・制約・型環境の構成を Rust に落とし込むための差分を整理する。

### FRG-12

以下の一覧は Day1 において模式的に写経すべきモジュール群と、現行 `TypecheckDriver` (PoC) に足りない要素を示し、`docs/plans/rust-migration/p1-rust-frontend-gap-report.md#FRG-12` の Day1 ログと並行して検証・双方向比較を進める。

| 項目 | 仕様 / OCaml 実装 | Rust 現状 | FRG-12 で補う差分と移植方針 |
| --- | --- | --- | --- |
| 型表現とスキーム | `docs/spec/1-2 §A` の `ty`/`type_scheme`/`constrained_scheme` を `compiler/ocaml/src/types.ml` で表現。`constrained_scheme` は量化変数・トレイト制約・型本体を保持し、`typeck/type_inference.ml:626-685` の `generalize`/`instantiate` で操作。 | `compiler/rust/frontend/src/typeck/driver.rs:458-517` の `SimpleType` は `Int`/`Bool`/`Unknown` だけで効果行やトレイト制約、スキームが欠けている。 typed AST も `SimpleType` のラベルを返すのみ。 | `typeck/types.rs` を新規に設け、`Type`（`Var`, `Builtin`, `App`, `Arrow` 等）、`TypeKind`、`TypeVariable`、`CapabilityContext` を定義。`serde::Serialize`/`Display` を実装し、OCaml `Type_expr` と comparable な JSON を生成する。量化変数の生成（`TypeVarGen`）やトレイト制約の記録もここに置く。 |
| 型環境 (`Type_env`) | `compiler/ocaml/src/type_env.ml:32-180` に `env` 型、`empty`/`extend`/`lookup`/`enter_scope`/`exit_scope`、`initial_env`（`Some`, `None`, `Never` などの組み込み束縛）が定義され、`lookup` は親スコープまで再帰、`extend` はシャドーイングを許す。 | Rust 版では `TypecheckDriver` 内部で `env: &HashMap<String, SimpleType>` を使用 (`driver.rs:328-344`)、スコープの概念や `Scheme` を保持する仕組みがなく、`insert`/`lookup` も単純な `HashMap::get` しかない。 | `typeck/env` に `TypeEnv` 構造体を追加し、`bindings: IndexMap<String, Binding>` + `parent` を持ち、OCaml 版と同様に `insert`（`extend`）はシャドーイング。`lookup` は親方向に再帰し、`enter_scope`/`exit_scope` を提供。初期環境で OCaml と同じ型スコープを再現し、`StageContext` / `runtime_capabilities` を注入する。 |
| 制約生成・一般化・インスタンス化 | `docs/spec/1-2 §C` / `compiler/ocaml/src/type_inference.ml:626-716` で `generalize` が自由変数収集・量化、`instantiate` が新鮮な型変数への置換を行い、`infer_expr` 系で `constraint` を生成。返却値に `typed_expr`/`ty`/`substitution`/`constraints` が含まれる。 | 現行 `TypecheckDriver` は `infer_expr` が `SimpleType` を返すのみ。制約（`Constraint`）、辞書（`dict`）、`Substitution` を扱う構造が存在せず、`generalize`/`instantiate` の概念もない。 | `typeck/scheme.rs` で `Scheme`（`quantifiers`, `constraints`）と `instantiate`/`generalize` を実装し、`constraints` を `IndexMap<Name, Type>` で保持。`typeck/constraint.rs` に `Constraint`（`Equal`, `HasCapability`, `ImplBound`）と `Substitution` を追加して `ConstraintSolver` に渡すことで `infer_expr` から制約を返せるようにする。 |
| 制約ソルバと impl レジストリ | `compiler/ocaml/src/constraint.ml` は `Constraint`/`substitution`/`apply_subst`/`compose_subst` などを定義し、`Constraint_solver` で `solve` を実行。`type_inference.ml:44-56` に global impl registry（`Impl_registry.impl_registry ref`）を置き、`ConstraintSolver` 内で `dict` を解決する。 | Rust には `Constraint` の実装も `Substitution` もなく、`impl` 情報も保持していない。型検査でどの `impl` を使ったかの記録も `TypecheckReport` にない。 | `typeck/constraint.rs` 内で `Substitution::apply_unwrap`/`merge` と `ConstraintSolver::solve` を定義し、`crate::frontend::impl_registry` モジュールを用意。`TypeEnv` は `ConstraintSolver` に `impl_registry` への参照を渡し、`Dict` 構造体で使用された `impl` を追跡して dual-write へ出力。 |
| 型推論ドライバと Dual-write | `type_inference.ml` は `infer_module`/`infer_expr`/`infer_pattern` で `InferContext` を使い、`TypecheckReport`（型付き AST、制約、辞書など）と `TypeckArtifacts` を生成し、`docs/spec/1-3-effects-safety.md` の能力チェックを `Constraint` として挿入する。 | Rust の `TypecheckDriver` は `TypecheckReport` に関数一覧・指標・簡易な `TypedModule` を入れるのみで、`constraints`/`used_impls`/`effects` 情報がなく、`StageContext` も伝播していない。 | `TypecheckDriver` を再構成し、`InferContext` で `TypeEnv` と `Substitution` を共有して `Constraint` を蓄積。 `TypecheckReport` を拡張して `constraints`/`typed_module`/`used_impls` を dual-write し、`StageContext`/`runtime_capabilities` を `TypeEnv` に注入して `resolver` により `Capability` を `Constraint` として追加できるようにする。 `reports/dual-write/front-end/w3-type-inference` 形式で制約を JSON 化し、`FRG-13` の Capability Registry 整合へつなぐ。 |

本表を参照して `docs/plans/rust-migration/p1-front-end-checklists.csv` の “Type inference” 項目を `FRG-12` にリンクし、ステータスや検証手順を `docs/plans/rust-migration/p1-spec-compliance-gap.md#FRG-12` で補足すること。

## 1. 構文解析（lexer / parser_driver）

| ID | ギャップ | 根拠仕様 | 現状 | 対応案 |
| --- | --- | --- | --- | --- |
| SCG-01 | Unicode ベースの `IDENT`/`UPPER_IDENT` 分類、`RunConfig.extensions["lex"]` プロファイル切替が未実装 | 1-1 §A.3, 2-3 §D | `lexer/mod.rs` は ASCII 正規表現のみ、`UPPER_IDENT` なし、`parser_expectation` へのプロファイル連動なし | `unicode-ident`/`rustc_lexer` などで XID 判定を導入し、`LexProfile` を `ParserDriver` へ渡す |
| SCG-02 | 予約語・演算子・リテラル種別が最小限（`var`, `match`, `type`, `Option`, 数値基数・複数行文字列などが欠落） | 1-1 §A.3〜A.4, 2-3 §E〜F | `TokenKind`/`RawToken` が 20 程度で固定。`0x`/`0b`、`r#""#` 等を解析不能 | 仕様一覧を写経し `TokenKind` を増補。`parser_expectation` 同等の `ExpectedToken` ラベルを整備 |
| SCG-03 | `Parser<T> = fn(&mut State)->Reply<T>` / `RunConfig` / `ParseResult` が未整備 | 2-1 §A〜C, 2-6 §A〜D | `ParserDriver::parse` は `ParsedModule` を返す PoC。`State`/`Reply`/`RunConfig` の概念が無い | `crate::frontend` 直下に `state.rs` を新設し、OCaml `parser_driver.ml` と同じ API に揃える |
| SCG-04 | Packrat/左再帰/`cut`/`attempt` 等のコンビネータ移植なし | 2-2 §A〜C | `parser/mod.rs` は `chumsky` で単純な構文のみ。`cut_here` や `recover` はダミー | `Core_parse` のコアコンビネータを Rust で再実装し、Menhir 相当のテーブル生成方法を明示 |

### 1.1 FRG-06 トークン網羅性リファレンス

| 区分 | 仕様上の要素（出典: 1-1 §A.3〜A.4, `compiler/ocaml/src/token.ml`） | Rust `TokenKind` (2028-02) | 差分メモ |
| --- | --- | --- | --- |
| キーワード（38 種） | `module,use,as,pub,self,super,let,var,fn,type,alias,new,trait,impl,extern,effect,operation,handler,conductor,channels,execution,monitoring,if,then,else,match,with,for,in,while,loop,return,defer,unsafe,perform,do,handle,where` + 真偽語 `true,false` | `KeywordFn/KeywordLet/KeywordElse/KeywordIf/KeywordThen/KeywordTrue/KeywordFalse/KeywordModule/KeywordEffect/KeywordPerform` の 10 個のみ | 28 個不足。Rust 版では `var/trait/handler/...` を識別できず、AST/診断で `identifier` 扱いになる |
| 将来予約語 | `break, continue` | 無し | 予約語扱いされないため `continue {` などが `IDENT` として解析される |
| 識別子 | `IDENT`, `UPPER_IDENT`（先頭大文字・Unicode 可）、`RunConfig.extensions["lex"].identifier_profile` で `unicode`/`ascii-compat` 切替 | `Identifier` の 1 種類のみ。`UPPER_IDENT` と ASCII モード無し | `lexer` 側で `unicode-ident` による XID 判定と ASCII 限定プロファイルを実装する必要がある |
| 演算子 / 区切り （26 種） | `|>, ~>, ., ,, ;, :, @, |, =, :=, ->, =>, (, ), [, ], {, }, +, -, *, /, %, ^, ==, !=, <, <=, >, >=, &&, ||, !, ?, .., _` | `Arrow`, `Assign`, `Comma`, `Colon`, `Semi`, `Plus`, `Paren/Brace/Bracket` 程度 | 複数文字演算子（`~>`, `:=`, `=>`, `..` 等）と論理演算子が欠落。`_` も `Identifier` になる |
| リテラル | `INT(base=dec/bin/oct/hex)`, `FLOAT`, `CHAR`, `STRING(通常/生/複数行)` | `IntLiteral`, `FloatLiteral`, `StringLiteral`（エスケープ最小限） | 基数や `_` 区切り、`r#"..."#`/`"""`/`'a'` が未対応。`Token` 側に `Ast.int_base`/`string_kind` 相当の情報が無い |

> 備考: 上表は `FRG-06` の実装スコープを Rust/OCaml 双方で共有するための参照であり、実装完了時には `TokenKind` の列挙名と `lexer::RawToken` の網羅性チェックをこのリストと突き合わせて確認する。`p1-rust-frontend-gap-report.md` では本表を参照して進捗を更新する。

## 2. AST / IR モデル

| ID | ギャップ | 根拠 | 現状 | 対応案 |
| --- | --- | --- | --- | --- |
| SCG-05 | `expr_kind/pattern/decl` の全列挙が未移植 | 1-1-ast-and-ir-alignment.md §1.1.3 | `parser/ast.rs` は `Expr::{Int,Bool,String,Identifier,Call,Binary,IfElse,Perform}` のみ、`Decl`/`Pattern` 型が存在しない | `Ast` モジュール（OCaml `ast.ml`）をベースに `ExprKind`, `PatternKind`, `DeclKind`, `ModuleItem` を追加し JSON 表現を揃える |
| SCG-06 | Typed AST / 型情報 (`typed_expr`, `typed_pattern`, `scheme`, `dict_ref`) が全く無い | 1-1-ast-and-ir-alignment.md §1.1.4 | `typeck` は統計専用で `TypedExpr` を生成しない | `TypedAst` 用 crate (`crate::semantics::typed`) を分離し、dual-write 用 JSON フォーマットを OCaml 版と一致させる |
| SCG-07 | Packrat/Streaming 状態 (`Core_parse_streaming`) の構造が未対応 | 1-1-ast-and-ir-alignment.md 表 | `streaming/mod.rs` には Packrat cache/SpanTrace の雛形があるが Parser と接続されていない | `ParserDriver` の `State` に `StreamingState` を組み込み、`packrat_snapshot`/`span_trace` を `ParseResult` へ出力 |

## 3. 型推論・制約解決

| ID | ギャップ | 根拠仕様 | 現状 | 対応案 |
| --- | --- | --- | --- | --- |
| SCG-08 | Hindley–Milner (Algorithm W) / 値制限 / 制約ソルバが未実装 | 1-2 §B〜F, `p1-front-end-checklists.csv` Typed AST 項目 | `typeck/driver.rs` が Algorithm W 風の制約生成・型環境操作・一般化を行い、`ConstraintSolver`・`Scheme`・`TypedExpr` を用いて typed AST／dict_ref を構築し dual-write に供給。 | `typeck/types.rs`/`env.rs`/`constraint.rs`/`driver.rs` を更新し、`TypecheckReport` に typed module・scheme table 及び dict_ref を含め、`cargo check` でビルドが通ることを確認した |
| SCG-09 | 効果行 / 残余効果 / Capability Stage 監査 (`effects.contract.*`) 未実装 | 1-3 §I, EFFECT-002, `w4` cases | `TypecheckViolation::ResidualLeak` は `perform` の文字列抽出のみ。Stage/Capability コンテキストと連動していない | `StageContext` と `RuntimeCapability` を参照し、効果タグと Capability Registry の照合作業を Rust でも実装 |
| SCG-10 | Typed AST / constraints / impl-registry の dual-write エクスポートが OCaml 形式と揃っていない | 1-1-ast-and-ir-alignment.md, `1-3-dual-write-runbook.md` | TypecheckReport が typed_module/constraints/used_impls を収集し、`poc_frontend` は `typeck/typed-ast.rust.json`・`typeck/constraints.rust.json`・`typeck/impl-registry.rust.json` を出力。`constraints` には `function_summaries`/`breakdown`/`ConstraintStats`（unify/ast/token）が含まれている。 | `scripts/poc_dualwrite_compare.sh --mode typeck` で `typeck/*.rust.json` を再生成し、OCaml 差分を確認しながら `p1-front-end-checklists.csv` と `reports/dual-write/front-end/w3-type-inference/README.md` を更新する。 |

## 4. 診断前処理・JSON 整形

| ID | ギャップ | 根拠仕様 | 現状 | 対応案 |
| --- | --- | --- | --- | --- |
| SCG-11 | `Diagnostic` の正式構造（severity/domain/audit/timestamp/expected_summary/context_note）を保持していない | 3-6 §1, 2-5 §A, DIAG-001〜003 | `diagnostic/mod.rs` の `FrontendDiagnostic` は `code/message/span/recoverability/notes/expected_*` のみ | `Diagnostic` 型を再定義し、`AuditEnvelope`, `Severity`, `SpanLabel`, `Hint`, `ExpectedSummary` を OCaml と同構造で保持 |
| SCG-12 | `parser_expectation` 相当の期待集合整形 + `ExpectedTokenCollector` の優先順位が一部ハードコード | 2-5 §A, `parser_expectation.ml` | `token_kind_expectations` が主要トークンのみ（識別子クラス名が日本語混在）、`Not/Class` ラベル未対応 | OCaml の `dedup_and_sort`/`humanize` を Rust へ移植し、`ExpectedToken` の `rule/class/not` を網羅 |
| SCG-13 | CLI JSON 出力で `severity="error"` 固定／`domain="parser"` 固定、`audit_id` も疑似値 | 3-6 §1, `diagnostic-format-regression.md` | `poc_frontend.rs` `build_parser_diagnostics` は `severity` を一律 `"error"` に設定し、`audit_id` を `cli/<timestamp>#0` で偽装 | `FrontendDiagnostic` に severity/domain/notes を持たせ `DiagnosticFormatter` 相当のロジックを Rust へ導入。`collect-iterator-audit-metrics.py` が参照する `effects.stage.*` 等も生成 |

## 5. ストリーミング / RunConfig 拡張

| ID | ギャップ | 根拠仕様 | 現状 | 対応案 |
| --- | --- | --- | --- | --- |
| SCG-14 | `run_stream` / `resume` API が存在せず `StreamOutcome::{Completed,Pending}` を返せない | 2-7 §A〜C | `ParserDriver` は常にバッチ。`StreamingState` はメトリクス採取のみで `Pending` 制御なし | `StreamingRunner` を新設し、Feeder/DemandHint/Continuation を管理。CLI `--streaming` が `run_stream` を呼ぶよう統合 |
| SCG-15 | `RunConfig`/`StreamingConfig`/`FlowController` の仕様字段と CLI 実装が同期していない | 2-6 §B, 2-7 §A, `p1-front-end-checklists.csv` | Rust CLI は `RunConfig` を `RunSettings` から構築し、`build_runconfig_*`/`build_audit_metadata` を `ParseResult.run_config` で生成した `parser_run_config` JSON（`diagnostics`, `parse_debug`, `parse/parser_run_config.rust.json`）で出力している | FRG-20 で `RunConfig` と dual-write `parser_run_config` の共通化を完了し、`docs/plans/rust-migration/p1-rust-frontend-gap-report.md#FRG-20` に達成記録を残した |

## 6. 具体的な計画

## 7. 今後の共有ポイント

- ギャップ修正後は `p1-front-end-checklists.csv` の該当行に完了日と根拠（dual-write run ID）を記載する。
- 仕様追加・用語変更が発生した場合は `docs/spec/0-2-glossary.md` と `README.md` を忘れずに同期させる。
- 大規模ファイル移動やリネームがある場合は `docs-migrations.log` に記録する（再編計画の運用ルール）。

### FRG-12 実装メモ（Day 2-3）

- ✅ `compiler/rust/frontend/src/typeck/env.rs` に `TypeEnv`/`Binding` を追加し、`IndexMap` を使ったスコープ付き環境を再帰的な `lookup`/`insert` で実装。`Scheme` を保持しつつ親環境を再参照できるようになった。
- ✅ `compiler/rust/frontend/src/typeck/types.rs`/`scheme.rs`/`constraint.rs` を追加し、`Type`/`Scheme`/`Constraint`/`Substitution` の骨格と `ConstraintSolver` を整備。`Constraint::equal`/`has_capability`/`impl_bound` と `Substitution::apply_unwrap`/`merge` に serde/Display を持たせ、dual-write に向けた型・制約データを出力できる。
- ✅ `compiler/rust/frontend/src/typeck/driver.rs` を `Type`/`Scheme`/`Constraint` ベースの API に再設計し、`infer_expr` が `Vec<Constraint>` を蓄積して `ConstraintSolver::solve` を呼び出しつつ、`TypedExpr` に `Type` ラベルを含めて dual-write 比較の準備を整えた。

### Day 6: constraints/dual-write 検証

- `TypecheckReport` に `constraints`/`used_impls` を追加し、`poc_frontend` の `TypeckArtifacts::constraints` で `reports/dual-write/front-end/w3-type-inference/<case>/constraints.rust.json` に `Constraint` の JSON を直接出力できるようにした。
- `compiler/rust/frontend/tests/typeck_hindley_milner.rs` を追加して `Constraint::Equal` 集約と `ConditionLiteralBool` 診断のトリガーを検証し、`cargo test -p reml_frontend hindley_milner` を通じて dual-write に必要なデータ生成の仕組みを確認した。
- `docs/spec/1-3-effects-safety.md` を FRG-12 検証スコープへリンクし、`StageContext`/`Capability` による制約追加が必要であることを明示的に記録している。
