(* Main — Reml コンパイラエントリーポイント (Phase 1)
 *
 * コマンドライン引数を解析し、パーサーを実行してASTを出力する。
 * Phase 1 M1 マイルストーン: --emit-ast オプションのみ実装。
 *)

let usage_msg = "remlc-ocaml [options] <file>"
let emit_ast = ref false
let emit_tast = ref false
let input_file = ref ""

let speclist = [
  ("--emit-ast", Arg.Set emit_ast, "Emit AST to stdout");
  ("--emit-tast", Arg.Set emit_tast, "Emit Typed AST to stdout");
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

  (* ファイルを開いてソース文字列を読み込む *)
  let ic = open_in !input_file in
  let source = really_input_string ic (in_channel_length ic) in
  close_in ic;

  (* パース用にソース文字列から lexbuf を作成 *)
  let lexbuf = Lexing.from_string source in
  lexbuf.Lexing.lex_curr_p <- { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = !input_file };

  match Parser_driver.parse lexbuf with
  | Ok ast ->
      if !emit_ast then begin
        let rendered = Ast_printer.string_of_compilation_unit ast in
        Printf.printf "%s\n" rendered;
      end;
      if !emit_tast then begin
        (* 型推論を実行 *)
        match Type_inference.infer_compilation_unit ast with
        | Ok tast ->
            let rendered = Typed_ast.string_of_typed_compilation_unit tast in
            Printf.printf "%s\n" rendered;
        | Error type_err ->
            (* ソース情報を使って正確な診断を生成 *)
            let diag = Type_error.to_diagnostic_with_source source !input_file type_err in
            Printf.eprintf "%s\n" (Diagnostic.to_string diag);
            exit 1
      end;
      exit 0
  | Error diag ->
      Printf.eprintf "%s\n" (Diagnostic.to_string diag);
      exit 1
