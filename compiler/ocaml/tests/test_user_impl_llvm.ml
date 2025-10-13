(* test_user_impl_llvm.ml - ユーザー定義impl宣言のLLVM IR検証テスト
 *
 * Phase 2 Week 24 で実装されたユーザー定義impl宣言の統合検証。
 *
 * 検証項目:
 * 1. ユーザー定義impl宣言がパースされること
 * 2. 型推論が成功すること
 * 3. Implレジストリに登録されること
 * 4. 制約ソルバーがユーザー定義implを解決すること
 * 5. LLVM IRが生成されること（ビルトイン+ユーザー定義）
 *
 * 注意: Phase 2時点では完全な型クラス実装は未完了のため、
 * 本テストでは基本的なケースの検証に焦点を当てる。
 *)

(* ========== パス設定 ========== *)

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path
let integration_dir = resolve "tests/integration"
let source_file = Filename.concat integration_dir "test_user_impl_e2e.reml"

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

(* ユーザー定義impl宣言テストファイルからLLVM IRを生成 *)
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

(* LLVM IRに特定の文字列が含まれているかチェック *)
let assert_contains ir substr =
  try
    let _ = Str.search_forward (Str.regexp_string substr) ir 0 in
    ()
  with Not_found ->
    failwith (Printf.sprintf "LLVM IRに '%s' が含まれていません" substr)

(* ユーザー定義impl宣言がパースされること *)
let test_user_impl_parsed () =
  (* パースエラーが発生しなければ成功 *)
  let ic = open_in source_file in
  let source = really_input_string ic (in_channel_length ic) in
  close_in ic;

  let lexbuf = Lexing.from_string source in
  lexbuf.Lexing.lex_curr_p <-
    { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = source_file };

  match Parser_driver.parse lexbuf with
  | Error diag ->
      Printf.eprintf "Parse error: %s\n" (Diagnostic.to_string diag);
      failwith "Parse error"
  | Ok ast ->
      (* impl宣言が含まれていることを確認 *)
      let has_impl_decl =
        List.exists
          (fun decl ->
            match decl.Ast.decl_kind with
            | Ast.ImplDecl _ -> true
            | _ -> false)
          ast.Ast.decls
      in
      if not has_impl_decl then failwith "impl宣言が見つかりません"

(* ユーザー定義型が型推論を通過すること *)
let test_user_types_inferred () =
  let ic = open_in source_file in
  let source = really_input_string ic (in_channel_length ic) in
  close_in ic;

  let lexbuf = Lexing.from_string source in
  lexbuf.Lexing.lex_curr_p <-
    { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = source_file };

  match Parser_driver.parse lexbuf with
  | Error diag -> failwith (Diagnostic.to_string diag)
  | Ok ast -> (
      match Type_inference.infer_compilation_unit ast with
      | Error type_err ->
          let diag =
            Type_error.to_diagnostic_with_source source source_file type_err
          in
          Printf.eprintf "Type error: %s\n" (Diagnostic.to_string diag);
          failwith "Type error"
      | Ok _tast -> ())

(* LLVM IRにユーザー定義impl関連の関数が含まれていること *)
let test_user_impl_in_ir () =
  let ir = generate_ir () in

  (* テスト関数が存在することを確認 *)
  assert_contains ir "test_eq_i64";
  assert_contains ir "test_ord_i64"

(* ビルトインメソッドが依然として生成されていること *)
let test_builtin_methods_still_exist () =
  let ir = generate_ir () in

  (* 既存のビルトインメソッドが継続して存在 *)
  assert_contains ir "__Eq_i64_eq";
  assert_contains ir "__Eq_String_eq";
  assert_contains ir "__Eq_Bool_eq"

(* メイン関数が生成されていること *)
let test_main_function_exists () =
  let ir = generate_ir () in
  assert_contains ir "define i64 @main"

(* ========== メインテストランナー ========== *)

let () =
  Printexc.record_backtrace true;

  Printf.printf "\nユーザー定義impl宣言 LLVM IR検証テスト\n";
  Printf.printf "=====================================\n\n";

  (* ソースファイルの存在確認 *)
  if not (Sys.file_exists source_file) then (
    Printf.eprintf "エラー: ソースファイルが見つかりません: %s\n" source_file;
    exit 1);

  Printf.printf "--- impl宣言パースと型推論テスト ---\n";
  run_test "test_user_impl_parsed" test_user_impl_parsed;
  run_test "test_user_types_inferred" test_user_types_inferred;

  Printf.printf "\n--- LLVM IR生成テスト ---\n";
  run_test "test_user_impl_in_ir" test_user_impl_in_ir;
  run_test "test_builtin_methods_still_exist" test_builtin_methods_still_exist;
  run_test "test_main_function_exists" test_main_function_exists;

  Printf.printf "\n=====================================\n";
  if !success_count = !test_count then
    Printf.printf "✓ 全 %d 件のテストが成功しました！\n\n" !test_count
  else (
    Printf.printf "✗ %d/%d 件のテストが失敗しました\n\n"
      (!test_count - !success_count)
      !test_count;
    exit 1)
