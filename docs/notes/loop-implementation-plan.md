# ループ構文実装計画

**作成日**: 2025-10-13
**Phase**: Phase 2 Week 20-21
**関連タスク**: [2-1-typeclass-strategy.md](../plans/bootstrap-roadmap/2-1-typeclass-strategy.md) セクション4

## 概要

型クラスベンチマーク実行のために while/for ループの実装を試みたところ、現在のコンパイラアーキテクチャにおける構造的課題が明らかになりました。本文書では、発見された課題と、Phase 3 以降での完全実装に向けた戦略を記録します。

## 現状の実装状況

### ✅ 完了した実装（Phase 2）

| コンポーネント | ファイル | 実装内容 | 完了度 |
|--------------|---------|---------|--------|
| AST定義 | `src/ast.ml:106-108` | `While`, `For`, `Loop` 式の定義 | 100% |
| パーサー | `src/parser.mly:718-737` | while/for/loop のパース規則 | 100% |
| 型付きAST | `src/typed_ast.ml:43-45` | `TWhile`, `TFor`, `TLoop` の定義 | 100% |
| 型推論 | `src/type_inference.ml:702-777` | while/for/loop/unsafe/return/defer/assign の型推論 | 100% |
| Core IR脱糖 | `src/core_ir/desugar.ml:366-404` | 簡易実装（Unit値を返す） | 30% |

### 🚧 未完了の実装

| コンポーネント | 実装が必要な内容 | 優先度 | 予定Phase |
|--------------|-----------------|--------|----------|
| Core IR脱糖 | CFGブロックへの展開 | High | Phase 3 |
| CFG構築 | ループブロックの生成 | High | Phase 3 |
| 最適化 | ループ不変式の移動 | Medium | Phase 3-4 |
| LLVM IR生成 | br/phi命令の生成 | High | Phase 3 |
| ランタイム | break/continue サポート | Low | Phase 4 |

## 発見された課題

### 課題1: Core IRの構造的制約

#### 問題の詳細

Core IRは式（`expr`）とブロック（`block`）が明確に分離された設計になっています：

```ocaml
(* src/core_ir/ir.ml *)
type expr = {
  expr_kind : expr_kind;
  expr_ty : ty;
  expr_span : span;
}

type block = {
  label : label;
  params : var_id list;
  stmts : stmt list;
  terminator : terminator;
  block_span : span;
}
```

**制約**:
- `desugar_expr : var_scope_map -> typed_expr -> expr` は単一の式を返す
- ループは複数の基本ブロックで表現されるべき（条件チェックブロック、ボディブロック、出口ブロック）
- `expr_kind` にはループブロックを表現するバリアントが存在しない

**影響**:
- while/for ループを単一の式として脱糖できない
- CFG構築時にループを処理する必要がある
- 関数定義レベル（`fn_def.body: block list`）でないとブロックを生成できない

#### 根本原因

設計意図として、Core IRは「糖衣を剥がした後の正規化された形式」であり、制御フローは基本ブロックとして表現されます。しかし、現在の脱糖パスは式単位で処理を行っており、ブロックを返却できません。

### 課題2: 脱糖パスとCFG構築の責務分担

#### 問題の詳細

現在のコンパイラパイプラインは以下の順序で処理を行います：

```
Typed AST
  ↓ desugar (式を変換)
Core IR (expr)
  ↓ build_cfg_from_expr
CFG (block list)
  ↓ optimize
Optimized CFG
  ↓ codegen
LLVM IR
```

**制約**:
- `desugar` は式レベルの変換のみ（単一exprを返す）
- `build_cfg_from_expr` は既に脱糖された式をブロックに分解
- ループは脱糖時にブロックへ展開すべきだが、脱糖パスでは式しか返せない

**影響**:
- ループを適切なタイミングで処理できない
- 暫定実装として `make_expr (Literal Unit)` を返している（機能しない）

#### 考えられる解決策

**選択肢A: CFG構築時にループを処理**
- 脱糖パスでは「ループマーカー」式を残す
- CFG構築時に専用のループ展開ロジックを実装
- 利点: 既存のアーキテクチャを大きく変更しない
- 欠点: CFGビルダーが複雑化

**選択肢B: 脱糖パスを二段階に分割**
1. 式レベルの脱糖（現在の `desugar_expr`）
2. 制御フロー展開（新規 `expand_control_flow`）
- 利点: 責務が明確
- 欠点: パイプラインの変更が必要

**選択肢C: ブロックビルダーAPIの導入**
- `desugar_expr` がビルダーコンテキストを受け取る
- ループ時にブロックをビルダーに追加
- 利点: 柔軟性が高い
- 欠点: 大規模なリファクタリングが必要

### 課題3: LLVM IR生成での制御フロー表現

#### 問題の詳細

LLVM IRでのループは以下の構造で表現されます：

```llvm
entry:
  %i = alloca i64
  store i64 0, i64* %i
  br label %loop.cond

loop.cond:
  %i.val = load i64, i64* %i
  %cond = icmp slt i64 %i.val, 1000000
  br i1 %cond, label %loop.body, label %loop.exit

loop.body:
  ; ループボディ
  %i.next = add i64 %i.val, 1
  store i64 %i.next, i64* %i
  br label %loop.cond

loop.exit:
  ret void
```

**必要な機能**:
1. ミュータブル変数（`alloca`, `load`, `store`）
2. ラベルとブランチ（`br label`, `br i1`）
3. PHIノード（最適化時）
4. 配列イテレーション（for式用）

**現状**:
- ミュータブル変数は未サポート（参照カウントのみ）
- CFGからLLVM IRへの変換は基本的な制御フローのみ対応
- PHIノードは未生成

## 実装戦略

### Phase 3 での実装計画

#### ステップ1: ミュータブル変数のサポート（Week 25-26）

**目標**: ループカウンタを保持できるようにする

**実装内容**:
1. AST に `mut` キーワードのサポート追加
   ```reml
   let mut i = 0
   ```
2. 型推論で可変変数を追跡
3. Core IR に `Assign` 式を追加（既に stmt には存在）
4. LLVM IR で `alloca`/`load`/`store` を生成

**成果物**:
- `compiler/ocaml/src/ast.ml`: `let_decl` に `is_mutable: bool` フィールド追加
- `compiler/ocaml/src/type_inference.ml`: 可変変数の型チェック
- `compiler/ocaml/src/llvm_gen/codegen.ml`: alloca/load/store 生成

**検証方法**:
```reml
fn test_mut() -> i64 {
  let mut x = 0
  x := 10
  x
}
```

**進捗メモ（2025-10-13）**

- [x] 型環境に `mutability` 情報を追加し、`var` 宣言を `Type_env.extend`/`infer_decl` で処理（`compiler/ocaml/src/type_env.ml`, `compiler/ocaml/src/type_inference.ml`）
- [x] `:=` の型推論で `var` 以外への再代入を拒否し、`ImmutableBinding` / `NotAssignable` 診断を追加（`compiler/ocaml/src/type_error.ml`）
- [x] Core IR で `var_id.vmutable`、`AssignMutable` 式、`Alloca`/`Store` 文を導入し、脱糖パスと CFG 線形化がミュータブル変数をメモリ経由で扱うよう更新（`compiler/ocaml/src/core_ir/*.ml`）
- [x] LLVM コード生成で `Alloca` → `Store` → `Load` のシーケンスを生成し、`Var` 参照時に自動的にロードを挿入（`compiler/ocaml/src/llvm_gen/codegen.ml`）
- [x] 付随パス（定数畳み込み / DCE / モノモルフィゼーション PoC / テストスイート）を新しい IR ノードに対応させ、`desugar` 単体テストを更新

**現状の確認ポイント**

- `let mut` / `:=` の Core IR は `Alloca → Store` に正規化され、再参照は `Load` を経由（SSA への移行準備完了）
- DCE・ConstFold は `Store` を常に副作用アリとして保持するため、ループカウンタが消去されない
- `dune build` は CLI 側の既知 Warning（`warning 21`）で停止するが、新規変更部は型チェックを通過済み

**次ステップ**

1. ループ専用の CFG 展開を実装し、`TWhile`/`TFor`/`TLoop` を基本ブロック列へ変換（`core_ir/desugar.ml` のマーカー処理・`cfg.ml` のブロック生成を拡張）
2. ループカウンタの `phi` ノード導入と `AssignMutable` からの SSA 変換戦略を検討（Step2 の出口条件）
3. `let mut` / 単純な while のゴールデンテスト・LLVM IR スナップショットを追加して、`alloca`/`load`/`store` パターンを回帰検出に組み込む
4. ランタイム側で追加メモリアクセスの診断（トレース/メトリクス）にフックするかを検討し、必要なら `docs/notes/llvm-spec-status-survey.md` に TODO を追記

#### ステップ2: CFG構築でのループ展開（Week 26-27）

**目標**: `TWhile` / `TFor` / `TLoop` を Core IR の基本ブロック列へ展開し、SSA 変換に備えたループヘッダ `Phi` 計画を固める。

**前提整理（2025-10-13 更新）**
- `desugar_expr` がまだループを `Literal Unit` に潰しているため、`cfg.ml` 側ではループ構造を検出できない。
- `core_ir/ir.ml` には `Phi` 文が定義済みだが、`linearize_expr` は if/match 以外で未使用。
- `AssignMutable` は `Alloca`/`Store` ベースで動いており、SSA 化（mem2reg 相当）は未着手。ループカウンタは現在メモリアクセスでのみ表現される。

**実装方針**

**2.1 ループIRマーカーの導入**
- `compiler/ocaml/src/core_ir/ir.ml` に `Loop` 系の `expr_kind` を追加する。`LoopMarker` は以下の情報を保持する想定：
  ```ocaml
  type loop_kind =
    | WhileLoop of expr  (* cond *)
    | ForLoop of for_lowering  (* desugar済みの初期化/更新/イテレータ情報 *)
    | InfiniteLoop

  and loop_info = {
    loop_kind : loop_kind;
    loop_body : expr;
    loop_span : span;
    loop_carried : loop_carried_var list;  (* PHI 候補メタデータ *)
  }
  ```
  - `loop_carried` は `let mut` などループ内で再代入される変数を記録するメタ情報（後述の PHI 挿入で使用）。
  - `for_lowering` には `init`（初期代入列）、`iter_state`（配列長・インデックス変数）、`advance`（更新式）、`pattern`（パターン束縛）を保持する構造体を定義し、`TFor` を while 相当に展開できる粒度まで情報を詰める。
- `compiler/ocaml/src/core_ir/desugar.ml` で `TWhile` / `TFor` / `TLoop` を再帰的に脱糖したサブ式を `LoopMarker` に詰めて返す。既存の `Literal Unit` 返却は廃止する。
  - `loop_carried` の充填は初期段階では空にし、Step2 の後半で静的解析を加える（`collect_loop_carried_mutables` ヘルパを追加）。
  - `for` の場合は、`typed_pattern` から `VarIdGen` を通じた一時変数（インデックス、配列長、現在要素）をここで生成し、CFG 層が追加の糖衣を考慮しなくて済むようにする。

**2.2 CFG ビルダーの拡張**
- `compiler/ocaml/src/core_ir/cfg.ml` に `linearize_loop`（仮名）関数を追加し、`LoopMarker` を検出したら以下のブロック構成を生成する：
  ```
  preheader → header → body → latch ┐
        └─────────────── exit ←─────┘
  ```
  - **preheader**: 既存ブロック（`linearize_expr` 呼出元）を閉じ、`TermJump header` で開始させる。`ForLoop` の初期化式はここで `Assign`/`Store` に展開。
  - **header**: 条件式を評価し、`TermBranch (cond, body_label, exit_label)` を生成。`InfiniteLoop` は `TermJump body_label`。
  - **body**: `loop_body` を通常の式として線形化。`Unit` 結果は `VarIdGen` でダミー変数に束縛。
  - **latch**: `for` の更新式や `continue` 相当の後処理をまとめるブロック。終了時に `TermJump header_label`。
  - **exit**: ループ式の戻り値（現状 Unit）を `TermReturn` か上位ブロックへの `TermJump` に変換。`linearize_expr` から返す結果変数は exit で生成。
- `LabelGen` の連番管理が崩れないよう、`linearize_loop` 内でラベル生成を完結させ、`linearize_expr` は戻り値変数のみを扱う。
- `build_cfg_from_expr` / `build_cfg` の戻り値は既存と同じ `block list`。ループ導入後は `validate_cfg` を活用して基本整形性を都度確認する。

**2.3 ループ用 PHI ノード導入方針**
- `loop_carried` に記録された変数について、`header` ブロック先頭に `Phi` 文を挿入する。基本形：
  ```ocaml
  add_stmt builder
    (Phi (var_phi, [ (preheader_label, init_var); (latch_label, updated_var) ]));
  ```
  - `init_var` はループ外で定義された SSA 変数（`let mut` の初期値など）。
  - `updated_var` は `latch` ブロック内で生成する新しい SSA 変数。`AssignMutable` をこの時点で一旦 `Assign` ベースに切り替える必要があるため、Loop 内だけでも `Store` を純 SSA 代入へ昇格させる mini mem2reg を実装する。
- 実装ステップ：
  1. `desugar` 段階で `let mut` の初期値を `loop_carried` に記録する（`VarIdGen` 情報を保持）。
  2. `linearize_loop` で `loop_carried` を参照し、`preheader` / `latch` にそれぞれ `Assign` を挿入する。現状の `Alloca`/`Store` は残しつつ `Phi` を併記し、後段の最適化パスで二重管理を解消する。
  3. `compiler/ocaml/src/core_ir/pipeline.ml` に「ループ内の `Store` を SSA へ昇格させる」軽量パス（仮称 `promote_mutable_in_loops`）を追加し、`Phi` を生成し終えた後に `Store` を削除する計画を明記。
- `break` / `continue` 未実装の間は、`loop_carried` の後辺は常に `latch` 1 箇所に限定されるため、`Phi` の引数は 2 本で固定できる。将来 `continue` が追加される際は、`continue` 先のブロックラベルを `Phi` のソースに追加する設計とする。

**2.4 検証とフォローアップ**
- `./remlc samples/loop/reml --emit-cfg` のようなサンプルを用意し、`loop_cond_X` / `loop_body_X` / `loop_exit_X` の生成を確認。
- `compiler/ocaml/tests/test_cfg_loop.ml`（新規）を追加し、`build_cfg_from_expr` の出力ブロック列をスナップショット化。
- `llvm_ir` ゴールデンに while ループ 1 種を追加し、`Phi` が IR に降りるまでは `alloca` ベースで比較。`Phi` 有効化時点で golden の更新を伴うことをあらかじめ `docs/notes/llvm-spec-status-survey.md` に記載する。
- `loop_carried` 収集の精度を検証するため、`let mut i = 0; while ... { i := i + 1 }` で `Phi` が 1 つだけ生成されること、ネストループでも外側・内側の `Phi` が独立することをユニットテスト化する。

**成果物**
- `compiler/ocaml/src/core_ir/ir.ml`: `LoopMarker`（正式名称要検討）および補助型追加。
- `compiler/ocaml/src/core_ir/desugar.ml`: ループマーカー生成＋`loop_carried` メタデータ収集。
- `compiler/ocaml/src/core_ir/cfg.ml`: `linearize_loop` 実装、`Phi` 挿入処理、`LabelGen` 管理拡張。
- `compiler/ocaml/src/core_ir/pipeline.ml`: ループ内 `Store` を SSA へ昇格させるフェーズの TODO 追記。
- `docs/notes/llvm-spec-status-survey.md`: ループ SSA 化に伴う LLVM 側の差分とリスクの記録。

**検証方法**
```bash
# 1. CFG スナップショット
remlc samples/loop/simple_while.reml --emit-cfg > /tmp/simple_while.cfg

# 2. ループヘッダに Phi が入っていることを確認
grep -n "Phi" /tmp/simple_while.cfg

# 3. 既存テストの回帰確認
dune runtest compiler/ocaml/tests
```

**リスク・検討事項**
- ループボディ線形化中に `LabelGen` を再帰呼出で共有するため、`linearize_loop` 内でのブロック開始/終了順序を誤ると未閉じブロックが発生する。`builder.current_label` の状態管理をユニットテストでカバーする。
- `loop_carried` の自動検出は Phase 2 の残期間で全パターンを網羅するのが難しいため、初期実装では「ループ直前で `let mut` 宣言された変数のみ」を扱い、他ケースは Warning として診断へ接続する案を採用する。
- LLVM 側で `Phi` と `alloca` が混在すると冗長コードになる。`promote_mutable_in_loops` を遅らせる場合でも、ステップ内で「`alloca` は暫定的に残るが Phase 3 で削除する」旨を技術的負債リストへ追加する。

#### ステップ3: For式の配列イテレーション（Week 27-28）

**目標**: `for x in array { ... }` を動作させる

**実装内容**:

1. インデックス変数の自動生成
2. 配列長の取得（プリミティブ演算追加）
3. パターン変数の束縛

```ocaml
(* desugar.ml での for式処理 *)
| TFor (pat, source, body) ->
    let source_expr = desugar_expr map source in
    let body_expr = desugar_expr map body in

    (* インデックス変数 *)
    let index_var = VarIdGen.fresh "__for_index" ty_i64 span in

    (* パターン変数抽出 *)
    let pat_var = extract_pattern_var pat in

    (* 初期化: index = 0 *)
    let init = [(index_var, make_expr (Literal (Int ("0", Base10))) ty_i64 span)] in

    (* 条件: index < array.length *)
    let index_ref = make_expr (Var index_var) ty_i64 span in
    let array_len = make_expr (ArrayLength source_expr) ty_i64 span in
    let cond = make_expr (Primitive (PrimLt, [index_ref; array_len])) ty_bool span in

    (* 更新: index = index + 1 *)
    let one = make_expr (Literal (Int ("1", Base10))) ty_i64 span in
    let index_incr = make_expr (Primitive (PrimAdd, [index_ref; one])) ty_i64 span in
    let update = [(index_var, index_incr)] in

    (* ボディ: let pat_var = array[index] in body *)
    let array_access = make_expr (ArrayAccess (source_expr, index_ref)) pat.tpat_ty span in
    let body_with_binding = make_expr (Let (pat_var, array_access, body_expr)) ty span in

    make_expr
      (LoopMarker {
        loop_kind = ForLoop (pat, source);
        loop_cond = Some cond;
        loop_init = init;
        loop_update = update;
        loop_body = body_with_binding;
      })
      ty span
```

**必要な追加機能**:
- `ArrayLength` 式の追加（`expr_kind` に追加）
- LLVM IR での配列長取得（構造体の先頭フィールド）

**成果物**:
- `compiler/ocaml/src/core_ir/ir.ml`: `ArrayLength` 式追加
- `compiler/ocaml/src/llvm_gen/codegen.ml`: 配列長の取得実装

**検証方法**:
```reml
fn test_for() -> i64 {
  let arr = [1, 2, 3, 4, 5]
  let mut sum = 0
  for x in arr {
    sum := sum + x
  }
  sum  // 15
}
```

#### ステップ4: LLVM IR生成の完成（Week 28-29）

**目標**: CFGをLLVM IRのbr/phi命令に変換

**実装内容**:

**4.1 基本ブロックのラベル生成**

```ocaml
(* codegen.ml *)
let codegen_block (llctx : llcontext) (llmod : llmodule) (llfn : llvalue)
    (block : block) (block_map : (label, llbasicblock) Hashtbl.t) : unit =

  (* ラベルに対応するLLVMブロックを取得または作成 *)
  let llblock =
    match Hashtbl.find_opt block_map block.label with
    | Some bb -> bb
    | None ->
        let bb = append_block llctx block.label llfn in
        Hashtbl.add block_map block.label bb;
        bb
  in

  position_at_end llblock builder;

  (* ブロック内の命令を生成 *)
  List.iter (codegen_stmt llctx llmod builder) block.stmts;

  (* 終端命令を生成 *)
  codegen_terminator llctx llmod builder block.terminator block_map

and codegen_terminator (llctx : llcontext) (llmod : llmodule) (builder : llbuilder)
    (term : terminator) (block_map : (label, llbasicblock) Hashtbl.t) : unit =

  match term with
  | TermReturn expr ->
      let llval = codegen_expr llctx llmod builder expr in
      ignore (build_ret llval builder)

  | TermJump target_label ->
      let target_block = Hashtbl.find block_map target_label in
      ignore (build_br target_block builder)

  | TermBranch (cond_expr, then_label, else_label) ->
      let llcond = codegen_expr llctx llmod builder cond_expr in
      let then_block = Hashtbl.find block_map then_label in
      let else_block = Hashtbl.find block_map else_label in
      ignore (build_cond_br llcond then_block else_block builder)

  | TermSwitch (scrutinee, cases, default_label) ->
      let llscrutinee = codegen_expr llctx llmod builder scrutinee in
      let default_block = Hashtbl.find block_map default_label in
      let llswitch = build_switch llscrutinee default_block (List.length cases) builder in
      List.iter (fun (lit, case_label) ->
        let llcase_val = codegen_literal lit in
        let case_block = Hashtbl.find block_map case_label in
        add_case llswitch llcase_val case_block
      ) cases

  | TermUnreachable ->
      ignore (build_unreachable builder)
```

**4.2 関数全体のCFG処理**

```ocaml
(* codegen.ml *)
let codegen_function (llctx : llcontext) (llmod : llmodule) (fn_def : fn_def) : llvalue =
  (* 関数シグネチャを生成 *)
  let llfn = declare_function fn_def.fn_name fn_ty llmod in

  (* ブロックマップを作成（ラベル → LLVMブロック） *)
  let block_map = Hashtbl.create 16 in

  (* エントリブロックを作成 *)
  let entry_block = append_block llctx "entry" llfn in
  Hashtbl.add block_map "entry" entry_block;

  (* 全ブロックのラベルを事前登録（前方参照対応） *)
  List.iter (fun block ->
    let llblock = append_block llctx block.label llfn in
    Hashtbl.add block_map block.label llblock
  ) fn_def.body;

  (* 各ブロックのコード生成 *)
  List.iter (fun block ->
    codegen_block llctx llmod llfn block block_map
  ) fn_def.body;

  llfn
```

**成果物**:
- `compiler/ocaml/src/llvm_gen/codegen.ml`: ブロック単位のコード生成
- `compiler/ocaml/tests/llvm-ir/golden/while_loop.ll.golden`: ゴールデンテスト

**検証方法**:
```bash
# LLVM IR を生成
./remlc test.reml --emit-ir

# 生成されたIRを確認
cat test.ll

# llc でネイティブコードに変換
llc test.ll -o test.s

# 実行可能ファイルを生成
gcc test.s runtime.a -o test
./test
```

#### ステップ5: 統合テストとベンチマーク（Week 29-30）

**目標**: 型クラスベンチマークを実行可能にする

**実装内容**:

1. while/forループの統合テスト作成
2. ベンチマークコードの動作確認
3. 型クラスベンチマークスクリプトの実行

**テストケース**:

```reml
// tests/integration/test_while_loop.reml
fn test_simple_while() -> i64 {
  let mut i = 0
  let mut sum = 0
  while i < 10 {
    sum := sum + i
    i := i + 1
  }
  sum  // 45
}

fn test_for_array() -> i64 {
  let arr = [1, 2, 3, 4, 5]
  let mut sum = 0
  for x in arr {
    sum := sum + x
  }
  sum  // 15
}

fn test_nested_loop() -> i64 {
  let mut sum = 0
  let mut i = 0
  while i < 10 {
    let mut j = 0
    while j < 10 {
      sum := sum + 1
      j := j + 1
    }
    i := i + 1
  }
  sum  // 100
}
```

**ベンチマーク実行**:

```bash
# ベンチマークスクリプトを実行
cd compiler/ocaml
./scripts/benchmark_typeclass.sh

# 結果を確認
cat benchmark_results/comparison_report.md
```

**成果物**:
- `compiler/ocaml/tests/integration/test_loops.reml`: 統合テスト
- `compiler/ocaml/benchmarks/*.reml`: 動作確認済みベンチマーク
- `docs/notes/typeclass-performance-evaluation.md`: 計測結果

### Phase 4 での拡張機能

#### break/continue サポート

**実装方針**:
- `break` → 出口ブロックへの直接ジャンプ
- `continue` → 条件チェックブロックへの直接ジャンプ
- ネストしたループでのスコープ管理

#### ループ最適化

**最適化項目**:
1. ループ不変式の移動（Loop-Invariant Code Motion）
2. ループ融合（Loop Fusion）
3. ループ展開（Loop Unrolling）
4. 強度削減（Strength Reduction）

## 技術的な詳細

### ミュータブル変数の実装詳細

#### 型システムへの影響

```ocaml
(* types.ml *)
type mutability = Immutable | Mutable

type var_info = {
  var_name : string;
  var_ty : ty;
  var_mutability : mutability;
  var_span : span;
}
```

#### 型チェック

```ocaml
(* type_inference.ml *)
let check_assignment (env : env) (lhs : expr) (rhs : expr) : (unit, type_error) result =
  match lhs.expr_kind with
  | Var id ->
      (match lookup_var_info env id.name with
      | Some var_info when var_info.var_mutability = Mutable ->
          Ok ()
      | Some var_info ->
          Error (ImmutableAssignment (id.name, lhs.expr_span))
      | None ->
          Error (UnboundVariable (id.name, lhs.expr_span)))
  | _ ->
      Error (InvalidLValue lhs.expr_span)
```

### CFG構築の詳細

#### ブロック結合アルゴリズム

```ocaml
(* cfg.ml *)
let patch_block_terminator (block : block) (new_term : terminator) : block =
  { block with terminator = new_term }

let connect_blocks (pred_block : block) (succ_label : label) : block =
  match pred_block.terminator with
  | TermReturn _ ->
      (* 既にreturnがある場合は変更しない *)
      pred_block
  | TermJump _ | TermBranch _ | TermSwitch _ | TermUnreachable ->
      (* 既にジャンプがある場合も変更しない *)
      pred_block
  | _ ->
      (* 終端命令がない場合、ジャンプを追加 *)
      patch_block_terminator pred_block (TermJump succ_label)
```

#### ドミネータ解析（将来の最適化用）

```ocaml
(* cfg.ml *)
type dominator_tree = {
  idom : (label, label option) Hashtbl.t;  (* 即座ドミネータ *)
  dominated : (label, label list) Hashtbl.t;  (* 支配されるブロック *)
}

let compute_dominators (blocks : block list) (entry_label : label) : dominator_tree =
  (* Lengauer-Tarjan アルゴリズム *)
  ...
```

### LLVM IR生成の詳細

#### alloca/load/store パターン

```llvm
; ミュータブル変数のalloca（関数エントリで生成）
entry:
  %i.addr = alloca i64
  store i64 0, i64* %i.addr
  br label %loop.cond

; ループ内でのload/store
loop.body:
  %i.val = load i64, i64* %i.addr
  %i.next = add i64 %i.val, 1
  store i64 %i.next, i64* %i.addr
  br label %loop.cond
```

#### PHIノード生成（最適化版）

```llvm
; SSA形式でのループカウンタ
loop.cond:
  %i = phi i64 [ 0, %entry ], [ %i.next, %loop.body ]
  %cond = icmp slt i64 %i, 1000000
  br i1 %cond, label %loop.body, label %loop.exit

loop.body:
  ; ボディ処理
  %i.next = add i64 %i, 1
  br label %loop.cond
```

## テスト戦略

### 単体テスト

| テスト対象 | ファイル | テスト内容 |
|----------|---------|----------|
| ミュータブル変数 | `test_mut_var.ml` | let mut、代入、型チェック |
| Whileループ | `test_while.ml` | 基本while、ネストwhile、早期脱出 |
| Forループ | `test_for.ml` | 配列イテレーション、範囲、パターン |
| CFG構築 | `test_cfg_loops.ml` | ブロック生成、ジャンプ先、ドミネータ |
| LLVM IR | `test_codegen_loops.ml` | br命令、phi命令、最適化 |

### 統合テスト

```reml
// tests/integration/comprehensive_loops.reml

// フィボナッチ数列
fn fibonacci(n: i64) -> i64 {
  if n <= 1 { return n }

  let mut a = 0
  let mut b = 1
  let mut i = 2

  while i <= n {
    let tmp = a + b
    a := b
    b := tmp
    i := i + 1
  }

  b
}

// 配列の合計
fn array_sum(arr: [i64]) -> i64 {
  let mut sum = 0
  for x in arr {
    sum := sum + x
  }
  sum
}

// 二重ループ
fn matrix_sum(rows: i64, cols: i64) -> i64 {
  let mut sum = 0
  let mut i = 0

  while i < rows {
    let mut j = 0
    while j < cols {
      sum := sum + (i * cols + j)
      j := j + 1
    }
    i := i + 1
  }

  sum
}
```

### ベンチマーク

```bash
# 型クラスベンチマークの実行
./scripts/benchmark_typeclass.sh

# 期待される出力:
# ========================================
# Reml 型クラス実装ベンチマーク
# ========================================
#
# [マイクロベンチマーク]
# - bench_eq_i64:     実行完了 (10^6 回)
# - bench_eq_string:  実行完了 (10^6 回)
# - bench_ord_i64:    実行完了 (10^6 回)
#
# [マクロベンチマーク]
# - find_element:     実行完了
# - bubble_sort:      実行完了
# - count_in_range:   実行完了
#
# 詳細レポート: benchmark_results/comparison_report.md
```

## 既知の制約と回避策

### 制約1: 配列の実装が不完全

**制約**: 現時点で配列リテラル `[1, 2, 3]` の型推論は未実装

**回避策**: Phase 3で配列型を完全実装してからforループを実装

**技術的負債**: `docs/compiler/ocaml/docs/technical-debt.md` の M1 に記録済み

### 制約2: クロージャでのキャプチャ

**制約**: ループ内でクロージャを生成する場合、ループ変数のキャプチャが未対応

**回避策**: Phase 3ではクロージャ内でのループ変数使用を制限

**将来の実装**: Phase 4でクロージャ変換を完全実装

### 制約3: break/continueの欠如

**制約**: 現時点ではbreak/continueをサポートしない

**回避策**: フラグ変数を使った早期脱出パターンを推奨

```reml
// break の代替パターン
let mut found = false
let mut i = 0
while i < n && !found {
  if condition {
    found := true
  }
  i := i + 1
}
```

## 成功基準

### Phase 3 完了時の基準

- [ ] ミュータブル変数の宣言・代入が動作
- [ ] Whileループが正しくコンパイルされてLLVM IRを生成
- [ ] Forループが配列をイテレートできる
- [ ] ネストしたループが正しく動作
- [ ] ベンチマークスクリプトが完走する
- [ ] 型クラス性能評価レポートが完成
- [ ] 全既存テストが引き続き成功（レグレッションゼロ）

### Phase 4 完了時の基準

- [ ] break/continueが実装されている
- [ ] ループ最適化が有効化されている
- [ ] PHIノードが生成されている
- [ ] クロージャ内でループ変数をキャプチャできる
- [ ] 配列以外のイテレータ（Range、Customなど）をサポート

## 参考資料

### 内部資料

- [Bootstrap Roadmap Phase 3](../plans/bootstrap-roadmap/3-0-phase3-overview.md)
- [型クラス戦略](../plans/bootstrap-roadmap/2-1-typeclass-strategy.md)
- [Core IR設計](../plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)
- [LLVM統合ガイド](../guides/llvm-integration-notes.md)
- [技術的負債リスト](../../compiler/ocaml/docs/technical-debt.md)

### 外部資料

- [LLVM Language Reference Manual - Terminator Instructions](https://llvm.org/docs/LangRef.html#terminator-instructions)
- [SSA Book - Chapter 3: Control Flow Graphs](http://ssabook.gforge.inria.fr/latest/book.pdf)
- [Modern Compiler Implementation in ML - Chapter 7: Activation Records](https://www.cs.princeton.edu/~appel/modern/ml/)

## 更新履歴

- **2025-10-13**: 初版作成（Phase 2 Week 20-21 でのループ実装試行の結果を記録）
