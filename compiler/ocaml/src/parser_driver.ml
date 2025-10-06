(* parser_driver.ml — Parser ランナーと診断生成 *)

module I = Parser.MenhirInterpreter

let process_lexer_error lexbuf msg =
  let start_pos = Lexing.lexeme_start_p lexbuf in
  let end_pos = Lexing.lexeme_end_p lexbuf in
  Diagnostic.of_lexer_error ~message:msg ~start_pos ~end_pos

let process_parser_error lexbuf message =
  let start_pos = Lexing.lexeme_start_p lexbuf in
  let end_pos = Lexing.lexeme_end_p lexbuf in
  Diagnostic.of_parser_error ~message ~start_pos ~end_pos ~expected:[]

let parse lexbuf =
  let read_token lexbuf =
    let token = Lexer.token lexbuf in
    let start_pos = Lexing.lexeme_start_p lexbuf in
    let end_pos = Lexing.lexeme_end_p lexbuf in
    (token, start_pos, end_pos)
  in
  let rec loop checkpoint =
    match checkpoint with
    | I.InputNeeded _ ->
        begin
          try
            let triple = read_token lexbuf in
            loop (I.offer checkpoint triple)
          with
          | Lexer.Lexer_error (msg, _) ->
              Result.Error (process_lexer_error lexbuf msg)
        end
    | I.AboutToReduce _
    | I.Shifting _ ->
        loop (I.resume checkpoint)
    | I.Accepted ast -> Ok ast
    | I.HandlingError _ ->
        Result.Error (process_parser_error lexbuf "構文エラー: 入力を解釈できません")
    | I.Rejected ->
        let pos = lexbuf.Lexing.lex_curr_p in
        let diag = Diagnostic.of_parser_error
          ~message:"構文エラー: 解析を続行できません"
          ~start_pos:pos ~end_pos:pos ~expected:[]
        in
        Result.Error diag
  in
  loop (Parser.Incremental.compilation_unit lexbuf.Lexing.lex_curr_p)

let parse_string ?(filename = "<入力>") text =
  let lexbuf = Lexing.from_string text in
  lexbuf.Lexing.lex_curr_p <- { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = filename };
  parse lexbuf
