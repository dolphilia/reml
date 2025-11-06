# Rust AST / Typed AST データモデル草案（W2）

## 1. 目的と適用範囲

- W2 マイルストーンの「Rust AST/Typed AST データモデル草案の確定」に対応し、OCaml 実装 (`compiler/ocaml/src/{ast,typed_ast}.ml`) と 1:1 で整合する Rust データモデルの雛形を定義する。  
- 本草案は `compiler/rust/frontend/` クレートの `syntax::*`（AST 層）と `semantics::*`（Typed AST・型推論層）に実装する構造体・列挙体・補助型の仕様をまとめ、`1-1-ast-and-ir-alignment.md` や `p1-front-end-checklists.csv` から参照する。  
- Dual-write 比較（`reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/`）で生成する AST / Typed AST JSON のスキーマを Rust 側で固定し、`collect-iterator-audit-metrics.py --section {parser,effects}` に渡すフィールド名・値の整合を保証する。

## 2. モジュール構成（Rust 側）

| モジュール | 役割 | OCaml 参照元 | 主な公開型 |
| --- | --- | --- | --- |
| `crate::syntax::span` | `Span` / `NodeId` / `SpanId` 等の位置情報。`u32` バイトオフセットを採用 | `Ast.span` | `Span`, `SpanId` |
| `crate::syntax::ident` | 識別子・モジュールパス・Stage 要件 | `Ast.ident`, `Ast.module_path` | `Ident`, `ModulePath`, `StageRequirement` |
| `crate::syntax::ast` | Expr/Pattern/Decl など AST ノード本体 | `Ast.expr`, `Ast.decl` | `Expr`, `Pattern`, `Decl` |
| `crate::syntax::effect` | 効果参照・`perform` シュガー情報 | `Ast.effect_reference` | `EffectRef`, `EffectCall` |
| `crate::semantics::types` | `Ty`, `TyKind`, `EffectRow`、インターナ ID | `Types`, `Effect_profile` | `Ty`, `TyKind`, `EffectRow`, `TyId` |
| `crate::semantics::typed` | TypedExpr/TypedPattern/TypedDecl、`Scheme`, `DictRef` | `Typed_ast` | `TypedExpr`, `TypedPattern`, `TypedDecl`, `Scheme` |
| `crate::semantics::constraints` | `Constraint`, `ImplRegistry`、`dict_ref` 互換 | `Constraint_solver` | `Constraint`, `DictRef`, `ImplEntry` |

> **実装メモ**: `syntax` と `semantics` は `lib.rs` から再輸出し、Dual-write CLI (`bin/poc_frontend.rs`) で `serde::Serialize` を実装済みのデータをそのまま JSON 化する。

## 3. 共通基本型

| OCaml 型 | Rust 型案 | メモ |
| --- | --- | --- |
| `span` | `#[derive(Copy, Clone, Serialize)] struct Span { start: u32, end: u32 }` | 終端は排他的。`Span::EMPTY` を `const` 化し、`u32::MAX` を未初期化検出に使わない。 |
| `ident` | `struct Ident { symbol: SmolStr, span: Span }` | `SmolStr` でインターンし、Dual-write のため `symbol` を UTF-8 のまま保持。 |
| `stage_requirement_annot` | `enum StageRequirement { Exact(IdentId), AtLeast(IdentId) }` | `IdentId` は `NonZeroU32`。JSON 変換時は `{"kind":"exact","ident":"StageName"}` 形式。 |
| `module_path` | `enum ModulePath { Root(SmolStrVec), Relative(RelativeHead, SmallVec<[IdentId; 4]>) }` | `RelativeHead` は `self/super` を数値化。 |
| `effect_reference` | `struct EffectRef { path: Option<ModulePath>, effect: IdentId, operation: IdentId, span: Span }` | JSON では OCaml と同じ `effect_path/effect_name/effect_operation` キーを維持。 |

## 4. AST スキーマ

### 4.1 ルートノード

```text
AstModule {
  header: ModuleHeader,
  decls: Vec<Decl>,
  eof_span: Span,
  packrat_stats: Option<PackratStats>,   // streaming 連携
  span_trace: Vec<SpanTraceEntry>
}
```

- `ModuleHeader` や `UseDecl` は OCaml 版と同じレイアウト。  
- `PackratStats/span_trace` は `1-1-ast-and-ir-alignment.md#1-1-5` に準拠し、Dual-write JSON の `parse_result.*` に同梱する。

### 4.2 Expr / Pattern / Decl

| カテゴリ | OCaml (`Ast`) | Rust (`crate::syntax::ast`) | JSON メモ |
| --- | --- | --- | --- |
| Expr | `expr = { expr_kind; expr_span }` | `struct Expr { id: NodeId, kind: ExprKind, span: Span, stage: Option<StageRequirement>, effect: Option<EffectMeta> }` | `stage` と `effect` は Phase 1 では `null`。 |
| ExprKind | `Literal`, `Var`, `ModulePath`, … | `enum ExprKind { Literal(Literal), Var(IdentId), ModulePath { module_path: ModulePathId, ident: IdentId }, Call { callee: ExprId, args: Vec<Arg> }, … }` | JSON `kind` は OCaml と同じスネークケース。 |
| Pattern | `pattern = { pat_kind; pat_span }` | `struct Pattern { id: NodeId, kind: PatternKind, span: Span }` | `PatternKind` の列挙子順を OCaml ファイル順に合わせる。 |
| Decl | `decl = { decl_attrs; decl_vis; decl_kind; decl_span }` | `struct Decl { id: NodeId, attrs: Vec<Attribute>, vis: Visibility, kind: DeclKind, span: Span }` | `Attribute` に `args: Vec<ExprId>` を保持し JSON では `attrs`。 |
| EffectCall | `effect_call` | `struct EffectCall { ref_: EffectRef, args: Vec<Arg>, sugar: EffectSugar }` | `EffectSugar` は `perform`/`do` を列挙。 |

### 4.3 JSON 直列化ルール

1. `serde(with = "ordered_map")` を利用してフィールド順をソート。`IndexMap` を導入し AST ノード配列も安定順序にする。  
2. `Span` は `{ "start": <u32>, "end": <u32> }`。  
3. `Ident` は `{ "name": "...", "span": {...} }`。内部的にインターンしても JSON では文字列を再出力。  
4. `Option` フィールドは `null` を出さず省略し、OCaml と同じキー集合に合わせる。  
5. Dual-write 比較では `jq --sort-keys` 済み JSON を `reports/dual-write/front-end/w2-ast-alignment/<case>/ast.{ocaml,rust}.json` に保存する。

## 5. Typed AST スキーマ

### 5.1 TypedExpr

```rust
pub struct TypedExpr {
    pub id: NodeId,
    pub kind: TypedExprKind,
    pub ty: TyId,
    pub span: Span,
    pub dict_refs: SmallVec<[DictRefId; 2]>,
}
```

- `TypedExprKind` は OCaml `typed_expr_kind` を 1:1 対応させ、`TFor` の `dict_ref` や `iterator_dict_info` を `ForLoopInfo` に包含。  
- `TyId` は `Idx<Ty>`（`slotmap` もしくは `arena::Idx`）を利用し、`Arc<Ty>` ではなく ID + `TyPool` を採用。これにより `serde` 直列化時に `ty_table` を添付できる。

### 5.2 TypedPattern / TypedDecl

| コンポーネント | Rust 仕様 | 備考 |
| --- | --- | --- |
| `TypedPattern` | `struct TypedPattern { id: NodeId, kind: TypedPatternKind, ty: TyId, bindings: SmallVec<[Binding; 4]>, span: Span }` | `Binding` は `{ name: SmolStr, ty: TyId }`。 |
| `TypedDecl` | `struct TypedDecl { id: NodeId, attrs: Vec<Attribute>, vis: Visibility, kind: TypedDeclKind, scheme: SchemeId, span: Span, dict_refs: Vec<DictRefId> }` | `SchemeId` は `Idx<Scheme>`。 |
| `TypedStmt` | `enum TypedStmt { Decl(TypedDeclId), Expr(TypedExprId), Assign(TypedExprId, TypedExprId), Defer(TypedExprId) }` | OCaml の `typed_stmt` と対応。 |

### 5.3 型・制約・辞書

| 要素 | Rust 型案 | ポイント |
| --- | --- | --- |
| 型表現 | `enum TyKind { Prim(PrimTy), Tuple(SmallVec<[TyId; 3]>), Fn { params: Box<[TyId]>, ret: TyId, effect: EffectRowId }, Record(Vec<RecordField>), Array(TyId), Alias { ident: IdentId, args: Box<[TyId]> }, Infer(InferVarId) }` | `EffectRowId` は `effect_row::Id`。 |
| 型 ID | `struct TyId(NonZeroU32)` | `TyPool` で実体化。Dual-write JSON では `ty_id` を整数で出力し、別テーブルで展開。 |
| `Scheme` | `struct Scheme { vars: SmallVec<[InferVarId; 4]>, body: TyId, constraints: Vec<ConstraintId> }` | `forall` 変数順序は OCaml と同じ（AST 由来の出現順）。 |
| `Constraint` | `enum Constraint { Equals(TyId, TyId), Implements { ty: TyId, trait_id: IdentId }, EffectSubRow { lhs: EffectRowId, rhs: EffectRowId } }` | `ConstraintId` で参照し、JSON では同名キーを使う。 |
| `DictRef` | `struct DictRef { trait_name: IdentId, witness: NodeId, stage: StageRequirement }` | `typed_expr_dict_refs` を 1:1 対応。 |

### 5.4 JSON ダンプ構造

```text
{
  "typed_exprs": [ ... ],
  "typed_patterns": [ ... ],
  "typed_decls": [ ... ],
  "types": [ { "id": 1, "kind": { "prim": "i64" } }, ... ],
  "schemes": [ ... ],
  "constraints": [ ... ]
}
```

- `reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/typed_ast.{ocaml,rust}.json` に保存。  
- `collect-iterator-audit-metrics.py --section effects` 実行時に `typed_decls[*].tfn_effect_row` から `effects.row.size` などを抽出し、0.5pt 以内の一致を検証する。

## 6. Dual-write 連携と検証フロー

1. `scripts/poc_dualwrite_compare.sh --emit-ast --emit-typed --emit-parse-debug` を実行し、`reports/dual-write/front-end/w2-ast-alignment/<case>/` に OCaml / Rust の JSON を保存。  
2. `tooling/ci/collect-iterator-audit-metrics.py --section parser --input <parse-debug.json>` で `packrat_hits`, `span_trace_pairs` を算出し、`w2-parser-metrics.json` として保存。  
3. `--section effects` では `typed_decls` に含まれる `effect_row` / `dict_refs` から `effects.row.len`, `effects.dict_refs` を算出し、`w2-effects-metrics.json` に出力。  
4. `1-1-ast-and-ir-alignment.md` のチェックリストを更新し、完了列に `2025-12-12` と本書ファイルパスを追記する。

## 7. 未決事項 / TODO

| ID | 内容 | 対応先 |
| --- | --- | --- |
| TODO-W2-AST-001 | `EffectMeta`（効果注釈）へ `CapabilityStage` をどの層で付与するか未確定。`docs/spec/3-8-core-runtime-capability.md` の整理後に決定。 | `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` |
| TODO-W2-AST-002 | `TyPool` を `slotmap` / `index_vec` のどちらで実装するか検討が必要。メモリ断片化の測定を W3 の型推論移植開始時に実施。 | `compiler/rust/frontend/src/semantics/types.rs` |
| TODO-W2-AST-003 | `typed_expr.dict_refs` を JSON でフラット配列にするか、`dict_ref_table` を共有するか再検討。`collect-iterator-audit-metrics.py` の入力要件を確認する。 | `tooling/ci/collect-iterator-audit-metrics.py` |

- 上記 TODO は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にも登録し、追跡番号で参照できるようにする。  
- 新しい用語（例: `NodeId`, `TyPool`）は `appendix/glossary-alignment.md` の W2 セクションへ反映する予定。

---

**更新履歴**  
- 2025-12-12: 初稿（W2 AST/Typed AST データモデル草案）。作者: Codex (Rust migration support)。
