(* test_llvm_array_access.ml — Array access codegen validation
 *
 * Core IR で ArrayAccess ノードを含む関数を生成し、LLVM IR 変換時に
 * FAT pointer から要素をロードするコードが生成されることを確認する。
 *)

open Core_ir.Ir
open Types

let dummy_span = Ast.dummy_span

let build_test_function () =
  VarIdGen.reset ();
  LabelGen.reset ();

  let array_ty = TArray ty_i64 in
  let param_var = VarIdGen.fresh "xs" array_ty dummy_span in
  let index_var = VarIdGen.fresh "index" ty_i64 dummy_span in
  let tmp_var = VarIdGen.fresh "$tmp" ty_i64 dummy_span in

  let array_expr = make_expr (Var param_var) array_ty dummy_span in
  let index_expr = make_expr (Var index_var) ty_i64 dummy_span in
  let access_expr =
    make_expr (ArrayAccess (array_expr, index_expr)) ty_i64 dummy_span
  in

  let assign_stmt = Assign (tmp_var, access_expr) in
  let return_expr = make_expr (Var tmp_var) ty_i64 dummy_span in
  let entry_block =
    make_block
      (LabelGen.fresh "entry")
      []
      [ assign_stmt ]
      (TermReturn return_expr)
      dummy_span
  in
  let metadata = default_metadata dummy_span in

  make_function "array_get"
    [
      { param_var; param_default = None };
      { param_var = index_var; param_default = None };
    ]
    ty_i64 [ entry_block ] metadata

let assert_contains substring text =
  try
    ignore (Str.search_forward (Str.regexp_string substring) text 0)
  with Not_found ->
    Printf.printf "期待した断片 '%s' が LLVM IR に見つかりません。\n" substring;
    Printf.printf "=== LLVM IR ===\n%s\n===\n" text;
    failwith "LLVM IR substring not found"

let test_codegen_array_access () =
  let fn_def = build_test_function () in
  let module_def =
    {
      module_name = "array_access_test";
      type_defs = [];
      global_defs = [];
      function_defs = [ fn_def ];
    }
  in
  let llvm_module = Codegen.codegen_module module_def in
  let ir = Llvm.string_of_llmodule llvm_module in

  (* FAT pointer からのポインタ演算とロードが生成されていることを確認 *)
  assert_contains "ptrtoint" ir;
  assert_contains "mul i64" ir;
  assert_contains "load i64" ir;
  print_endline "✓ array access codegen test passed"

let () =
  print_endline "\n=== LLVM Array Access Tests ===\n";
  test_codegen_array_access ();
  print_endline "\n=== All Array Access Tests Passed ===\n"
