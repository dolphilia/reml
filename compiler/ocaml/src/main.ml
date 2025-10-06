(* Main — Reml コンパイラエントリーポイント (Phase 1)
 *
 * コマンドライン引数を解析し、パーサーを実行してASTを出力する。
 * Phase 1 M1 マイルストーン: --emit-ast オプションのみ実装。
 *)

let usage_msg = "remlc-ocaml [options] <file>"
let emit_ast = ref false
let input_file = ref ""

let speclist = [
  ("--emit-ast", Arg.Set emit_ast, "Emit AST to stdout");
]

let anon_fun filename =
  input_file := filename

let () =
  Arg.parse speclist anon_fun usage_msg;

  if !input_file = "" then begin
    prerr_endline "Error: no input file";
    Arg.usage speclist usage_msg;
    exit 1
  end;

  (* ファイルを開く *)
  let ic = open_in !input_file in
  let lexbuf = Lexing.from_channel ic in
  lexbuf.Lexing.lex_curr_p <- { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = !input_file };

  match Parser_driver.parse lexbuf with
  | Ok ast ->
      close_in ic;
      if !emit_ast then begin
        let rendered = Ast_printer.string_of_compilation_unit ast in
        Printf.printf "%s\n" rendered;
      end;
      exit 0
  | Error diag ->
      close_in ic;
      Printf.eprintf "%s\n" (Diagnostic.to_string diag);
      exit 1
