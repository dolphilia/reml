(* Test_const_fold — Constant Folding Pass Tests (Phase 3)
 *
 * このファイルは定数畳み込み最適化パスのテストを提供する。
 * 計画書: docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §4
 *)

open Types
open Ast
open Core_ir.Ir
open Core_ir.Const_fold

(* ========== テストヘルパー ========== *)

let dummy_span = { Ast.start = 0; Ast.end_ = 0 }

(** 整数リテラル式を作成 *)
let make_int_lit (i: int64) : expr =
  make_expr (Literal (Ast.Int (Int64.to_string i, Ast.Base10))) ty_i64 dummy_span

(** ブールリテラル式を作成 *)
let make_bool_lit (b: bool) : expr =
  make_expr (Literal (Ast.Bool b)) ty_bool dummy_span

(** 浮動小数リテラル式を作成 *)
let make_float_lit (f: float) : expr =
  make_expr (Literal (Ast.Float (Float.to_string f))) ty_f64 dummy_span

(* 文字列リテラルは現在未使用 *)
(* let make_string_lit (s: string) : expr =
  make_expr (Literal (Ast.String (s, Ast.Normal))) ty_string dummy_span *)

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
let optimize_expr (e: expr) : expr =
  let fn = make_test_function e in
  let optimized, _ = optimize_function fn in
  match optimized.fn_blocks with
  | [{ terminator = TermReturn result; _ }] -> result
  | _ -> failwith "Unexpected optimization result"

(** リテラル値の比較 *)
let assert_literal_eq (expected: literal) (actual: expr) : unit =
  match actual.expr_kind with
  | Literal lit ->
      begin match expected, lit with
      | Ast.Int (exp_s, exp_base), Ast.Int (act_s, act_base) ->
          (* 数値として比較 *)
          let exp_val = Int64.of_string (if exp_base = Ast.Base10 then exp_s else "0x" ^ String.sub exp_s 2 (String.length exp_s - 2)) in
          let act_val = Int64.of_string (if act_base = Ast.Base10 then act_s else "0x" ^ String.sub act_s 2 (String.length act_s - 2)) in
          if exp_val <> act_val then begin
            Printf.printf "Expected Int %Ld but got Int %Ld\n" exp_val act_val;
            assert false
          end
      | Ast.Float exp_s, Ast.Float act_s ->
          let exp_val = Float.of_string exp_s in
          let act_val = Float.of_string act_s in
          if exp_val <> act_val then begin
            Printf.printf "Expected Float %f but got Float %f\n" exp_val act_val;
            assert false
          end
      | Ast.Bool exp_b, Ast.Bool act_b ->
          if exp_b <> act_b then begin
            Printf.printf "Expected Bool %b but got Bool %b\n" exp_b act_b;
            assert false
          end
      | Ast.String (exp_s, _), Ast.String (act_s, _) ->
          if exp_s <> act_s then begin
            Printf.printf "Expected String %S but got String %S\n" exp_s act_s;
            assert false
          end
      | _ ->
          Printf.printf "Literal type mismatch\n";
          assert false
      end
  | _ ->
      Printf.printf "Expected literal but got non-literal expr\n";
      assert false

(* ========== 算術演算のテスト ========== *)

let test_add_ints () =
  let e = make_prim PrimAdd [make_int_lit 10L; make_int_lit 20L] ty_i64 in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Int ("30", Ast.Base10)) result

let test_sub_ints () =
  let e = make_prim PrimSub [make_int_lit 50L; make_int_lit 20L] ty_i64 in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Int ("30", Ast.Base10)) result

let test_mul_ints () =
  let e = make_prim PrimMul [make_int_lit 5L; make_int_lit 6L] ty_i64 in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Int ("30", Ast.Base10)) result

let test_div_ints () =
  let e = make_prim PrimDiv [make_int_lit 60L; make_int_lit 2L] ty_i64 in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Int ("30", Ast.Base10)) result

let test_mod_ints () =
  let e = make_prim PrimMod [make_int_lit 35L; make_int_lit 10L] ty_i64 in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Int ("5", Ast.Base10)) result

let test_nested_arithmetic () =
  (* (10 + 20) * 3 = 90 *)
  let inner = make_prim PrimAdd [make_int_lit 10L; make_int_lit 20L] ty_i64 in
  let outer = make_prim PrimMul [inner; make_int_lit 3L] ty_i64 in
  let result = optimize_expr outer in
  assert_literal_eq (Ast.Int ("90", Ast.Base10)) result

(* ========== 浮動小数演算のテスト ========== *)

let test_add_floats () =
  let e = make_prim PrimAdd [make_float_lit 1.5; make_float_lit 2.5] ty_f64 in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Float "4.") result

let test_mul_floats () =
  let e = make_prim PrimMul [make_float_lit 2.5; make_float_lit 4.0] ty_f64 in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Float "10.") result

(* ========== 比較演算のテスト ========== *)

let test_eq_ints () =
  let e = make_prim PrimEq [make_int_lit 42L; make_int_lit 42L] ty_bool in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Bool true) result

let test_ne_ints () =
  let e = make_prim PrimNe [make_int_lit 42L; make_int_lit 10L] ty_bool in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Bool true) result

let test_lt_ints () =
  let e = make_prim PrimLt [make_int_lit 10L; make_int_lit 20L] ty_bool in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Bool true) result

let test_le_ints () =
  let e = make_prim PrimLe [make_int_lit 20L; make_int_lit 20L] ty_bool in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Bool true) result

let test_gt_ints () =
  let e = make_prim PrimGt [make_int_lit 30L; make_int_lit 20L] ty_bool in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Bool true) result

let test_ge_ints () =
  let e = make_prim PrimGe [make_int_lit 20L; make_int_lit 20L] ty_bool in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Bool true) result

(* ========== 論理演算のテスト ========== *)

let test_and_bools () =
  let e = make_prim PrimAnd [make_bool_lit true; make_bool_lit false] ty_bool in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Bool false) result

let test_or_bools () =
  let e = make_prim PrimOr [make_bool_lit true; make_bool_lit false] ty_bool in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Bool true) result

let test_not_bool () =
  let e = make_prim PrimNot [make_bool_lit true] ty_bool in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Bool false) result

(* ========== 条件分岐の静的評価 ========== *)

let test_if_true () =
  let cond = make_bool_lit true in
  let then_e = make_int_lit 42L in
  let else_e = make_int_lit 0L in
  let e = make_expr (If (cond, then_e, else_e)) ty_i64 dummy_span in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Int ("42", Ast.Base10)) result

let test_if_false () =
  let cond = make_bool_lit false in
  let then_e = make_int_lit 42L in
  let else_e = make_int_lit 0L in
  let e = make_expr (If (cond, then_e, else_e)) ty_i64 dummy_span in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Int ("0", Ast.Base10)) result

let test_if_constant_condition () =
  (* if (10 > 5) then 100 else 200 → 100 *)
  let cond = make_prim PrimGt [make_int_lit 10L; make_int_lit 5L] ty_bool in
  let then_e = make_int_lit 100L in
  let else_e = make_int_lit 200L in
  let e = make_expr (If (cond, then_e, else_e)) ty_i64 dummy_span in
  let result = optimize_expr e in
  assert_literal_eq (Ast.Int ("100", Ast.Base10)) result

(* ========== 定数伝播のテスト ========== *)

let test_let_constant_propagation () =
  (* let x = 42 in x + 10 → let x = 42 in 52 *)
  let var = VarIdGen.fresh "x" ty_i64 dummy_span in
  let bound = make_int_lit 42L in
  let var_ref = make_expr (Var var) ty_i64 dummy_span in
  let body = make_prim PrimAdd [var_ref; make_int_lit 10L] ty_i64 in
  let e = make_expr (Let (var, bound, body)) ty_i64 dummy_span in
  let result = optimize_expr e in
  (* 結果はLet式のまま。本体が52に畳み込まれていることを確認すべき *)
  match result.expr_kind with
  | Let (_, _, body) ->
      begin match body.expr_kind with
      | Literal (Ast.Int ("52", _)) -> ()
      | _ -> assert false
      end
  | _ -> assert false

let test_nested_let_propagation () =
  (* let x = 10 in let y = 20 in x + y → let x = 10 in let y = 20 in 30 *)
  let var_x = VarIdGen.fresh "x" ty_i64 dummy_span in
  let var_y = VarIdGen.fresh "y" ty_i64 dummy_span in
  let bound_x = make_int_lit 10L in
  let bound_y = make_int_lit 20L in
  let x_ref = make_expr (Var var_x) ty_i64 dummy_span in
  let y_ref = make_expr (Var var_y) ty_i64 dummy_span in
  let body = make_prim PrimAdd [x_ref; y_ref] ty_i64 in
  let inner_let = make_expr (Let (var_y, bound_y, body)) ty_i64 dummy_span in
  let e = make_expr (Let (var_x, bound_x, inner_let)) ty_i64 dummy_span in
  let result = optimize_expr e in
  (* 最も内側の加算が30に畳み込まれていることを確認 *)
  match result.expr_kind with
  | Let (_, _, inner) ->
      begin match inner.expr_kind with
      | Let (_, _, body) ->
          begin match body.expr_kind with
          | Literal (Ast.Int ("30", _)) -> ()
          | _ -> assert false
          end
      | _ -> assert false
      end
  | _ -> assert false

(* ========== エラーケースのテスト ========== *)

let test_division_by_zero () =
  let e = make_prim PrimDiv [make_int_lit 42L; make_int_lit 0L] ty_i64 in
  try
    let _ = optimize_expr e in
    Printf.printf "Expected DivisionByZero error but got success\n";
    assert false
  with FoldError (DivisionByZero _) -> ()

let test_modulo_by_zero () =
  let e = make_prim PrimMod [make_int_lit 42L; make_int_lit 0L] ty_i64 in
  try
    let _ = optimize_expr e in
    Printf.printf "Expected DivisionByZero error but got success\n";
    assert false
  with FoldError (DivisionByZero _) -> ()

(* ========== 複雑な式のテスト ========== *)

let test_complex_expression () =
  (* ((10 + 20) * 2) - (5 * 4) = 60 - 20 = 40 *)
  let add = make_prim PrimAdd [make_int_lit 10L; make_int_lit 20L] ty_i64 in
  let mul1 = make_prim PrimMul [add; make_int_lit 2L] ty_i64 in
  let mul2 = make_prim PrimMul [make_int_lit 5L; make_int_lit 4L] ty_i64 in
  let sub = make_prim PrimSub [mul1; mul2] ty_i64 in
  let result = optimize_expr sub in
  assert_literal_eq (Ast.Int ("40", Ast.Base10)) result

(* ========== 統計情報のテスト ========== *)

let test_statistics () =
  let e = make_prim PrimAdd [make_int_lit 10L; make_int_lit 20L] ty_i64 in
  let fn = make_test_function e in
  let _, stats = optimize_function fn in
  assert (stats.folded_exprs > 0);
  Printf.printf "Folded expressions: %d\n" stats.folded_exprs

(* ========== テストランナー ========== *)

let () =
  (* 変数ID生成器をリセット *)
  VarIdGen.reset ();

  let tests = [
    ("add_ints", test_add_ints);
    ("sub_ints", test_sub_ints);
    ("mul_ints", test_mul_ints);
    ("div_ints", test_div_ints);
    ("mod_ints", test_mod_ints);
    ("nested_arithmetic", test_nested_arithmetic);
    ("add_floats", test_add_floats);
    ("mul_floats", test_mul_floats);
    ("eq_ints", test_eq_ints);
    ("ne_ints", test_ne_ints);
    ("lt_ints", test_lt_ints);
    ("le_ints", test_le_ints);
    ("gt_ints", test_gt_ints);
    ("ge_ints", test_ge_ints);
    ("and_bools", test_and_bools);
    ("or_bools", test_or_bools);
    ("not_bool", test_not_bool);
    ("if_true", test_if_true);
    ("if_false", test_if_false);
    ("if_constant_condition", test_if_constant_condition);
    ("let_constant_propagation", test_let_constant_propagation);
    ("nested_let_propagation", test_nested_let_propagation);
    ("division_by_zero", test_division_by_zero);
    ("modulo_by_zero", test_modulo_by_zero);
    ("complex_expression", test_complex_expression);
    ("statistics", test_statistics);
  ] in

  let total = List.length tests in
  let passed = ref 0 in

  List.iter (fun (name, test_fn) ->
    try
      test_fn ();
      Printf.printf "✓ %s\n" name;
      passed := !passed + 1
    with e ->
      Printf.printf "✗ %s: %s\n" name (Printexc.to_string e)
  ) tests;

  Printf.printf "\nConst Fold Tests: %d/%d passed\n" !passed total;
  if !passed < total then exit 1
