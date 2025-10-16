(* Test_core_ir — Unit tests for Core IR (Phase 3)
 *
 * Core IR のデータ構造と Pretty Printer の基本テスト。
 *)

module IR = Core_ir.Ir
module Printer = Core_ir.Ir_printer
open Types

(* ========== テストユーティリティ ========== *)

let dummy_span = { Ast.start = 0; Ast.end_ = 0 }

(* ========== 変数ID生成のテスト ========== *)

let test_var_id_generation () =
  IR.VarIdGen.reset ();
  let v1 = IR.VarIdGen.fresh "x" ty_i64 dummy_span in
  let v2 = IR.VarIdGen.fresh "y" ty_bool dummy_span in
  let v3 = IR.VarIdGen.fresh "x" ty_i64 dummy_span in

  assert (v1.IR.vid = 0);
  assert (v2.IR.vid = 1);
  assert (v3.IR.vid = 2);
  assert (v1.IR.vname = "x");
  assert (v2.IR.vname = "y");
  assert (v3.IR.vname = "x");
  assert (type_equal v1.IR.vty ty_i64);
  assert (type_equal v2.IR.vty ty_bool);
  print_endline "✓ test_var_id_generation passed"

(* ========== ラベル生成のテスト ========== *)

let test_label_generation () =
  IR.LabelGen.reset ();
  let l1 = IR.LabelGen.fresh "entry" in
  let l2 = IR.LabelGen.fresh "loop" in
  let l3 = IR.LabelGen.fresh "exit" in

  assert (l1 = "entry_0");
  assert (l2 = "loop_1");
  assert (l3 = "exit_2");
  print_endline "✓ test_label_generation passed"

(* ========== 基本式の構築テスト ========== *)

let test_literal_expr () =
  let lit = Ast.Int ("42", Ast.Base10) in
  let expr = IR.make_expr (IR.Literal lit) ty_i64 dummy_span in

  assert (type_equal expr.IR.expr_ty ty_i64);
  let str = Printer.string_of_expr expr in
  assert (String.length str > 0);
  print_endline "✓ test_literal_expr passed"

let test_var_expr () =
  IR.VarIdGen.reset ();
  let var = IR.VarIdGen.fresh "x" ty_i64 dummy_span in
  let expr = IR.make_expr (IR.Var var) ty_i64 dummy_span in

  assert (type_equal expr.IR.expr_ty ty_i64);
  let str = Printer.string_of_expr expr in
  assert (String.contains str 'x');
  print_endline "✓ test_var_expr passed"

let test_primitive_expr () =
  IR.VarIdGen.reset ();
  let v1 = IR.VarIdGen.fresh "a" ty_i64 dummy_span in
  let v2 = IR.VarIdGen.fresh "b" ty_i64 dummy_span in
  let e1 = IR.make_expr (IR.Var v1) ty_i64 dummy_span in
  let e2 = IR.make_expr (IR.Var v2) ty_i64 dummy_span in
  let expr =
    IR.make_expr (IR.Primitive (IR.PrimAdd, [ e1; e2 ])) ty_i64 dummy_span
  in

  assert (type_equal expr.IR.expr_ty ty_i64);
  let str = Printer.string_of_expr expr in
  assert (String.contains str 'a');
  assert (String.contains str 'b');
  print_endline "✓ test_primitive_expr passed"

(* ========== Let式のテスト ========== *)

let test_let_expr () =
  IR.VarIdGen.reset ();
  let var = IR.VarIdGen.fresh "x" ty_i64 dummy_span in
  let bound =
    IR.make_expr (IR.Literal (Ast.Int ("10", Ast.Base10))) ty_i64 dummy_span
  in
  let body = IR.make_expr (IR.Var var) ty_i64 dummy_span in
  let expr = IR.make_expr (IR.Let (var, bound, body)) ty_i64 dummy_span in

  assert (type_equal expr.IR.expr_ty ty_i64);
  let str = Printer.string_of_expr expr in
  assert (String.contains str 'x');
  print_endline "✓ test_let_expr passed"

(* ========== If式のテスト ========== *)

let test_if_expr () =
  IR.VarIdGen.reset ();
  let cond = IR.make_expr (IR.Literal (Ast.Bool true)) ty_bool dummy_span in
  let then_e =
    IR.make_expr (IR.Literal (Ast.Int ("1", Ast.Base10))) ty_i64 dummy_span
  in
  let else_e =
    IR.make_expr (IR.Literal (Ast.Int ("2", Ast.Base10))) ty_i64 dummy_span
  in
  let expr = IR.make_expr (IR.If (cond, then_e, else_e)) ty_i64 dummy_span in

  assert (type_equal expr.IR.expr_ty ty_i64);
  let str = Printer.string_of_expr expr in
  assert (String.length str > 0);
  (* if式が出力されることを確認 *)
  print_endline "✓ test_if_expr passed"

(* ========== メタデータのテスト ========== *)

let test_metadata () =
  let meta = IR.default_metadata dummy_span in
  assert (meta.IR.opt_flags.IR.allow_dce = true);
  assert (meta.IR.opt_flags.IR.allow_inline = true);
  assert (meta.IR.opt_flags.IR.preserve_for_diagnostics = false);
  assert (List.length meta.IR.effects.declared = 0);
  assert (List.length meta.IR.capabilities.IR.required = 0);
  print_endline "✓ test_metadata passed"

(* ========== Pretty Printer のテスト ========== *)

let test_prim_op_printing () =
  assert (Printer.string_of_prim_op IR.PrimAdd = "add");
  assert (Printer.string_of_prim_op IR.PrimEq = "eq");
  assert (Printer.string_of_prim_op IR.PrimNot = "not");
  print_endline "✓ test_prim_op_printing passed"

(* ========== すべてのテストを実行 ========== *)

let run_all_tests () =
  print_endline "\n=== Running Core IR Tests ===\n";
  test_var_id_generation ();
  test_label_generation ();
  test_literal_expr ();
  test_var_expr ();
  test_primitive_expr ();
  test_let_expr ();
  test_if_expr ();
  test_metadata ();
  test_prim_op_printing ();
  print_endline "\n=== All Core IR Tests Passed ===\n"

let () = run_all_tests ()
