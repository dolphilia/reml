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

---

作成日: 2025-03-12 / 作成者: Rust 移植チーム支援エージェント  
参照元: `compiler/ocaml/docs/parser_design.md`, `compiler/ocaml/src/parser_driver.ml`, `compiler/ocaml/src/parser_expectation.ml`, `compiler/ocaml/src/token.ml`, `compiler/ocaml/src/ast.ml`
