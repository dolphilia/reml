(* Test_pipeline — Optimization Pipeline Tests (Phase 3)
 *
 * このファイルは最適化パイプラインのテストを提供する。
 * 計画書: docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §6
 *)

[@@@warning "-33"]  (* unused-open を抑制 *)

open Types
open Ast
open Core_ir.Ir
open Core_ir.Pipeline

(* ========== テストヘルパー ========== *)

let dummy_span = Ast.dummy_span

(** 整数リテラル式を作成 *)
let make_int_lit (i: int64) : expr =
  make_expr (Literal (Ast.Int (Int64.to_string i, Ast.Base10))) ty_i64 dummy_span

(** プリミティブ演算式を作成 *)
let make_prim (op: prim_op) (args: expr list) (ty: ty) : expr =
  make_expr (Primitive (op, args)) ty dummy_span

(** テスト用の簡易関数を作成 *)
let make_test_function (body_expr: expr) : function_def =
  let entry_block = make_block
    "entry"
    []
    []
    (TermReturn body_expr)
    dummy_span
  in
  let metadata = {
    fn_span = dummy_span;
    effects = { declared = []; residual = [] };
    capabilities = { required = []; stage = None };
    dict_instances = [];
    opt_flags = { allow_dce = true; allow_inline = false; preserve_for_diagnostics = false };
  } in
  {
    fn_name = "test_fn";
    fn_params = [];
    fn_return_ty = body_expr.expr_ty;
    fn_blocks = [entry_block];
    fn_metadata = metadata;
  }

(** 式を最適化して結果を取得 *)
let optimize_expr_with_config (config: pipeline_config) (e: expr) : expr * pipeline_stats =
  let fn = make_test_function e in
  let optimized, stats = optimize_function ~config fn in
  match optimized.fn_blocks with
  | [{ terminator = TermReturn result; _ }] -> (result, stats)
  | _ -> failwith "Unexpected optimization result"

let optimize_expr (e: expr) : expr * pipeline_stats =
  optimize_expr_with_config config_o1 e

(* ========== O0 レベルのテスト（最適化なし） ========== *)

let test_o0_no_optimization () =
  (* O0 では最適化が行われないことを確認 *)
  let e = make_prim PrimAdd [make_int_lit 10L; make_int_lit 20L] ty_i64 in
  let result, stats = optimize_expr_with_config config_o0 e in
  assert (stats.iterations = 0);
  assert (stats.total_folded_exprs = 0);
  (* 結果は最適化されていない *)
  match result.expr_kind with
  | Primitive (PrimAdd, _) -> ()
  | _ -> failwith "Expected unoptimized expression"

(* ========== O1 レベルのテスト（基本最適化） ========== *)

let test_o1_const_fold () =
  (* O1 で定数畳み込みが実行されることを確認 *)
  let e = make_prim PrimAdd [make_int_lit 10L; make_int_lit 20L] ty_i64 in
  let result, stats = optimize_expr e in
  assert (stats.iterations > 0);
  assert (stats.total_folded_exprs > 0);
  (* 結果は 30 に畳み込まれている *)
  match result.expr_kind with
  | Literal (Ast.Int ("30", _)) -> ()
  | _ -> failwith "Expected folded constant 30"

let test_o1_dce () =
  (* O1 で死コード削除が実行されることを確認 *)
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let bound = make_int_lit 42L in
  let body = make_int_lit 10L in
  let e = make_expr (Let (var_x, bound, body)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  assert (stats.iterations > 0);
  assert (stats.total_removed_bindings > 0);
  (* 未使用束縛が削除されている *)
  match result.expr_kind with
  | Literal (Ast.Int ("10", _)) -> ()
  | _ -> failwith "Expected dead code eliminated"

let test_o1_combined () =
  (* 定数畳み込み + DCE の組み合わせ *)
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let bound = make_prim PrimAdd [make_int_lit 10L; make_int_lit 20L] ty_i64 in
  let body = make_int_lit 5L in
  let e = make_expr (Let (var_x, bound, body)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  assert (stats.iterations > 0);
  assert (stats.total_folded_exprs > 0);
  assert (stats.total_removed_bindings > 0);
  (* 定数畳み込み後に DCE が実行され、最終的に 5 になる *)
  match result.expr_kind with
  | Literal (Ast.Int ("5", _)) -> ()
  | _ -> failwith "Expected combined optimization result"

(* ========== 不動点反復のテスト ========== *)

let test_fixpoint_iteration () =
  (* 複数回の最適化で収束することを確認 *)
  (* let a = 10 + 20 in let b = a + 5 in let c = b in 100 *)
  let var_a = VarIdGen.fresh "a" ty_i64 dummy_span in
  let var_b = VarIdGen.fresh "b" ty_i64 dummy_span in
  let var_c = VarIdGen.fresh "c" ty_i64 dummy_span in
  let bound_a = make_prim PrimAdd [make_int_lit 10L; make_int_lit 20L] ty_i64 in
  let var_a_ref = make_expr (Var var_a) ty_i64 dummy_span in
  let bound_b = make_prim PrimAdd [var_a_ref; make_int_lit 5L] ty_i64 in
  let var_b_ref = make_expr (Var var_b) ty_i64 dummy_span in
  let bound_c = var_b_ref in
  let body_final = make_int_lit 100L in
  let body_c = make_expr (Let (var_c, bound_c, body_final)) ty_i64 dummy_span in
  let body_b = make_expr (Let (var_b, bound_b, body_c)) ty_i64 dummy_span in
  let e = make_expr (Let (var_a, bound_a, body_b)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  (* 複数回の反復で全て削除され、100 になる *)
  assert (stats.iterations >= 1);
  match result.expr_kind with
  | Literal (Ast.Int ("100", _)) -> ()
  | _ -> failwith "Expected fixpoint result 100"

(* ========== 条件分岐の最適化 ========== *)

let test_if_optimization () =
  (* if (10 > 5) then 100 else 200 → 100 *)
  let cond = make_prim PrimGt [make_int_lit 10L; make_int_lit 5L] ty_bool in
  let then_e = make_int_lit 100L in
  let else_e = make_int_lit 200L in
  let e = make_expr (If (cond, then_e, else_e)) ty_i64 dummy_span in
  let result, stats = optimize_expr e in
  assert (stats.iterations > 0);
  (* 条件が定数畳み込みで true になり、then 分岐のみ残る *)
  match result.expr_kind with
  | Literal (Ast.Int ("100", _)) -> ()
  | _ -> failwith "Expected optimized if result 100"

(* ========== 統計情報のテスト ========== *)

let test_statistics () =
  let e = make_prim PrimAdd [make_int_lit 10L; make_int_lit 20L] ty_i64 in
  let _, stats = optimize_expr e in
  Printf.printf "Iterations: %d\n" stats.iterations;
  Printf.printf "Folded expressions: %d\n" stats.total_folded_exprs;
  Printf.printf "Const fold time: %.6f sec\n" stats.total_const_fold_time;
  Printf.printf "DCE time: %.6f sec\n" stats.total_dce_time;
  assert (stats.iterations > 0);
  assert (stats.total_folded_exprs > 0)

(* ========== テストランナー ========== *)

let tests = [
  ("o0_no_optimization", test_o0_no_optimization);
  ("o1_const_fold", test_o1_const_fold);
  ("o1_dce", test_o1_dce);
  ("o1_combined", test_o1_combined);
  ("fixpoint_iteration", test_fixpoint_iteration);
  ("if_optimization", test_if_optimization);
  ("statistics", test_statistics);
]

let run_tests () =
  let passed = ref 0 in
  let failed = ref 0 in
  List.iter (fun (name, test) ->
    try
      test ();
      Printf.printf "✓ %s\n" name;
      incr passed
    with e ->
      Printf.printf "✗ %s: %s\n" name (Printexc.to_string e);
      incr failed
  ) tests;
  Printf.printf "\nPipeline Tests: %d/%d passed\n" !passed (List.length tests);
  if !failed > 0 then exit 1

let () = run_tests ()
