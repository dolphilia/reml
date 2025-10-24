(* parser_driver.ml — Parser ランナーと `ParseResult` シム *)

module I = Parser.MenhirInterpreter

type run_config = {
  require_eof : bool;
  legacy_result : bool;
}

let default_run_config = { require_eof = true; legacy_result = false }
let legacy_run_config = { require_eof = true; legacy_result = true }

type parse_error = {
  span : Diagnostic.span;
  expected : Diagnostic.expectation list;
  committed : bool;
  far_consumed : bool;
}

type reply =
  | Ok of {
      value : Ast.compilation_unit;
      span : Diagnostic.span option;
      consumed : bool;
    }
  | Err of {
      diagnostic : Diagnostic.t;
      consumed : bool;
      committed : bool;
    }

type parse_result = {
  value : Ast.compilation_unit option;
  span : Diagnostic.span option;
  diagnostics : Diagnostic.t list;
  recovered : bool;
  legacy_error : parse_error option;
  consumed : bool;
  committed : bool;
  farthest_error_offset : int option;
}

type parse_result_with_rest = {
  result : parse_result;
  rest : string option;
}

let process_lexer_error lexbuf msg =
  let start_pos = Lexing.lexeme_start_p lexbuf in
  let end_pos = Lexing.lexeme_end_p lexbuf in
  Diagnostic.of_lexer_error ~message:msg ~start_pos ~end_pos

let process_parser_error lexbuf message =
  let start_pos = Lexing.lexeme_start_p lexbuf in
  let end_pos = Lexing.lexeme_end_p lexbuf in
  Diagnostic.of_parser_error ~message ~start_pos ~end_pos ~expected:[]

let process_rejected_error lexbuf =
  let pos = lexbuf.Lexing.lex_curr_p in
  Diagnostic.of_parser_error
    ~message:"構文エラー: 解析を続行できません" ~start_pos:pos ~end_pos:pos
    ~expected:[]

let diagnostic_to_parse_error diag ~consumed ~committed =
  let expected =
    match diag.Diagnostic.expected with
    | Some summary -> summary.alternatives
    | None -> []
  in
  {
    span = diag.Diagnostic.primary;
    expected;
    committed;
    far_consumed = consumed;
  }

let finalize_result diag_state ~value ~span ~legacy_error ~consumed ~committed =
  {
    value;
    span;
    diagnostics = Parser_diag_state.diagnostics diag_state;
    recovered = Parser_diag_state.recovered diag_state;
    legacy_error;
    consumed;
    committed;
    farthest_error_offset = Parser_diag_state.farthest_offset diag_state;
  }

let register_diagnostic diag_state diag ~consumed ~committed =
  Parser_diag_state.record_diagnostic diag_state ~diagnostic:diag ~committed
    ~consumed

let build_failure diag_state diag ~consumed ~committed =
  register_diagnostic diag_state diag ~consumed ~committed;
  let legacy_error = diagnostic_to_parse_error diag ~consumed ~committed in
  finalize_result diag_state ~value:None ~span:None
    ~legacy_error:(Some legacy_error) ~consumed ~committed

let run ?(config = default_run_config) lexbuf =
  let diag_state = Parser_diag_state.create () in
  let consumed = ref false in
  let committed = ref false in
  let start_pos = lexbuf.Lexing.lex_curr_p in
  let read_token () =
    let token = Lexer.token lexbuf in
    (match token with Token.EOF -> () | _ -> consumed := true);
    let start_pos = Lexing.lexeme_start_p lexbuf in
    let end_pos = Lexing.lexeme_end_p lexbuf in
    (token, start_pos, end_pos)
  in
  let rec loop checkpoint =
    match checkpoint with
    | I.InputNeeded _ -> (
        try
          let triple = read_token () in
          loop (I.offer checkpoint triple)
        with Lexer.Lexer_error (msg, _) ->
          let diag = process_lexer_error lexbuf msg in
          build_failure diag_state diag ~consumed:!consumed ~committed:!committed)
    | I.Shifting _ | I.AboutToReduce _ -> loop (I.resume checkpoint)
    | I.Accepted ast ->
        let span =
          Diagnostic.span_of_positions start_pos lexbuf.Lexing.lex_curr_p
        in
        finalize_result diag_state ~value:(Some ast) ~span:(Some span)
          ~legacy_error:None ~consumed:!consumed ~committed:!committed
    | I.HandlingError _ ->
        let diag =
          process_parser_error lexbuf "構文エラー: 入力を解釈できません"
        in
        build_failure diag_state diag ~consumed:!consumed ~committed:!committed
    | I.Rejected ->
        let diag = process_rejected_error lexbuf in
        build_failure diag_state diag ~consumed:!consumed ~committed:!committed
  in
  let checkpoint = Parser.Incremental.compilation_unit lexbuf.Lexing.lex_curr_p in
  let result = loop checkpoint in
  if config.require_eof then result else result

let run_partial ?(config = default_run_config) lexbuf =
  let cfg = { config with require_eof = false } in
  { result = run ~config:cfg lexbuf; rest = None }

let run_string ?(filename = "<入力>") ?(config = default_run_config) text =
  let lexbuf = Lexing.from_string text in
  lexbuf.Lexing.lex_curr_p <-
    { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = filename };
  run ~config lexbuf

let parse_result_to_legacy_fields
    (value : Ast.compilation_unit option)
    (diagnostics : Diagnostic.t list)
    : (Ast.compilation_unit, Diagnostic.t) result =
  match value with
  | Some v -> Ok v
  | None -> (
      match diagnostics with
      | diag :: _ -> Error diag
      | [] ->
          let default_pos =
            {
              Lexing.pos_fname = "<parser>";
              pos_lnum = 1;
              pos_bol = 0;
              pos_cnum = 0;
            }
          in
          Error
            (Diagnostic.make ~message:"構文エラー: 詳細情報がありません"
               ~start_pos:default_pos ~end_pos:default_pos ()))

let parse_result_to_legacy (result : parse_result) =
  parse_result_to_legacy_fields result.value result.diagnostics

let parse lexbuf =
  run ~config:legacy_run_config lexbuf |> parse_result_to_legacy

let parse_string ?(filename = "<入力>") text =
  let lexbuf = Lexing.from_string text in
  lexbuf.Lexing.lex_curr_p <-
    { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = filename };
  parse lexbuf
