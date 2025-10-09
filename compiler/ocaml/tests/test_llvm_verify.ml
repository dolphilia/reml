(* LLVM IR 検証パイプラインのテスト (Phase 3 Week 16)
 *
 * `src/llvm_gen/verify.ml` と `scripts/verify_llvm_ir.sh` の統合を検証する。
 * ここでは最小限の LLVM モジュールを生成して検証パスが成功すること、
 * 無効な IR で期待どおりにエラーが返ることを確認する。
 *)

open Verify

(* ========== テストユーティリティ ========== *)

let test_index = ref 0
let passed = ref 0

let run_test name fn =
  incr test_index;
  Printf.printf "[%02d] %s ... %!" !test_index name;
  try
    fn ();
    incr passed;
    print_endline "成功";
  with exn ->
    Printf.printf "失敗\n    %s\n" (Printexc.to_string exn)

let finish () =
  Printf.printf "\n結果: %d / %d テスト成功\n%!" !passed !test_index;
  if !passed <> !test_index then exit 1

(* ========== ヘルパー ========== *)

let build_constant_module () =
  let ctx = Llvm.global_context () in
  let llmodule = Llvm.create_module ctx "verify_constant" in
  let i64 = Llvm.i64_type ctx in
  let fn_ty = Llvm.function_type i64 [||] in
  let fn = Llvm.define_function "const42" fn_ty llmodule in
  let entry_block = Llvm.append_block ctx "entry" fn in
  let builder = Llvm.builder_at_end ctx entry_block in
  ignore (Llvm.build_ret (Llvm.const_int i64 42) builder);
  llmodule

let with_temp_file contents k =
  let filename = Filename.temp_file "reml_verify_test" ".ll" in
  Fun.protect
    ~finally:(fun () -> Sys.remove filename)
    (fun () ->
       let oc = open_out filename in
       output_string oc contents;
       close_out oc;
       k filename)

(* ========== テストケース ========== *)

let test_verify_success () =
  let llmodule = build_constant_module () in
  match verify_llvm_ir llmodule with
  | Ok () -> ()
  | Error err ->
      failwith (Printf.sprintf "検証が失敗しました: %s" (string_of_error err))

let test_verify_failure () =
  (* 明らかに壊れたIR（関数終端がない）を渡して失敗を確認する *)
  let invalid_ir = {|
; ModuleID = 'broken'
define i64 @broken() {
entry:
  %0 = add i64 1, 2
|} in
  with_temp_file invalid_ir (fun path ->
      match verify_llvm_ir_file path with
      | Error (AssembleError _) -> ()
      | Error err ->
          failwith (Printf.sprintf "期待したAssembleErrorではありません: %s" (string_of_error err))
      | Ok () ->
          failwith "無効なIRが検証を通過しました")

(* ========== エントリポイント ========== *)

let () =
  print_endline "LLVM IR 検証パイプライン テスト";
  run_test "有効なモジュールが検証を通過する" test_verify_success;
  run_test "無効なIRはエラーになる" test_verify_failure;
  finish ()
