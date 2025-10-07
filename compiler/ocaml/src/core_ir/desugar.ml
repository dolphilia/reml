(* Core_ir.Desugar — Desugaring Pass for Core IR (Phase 3)
 *
 * このファイルは Typed AST を Core IR へ変換する際の糖衣削除パスを提供する。
 * 計画書: docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §2
 *
 * 主要機能:
 * 1. パターンマッチの変換 (match → 決定木)
 * 2. パイプ演算子の展開 (|> → let 束縛)
 * 3. let再束縛の正規化 (SSA形式準備)
 *
 * 設計原則:
 * - Typed AST の型情報を完全に保持
 * - Span情報を引き継いで診断を容易に
 * - 段階的な変換（リテラル → タプル → コンストラクタ）
 *)

open Types
open Ast
open Typed_ast
open Ir

(* ========== ユーティリティ ========== *)

(** エラー報告用のヘルパー *)
exception DesugarError of string * span

let desugar_error msg span =
  raise (DesugarError (msg, span))

(** 一時変数の生成 *)
let fresh_temp_var prefix ty span =
  VarIdGen.fresh (Printf.sprintf "$%s" prefix) ty span

(** Typed AST の型を Core IR の型へ変換（現在は単純にコピー） *)
let convert_ty ty = ty

(* ========== 変数スコープマップ ========== *)

(** 変数スコープマップ
 *
 * 同名変数の再束縛を追跡し、SSA形式への変換を準備する。
 * Key: 変数名, Value: 現在のスコープでのvar_id
 *)
type var_scope_map = (string, var_id) Hashtbl.t

let create_scope_map () : var_scope_map =
  Hashtbl.create 64

let lookup_var (map: var_scope_map) (name: string) : var_id option =
  Hashtbl.find_opt map name

let bind_var (map: var_scope_map) (name: string) (var: var_id) : unit =
  Hashtbl.replace map name var

(** スコープのコピー（分岐処理用） *)
let copy_scope_map (map: var_scope_map) : var_scope_map =
  Hashtbl.copy map

(* ========== パターンマッチ決定木 ========== *)

(** 決定木ノード
 *
 * パターンマッチを決定木に変換するための中間表現。
 * 最終的に Core IR の if/match 式へ降格される。
 *)
type decision_tree =
  | Leaf of expr                              (** 葉ノード: 実行する式 *)
  | Fail                                      (** 失敗ノード: マッチ失敗 *)
  | Switch of var_id * switch_case list       (** スイッチノード: 値による分岐 *)
  | Guard of expr * decision_tree * decision_tree  (** ガードノード: 条件付き分岐 *)

and switch_case = {
  test: test_kind;                            (** テスト種別 *)
  subtree: decision_tree;                     (** マッチした場合のサブツリー *)
}

and test_kind =
  | TestLiteral of literal                    (** リテラル値テスト *)
  | TestConstructor of string * int           (** コンストラクタテスト (名前, アリティ) *)
  | TestTuple of int                          (** タプルテスト (要素数) *)
  | TestWildcard                              (** ワイルドカード（常に成功） *)

(* ========== リテラル・単純式の変換 ========== *)

(** リテラルの変換 *)
let desugar_literal lit ty span =
  make_expr (Literal lit) ty span

(** 変数参照の変換 *)
let desugar_var (map: var_scope_map) (id: ident) (ty: ty) (span: span) : expr =
  match lookup_var map id.name with
  | Some var ->
      make_expr (Var var) ty span
  | None ->
      (* 未定義変数（型推論で検出済みのはず） *)
      let var = VarIdGen.fresh id.name ty span in
      bind_var map id.name var;
      make_expr (Var var) ty span

(** 関数適用の変換 *)
let rec desugar_expr (map: var_scope_map) (texpr: typed_expr) : expr =
  let ty = convert_ty texpr.texpr_ty in
  let span = texpr.texpr_span in

  match texpr.texpr_kind with
  | TLiteral lit ->
      desugar_literal lit ty span

  | TVar (id, _scheme) ->
      desugar_var map id ty span

  | TCall (fn, args) ->
      let fn_expr = desugar_expr map fn in
      let arg_exprs = List.map (desugar_arg map) args in
      make_expr (App (fn_expr, arg_exprs)) ty span

  | TLambda (_params, _ret_ty, _body) ->
      (* クロージャ変換は後のフェーズで実装 *)
      (* Phase 1 では簡易実装: 環境キャプチャなし *)
      let closure_span = span in
      let env_vars = [] in  (* TODO: 環境キャプチャの実装 *)
      let fn_ref = "$lambda" in  (* TODO: 一意な名前生成 *)
      let closure_info = { env_vars; fn_ref; closure_span } in
      make_expr (Closure closure_info) ty span

  | TPipe (e1, e2) ->
      desugar_pipe map e1 e2 ty span

  | TBinary (_op, e1, e2) ->
      (* 二項演算子はプリミティブ演算に変換 *)
      (* TODO: 演算子の完全な対応表を実装 *)
      let lhs = desugar_expr map e1 in
      let rhs = desugar_expr map e2 in
      (* 仮実装: 加算のみ *)
      make_expr (Primitive (PrimAdd, [lhs; rhs])) ty span

  | TIf (cond, then_e, else_opt) ->
      let cond_expr = desugar_expr map cond in
      let then_expr = desugar_expr map then_e in
      let else_expr = match else_opt with
        | Some e -> desugar_expr map e
        | None -> make_expr (Literal Unit) ty_unit span
      in
      make_expr (If (cond_expr, then_expr, else_expr)) ty span

  | TMatch (scrut, arms) ->
      desugar_match map scrut arms ty span

  | TBlock stmts ->
      desugar_block map stmts ty span

  | TFieldAccess (e, field) ->
      let obj_expr = desugar_expr map e in
      make_expr (RecordAccess (obj_expr, field.name)) ty span

  | TTupleAccess (e, idx) ->
      let tuple_expr = desugar_expr map e in
      make_expr (TupleAccess (tuple_expr, idx)) ty span

  | TIndex (arr, idx) ->
      let arr_expr = desugar_expr map arr in
      let idx_expr = desugar_expr map idx in
      make_expr (ArrayAccess (arr_expr, idx_expr)) ty span

  | _ ->
      (* その他の式は後のフェーズで実装 *)
      desugar_error "未実装の式種別" span

and desugar_arg (map: var_scope_map) (arg: typed_arg) : expr =
  match arg with
  | TPosArg e -> desugar_expr map e
  | TNamedArg (_name, e) -> desugar_expr map e

(* ========== パイプ演算子の展開 ========== *)

and desugar_pipe (map: var_scope_map) (e1: typed_expr) (e2: typed_expr) (result_ty: ty) (span: span) : expr =
  (* a |> f を let t = a in f(t) へ変換 *)
  let arg_expr = desugar_expr map e1 in
  let arg_ty = convert_ty e1.texpr_ty in
  let temp_var = fresh_temp_var "pipe" arg_ty e1.texpr_span in

  (* e2 は関数適用（仮定） *)
  let fn_expr = desugar_expr map e2 in

  (* 一時変数への参照を構築 *)
  let temp_ref = make_expr (Var temp_var) arg_ty e1.texpr_span in

  (* 関数適用: f(temp_var) *)
  let app_expr = make_expr (App (fn_expr, [temp_ref])) result_ty span in

  (* let 束縛: let temp_var = arg_expr in app_expr *)
  make_expr (Let (temp_var, arg_expr, app_expr)) result_ty span

(* ========== ブロック式の変換 ========== *)

and desugar_block (map: var_scope_map) (stmts: typed_stmt list) (result_ty: ty) (span: span) : expr =
  match stmts with
  | [] ->
      (* 空のブロック → Unit *)
      make_expr (Literal Unit) ty_unit span

  | [TExprStmt e] ->
      (* 単一の式 *)
      desugar_expr map e

  | stmt :: rest ->
      (* 複数の文 → let 束縛の連鎖 *)
      begin match stmt with
      | TDeclStmt decl ->
          (* let/var 宣言 *)
          desugar_block_with_decl map decl rest result_ty span

      | TExprStmt e ->
          (* 式文 → 評価して次へ *)
          let e_expr = desugar_expr map e in
          let rest_expr = desugar_block map rest result_ty span in
          (* 副作用のみを評価する式として扱う *)
          let dummy_var = fresh_temp_var "unused" (convert_ty e.texpr_ty) e.texpr_span in
          make_expr (Let (dummy_var, e_expr, rest_expr)) result_ty span

      | TAssignStmt (_lhs, _rhs) ->
          (* 代入文 → Core IR の Assign に変換（後のフェーズ） *)
          desugar_error "代入文の変換は未実装" span

      | TDeferStmt _ ->
          (* defer 文 → ランタイムサポートが必要（後のフェーズ） *)
          desugar_error "defer 文の変換は未実装" span
      end

and desugar_block_with_decl (map: var_scope_map) (decl: typed_decl) (rest: typed_stmt list) (result_ty: ty) (span: span) : expr =
  match decl.tdecl_kind with
  | TLetDecl (pat, e) ->
      let bound_expr = desugar_expr map e in
      let rest_expr = desugar_block map rest result_ty span in
      (* パターン束縛を let に変換 *)
      desugar_pattern_binding map pat bound_expr rest_expr result_ty span

  | TVarDecl (pat, e) ->
      (* var 宣言も同様（可変性は後で処理） *)
      let bound_expr = desugar_expr map e in
      let rest_expr = desugar_block map rest result_ty span in
      desugar_pattern_binding map pat bound_expr rest_expr result_ty span

  | TFnDecl _ ->
      (* 関数宣言はトップレベル処理（ブロック内関数は後のフェーズ） *)
      desugar_error "ブロック内関数宣言は未実装" span

  | _ ->
      desugar_error "未対応の宣言種別" span

(* ========== パターン束縛の変換 ========== *)

and desugar_pattern_binding (map: var_scope_map) (pat: typed_pattern) (bound_expr: expr) (rest_expr: expr) (result_ty: ty) (span: span) : expr =
  match pat.tpat_kind with
  | TPatVar id ->
      (* 単純な変数束縛 *)
      let var = VarIdGen.fresh id.name (convert_ty pat.tpat_ty) pat.tpat_span in
      bind_var map id.name var;
      make_expr (Let (var, bound_expr, rest_expr)) result_ty span

  | TPatWildcard ->
      (* ワイルドカード → 値を無視 *)
      let dummy_var = fresh_temp_var "wildcard" (convert_ty pat.tpat_ty) pat.tpat_span in
      make_expr (Let (dummy_var, bound_expr, rest_expr)) result_ty span

  | TPatTuple pats ->
      (* タプルパターン → タプルアクセスで分解 *)
      let temp_var = fresh_temp_var "tuple" (convert_ty pat.tpat_ty) pat.tpat_span in
      let bindings = List.mapi (fun i sub_pat ->
        let access_expr = make_expr (TupleAccess (make_expr (Var temp_var) (convert_ty pat.tpat_ty) pat.tpat_span, i))
          (convert_ty sub_pat.tpat_ty) sub_pat.tpat_span in
        (sub_pat, access_expr)
      ) pats in
      (* ネストした let 束縛を生成 *)
      let inner_expr = List.fold_right (fun (sub_pat, access) acc ->
        desugar_pattern_binding map sub_pat access acc result_ty span
      ) bindings rest_expr in
      make_expr (Let (temp_var, bound_expr, inner_expr)) result_ty span

  | _ ->
      (* その他のパターンは後のフェーズで実装 *)
      desugar_error "未実装のパターン種別" pat.tpat_span

(* ========== パターンマッチの変換 ========== *)

and desugar_match (map: var_scope_map) (scrut: typed_expr) (arms: typed_match_arm list) (result_ty: ty) (span: span) : expr =
  (* scrutinee を一時変数に束縛 *)
  let scrut_expr = desugar_expr map scrut in
  let scrut_ty = convert_ty scrut.texpr_ty in
  let scrut_var = fresh_temp_var "match" scrut_ty scrut.texpr_span in

  (* 決定木を構築 *)
  let decision_tree = compile_decision_tree map scrut_var arms in

  (* 決定木を Core IR 式に変換 *)
  let match_body = decision_tree_to_expr map decision_tree result_ty span in

  (* let scrut_var = scrut_expr in match_body *)
  make_expr (Let (scrut_var, scrut_expr, match_body)) result_ty span

and compile_decision_tree (map: var_scope_map) (scrut_var: var_id) (arms: typed_match_arm list) : decision_tree =
  match arms with
  | [] ->
      (* マッチアームがない → 失敗 *)
      Fail

  | arm :: rest ->
      (* 最初のアームを処理 *)
      compile_arm map scrut_var arm rest

and compile_arm (map: var_scope_map) (scrut_var: var_id) (arm: typed_match_arm) (rest: typed_match_arm list) : decision_tree =
  let pattern = arm.tarm_pattern in
  let guard = arm.tarm_guard in
  let body = arm.tarm_body in

  (* パターンに基づいて決定木ノードを生成 *)
  let success_tree = Leaf (desugar_expr map body) in
  let failure_tree = compile_decision_tree map scrut_var rest in

  match pattern.tpat_kind with
  | TPatWildcard ->
      (* ワイルドカード → 常に成功 *)
      begin match guard with
      | Some g ->
          let guard_expr = desugar_expr map g in
          Guard (guard_expr, success_tree, failure_tree)
      | None ->
          success_tree
      end

  | TPatLiteral lit ->
      (* リテラルパターン → スイッチノード *)
      let test_case = { test = TestLiteral lit; subtree = success_tree } in
      Switch (scrut_var, [test_case])

  | TPatVar id ->
      (* 変数パターン → 束縛して成功 *)
      let var = VarIdGen.fresh id.name (convert_ty pattern.tpat_ty) pattern.tpat_span in
      bind_var map id.name var;
      (* TODO: 変数束縛を決定木に組み込む *)
      success_tree

  | _ ->
      (* その他のパターンは後で実装 *)
      Fail

and decision_tree_to_expr (map: var_scope_map) (tree: decision_tree) (result_ty: ty) (span: span) : expr =
  match tree with
  | Leaf e ->
      e

  | Fail ->
      (* マッチ失敗 → panic（ランタイムサポート） *)
      desugar_error "パターンマッチ失敗の処理は未実装" span

  | Switch (var, cases) ->
      (* スイッチノードを if 式の連鎖に変換 *)
      compile_switch_to_if map var cases result_ty span

  | Guard (cond, then_tree, else_tree) ->
      (* ガードノードを if 式に変換 *)
      let then_expr = decision_tree_to_expr map then_tree result_ty span in
      let else_expr = decision_tree_to_expr map else_tree result_ty span in
      make_expr (If (cond, then_expr, else_expr)) result_ty span

and compile_switch_to_if (map: var_scope_map) (var: var_id) (cases: switch_case list) (result_ty: ty) (span: span) : expr =
  match cases with
  | [] ->
      (* ケースなし → 失敗 *)
      desugar_error "スイッチケースが空" span

  | [case] ->
      (* 単一ケース *)
      decision_tree_to_expr map case.subtree result_ty span

  | case :: rest ->
      (* 複数ケース → if 式の連鎖 *)
      let test_expr = compile_test_expr var case.test span in
      let then_expr = decision_tree_to_expr map case.subtree result_ty span in
      let else_expr = compile_switch_to_if map var rest result_ty span in
      make_expr (If (test_expr, then_expr, else_expr)) result_ty span

and compile_test_expr (var: var_id) (test: test_kind) (span: span) : expr =
  match test with
  | TestLiteral lit ->
      (* var == lit *)
      let var_ref = make_expr (Var var) var.vty span in
      let lit_expr = make_expr (Literal lit) var.vty span in
      make_expr (Primitive (PrimEq, [var_ref; lit_expr])) ty_bool span

  | TestWildcard ->
      (* 常に true *)
      make_expr (Literal (Bool true)) ty_bool span

  | _ ->
      (* その他のテストは後で実装 *)
      desugar_error "未実装のテスト種別" span

(* ========== トップレベル変換 ========== *)

(** トップレベル宣言の変換 *)
let desugar_decl (_map: var_scope_map) (_decl: typed_decl) : function_def option =
  (* 関数宣言のみを Core IR に変換 *)
  (* その他の宣言（型定義、グローバル変数）は後のフェーズで実装 *)
  None

(** コンパイル単位の変換 *)
let desugar_compilation_unit (tcu: typed_compilation_unit) : module_def =
  let map = create_scope_map () in

  (* 関数定義のみを抽出（暫定） *)
  let function_defs = List.filter_map (desugar_decl map) tcu.tcu_items in

  {
    module_name = "main";  (* TODO: モジュール名の取得 *)
    type_defs = [];        (* TODO: 型定義の変換 *)
    global_defs = [];      (* TODO: グローバル変数の変換 *)
    function_defs;
  }
