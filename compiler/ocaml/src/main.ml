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

  try
    (* パースを実行 *)
    let ast = Parser.compilation_unit Lexer.token lexbuf in
    close_in ic;

    (* AST 出力 *)
    if !emit_ast then begin
      Printf.printf "Compilation unit parsed successfully.\n";
      Printf.printf "Header: %s\n" (match ast.Ast.header with
        | None -> "None"
        | Some h -> "module " ^ (match h.module_path with
          | Ast.Root ids -> "::" ^ String.concat "." (List.map (fun i -> i.Ast.name) ids)
          | Ast.Relative (head, tail) -> "relative path"));
      Printf.printf "Uses: %d declarations\n" (List.length ast.uses);
      Printf.printf "Decls: %d declarations\n" (List.length ast.decls);
    end;

    exit 0

  with
  | Lexer.Lexer_error (msg, span) ->
      close_in ic;
      Printf.eprintf "Lexer error at %d-%d: %s\n" span.Ast.start span.end_ msg;
      exit 1
  | Parser.Error ->
      close_in ic;
      let pos = lexbuf.Lexing.lex_curr_p in
      Printf.eprintf "Parse error at line %d, column %d\n"
        pos.Lexing.pos_lnum
        (pos.Lexing.pos_cnum - pos.Lexing.pos_bol);
      exit 1
  | e ->
      close_in ic;
      Printf.eprintf "Unexpected error: %s\n" (Printexc.to_string e);
      exit 1
