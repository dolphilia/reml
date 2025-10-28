(* parser_driver.ml — Parser ランナーと `ParseResult` シム
 *
 * Phase 2-5: RunConfig を正式導入し、診断状態へのスイッチ伝播を整備する。
 *)

module I = Parser.MenhirInterpreter
module Builder = Diagnostic.Builder
module Run_config = Parser_run_config

let default_run_config = Run_config.default

let legacy_run_config =
  let open Run_config.Legacy in
  bridge { require_eof = true; legacy_result = true }

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
  span_trace : (string option * Diagnostic.span) list option;
}

type parse_result_with_rest = {
  result : parse_result;
  rest : string option;
}

let process_lexer_error lexbuf msg =
  let start_pos = Lexing.lexeme_start_p lexbuf in
  let end_pos = Lexing.lexeme_end_p lexbuf in
  Diagnostic.of_lexer_error ~message:msg ~start_pos ~end_pos

let build_parser_diagnostic ~message ~start_pos ~end_pos ~summary =
  Builder.create
    ~message
    ~primary:(Diagnostic.span_of_positions start_pos end_pos)
    ~domain:Diagnostic.Parser
    ()
  |> Builder.set_expected summary
  |> Builder.build

let process_parser_error lexbuf message summary =
  let start_pos = Lexing.lexeme_start_p lexbuf in
  let end_pos = Lexing.lexeme_end_p lexbuf in
  build_parser_diagnostic ~message ~start_pos ~end_pos ~summary

let process_rejected_error lexbuf summary =
  let pos = lexbuf.Lexing.lex_curr_p in
  build_parser_diagnostic
    ~message:"構文エラー: 解析を続行できません"
    ~start_pos:pos ~end_pos:pos ~summary

let summarize_snapshot snapshot =
  match snapshot.Parser_diag_state.expected_summary with
  | Some summary -> summary
  | None -> Parser_expectation.summarize_with_defaults snapshot.expected

let expectation_summary_for_checkpoint diag_state checkpoint =
  let { Parser_expectation.summary; expectations; _ } =
    Parser_expectation.collect ~checkpoint
  in
  if expectations <> [] then summary
  else
    match Parser_diag_state.farthest_snapshot diag_state with
    | Some snapshot -> summarize_snapshot snapshot
    | None -> Parser_expectation.empty_summary

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
    span_trace = Parser_diag_state.span_trace_pairs diag_state;
  }

let register_diagnostic diag_state diag ~consumed ~committed =
  Parser_diag_state.record_diagnostic diag_state ~diagnostic:diag ~committed
    ~consumed

let build_failure diag_state diag ~consumed ~committed =
  register_diagnostic diag_state diag ~consumed ~committed;
  let legacy_error = diagnostic_to_parse_error diag ~consumed ~committed in
  finalize_result diag_state ~value:None ~span:None
    ~legacy_error:(Some legacy_error) ~consumed ~committed

let effective_require_eof config =
  match Run_config.Config.find config with
  | None -> config.require_eof
  | Some namespace -> (
      match Run_config.Config.require_eof_override namespace with
      | Some value -> value
      | None -> config.require_eof)

let warn_unimplemented_feature diag_state lexbuf ~code ~message =
  let pos = lexbuf.Lexing.lex_curr_p in
  let span = Diagnostic.span_of_positions pos pos in
  let diagnostic =
    Builder.create ~message ~primary:span ()
    |> Builder.set_severity Diagnostic.Warning
    |> Builder.set_domain Diagnostic.Config
    |> Builder.set_primary_code code
    |> Builder.build
  in
  Parser_diag_state.record_warning diag_state ~diagnostic

let warn_packrat diag_state lexbuf =
  warn_unimplemented_feature diag_state lexbuf
    ~code:"parser.runconfig.packrat_unimplemented"
    ~message:
      "RunConfig.packrat=true が指定されましたが、Packrat メモ化はまだ \
       実装されていません（Phase 2-5 計画）。"

let warn_left_recursion diag_state lexbuf mode =
  let mode_text =
    match mode with
    | Run_config.On -> "on"
    | Run_config.Auto -> "auto"
    | Run_config.Off -> "off"
  in
  warn_unimplemented_feature diag_state lexbuf
    ~code:"parser.runconfig.left_recursion_unimplemented"
    ~message:
      (Printf.sprintf
         "RunConfig.left_recursion=\"%s\" を利用できません。左再帰シムは \
          PARSER-003 で導入予定です。"
         mode_text)

let run ?(config = default_run_config) lexbuf =
  let recover_config = Run_config.Recover.of_run_config config in
  let diag_state =
    Parser_diag_state.create ~trace:config.trace
      ~merge_warnings:config.merge_warnings ?locale:config.locale
      ~recover:recover_config ()
  in
  let require_eof = effective_require_eof config in
  let consumed = ref false in
  let committed = ref false in
  let eof_seen = ref false in
  let start_pos = lexbuf.Lexing.lex_curr_p in
  if config.packrat then warn_packrat diag_state lexbuf;
  (match config.left_recursion with
  | Run_config.On -> warn_left_recursion diag_state lexbuf Run_config.On
  | Run_config.Auto when config.packrat ->
      warn_left_recursion diag_state lexbuf Run_config.Auto
  | _ -> ());
  let read_token () =
    let token = Lexer.token lexbuf in
    (match token with
    | Token.EOF -> eof_seen := true
    | _ -> consumed := true);
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
        Parser_diag_state.record_span_trace diag_state
          ~label:(Some "compilation_unit") ~span;
        finalize_result diag_state ~value:(Some ast) ~span:(Some span)
          ~legacy_error:None ~consumed:!consumed ~committed:!committed
    | I.HandlingError _ ->
        let summary =
          expectation_summary_for_checkpoint diag_state checkpoint
        in
        let diag =
          process_parser_error lexbuf "構文エラー: 入力を解釈できません"
            summary
        in
        build_failure diag_state diag ~consumed:!consumed ~committed:!committed
    | I.Rejected ->
        let summary =
          expectation_summary_for_checkpoint diag_state checkpoint
        in
        let diag = process_rejected_error lexbuf summary in
        build_failure diag_state diag ~consumed:!consumed ~committed:!committed
  in
  let checkpoint = Parser.Incremental.compilation_unit lexbuf.Lexing.lex_curr_p in
  let result = loop checkpoint in
  if require_eof && not !eof_seen then (
    let pos = lexbuf.Lexing.lex_curr_p in
    let span = Diagnostic.span_of_positions pos pos in
    let diag =
      Builder.create
        ~message:"RunConfig.require_eof=true のため未消費入力を許可できません"
        ~primary:span ()
      |> Builder.set_severity Diagnostic.Error
      |> Builder.set_domain Diagnostic.Parser
      |> Builder.set_primary_code "parser.require_eof.unconsumed_input"
      |> Builder.build
    in
    register_diagnostic diag_state diag ~consumed:!consumed ~committed:!committed;
    let legacy_error =
      diagnostic_to_parse_error diag ~consumed:!consumed ~committed:!committed
    in
    finalize_result diag_state ~value:None ~span:result.span
      ~legacy_error:(Some legacy_error) ~consumed:!consumed
      ~committed:!committed)
  else result

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
