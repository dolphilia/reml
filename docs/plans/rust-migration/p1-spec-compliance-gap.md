# P1 フロントエンド仕様未達リスト（2028-02 現状）

Phase P1（フロントエンド移植）の達成条件を Reml 仕様に照らして再確認したところ、現行 Rust 実装では未実装／不完全な領域が複数残っている。本書では `docs/plans/rust-migration/1-0-front-end-transition.md` が規定する 4 つの対象範囲（構文解析・AST/IR・型推論・診断前処理）と `docs/plans/rust-migration/p1-front-end-checklists.csv` の達成要件に基づき、優先度順にギャップと対応方針を整理する。

各表には ID（`SCG-XX`）を付与し、未達項目の参照とチケット連携を容易にした。

## 0. 参照資料

- 仕様: `docs/spec/1-1-syntax.md`, `1-2-types-Inference.md`, `1-3-effects-safety.md`, `2-1-parser-type.md`, `2-5-error.md`, `2-7-core-parse-streaming.md`, `3-6-core-diagnostics-audit.md`
- 計画: `docs/plans/rust-migration/1-0-front-end-transition.md`, `1-1-ast-and-ir-alignment.md`, `1-2-diagnostic-compatibility.md`, `1-3-dual-write-runbook.md`
- 実装: `compiler/rust/frontend/src/{lexer,parser,diagnostic,streaming,typeck}` および CLI `compiler/rust/frontend/src/bin/poc_frontend.rs`

## 1. 構文解析（lexer / parser_driver）

| ID | ギャップ | 根拠仕様 | 現状 | 対応案 |
| --- | --- | --- | --- | --- |
| SCG-01 | Unicode ベースの `IDENT`/`UPPER_IDENT` 分類、`RunConfig.extensions["lex"]` プロファイル切替が未実装 | 1-1 §A.3, 2-3 §D | `lexer/mod.rs` は ASCII 正規表現のみ、`UPPER_IDENT` なし、`parser_expectation` へのプロファイル連動なし | `unicode-ident`/`rustc_lexer` などで XID 判定を導入し、`LexProfile` を `ParserDriver` へ渡す |
| SCG-02 | 予約語・演算子・リテラル種別が最小限（`var`, `match`, `type`, `Option`, 数値基数・複数行文字列などが欠落） | 1-1 §A.3〜A.4, 2-3 §E〜F | `TokenKind`/`RawToken` が 20 程度で固定。`0x`/`0b`、`r#""#` 等を解析不能 | 仕様一覧を写経し `TokenKind` を増補。`parser_expectation` 同等の `ExpectedToken` ラベルを整備 |
| SCG-03 | `Parser<T> = fn(&mut State)->Reply<T>` / `RunConfig` / `ParseResult` が未整備 | 2-1 §A〜C, 2-6 §A〜D | `ParserDriver::parse` は `ParsedModule` を返す PoC。`State`/`Reply`/`RunConfig` の概念が無い | `crate::frontend` 直下に `state.rs` を新設し、OCaml `parser_driver.ml` と同じ API に揃える |
| SCG-04 | Packrat/左再帰/`cut`/`attempt` 等のコンビネータ移植なし | 2-2 §A〜C | `parser/mod.rs` は `chumsky` で単純な構文のみ。`cut_here` や `recover` はダミー | `Core_parse` のコアコンビネータを Rust で再実装し、Menhir 相当のテーブル生成方法を明示 |

## 2. AST / IR モデル

| ID | ギャップ | 根拠 | 現状 | 対応案 |
| --- | --- | --- | --- | --- |
| SCG-05 | `expr_kind/pattern/decl` の全列挙が未移植 | 1-1-ast-and-ir-alignment.md §1.1.3 | `parser/ast.rs` は `Expr::{Int,Bool,String,Identifier,Call,Binary,IfElse,Perform}` のみ、`Decl`/`Pattern` 型が存在しない | `Ast` モジュール（OCaml `ast.ml`）をベースに `ExprKind`, `PatternKind`, `DeclKind`, `ModuleItem` を追加し JSON 表現を揃える |
| SCG-06 | Typed AST / 型情報 (`typed_expr`, `typed_pattern`, `scheme`, `dict_ref`) が全く無い | 1-1-ast-and-ir-alignment.md §1.1.4 | `typeck` は統計専用で `TypedExpr` を生成しない | `TypedAst` 用 crate (`crate::semantics::typed`) を分離し、dual-write 用 JSON フォーマットを OCaml 版と一致させる |
| SCG-07 | Packrat/Streaming 状態 (`Core_parse_streaming`) の構造が未対応 | 1-1-ast-and-ir-alignment.md 表 | `streaming/mod.rs` には Packrat cache/SpanTrace の雛形があるが Parser と接続されていない | `ParserDriver` の `State` に `StreamingState` を組み込み、`packrat_snapshot`/`span_trace` を `ParseResult` へ出力 |

## 3. 型推論・制約解決

| ID | ギャップ | 根拠仕様 | 現状 | 対応案 |
| --- | --- | --- | --- | --- |
| SCG-08 | Hindley–Milner (Algorithm W) / 値制限 / 制約ソルバが未実装 | 1-2 §B〜F, `p1-front-end-checklists.csv` Typed AST 項目 | `typeck/driver.rs` は `SimpleType` で Int/Bool/Unknown を返すだけ。制約/環境/強制力なし | `Types`, `Scheme`, `Constraint`, `Type_env` を OCaml 実装から写像し、`type_inference.ml` のアルゴリズムを Rust 化 |
| SCG-09 | 効果行 / 残余効果 / Capability Stage 監査 (`effects.contract.*`) 未実装 | 1-3 §I, EFFECT-002, `w4` cases | `TypecheckViolation::ResidualLeak` は `perform` の文字列抽出のみ。Stage/Capability コンテキストと連動していない | `StageContext` と `RuntimeCapability` を参照し、効果タグと Capability Registry の照合作業を Rust でも実装 |
| SCG-10 | Typed AST / constraints / impl-registry の dual-write エクスポートが無い | 1-1-ast-and-ir-alignment.md, `1-3-dual-write-runbook.md` | CLI フラグ `--emit typed-ast/constraints/typeck-debug` は存在するがダミー JSON を出力 | HM 実装と連動させ、OCaml ゴールデンと diff できる JSON を生成 |

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
| SCG-15 | `RunConfig`/`StreamingConfig`/`FlowController` の仕様字段と CLI 実装が同期していない | 2-6 §B, 2-7 §A, `p1-front-end-checklists.csv` | CLI 側 `RunSettings` は独自フィールド (`legacy_result` など) を持つが `RunConfig.extensions` を JSON に落とさない | 共通 `RunConfig` 構造を crate に配置し CLI/テスト/ランナーで共有、`parser_run_config` JSON を出力して dual-write で比較 |

## 6. 具体的な計画

## 7. 今後の共有ポイント

- ギャップ修正後は `p1-front-end-checklists.csv` の該当行に完了日と根拠（dual-write run ID）を記載する。
- 仕様追加・用語変更が発生した場合は `docs/spec/0-2-glossary.md` と `README.md` を忘れずに同期させる。
- 大規模ファイル移動やリネームがある場合は `docs-migrations.log` に記録する（再編計画の運用ルール）。
