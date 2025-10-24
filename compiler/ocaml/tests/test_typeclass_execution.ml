(* test_typeclass_execution.ml - 型クラス実行テスト
 *
 * Phase 2 Week 22-23 で実装された型クラス辞書渡し機構の
 * 実行レベルでの統合テスト。
 *
 * 検証項目:
 * 1. LLVM IR から実行可能バイナリへのコンパイル
 * 2. ランタイムライブラリとのリンク
 * 3. ビルトインメソッドの実行（__Eq_i64_eq等）
 * 4. プログラムの実行結果検証
 *
 * 注意: Phase 2時点では完全な型クラス実装は未完了のため、
 * 本テストではビルトインメソッド生成の検証に焦点を当てる。
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
            Codegen.codegen_module optimized_ir
          with e ->
            Printf.eprintf "Codegen error: %s\n" (Printexc.to_string e);
            Printexc.print_backtrace stderr;
            failwith "Codegen error"))

(* ========== 外部ツール実行 ========== *)

(* コマンド実行ヘルパー *)
let run_command cmd =
  let exit_code = Sys.command cmd in
  if exit_code <> 0 then
    failwith
      (Printf.sprintf "Command failed with exit code %d: %s" exit_code cmd)

(* 一時ファイル管理 *)
let with_temp_file suffix f =
  let filename = Filename.temp_file "reml_test_" suffix in
  Fun.protect ~finally:(fun () -> Sys.remove filename) (fun () -> f filename)

let llc_path = Llvm_toolchain_helpers.llc ()

(* ========== 検証関数 ========== *)

(* LLVM IRが検証を通過することを確認 *)
let test_llvm_ir_validation () =
  let llvm_module = generate_ir () in

  (* LLVM モジュール検証 *)
  match Verify.verify_llvm_ir llvm_module with
  | Ok () -> ()
  | Error err ->
      failwith (Printf.sprintf "LLVM IR検証失敗: %s" (Verify.string_of_error err))

(* LLVM IRからビットコードへの変換を確認 *)
let test_ir_to_bitcode () =
  let llvm_module = generate_ir () in

  with_temp_file ".bc" (fun bc_file ->
      (* ビットコードの書き出し *)
      let success = Llvm_bitwriter.write_bitcode_file llvm_module bc_file in
      if not success then failwith "ビットコードの書き出しに失敗";

      (* ファイルが生成されたことを確認 *)
      if not (Sys.file_exists bc_file) then failwith "ビットコードファイルが生成されていない")

(* LLVM IRからオブジェクトファイルへのコンパイルを確認 *)
let test_ir_to_object () =
  let llvm_module = generate_ir () in

  with_temp_file ".ll" (fun ll_file ->
      with_temp_file ".o" (fun obj_file ->
          (* LLVM IRをファイルに書き出し *)
          let oc = open_out ll_file in
          output_string oc (Llvm.string_of_llmodule llvm_module);
          close_out oc;

          (* llc でオブジェクトファイルにコンパイル *)
          let llc_cmd =
            Printf.sprintf "%s -filetype=obj -o %s %s 2>&1"
              (Filename.quote llc_path) (Filename.quote obj_file)
              (Filename.quote ll_file)
          in

          (try run_command llc_cmd
           with Failure msg ->
             Printf.eprintf "llc command failed: %s\n" msg;
             failwith "llc compilation failed");

          (* オブジェクトファイルが生成されたことを確認 *)
          if not (Sys.file_exists obj_file) then failwith "オブジェクトファイルが生成されていない"))

(* ビルトインメソッド関数のシンボル存在確認 *)
let test_builtin_symbols_exist () =
  let llvm_module = generate_ir () in
  let ir_string = Llvm.string_of_llmodule llvm_module in

  (* ビルトインメソッドのシンボルがIRに含まれていることを確認 *)
  let builtin_methods =
    [
      "__Eq_i64_eq";
      "__Eq_String_eq";
      "__Eq_Bool_eq";
      "__Ord_i64_compare";
      "__Ord_String_compare";
    ]
  in

  List.iter
    (fun method_name ->
      let pattern = "@" ^ method_name in
      try
        let _ = Str.search_forward (Str.regexp_string pattern) ir_string 0 in
        ()
      with Not_found ->
        failwith (Printf.sprintf "ビルトインメソッド %s が見つかりません" method_name))
    builtin_methods

(* ========== メインテストランナー ========== *)

let () =
  Printexc.record_backtrace true;

  Printf.printf "\n型クラス実行テスト\n";
  Printf.printf "================\n\n";

  (* ソースファイルの存在確認 *)
  if not (Sys.file_exists source_file) then (
    Printf.eprintf "エラー: ソースファイルが見つかりません: %s\n" source_file;
    exit 1);

  Printf.printf "--- LLVM IR検証テスト ---\n";
  run_test "test_llvm_ir_validation" test_llvm_ir_validation;

  Printf.printf "\n--- ビルトインシンボルテスト ---\n";
  run_test "test_builtin_symbols_exist" test_builtin_symbols_exist;

  Printf.printf "\n--- コンパイルパイプラインテスト ---\n";
  run_test "test_ir_to_bitcode" test_ir_to_bitcode;
  run_test "test_ir_to_object" test_ir_to_object;

  Printf.printf "\n================\n";
  if !success_count = !test_count then
    Printf.printf "✓ 全 %d 件のテストが成功しました！\n\n" !test_count
  else (
    Printf.printf "✗ %d/%d 件のテストが失敗しました\n\n"
      (!test_count - !success_count)
      !test_count;
    exit 1)
