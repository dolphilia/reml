(* test_golden.ml — ゴールデンテスト
 *
 * AST 出力をスナップショットと比較し、仕様からの逸脱を検知する。
 *)

open Ast
open Ast_printer

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path

let golden_dir = resolve "tests/golden"

let golden_path name = Filename.concat golden_dir (name ^ ".golden")

let fail_missing_golden name path actual =
  Printf.eprintf "✗ %s: ゴールデンファイル %s が存在しません。\n" name path;
  Printf.eprintf "  現在の出力:\n%s\n" actual;
  Printf.eprintf "  必要に応じてゴールデンを作成し直してください。\n";
  exit 1

let write_actual_snapshot name actual =
  let actual_dir = Filename.concat golden_dir "_actual" in
  if not (Sys.file_exists actual_dir) then Unix.mkdir actual_dir 0o755;
  let path = Filename.concat actual_dir (name ^ ".actual") in
  Out_channel.with_open_text path (fun oc ->
      output_string oc actual;
      output_char oc '\n');
  path

let compare_with_golden name input_file =
  let ic = open_in (resolve input_file) in
  let lexbuf = Lexing.from_channel ic in
  lexbuf.Lexing.lex_curr_p <- { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = input_file };
  let cu =
    try
      let parsed = Parser.compilation_unit Lexer.token lexbuf in
      close_in ic;
      parsed
    with
    | Parser.Error ->
        close_in ic;
        Printf.eprintf "✗ %s: Parser error in %s\n" name input_file;
        exit 1
    | Lexer.Lexer_error (msg, span) ->
        close_in ic;
        Printf.eprintf "✗ %s: Lexer error in %s (%d-%d): %s\n"
          name input_file span.Ast.start span.end_ msg;
        exit 1
  in
  let actual = string_of_compilation_unit cu |> String.trim in
  let golden_file = golden_path name in
  if not (Sys.file_exists golden_file) then
    fail_missing_golden name golden_file actual;
  let expected = In_channel.with_open_text golden_file (fun ic ->
      In_channel.input_all ic |> String.trim)
  in
  if expected = actual then begin
    Printf.printf "✓ %s\n" name;
  end else begin
    Printf.printf "✗ %s: ゴールデンとの差分を検出\n" name;
    Printf.printf "  ゴールデン: %s\n" golden_file;
    let actual_path = write_actual_snapshot name actual in
    Printf.printf "  現在の出力を %s に書き出しました。\n" actual_path;
    Printf.printf "  差分を確認し、意図的な変更であればゴールデンを更新してください。\n";
    exit 1
  end

let () =
  Printf.printf "Running Golden Tests\n";
  Printf.printf "====================\n\n";
  if not (Sys.file_exists golden_dir) then begin
    Printf.eprintf "✗ golden directory %s が存在しません。\n" golden_dir;
    exit 1
  end;
  compare_with_golden "simple" "tests/simple.reml";
  Printf.printf "\n";
  Printf.printf "====================\n";
  Printf.printf "All Golden tests passed!\n"
