(* LLVM IR 検証パイプラインのテスト (Phase 3 Week 15-16)
 *
 * このファイルは LLVM IR 検証機能をテストする。
 *
 * テスト戦略:
 * - 正常ケース: 基本式・関数呼び出し・条件分岐が検証成功
 * - エラーケース: 型不整合・未定義シンボル・無効終端命令
 * - 境界値: 空関数・大きなブロック・ネスト深い制御フロー
 *
 * 参考:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §6
 *)

open Core_ir.Ir
open Types

(* ========== テストヘルパー ========== *)

let dummy_span = {
  Ast.file = "<test>";
  start_line = 1;
  start_col = 1;
  end_line = 1;
  end_col = 1;
}

(** 基本型 *)
let ty_i64 = TInt I64
let ty_bool = TBool
let ty_unit = TUnit

(** テストカウンタ *)
let test_count = ref 0
let passed_count = ref 0

(** テスト実行 *)
let run_test name f =
  incr test_count;
  Printf.printf "  [%d] %s ... " !test_count name;
  flush stdout;
  try
    f ();
    incr passed_count;
    print_endline "✓";
  with e ->
    Printf.printf "✗\n    エラー: %s\n" (Printexc.to_string e)

(** LLVM モジュールから検証を実行 *)
let verify_module llmodule =
  match Llvm_gen.Verify.verify_llvm_ir llmodule with
  | Ok () -> ()
  | Error err ->
      let msg = Llvm_gen.Verify.string_of_error err in
      failwith msg

(* ========== 正常ケーステスト ========== *)

(** テスト1: 基本的な関数（引数なし・戻り値i64） *)
let test_basic_function () =
  let ctx = Codegen.create_codegen_context "test_basic" () in
  Codegen.declare_runtime_functions ctx;

  (* fn test_fn() -> i64 { return 42; } *)
  let fn_def = {
    name = "test_fn";
    params = [];
    return_ty = ty_i64;
    body = [
      {
        label = "entry";
        stmts = [];
        terminator = TermReturn (Some (make_expr (Literal (Int 42L)) ty_i64 dummy_span));
        span = dummy_span;
      }
    ];
    metadata = { span = dummy_span; effects = []; capabilities = [] };
  } in

  let _llvm_fn = Codegen.codegen_function_decl ctx fn_def in
  Codegen.codegen_blocks ctx _llvm_fn fn_def.body;

  verify_module (Codegen.get_llmodule ctx)

(** テスト2: 関数呼び出し *)
let test_function_call () =
  let ctx = Codegen.create_codegen_context "test_call" () in
  Codegen.declare_runtime_functions ctx;

  (* fn callee(x: i64) -> i64 { return x; } *)
  let param_var = VarId "x" in
  let callee_def = {
    name = "callee";
    params = [(param_var, ty_i64)];
    return_ty = ty_i64;
    body = [
      {
        label = "entry";
        stmts = [];
        terminator = TermReturn (Some (make_expr (Var param_var) ty_i64 dummy_span));
        span = dummy_span;
      }
    ];
    metadata = { span = dummy_span; effects = []; capabilities = [] };
  } in

  let _llvm_callee = Codegen.codegen_function_decl ctx callee_def in
  Codegen.codegen_blocks ctx _llvm_callee callee_def.body;

  (* fn caller() -> i64 { return callee(10); } *)
  let caller_def = {
    name = "caller";
    params = [];
    return_ty = ty_i64;
    body = [
      {
        label = "entry";
        stmts = [];
        terminator = TermReturn (Some (
          make_expr (App (
            make_expr (Var (VarId "callee")) (TArrow ([ty_i64], ty_i64)) dummy_span,
            [make_expr (Literal (Int 10L)) ty_i64 dummy_span]
          )) ty_i64 dummy_span
        ));
        span = dummy_span;
      }
    ];
    metadata = { span = dummy_span; effects = []; capabilities = [] };
  } in

  let _llvm_caller = Codegen.codegen_function_decl ctx caller_def in
  Codegen.codegen_blocks ctx _llvm_caller caller_def.body;

  verify_module (Codegen.get_llmodule ctx)

(** テスト3: 条件分岐（if式） *)
let test_conditional_branch () =
  let ctx = Codegen.create_codegen_context "test_if" () in
  Codegen.declare_runtime_functions ctx;

  (* fn test_if(cond: bool) -> i64 {
       if cond then 1 else 2
     } *)
  let param_var = VarId "cond" in
  let fn_def = {
    name = "test_if";
    params = [(param_var, ty_bool)];
    return_ty = ty_i64;
    body = [
      {
        label = "entry";
        stmts = [];
        terminator = TermBranch (
          make_expr (Var param_var) ty_bool dummy_span,
          "then_branch",
          "else_branch"
        );
        span = dummy_span;
      };
      {
        label = "then_branch";
        stmts = [];
        terminator = TermJump "merge";
        span = dummy_span;
      };
      {
        label = "else_branch";
        stmts = [];
        terminator = TermJump "merge";
        span = dummy_span;
      };
      {
        label = "merge";
        stmts = [
          Phi (VarId "result", ty_i64, [
            (make_expr (Literal (Int 1L)) ty_i64 dummy_span, "then_branch");
            (make_expr (Literal (Int 2L)) ty_i64 dummy_span, "else_branch");
          ])
        ];
        terminator = TermReturn (Some (make_expr (Var (VarId "result")) ty_i64 dummy_span));
        span = dummy_span;
      };
    ];
    metadata = { span = dummy_span; effects = []; capabilities = [] };
  } in

  let _llvm_fn = Codegen.codegen_function_decl ctx fn_def in
  Codegen.codegen_blocks ctx _llvm_fn fn_def.body;

  verify_module (Codegen.get_llmodule ctx)

(** テスト4: 算術演算 *)
let test_arithmetic_operations () =
  let ctx = Codegen.create_codegen_context "test_arith" () in
  Codegen.declare_runtime_functions ctx;

  (* fn test_arith(a: i64, b: i64) -> i64 {
       return (a + b) * 2;
     } *)
  let a_var = VarId "a" in
  let b_var = VarId "b" in
  let fn_def = {
    name = "test_arith";
    params = [(a_var, ty_i64); (b_var, ty_i64)];
    return_ty = ty_i64;
    body = [
      {
        label = "entry";
        stmts = [];
        terminator = TermReturn (Some (
          make_expr (Primitive (AddInt, [
            make_expr (Var a_var) ty_i64 dummy_span;
            make_expr (Var b_var) ty_i64 dummy_span;
          ])) ty_i64 dummy_span
        ));
        span = dummy_span;
      }
    ];
    metadata = { span = dummy_span; effects = []; capabilities = [] };
  } in

  let _llvm_fn = Codegen.codegen_function_decl ctx fn_def in
  Codegen.codegen_blocks ctx _llvm_fn fn_def.body;

  verify_module (Codegen.get_llmodule ctx)

(** テスト5: 空関数（境界値） *)
let test_empty_function () =
  let ctx = Codegen.create_codegen_context "test_empty" () in
  Codegen.declare_runtime_functions ctx;

  (* fn empty() -> () { } *)
  let fn_def = {
    name = "empty";
    params = [];
    return_ty = ty_unit;
    body = [
      {
        label = "entry";
        stmts = [];
        terminator = TermReturn None;
        span = dummy_span;
      }
    ];
    metadata = { span = dummy_span; effects = []; capabilities = [] };
  } in

  let _llvm_fn = Codegen.codegen_function_decl ctx fn_def in
  Codegen.codegen_blocks ctx _llvm_fn fn_def.body;

  verify_module (Codegen.get_llmodule ctx)

(** テスト6: 複数ブロック（CFG検証） *)
let test_multiple_blocks () =
  let ctx = Codegen.create_codegen_context "test_cfg" () in
  Codegen.declare_runtime_functions ctx;

  (* fn test_cfg(x: i64) -> i64 {
       if x > 0 then {
         if x > 10 then 100 else 10
       } else {
         0
       }
     } *)
  let x_var = VarId "x" in
  let fn_def = {
    name = "test_cfg";
    params = [(x_var, ty_i64)];
    return_ty = ty_i64;
    body = [
      {
        label = "entry";
        stmts = [];
        terminator = TermBranch (
          make_expr (Primitive (GtInt, [
            make_expr (Var x_var) ty_i64 dummy_span;
            make_expr (Literal (Int 0L)) ty_i64 dummy_span;
          ])) ty_bool dummy_span,
          "then1",
          "else1"
        );
        span = dummy_span;
      };
      {
        label = "then1";
        stmts = [];
        terminator = TermBranch (
          make_expr (Primitive (GtInt, [
            make_expr (Var x_var) ty_i64 dummy_span;
            make_expr (Literal (Int 10L)) ty_i64 dummy_span;
          ])) ty_bool dummy_span,
          "then2",
          "else2"
        );
        span = dummy_span;
      };
      {
        label = "then2";
        stmts = [];
        terminator = TermJump "merge1";
        span = dummy_span;
      };
      {
        label = "else2";
        stmts = [];
        terminator = TermJump "merge1";
        span = dummy_span;
      };
      {
        label = "merge1";
        stmts = [
          Phi (VarId "inner_result", ty_i64, [
            (make_expr (Literal (Int 100L)) ty_i64 dummy_span, "then2");
            (make_expr (Literal (Int 10L)) ty_i64 dummy_span, "else2");
          ])
        ];
        terminator = TermJump "merge2";
        span = dummy_span;
      };
      {
        label = "else1";
        stmts = [];
        terminator = TermJump "merge2";
        span = dummy_span;
      };
      {
        label = "merge2";
        stmts = [
          Phi (VarId "result", ty_i64, [
            (make_expr (Var (VarId "inner_result")) ty_i64 dummy_span, "merge1");
            (make_expr (Literal (Int 0L)) ty_i64 dummy_span, "else1");
          ])
        ];
        terminator = TermReturn (Some (make_expr (Var (VarId "result")) ty_i64 dummy_span));
        span = dummy_span;
      };
    ];
    metadata = { span = dummy_span; effects = []; capabilities = [] };
  } in

  let _llvm_fn = Codegen.codegen_function_decl ctx fn_def in
  Codegen.codegen_blocks ctx _llvm_fn fn_def.body;

  verify_module (Codegen.get_llmodule ctx)

(** テスト7: let 束縛 *)
let test_let_binding () =
  let ctx = Codegen.create_codegen_context "test_let" () in
  Codegen.declare_runtime_functions ctx;

  (* fn test_let() -> i64 {
       let x = 10;
       let y = x + 5;
       return y;
     } *)
  let fn_def = {
    name = "test_let";
    params = [];
    return_ty = ty_i64;
    body = [
      {
        label = "entry";
        stmts = [
          Assign (VarId "x", make_expr (Literal (Int 10L)) ty_i64 dummy_span);
          Assign (VarId "y", make_expr (Primitive (AddInt, [
            make_expr (Var (VarId "x")) ty_i64 dummy_span;
            make_expr (Literal (Int 5L)) ty_i64 dummy_span;
          ])) ty_i64 dummy_span);
        ];
        terminator = TermReturn (Some (make_expr (Var (VarId "y")) ty_i64 dummy_span));
        span = dummy_span;
      }
    ];
    metadata = { span = dummy_span; effects = []; capabilities = [] };
  } in

  let _llvm_fn = Codegen.codegen_function_decl ctx fn_def in
  Codegen.codegen_blocks ctx _llvm_fn fn_def.body;

  verify_module (Codegen.get_llmodule ctx)

(* ========== エラーケーステスト ========== *)

(* Note: エラーケースのテストは、意図的に無効なLLVM IRを生成する必要があるが、
 * 現在のCodegenモジュールは常に有効なIRを生成するため、エラーケーステストは
 * ファイルベースでの検証（手動作成した無効IR）にて実施する。
 *)

(* ========== テストスイート実行 ========== *)

let () =
  print_endline "========================================";
  print_endline "LLVM IR 検証パイプライン テスト";
  print_endline "========================================";
  print_endline "";

  print_endline "正常ケーステスト:";
  run_test "基本的な関数" test_basic_function;
  run_test "関数呼び出し" test_function_call;
  run_test "条件分岐（if式）" test_conditional_branch;
  run_test "算術演算" test_arithmetic_operations;
  run_test "空関数（境界値）" test_empty_function;
  run_test "複数ブロック（CFG検証）" test_multiple_blocks;
  run_test "let 束縛" test_let_binding;

  print_endline "";
  print_endline "========================================";
  Printf.printf "結果: %d/%d テスト成功\n" !passed_count !test_count;
  print_endline "========================================";

  if !passed_count <> !test_count then
    exit 1
