# OCaml パーサ資産棚卸し（Lexer/Parser スケルトン移植向け）

本ドキュメントは `docs/plans/rust-migration/1-0-front-end-transition.md` の W1 タスク「OCaml 実装の棚卸しと設計ノート整備」に対応して、Rust フロントエンド移植時に参照すべき OCaml 資産を整理する。出典は `compiler/ocaml/docs/parser_design.md`、`compiler/ocaml/src/parser_driver.ml`、`compiler/ocaml/src/parser_expectation.ml` を中心とし、仕様書 `docs/spec/1-1-syntax.md` と突き合わせている。

## 1. 字句トークン一覧

- **予約語（35 件）**  
  `module`, `use`, `as`, `pub`, `self`, `super`, `let`, `var`, `fn`, `type`, `alias`, `new`, `trait`, `impl`, `extern`, `effect`, `operation`, `handler`, `conductor`, `channels`, `execution`, `monitoring`, `if`, `then`, `else`, `match`, `with`, `for`, `in`, `while`, `loop`, `return`, `defer`, `unsafe`, `perform`, `do`, `handle`, `where`, `true`, `false`, `break`, `continue`（`break`/`continue` は将来予約）
- **演算子 / 区切り（35 件）**  
  `|>`, `~>`, `.`, `,`, `;`, `:`, `@`, `|`, `=`, `:=`, `->`, `=>`, `(`, `)`, `[`, `]`, `{`, `}`, `+`, `-`, `*`, `/`, `%`, `^`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`, `!`, `?`, `..`, `_`
- **識別子とリテラル**  
  - 識別子: `IDENT`（小文字開始）、`UPPER_IDENT`（大文字開始）、Unicode XID 準拠  
  - 数値: `INT`（基数: 2/8/10/16）、`FLOAT`  
  - 文字/文字列: `CHAR`、`STRING`（通常/Raw/複数行）  
  - 特殊: `EOF`
- **コメント**: `//` 行コメント、`/* ... */`（ネスト可）

> Rust 実装では `enum Token` に上記集合を 1:1 対応させる。`Token::UNDERSCORE` はパターン用の疑似トークンとして残置し、lexer 段階で `PatWildcard` へ直接変換しないこと。

## 2. AST ノード対応（Phase 1）

- **共通原則**  
  - すべてのノードに `Span { start: u32, end: u32 }` を保持 (`docs/spec/1-1-syntax.md` §A)  
  - 効果構文 PoC（`perform`/`do`/`handle`）は Phase 1 でも AST 上に存在  
  - 属性 (`attribute`) は `#[name(args...)]` 相当を許容し、宣言に多重付与可能
- **式 (`expr_kind`)**  
  `Literal`, `Var`, `ModulePath`, `Call`, `PerformCall`, `Lambda`, `Pipe`, `Binary`, `Unary`, `FieldAccess`, `TupleAccess`, `Index`, `Propagate`, `If`, `Match`, `While`, `For`, `Loop`, `Handle`, `Continue`, `Block`, `Unsafe`, `Return`, `Defer`, `Assign`  
  - `PerformCall` は `effect_ref`（パス・操作名）と `effect_args` を保持  
  - `Assign` は LHS が postfix 表現（`FieldAccess` 等）であることを前提に `expr` × `expr` で表現される
- **パターン (`pattern_kind`)**  
  `PatLiteral`, `PatVar`, `PatWildcard`, `PatTuple`, `PatRecord`（残余指定有無）, `PatConstructor`, `PatGuard`
- **宣言 (`decl_kind`)**  
  `LetDecl`, `VarDecl`, `FnDecl`, `TypeDecl`（Alias/Sum/Newtype）、`TraitDecl`, `ImplDecl`, `ExternDecl`, `EffectDecl`, `HandlerDecl`, `ConductorDecl`  
  - `FnDecl` は `generic_params`, `where_clause`, `effect_annot` を保持  
  - 効果関連（`EffectDecl`/`HandlerDecl`）は `docs/spec/1-3-effects-safety.md` の PoC 要件と対応
- **型注釈 (`type_kind`)**  
  `TyIdent`, `TyApp`, `TyTuple`, `TyRecord`, `TyFn`  
  - Rust 実装では `TyId` 生成を P1 後半（Typed AST）で統合予定
- **文 (`stmt`) / トップレベル**  
  `DeclStmt`, `ExprStmt`, `AssignStmt`, `DeferStmt`。コンパイル単位は `decl list` を保持し、モジュールヘッダの `module` 宣言をオプション扱い。

> AST ノードの列挙子は `compiler/ocaml/src/ast.ml` の順序を維持する。Rust 版で `serde` を用いる際はフィールド名と順序を固定し、`1-1-ast-and-ir-alignment.md` のチェックリストに従って diff を取る。

## 3. Parser Driver / Expectation の責務分割

- **`parser_driver.ml`（Core 連携 + 状態管理）**  
  - `Run_config` を介して `require_eof` や packrat 有効化を制御。`legacy_run_config` は OCaml CLI の互換モード。  
  - `Core_parse.rule` を呼び出すラッパで、Menhir のチェックポイントを `Core_reply.{Ok,Err}` へ変換。  
  - `Core_parse_streaming` と連携し、`span_trace`, `packrat_stats`, `packrat_cache` を収集して `parse_result` に格納。  
  - Lexer 例外 (`Lexer_error`) と構文エラー（`process_parser_error`/`process_rejected_error`）を `Diagnostic.Builder` で JSON 診断へ変換し、`extensions.recover` に `expected_tokens` / `message` / `context` を付与。  
  - `Run_config.packrat = Auto/On` の場合に左再帰警告を出し、dual-write で `consumed`/`committed` を完全一致させる責務を負う。
- **`parser_expectation.ml`（期待集合と packrat 補助）**  
  - Menhir 由来のターミナル/非終端を `Diagnostic.expectation` へマッピング。優先順位 (`keyword` → `token` → `class` → `rule`) に従って整列し、ヒューマンリーダブルなメッセージを生成。  
  - `collection` 型で `sample_tokens`（期待トークン例）と `expectations`（整列済み）、`summary`（`ExpectationSummary`）を保持し、`parser_driver` から診断拡張に流用。  
  - `Packrat` サブモジュールでキャッシュ生成・トリム・統計取得 (`metrics.entries`, `metrics.approx_bytes`) を提供し、`collect` で checkpoint 走査時のヒット状況を返す。  
  - `ExpectationSummary` の `humanized` 既定値は「ここで`<token>`が必要です」形式。Rust 側でも同じキー／文章を採用しない限り diff が発生するため、ロケール文字列の再利用が必須。

> Rust 実装では `parser::driver` が `core::streaming` と `diagnostic` モジュールをまたいで責務分担し、`parser_expectation` 相当のモジュールで JSON 拡張に必要なヒューマナイズ処理と Packrat メトリクスを提供する。

## 4. ギャップとフォローアップ

- Menhir 固有の状態遷移（`InputNeeded`, `Shifting`, `HandlingError`, `Rejected`）を Rust で再現する際、`parser_driver` の `loop` 実装を LALRPOP/自前 LR で模倣できるか要検討。`docs/plans/rust-migration/1-3-dual-write-runbook.md` の CLI モード仕様と照合する。  
- Packrat キャッシュの prune ポリシーは `Core_parse_streaming` 実装依存であり、Rust の `IndexMap`/`hashbrown` でのメモリ挙動を要測定。P1 W3 で統計差分を測った上で調整する。  
- `parser_expectation` の `Keyword`/`Token`/`Class` ラベルは日本語ヒューマンイズ済み。Rust 実装でも ICU 由来の正規化設定を適用するか `docs/spec/3-3-core-text-unicode.md` を参照し、文字列処理の差分を監視する。

## 5. Typed AST / IR インベントリ（W2 追加）

`compiler/ocaml/src/typed_ast.ml` から Typed AST と制約付き IR を洗い出し、Rust 側で 1:1 再現すべきフィールドを整理した。W2 の AST/IR 対応タスクでは以下の要素を差分基準に利用する。

### 5.1 `typed_expr` / `typed_pattern` / `typed_stmt`

- `typed_expr` レコードは `texpr_kind`, `texpr_ty`, `texpr_span`, `texpr_dict_refs`（`dict_ref list`）で構成される。辞書参照は型クラス解決済みの順序付きリストで、Rust 版でも `SmallVec<DictRefId, 4>` など順序保持構造が必要。  
- `typed_expr_kind` バリアント（23 種類）: `TLiteral`, `TVar`, `TModulePath`, `TCall`, `TEffectPerform`, `TLambda`, `TPipe`, `TBinary`, `TUnary`, `TFieldAccess`, `TTupleAccess`, `TIndex`, `TPropagate`, `TIf`, `TMatch`, `TWhile`, `TFor`, `TLoop`, `THandle`, `TContinue`, `TBlock`, `TUnsafe`, `TReturn`, `TDefer`, `TAssign`（`TFor` と `TEffectPerform` は追加メタ情報を保持）。  
- `typed_pattern` は `tpat_kind`, `tpat_ty`, `tpat_bindings`, `tpat_span` を保持。`tpat_bindings` は `(string * ty) list` で、Rust 側では `Vec<(SymbolId, TyId)>` へ正規化する。  
- `typed_pattern_kind` バリアントは `TPatLiteral`, `TPatVar`, `TPatWildcard`, `TPatTuple`, `TPatRecord`, `TPatConstructor`, `TPatGuard`。  
- `typed_stmt` バリアントは `TDeclStmt`, `TExprStmt`, `TAssignStmt`, `TDeferStmt`。W2 では `parser_driver` の `stmt list` ダンプに合わせ `typed_stmt` も JSON 直列化対象へ含めることを決定。

### 5.2 `typed_decl` と関数系ノード

| OCaml 型 | 主フィールド | Rust 移植メモ |
| --- | --- | --- |
| `typed_decl` | `tdecl_attrs`, `tdecl_vis`, `tdecl_kind`, `tdecl_scheme`, `tdecl_span`, `tdecl_dict_refs` | `tdecl_scheme` は `Constraint_solver.constrained_scheme`。Rust 版では `Scheme { ty: TyId, constraints: Vec<ConstraintId> }` へ落とし込み、辞書参照も含めて dual-write 比較する。 |
| `typed_decl_kind` | `TLetDecl`, `TVarDecl`, `TFnDecl`, `TTypeDecl`, `TTraitDecl`, `TImplDecl`, `TExternDecl`, `TEffectDecl`, `THandlerDecl`, `TConductorDecl` | 型推論対象でない宣言（型/trait/impl 等）は AST をそのまま転送。Rust 側でも「未解析」バリアントを維持し、JSON で `kind` を差分可能にする。 |
| `typed_fn_decl` | `tfn_name`, `tfn_generic_params`, `tfn_params`, `tfn_ret_type`, `tfn_effect_row`, `tfn_where_clause`, `tfn_effect_profile`, `tfn_body` | `tfn_effect_row` は `Effect_row`（`Types.effect_row`）を保持。Rust 版では `EffectRowId` をインターンし、`collect-iterator-audit-metrics.py --section effects` で比較するためのシリアライズを準備する。 |
| `typed_handler_decl` / `typed_handler_entry` | 効果ハンドラの `Operation/Return` バリアント | JSON では `entries` 配列内で `variant` を明示。 |

### 5.3 辞書参照と制約付随情報

- `texpr_dict_refs` / `tdecl_dict_refs` の要素型 `Constraint_solver.dict_ref` は `resolver_slot`, `trait_name`, `evidence_ty` を保持。Rust 版では `DictRefId`（整数）に正規化しつつ JSON へフラット化する。  
- `TFor` は `dict_ref` と `iterator_dict_info option` を保持し、`collect-iterator-audit-metrics.py --section parser` で `parser.stream.iterator_dict.*` を算出する際のキーとなる。Rust 側でも `IteratorDictInfo { trait: Ident, adapter: ModulePath, location: Span }` を保持する。  
- `typed_compilation_unit`（`tcu_*` フィールド）は `Module_env.use_binding list` を含む。Rust 実装では `use` 展開済みのバインディングも diff 対象にするため、`tcu_use_bindings` を `Vec<UseBinding>` で保持する草案とした。  
- 制約 (`constraint_ list`) は AST と同型を再利用するため、Rust 版でも `Constraint { trait: Ident, args: Vec<TyId>, span }` を提供し、`p1-front-end-checklists.csv` の「Scheme/Constraint/Impl Registry」の受入条件へ直結させる。

## 6. Core_parse / Streaming インベントリ（W2 追加）

`compiler/ocaml/src/core_parse_streaming.ml` と `parser_driver.ml` からストリーミング状態および `ParseResult` 付随情報を抽出し、Rust 実装で保持すべきテレメトリ項目を固定した。

### 6.1 セッション構造と `ParseResult`

- `Core_parse_streaming.session` は `config`, `diag_state`, `core_state`, `packrat_cache` を保持。Rust 版では `Session { run_config, diag, core, packrat: Option<PackratCache> }` を再現し、`packrat_cache` の有無で `collect-iterator` への出力可否を切り替える。  
- `parser_driver.parse_result` が保持するフィールドを以下に整理（JSON フィールド名も W2 で固定）。  

| フィールド | 内容 | Rust 側 TODO |
| --- | --- | --- |
| `value` | `Ast.compilation_unit option` | AST diff で `null` 可。 |
| `span` | `Diagnostic.span option` | `serde` で `{ "start": u32, "end": u32 }` に正規化。 |
| `diagnostics` | `Diagnostic.t list` | `1-2-diagnostic-compatibility.md` と共有。 |
| `recovered` | `bool` | Recover 拡張判定と同期。 |
| `legacy_error` | `parse_error option` (`expected`, `committed`, `far_consumed`) | Rust CLI でも `--legacy` 時のみ設定。 |
| `consumed` / `committed` | Packrat/Reply 状態から転記 | `Core_parse.Reply` の `consumed/committed` を `parser.stream.outcome_consistency` に流す。 |
| `farthest_error_offset` | `int option` | `Parser_diag_state.farthest_offset` を `u32` へ丸め。 |
| `span_trace` | `(string option * Diagnostic.span) list option` | Rust 版 `ParserDiagState` で `Vec<(Option<SmolStr>, Span)>` を用意。 |
| `packrat_stats` | `(queries, hits)` | `StreamingState::packrat_stats()` を通じ JSON 配列 `[queries, hits]` に固定。 |
| `packrat_cache` | `Parser_expectation.Packrat.t option` | Rust 版ではテスト限定で Base64 直列化し、CI では `None` を返す方針。 |

### 6.2 Packrat / SpanTrace / Metrics

- `Core_parse.State` は `packrat_queries`/`packrat_hits` ミュータブルカウンタを保持。`Core_parse_streaming.packrat_counters` で `(queries, hits)` を返し、`collect-iterator-audit-metrics.py --section parser` では `parser.stream.packrat_queries` / `parser.stream.packrat_hits` として記録している。Rust 側でも `StreamingState` に同名メソッドを実装する。  
- `Parser_diag_state.span_trace_pairs` は `Vec<(string option, Diagnostic.span)>` を返す。`span_trace` が `None` のケースでは diff をスキップする規約を W2 で明文化し、Rust 版 `ParserDiagState` に `record_span_trace` フックを実装する。  
- `Parser_expectation.collect` の返り値は `collection.summary` と `status`（`Hit/Miss/Bypassed`）。`record_packrat_status` 経由で `Core_parse.State.record_packrat_access` に伝播する挙動を Rust 版 `ExpectationSummary` へ写経する。  
- `expectation_summary_for_checkpoint` はレスキュー済みスナップショットを参照し、`recover.expected_tokens` の内容と `packrat` ヒット統計を同一関数で収集する。Rust 実装でも checkpoint 毎に再収集するため、`ParserExpectation` 相当の API に `collect_with_metrics` を用意する。

### 6.3 `collect-iterator` との接続ポイント

- `tooling/ci/collect-iterator-audit-metrics.py` で参照される `parser.stream.*` 系キーのうち、フロントエンド直下で供給する必要があるものを以下に抽出。  
  - `parser.stream.packrat_hits`, `parser.stream.packrat_queries`, `parser.stream.span_trace_pairs`（`span_trace` 要素数）。  
  - `parser.stream.outcome_consistency`（`consumed`/`committed` フラグの一致率）。  
  - `parser.stream_extension_field_coverage`（`extensions.recover` に含まれるフィールド数）。  
- W2 の棚卸しでは、上記キーを `reports/dual-write/front-end/w2-ast-alignment/metrics/parser-stream-baseline.json`（予定）へ出力するための JSON スキーマを `parser_driver` と `collect-iterator` の双方で確認済み。Rust 版では同スキーマを `serde` モジュールで公開し、dual-write ハーネスから直接呼び出せるようにする。


---

作成日: 2025-03-12 / 作成者: Rust 移植チーム支援エージェント  
参照元: `compiler/ocaml/docs/parser_design.md`, `compiler/ocaml/src/parser_driver.ml`, `compiler/ocaml/src/parser_expectation.ml`, `compiler/ocaml/src/token.ml`, `compiler/ocaml/src/ast.ml`
