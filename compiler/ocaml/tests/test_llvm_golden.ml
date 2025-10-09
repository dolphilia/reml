(* test_llvm_golden.ml — LLVM IR ゴールデンテスト (Phase 3 Week 16)
 *
 * Remlサンプルコードから生成されるLLVM IRと期待値(.ll.golden)を比較し、
 * コード生成の回帰を検出する。
 *
 * テストケース:
 * 1. basic_arithmetic - 基本的な算術演算と関数定義
 * 2. control_flow - 条件分岐と再帰関数
 * 3. function_calls - 関数呼び出しとABI
 *)

(* ========== パス設定 ========== *)

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path

let golden_dir = resolve "tests/llvm-ir/golden"
let actual_dir = Filename.concat golden_dir "_actual"

(* ========== ユーティリティ ========== *)

let ensure_dir dir =
  if not (Sys.file_exists dir) then Unix.mkdir dir 0o755

let golden_path name = Filename.concat golden_dir (name ^ ".ll.golden")

let actual_path name = Filename.concat actual_dir (name ^ ".ll")

let source_path name = Filename.concat golden_dir (name ^ ".reml")

(* ========== IR正規化 ========== *)

(* LLVM IRの非決定的要素を正規化して比較を可能にする *)
let normalize_ir_line line =
  (* コメント行はスキップ *)
  if String.starts_with ~prefix:";" line then
    Some line
  (* target datalayout/tripleは固定値として扱う *)
  else if String.starts_with ~prefix:"target " line then
    Some line
  (* declare/defineは関数シグネチャとして比較 *)
  else if String.starts_with ~prefix:"declare " line || String.starts_with ~prefix:"define " line then
    Some line
  (* attributes行も保持 *)
  else if String.starts_with ~prefix:"attributes " line then
    Some line
  (* source_filenameは正規化してスキップ *)
  else if String.starts_with ~prefix:"source_filename " line then
    None
  (* 空行は保持 *)
  else if String.trim line = "" then
    Some ""
  else
    Some line

let normalize_ir content =
  String.split_on_char '\n' content
  |> List.filter_map normalize_ir_line
  |> String.concat "\n"
  |> String.trim

(* ========== IR生成 ========== *)

(* サンプルRemlファイルからLLVM IRを生成 *)
let generate_ir source_file =
  let ic = open_in source_file in
  let source = really_input_string ic (in_channel_length ic) in
  close_in ic;

  let lexbuf = Lexing.from_string source in
  lexbuf.Lexing.lex_curr_p <- { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = source_file };

  match Parser_driver.parse lexbuf with
  | Error diag ->
      Printf.eprintf "Parse error in %s:\n%s\n" source_file (Diagnostic.to_string diag);
      exit 1
  | Ok ast ->
      match Type_inference.infer_compilation_unit ast with
      | Error type_err ->
          let diag = Type_error.to_diagnostic_with_source source source_file type_err in
          Printf.eprintf "Type error in %s:\n%s\n" source_file (Diagnostic.to_string diag);
          exit 1
      | Ok tast ->
          try
            (* Typed AST → Core IR *)
            let core_ir = Core_ir.Desugar.desugar_compilation_unit tast in

            (* Core IR 最適化 (O1) *)
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
            let target_name = "x86_64-linux" in
            let llvm_module = Codegen.codegen_module ~target_name optimized_ir in

            (* LLVM IR をテキスト形式で取得 *)
            Llvm.string_of_llmodule llvm_module

          with
          | Core_ir.Desugar.DesugarError (msg, _span) ->
              Printf.eprintf "Desugar error in %s: %s\n" source_file msg;
              exit 1
          | Codegen.CodegenError msg ->
              Printf.eprintf "Codegen error in %s: %s\n" source_file msg;
              exit 1

(* ========== ゴールデンテスト実行 ========== *)

let compare_with_golden name =
  let source_file = source_path name in
  let golden_file = golden_path name in
  let actual_file = actual_path name in

  (* 1. サンプルからIRを生成 *)
  let actual_ir = generate_ir source_file in

  (* 2. 期待値IRを読み込み *)
  if not (Sys.file_exists golden_file) then begin
    Printf.eprintf "✗ %s: ゴールデンファイル %s が存在しません。\n" name golden_file;
    Printf.eprintf "  現在の出力を確認し、意図した出力であれば以下のコマンドで登録してください:\n";
    Printf.eprintf "  cp %s %s\n" actual_file golden_file;

    (* 実際の出力を保存 *)
    ensure_dir actual_dir;
    Out_channel.with_open_text actual_file (fun oc -> output_string oc actual_ir);

    exit 1
  end;

  let expected_ir = In_channel.with_open_text golden_file In_channel.input_all in

  (* 3. 正規化して比較 *)
  let normalized_actual = normalize_ir actual_ir in
  let normalized_expected = normalize_ir expected_ir in

  if normalized_actual = normalized_expected then begin
    Printf.printf "✓ %s\n" name;
  end else begin
    Printf.printf "✗ %s: ゴールデンとの差分を検出\n" name;
    Printf.printf "  期待値: %s\n" golden_file;

    (* 実際の出力を保存 *)
    ensure_dir actual_dir;
    Out_channel.with_open_text actual_file (fun oc -> output_string oc actual_ir);

    Printf.printf "  実際の出力: %s\n" actual_file;
    Printf.printf "  差分を確認し、意図的な変更であればゴールデンを更新してください:\n";
    Printf.printf "  diff -u %s %s\n" golden_file actual_file;
    Printf.printf "  cp %s %s  # 更新する場合\n" actual_file golden_file;
    exit 1
  end

(* ========== メイン ========== *)

let () =
  Printf.printf "LLVM IR ゴールデンテスト\n";
  Printf.printf "========================\n\n";

  if not (Sys.file_exists golden_dir) then begin
    Printf.eprintf "✗ ゴールデンディレクトリ %s が存在しません。\n" golden_dir;
    exit 1
  end;

  (* テストケース実行 *)
  compare_with_golden "basic_arithmetic";
  compare_with_golden "control_flow";
  compare_with_golden "function_calls";

  Printf.printf "\n========================\n";
  Printf.printf "全てのLLVM IRゴールデンテストが成功しました!\n"
