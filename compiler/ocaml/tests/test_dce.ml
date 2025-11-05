(* Test_dce — Dead Code Elimination Pass Tests (Phase 3)
 *
 * このファイルは死コード削除最適化パスのテストを提供する。
 * 計画書: docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §5
 *)

[@@@warning "-33"] (* unused-open を抑制 *)

open Types
open Ast
open Core_ir.Ir
open Core_ir.Dce

(* ========== テストヘルパー ========== *)

let dummy_span = Ast.dummy_span

(** 整数リテラル式を作成 *)
let make_int_lit (i : int64) : expr =
  make_expr
    (Literal (Ast.Int (Int64.to_string i, Ast.Base10)))
    ty_i64 dummy_span

(** ブールリテラル式を作成 *)
let make_bool_lit (b : bool) : expr =
  make_expr (Literal (Ast.Bool b)) ty_bool dummy_span

(** 変数参照式を作成 *)
let make_var_ref (var : var_id) : expr = make_expr (Var var) var.vty dummy_span

(** プリミティブ演算式を作成 *)
let make_prim (op : prim_op) (args : expr list) (ty : ty) : expr =
  make_expr (Primitive (op, args)) ty dummy_span

(** テスト用の簡易関数を作成 *)
let make_test_function (body_expr : expr) : function_def =
  let entry_block =
    make_block "entry" [] [] (TermReturn body_expr) dummy_span
  in
  let metadata =
    {
      fn_span = dummy_span;
      effects = { declared = []; residual = [] };
      capabilities = { required = []; stage = None };
      dict_instances = [];
      opt_flags =
        {
          allow_dce = true;
          allow_inline = false;
          preserve_for_diagnostics = false;
        };
    }
  in
  {
    fn_name = "test_fn";
    fn_params = [];
    fn_return_ty = body_expr.expr_ty;
    fn_blocks = [ entry_block ];
    fn_metadata = metadata;
  }

(** 式を最適化して結果を取得 *)
let optimize_expr (e : expr) : expr * dce_stats =
  let fn = make_test_function e in
  let optimized, stats = optimize_function fn in
  match optimized.fn_blocks with
  | [ { terminator = TermReturn result; _ } ] -> (result, stats)
  | _ -> failwith "Unexpected optimization result"

(* ========== 未使用変数削除のテスト ========== *)

let test_unused_let_binding () =
  (* let x = 42 in 10 → 10 *)
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let bound = make_int_lit 42L in
  let body = make_int_lit 10L in
  let e = make_expr (Let (var_x, bound, body)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  (* 未使用束縛が削除されることを確認 *)
  assert (stats.removed_bindings = 1);
  match result.expr_kind with
  | Literal (Ast.Int ("10", _)) -> ()
  | _ -> failwith "Expected literal 10"

let test_used_let_binding () =
  (* let x = 42 in x → let x = 42 in x *)
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let bound = make_int_lit 42L in
  let body = make_var_ref var_x in
  let e = make_expr (Let (var_x, bound, body)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  (* 使用されている束縛は削除されないことを確認 *)
  assert (stats.removed_bindings = 0);
  match result.expr_kind with
  | Let _ -> ()
  | _ -> failwith "Expected Let expression"

let test_nested_unused_bindings () =
  (* let x = 10 in let y = 20 in 30 → 30 *)
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let var_y = VarIdGen.fresh "y" ty_i64 dummy_span in
  let bound_x = make_int_lit 10L in
  let bound_y = make_int_lit 20L in
  let body_inner = make_int_lit 30L in
  let body_outer =
    make_expr (Let (var_y, bound_y, body_inner)) ty_i64 dummy_span
  in
  let e = make_expr (Let (var_x, bound_x, body_outer)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  (* 2つの未使用束縛が削除されることを確認 *)
  assert (stats.removed_bindings = 2);
  match result.expr_kind with
  | Literal (Ast.Int ("30", _)) -> ()
  | _ -> failwith "Expected literal 30"

let test_partially_used_bindings () =
  (* let x = 10 in let y = 20 in x → let x = 10 in x *)
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let var_y = VarIdGen.fresh "y" ty_i64 dummy_span in
  let bound_x = make_int_lit 10L in
  let bound_y = make_int_lit 20L in
  let body_inner = make_var_ref var_x in
  let body_outer =
    make_expr (Let (var_y, bound_y, body_inner)) ty_i64 dummy_span
  in
  let e = make_expr (Let (var_x, bound_x, body_outer)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  (* y のみ削除、x は保持 *)
  assert (stats.removed_bindings = 1);
  match result.expr_kind with
  | Let (x, _, body) -> (
      assert (x.vid = var_x.vid);
      match body.expr_kind with
      | Var v -> assert (v.vid = var_x.vid)
      | _ -> failwith "Expected variable reference")
  | _ -> failwith "Expected Let expression"

(* ========== 副作用保存のテスト ========== *)

let test_side_effect_preservation () =
  (* let x = (関数呼び出し) in 10 → let x = (関数呼び出し) in 10 *)
  (* Phase 1 では関数呼び出しは副作用を持つと仮定するため、削除されない *)
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let fn_expr =
    make_var_ref (VarIdGen.fresh "some_fn" (ty_arrow ty_i64 ty_i64) dummy_span)
  in
  let bound =
    make_expr (App (fn_expr, [ make_int_lit 42L ])) ty_i64 dummy_span
  in
  let body = make_int_lit 10L in
  let e = make_expr (Let (var_x, bound, body)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  (* 副作用を持つ束縛は削除されないことを確認 *)
  assert (stats.removed_bindings = 0);
  match result.expr_kind with
  | Let _ -> ()
  | _ -> failwith "Expected Let expression with side effect"

let test_pure_expression_removal () =
  (* let x = 10 + 20 in 5 → 5 *)
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let bound = make_prim PrimAdd [ make_int_lit 10L; make_int_lit 20L ] ty_i64 in
  let body = make_int_lit 5L in
  let e = make_expr (Let (var_x, bound, body)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  (* 純粋な式の未使用束縛は削除される *)
  assert (stats.removed_bindings = 1);
  match result.expr_kind with
  | Literal (Ast.Int ("5", _)) -> ()
  | _ -> failwith "Expected literal 5"

(* ========== If式の最適化 ========== *)

let test_if_with_unused_branch () =
  (* if cond then 10 else 20 は最適化されない（定数畳み込みで処理） *)
  let cond = make_bool_lit true in
  let then_e = make_int_lit 10L in
  let else_e = make_int_lit 20L in
  let e = make_expr (If (cond, then_e, else_e)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  (* DCE は分岐削除を行わない（定数畳み込みが担当） *)
  assert (stats.removed_bindings = 0);
  match result.expr_kind with
  | If _ -> ()
  | _ -> () (* 定数畳み込みと組み合わせた場合は Literal になる可能性もある *)

(* ========== 複合式のテスト ========== *)

let test_complex_expression () =
  (* let a = 1 in let b = 2 in let c = 3 in a + b → let a = 1 in let b = 2 in a + b *)
  let var_a = VarIdGen.fresh "a" ty_i64 dummy_span in
  let var_b = VarIdGen.fresh "b" ty_i64 dummy_span in
  let var_c = VarIdGen.fresh "c" ty_i64 dummy_span in
  let bound_a = make_int_lit 1L in
  let bound_b = make_int_lit 2L in
  let bound_c = make_int_lit 3L in
  let var_a_ref = make_var_ref var_a in
  let var_b_ref = make_var_ref var_b in
  let body_inner = make_prim PrimAdd [ var_a_ref; var_b_ref ] ty_i64 in
  let body_c = make_expr (Let (var_c, bound_c, body_inner)) ty_i64 dummy_span in
  let body_b = make_expr (Let (var_b, bound_b, body_c)) ty_i64 dummy_span in
  let e = make_expr (Let (var_a, bound_a, body_b)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  (* c のみ削除、a と b は使用されているため保持 *)
  assert (stats.removed_bindings = 1);
  match result.expr_kind with
  | Let (a, _, body_b_result) -> (
      assert (a.vid = var_a.vid);
      match body_b_result.expr_kind with
      | Let (b, _, body_inner_result) -> (
          assert (b.vid = var_b.vid);
          match body_inner_result.expr_kind with
          | Primitive (PrimAdd, _) -> ()
          | _ -> failwith "Expected PrimAdd in innermost body")
      | _ -> failwith "Expected inner Let expression")
  | _ -> failwith "Expected outer Let expression"

(* ========== 統計情報のテスト ========== *)

let test_statistics () =
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let var_y = VarIdGen.fresh "y" ty_i64 dummy_span in
  let bound_x = make_int_lit 10L in
  let bound_y = make_int_lit 20L in
  let body_inner = make_int_lit 30L in
  let body_outer =
    make_expr (Let (var_y, bound_y, body_inner)) ty_i64 dummy_span
  in
  let e = make_expr (Let (var_x, bound_x, body_outer)) ty_i64 dummy_span in
  let _, stats = optimize_expr e in
  Printf.printf "Removed bindings: %d\n" stats.removed_bindings;
  Printf.printf "Removed blocks: %d\n" stats.removed_blocks;
  Printf.printf "Removed stmts: %d\n" stats.removed_stmts;
  assert (stats.removed_bindings = 2)

(* ========== テストランナー ========== *)

let tests =
  [
    ("unused_let_binding", test_unused_let_binding);
    ("used_let_binding", test_used_let_binding);
    ("nested_unused_bindings", test_nested_unused_bindings);
    ("partially_used_bindings", test_partially_used_bindings);
    ("side_effect_preservation", test_side_effect_preservation);
    ("pure_expression_removal", test_pure_expression_removal);
    ("if_with_unused_branch", test_if_with_unused_branch);
    ("complex_expression", test_complex_expression);
    ("statistics", test_statistics);
  ]

let run_tests () =
  let passed = ref 0 in
  let failed = ref 0 in
  List.iter
    (fun (name, test) ->
      try
        test ();
        Printf.printf "✓ %s\n" name;
        incr passed
      with e ->
        Printf.printf "✗ %s: %s\n" name (Printexc.to_string e);
        incr failed)
    tests;
  Printf.printf "\nDCE Tests: %d/%d passed\n" !passed (List.length tests);
  if !failed > 0 then exit 1

let () = run_tests ()
