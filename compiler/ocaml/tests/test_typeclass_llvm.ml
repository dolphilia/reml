(* test_typeclass_llvm.ml - 型クラス辞書生成のLLVM IR検証テスト
 *
 * Phase 2 Week 22-23 で実装された型クラス辞書渡し機構の
 * LLVM IRレベルでの統合テスト。
 *
 * 検証項目:
 * 1. ビルトインメソッド関数（__Eq_i64_eq等）が定義されていること
 * 2. 辞書構造体型が生成されていること（今後の拡張）
 * 3. vtableからのメソッド呼び出しが間接呼び出しになること（今後の拡張）
 *)

(* ========== パス設定 ========== *)

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path
let integration_dir = resolve "tests/integration"
let source_file = Filename.concat integration_dir "test_typeclass_e2e.reml"

(* ========== テストヘルパー ========== *)

let test_count = ref 0
let success_count = ref 0

let run_test name f =
  incr test_count;
  try
    f ();
    incr success_count;
    Printf.printf "  ✓ %s\n" name
  with e ->
    Printf.printf "  ✗ %s: %s\n" name (Printexc.to_string e);
    Printexc.print_backtrace stderr

(* ========== IR生成 ========== *)

(* サンプルRemlファイルからLLVM IRを生成 *)
let generate_ir () =
  let ic = open_in source_file in
  let source = really_input_string ic (in_channel_length ic) in
  close_in ic;

  let lexbuf = Lexing.from_string source in
  lexbuf.Lexing.lex_curr_p <-
    { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = source_file };

  match Parser_driver.parse lexbuf with
  | Error diag ->
      Printf.eprintf "Parse error in %s:\n%s\n" source_file
        (Diagnostic.to_string diag);
      failwith "Parse error"
  | Ok ast -> (
      match Type_inference.infer_compilation_unit ast with
      | Error type_err ->
          let diag =
            Type_error.to_diagnostic_with_source source source_file type_err
          in
          Printf.eprintf "Type error in %s:\n%s\n" source_file
            (Diagnostic.to_string diag);
          failwith "Type error"
      | Ok tast -> (
          try
            (* Typed AST → Core IR *)
            let core_ir = Core_ir.Desugar.desugar_compilation_unit tast in

            (* Core IR 最適化 (O1) *)
            let opt_config =
              Core_ir.Pipeline.
                {
                  opt_level = O1;
                  enable_const_fold = true;
                  enable_dce = true;
                  max_iterations = 10;
                  verbose = false;
                  emit_intermediate = false;
                }
            in
            let optimized_ir, _stats =
              Core_ir.Pipeline.optimize_module ~config:opt_config core_ir
            in

            (* Core IR → LLVM IR *)
            let llvm_module = Codegen.codegen_module optimized_ir in
            Llvm.string_of_llmodule llvm_module
          with e ->
            Printf.eprintf "Codegen error: %s\n" (Printexc.to_string e);
            Printexc.print_backtrace stderr;
            failwith "Codegen error"))

(* ========== 検証関数 ========== *)

(* LLVM IRに特定の関数宣言が含まれているかチェック *)
let assert_contains ir substr =
  try
    let _ = Str.search_forward (Str.regexp_string substr) ir 0 in
    ()
  with Not_found -> failwith (Printf.sprintf "LLVM IRに '%s' が含まれていません" substr)

(* ビルトインメソッド関数が定義されていることを確認 *)
let test_builtin_methods_defined () =
  let ir = generate_ir () in

  (* Eq<i64> メソッド *)
  assert_contains ir "__Eq_i64_eq";

  (* Eq<String> メソッド *)
  assert_contains ir "__Eq_String_eq";

  (* Eq<Bool> メソッド *)
  assert_contains ir "__Eq_Bool_eq";

  (* Ord<i64> メソッド *)
  assert_contains ir "__Ord_i64_compare";

  (* Ord<String> メソッド *)
  assert_contains ir "__Ord_String_compare"

(* ランタイム文字列比較関数が宣言されていることを確認 *)
let test_runtime_string_functions () =
  let ir = generate_ir () in

  (* string_eq 関数宣言 *)
  assert_contains ir "declare i32 @string_eq";

  (* string_compare 関数宣言 *)
  assert_contains ir "declare i32 @string_compare"

(* メイン関数が生成されていることを確認 *)
let test_main_function_exists () =
  let ir = generate_ir () in

  (* main関数の定義 *)
  assert_contains ir "define i64 @main"

(* ビルトインメソッドの関数シグネチャが正しいことを確認 *)
let test_builtin_method_signatures () =
  let ir = generate_ir () in

  (* __Eq_i64_eq: (i64, i64) -> Bool *)
  assert_contains ir "define i1 @__Eq_i64_eq(i64";

  (* __Ord_i64_compare: (i64, i64) -> i32 *)
  assert_contains ir "define i32 @__Ord_i64_compare(i64"

(* ========== メインテストランナー ========== *)

let () =
  Printexc.record_backtrace true;

  Printf.printf "\n型クラスLLVM IR検証テスト\n";
  Printf.printf "=======================\n\n";

  (* ソースファイルの存在確認 *)
  if not (Sys.file_exists source_file) then (
    Printf.eprintf "エラー: ソースファイルが見つかりません: %s\n" source_file;
    exit 1);

  Printf.printf "--- ビルトインメソッド生成テスト ---\n";
  run_test "test_builtin_methods_defined" test_builtin_methods_defined;
  run_test "test_builtin_method_signatures" test_builtin_method_signatures;

  Printf.printf "\n--- ランタイム関数宣言テスト ---\n";
  run_test "test_runtime_string_functions" test_runtime_string_functions;

  Printf.printf "\n--- コード生成基本テスト ---\n";
  run_test "test_main_function_exists" test_main_function_exists;

  Printf.printf "\n=======================\n";
  if !success_count = !test_count then
    Printf.printf "✓ 全 %d 件のテストが成功しました！\n\n" !test_count
  else (
    Printf.printf "✗ %d/%d 件のテストが失敗しました\n\n"
      (!test_count - !success_count)
      !test_count;
    exit 1)
