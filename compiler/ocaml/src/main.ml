(* Main — Reml コンパイラエントリーポイント (Phase 1-3)
 *
 * コマンドライン引数を解析し、パーサー、型推論、LLVM IR生成を実行する。
 * Phase 1 M1: --emit-ast オプション
 * Phase 2 M2: --emit-tast オプション
 * Phase 3 M3: --emit-ir, --verify-ir オプション
 *)

let usage_msg = "remlc-ocaml [options] <file>"
let emit_ast = ref false
let emit_tast = ref false
let emit_ir = ref false
let emit_bc = ref false
let verify_ir = ref false
let link_runtime = ref false
let runtime_path = ref "runtime/native/build/libreml_runtime.a"
let out_dir = ref "."
let target = ref "x86_64-linux"
let input_file = ref ""

let speclist = [
  ("--emit-ast", Arg.Set emit_ast, "Emit AST to stdout");
  ("--emit-tast", Arg.Set emit_tast, "Emit Typed AST to stdout");
  ("--emit-ir", Arg.Set emit_ir, "Emit LLVM IR (.ll) to output directory");
  ("--emit-bc", Arg.Set emit_bc, "Emit LLVM Bitcode (.bc) to output directory");
  ("--verify-ir", Arg.Set verify_ir, "Verify generated LLVM IR");
  ("--link-runtime", Arg.Set link_runtime, "Link with runtime library to produce executable");
  ("--runtime-path", Arg.Set_string runtime_path, "Path to runtime library (default: runtime/native/build/libreml_runtime.a)");
  ("--out-dir", Arg.Set_string out_dir, "Output directory (default: current directory)");
  ("--target", Arg.Set_string target, "Target triple (default: x86_64-linux)");
]

let anon_fun filename =
  input_file := filename

(** 出力ファイル名生成 *)
let output_filename basename suffix =
  Filename.concat !out_dir (basename ^ suffix)

(** ベース名取得（拡張子除去） *)
let get_basename filepath =
  Filename.remove_extension (Filename.basename filepath)

(** ランタイムライブラリとリンクして実行可能ファイルを生成
 *
 * @param ll_file LLVM IR ファイルパス
 * @param runtime_lib ランタイムライブラリパス
 * @param output_exe 出力実行可能ファイルパス
 *)
let link_with_runtime ll_file runtime_lib output_exe =
  (* LLVM IR → オブジェクトファイル *)
  let obj_file = (Filename.remove_extension ll_file) ^ ".o" in
  let llc_cmd = Printf.sprintf "llc -filetype=obj %s -o %s" ll_file obj_file in

  Printf.printf "Compiling to object file: %s\n" obj_file;
  let llc_result = Sys.command llc_cmd in
  if llc_result <> 0 then begin
    Printf.eprintf "Error: llc failed with exit code %d\n" llc_result;
    exit 1
  end;

  (* オブジェクトファイル + ランタイム → 実行可能ファイル *)
  (* clang に標準ライブラリリンクを任せる（デフォルト動作） *)
  let link_cmd = Printf.sprintf "cc %s %s -o %s" obj_file runtime_lib output_exe in

  Printf.printf "Linking with runtime: %s\n" output_exe;
  let link_result = Sys.command link_cmd in
  if link_result <> 0 then begin
    Printf.eprintf "Error: linking failed with exit code %d\n" link_result;
    exit 1
  end;

  (* 一時オブジェクトファイルを削除 *)
  Sys.remove obj_file;

  Printf.printf "Executable created: %s\n" output_exe

let () =
  Arg.parse speclist anon_fun usage_msg;

  if !input_file = "" then begin
    prerr_endline "Error: no input file";
    Arg.usage speclist usage_msg;
    exit 1
  end;

  (* ファイルを開いてソース文字列を読み込む *)
  let ic = open_in !input_file in
  let source = really_input_string ic (in_channel_length ic) in
  close_in ic;

  (* パース用にソース文字列から lexbuf を作成 *)
  let lexbuf = Lexing.from_string source in
  lexbuf.Lexing.lex_curr_p <- { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = !input_file };

  match Parser_driver.parse lexbuf with
  | Ok ast ->
      (* Phase 1: AST 出力 *)
      if !emit_ast then begin
        let rendered = Ast_printer.string_of_compilation_unit ast in
        Printf.printf "%s\n" rendered;
      end;

      (* Phase 2+: 型推論が必要な処理 *)
      if !emit_tast || !emit_ir || !emit_bc || !verify_ir then begin
        match Type_inference.infer_compilation_unit ast with
        | Ok tast ->
            (* Phase 2: Typed AST 出力 *)
            if !emit_tast then begin
              let rendered = Typed_ast.string_of_typed_compilation_unit tast in
              Printf.printf "%s\n" rendered;
            end;

            (* Phase 3: LLVM IR 生成パイプライン *)
            if !emit_ir || !emit_bc || !verify_ir then begin
              try
                (* Typed AST → Core IR (糖衣削除) *)
                let core_ir = Core_ir.Desugar.desugar_compilation_unit tast in

                (* Core IR 最適化 (O1レベル) *)
                let opt_config = Core_ir.Pipeline.{
                  opt_level = O1;
                  enable_const_fold = true;
                  enable_dce = true;
                  max_iterations = 10;
                  verbose = false;
                  emit_intermediate = false;
                } in
                let (optimized_ir, _stats) = Core_ir.Pipeline.optimize_module ~config:opt_config core_ir in

                (* Core IR → LLVM IR *)
                let llvm_module = Codegen.codegen_module ~target_name:!target optimized_ir in

                (* LLVM IR 検証 *)
                if !verify_ir then begin
                  match Verify.verify_llvm_ir llvm_module with
                  | Ok () ->
                      Printf.printf "LLVM IR verification passed.\n"
                  | Error err ->
                      let diag = Verify.error_to_diagnostic err None in
                      Printf.eprintf "%s\n" (Diagnostic.to_string diag);
                      exit 1
                end;

                (* LLVM IR テキスト出力 *)
                let ll_file_opt = ref None in
                if !emit_ir then begin
                  let basename = get_basename !input_file in
                  let output_path = output_filename basename ".ll" in
                  Codegen.emit_llvm_ir llvm_module output_path;
                  Printf.printf "LLVM IR written to: %s\n" output_path;
                  ll_file_opt := Some output_path;
                end;

                (* LLVM IR ビットコード出力 *)
                if !emit_bc then begin
                  let basename = get_basename !input_file in
                  let output_path = output_filename basename ".bc" in
                  Codegen.emit_llvm_bc llvm_module output_path;
                  Printf.printf "LLVM Bitcode written to: %s\n" output_path;
                end;

                (* ランタイムとリンク *)
                if !link_runtime then begin
                  let basename = get_basename !input_file in
                  (* LLVM IR ファイルが必要なので、まだ生成されていなければ一時ファイルとして生成 *)
                  let ll_file = match !ll_file_opt with
                    | Some path -> path
                    | None ->
                        let temp_path = output_filename basename ".ll" in
                        Codegen.emit_llvm_ir llvm_module temp_path;
                        temp_path
                  in

                  (* ランタイムライブラリの存在確認 *)
                  if not (Sys.file_exists !runtime_path) then begin
                    Printf.eprintf "Error: runtime library not found: %s\n" !runtime_path;
                    Printf.eprintf "Please build the runtime first with: make -C runtime/native runtime\n";
                    exit 1
                  end;

                  (* リンクして実行可能ファイルを生成 *)
                  let output_exe = output_filename basename "" in
                  link_with_runtime ll_file !runtime_path output_exe;

                  (* 一時 LLVM IR ファイルを削除（--emit-ir が指定されていない場合） *)
                  if !ll_file_opt = None && not !emit_ir then begin
                    Sys.remove ll_file
                  end;
                end;

              with
              | Core_ir.Desugar.DesugarError (msg, ast_span) ->
                  (* Ast.span を Diagnostic.span に変換 *)
                  let diag_span = Diagnostic.{
                    start_pos = { filename = !input_file; line = 0; column = 0; offset = ast_span.Ast.start };
                    end_pos = { filename = !input_file; line = 0; column = 0; offset = ast_span.Ast.end_ };
                  } in
                  let diag = Diagnostic.{
                    severity = Error;
                    severity_hint = None;
                    domain = None;
                    code = Some "E8001";
                    message = Printf.sprintf "Core IR 変換エラー: %s" msg;
                    span = diag_span;
                    expected_summary = None;
                    notes = [];
                    fixits = [];
                  } in
                  Printf.eprintf "%s\n" (Diagnostic.to_string diag);
                  exit 1
              | Codegen.CodegenError msg ->
                  let dummy_loc = Diagnostic.{
                    filename = !input_file;
                    line = 0;
                    column = 0;
                    offset = 0;
                  } in
                  let diag = Diagnostic.{
                    severity = Error;
                    severity_hint = None;
                    domain = None;
                    code = Some "E8002";
                    message = Printf.sprintf "LLVM IR 生成エラー: %s" msg;
                    span = { start_pos = dummy_loc; end_pos = dummy_loc };
                    expected_summary = None;
                    notes = [];
                    fixits = [];
                  } in
                  Printf.eprintf "%s\n" (Diagnostic.to_string diag);
                  exit 1
            end;

        | Error type_err ->
            (* 型推論エラー *)
            let diag = Type_error.to_diagnostic_with_source source !input_file type_err in
            Printf.eprintf "%s\n" (Diagnostic.to_string diag);
            exit 1
      end;

      exit 0
  | Error diag ->
      Printf.eprintf "%s\n" (Diagnostic.to_string diag);
      exit 1
