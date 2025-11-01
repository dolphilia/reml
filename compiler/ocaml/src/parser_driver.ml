(* parser_driver.ml — Parser ランナーと `ParseResult` シム
 *
 * Phase 2-5: RunConfig を正式導入し、診断状態へのスイッチ伝播を整備する。
 *)

module I = Parser.MenhirInterpreter
module Builder = Diagnostic.Builder
module Run_config = Parser_run_config
module Core_state = Core_parse.State
module Core_reply = Core_parse.Reply
module Core_stream = Core_parse_streaming

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
  packrat_stats : (int * int) option;
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

let finalize_result session ~value ~span ~legacy_error ~consumed ~committed
    ~packrat_stats =
  {
    value;
    span;
    diagnostics = Core_stream.diagnostics session;
    recovered = Core_stream.recovered session;
    legacy_error;
    consumed;
    committed;
    farthest_error_offset = Core_stream.farthest_error_offset session;
    span_trace = Core_stream.span_trace_pairs session;
    packrat_stats;
  }

let register_diagnostic session diag ~consumed ~committed =
  Core_stream.register_diagnostic session diag ~consumed ~committed

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
  let pack, config =
    let pack, config = Core_parse_lex.Bridge.derive config in
    match pack with
    | Core_parse_lex.Pack.{ space_id = Some space_id; _ } ->
        Core_parse_lex.Bridge.with_space_id pack config space_id
    | _ -> (pack, config)
  in
  Core_parse_lex.Api.config_trivia pack lexbuf;
  let session = Core_stream.create_session config in
  let diag_state = Core_stream.diag_state session in
  let core_state = Core_stream.core_state session in
  let require_eof = Core_stream.effective_require_eof config in
  let eof_seen = ref false in
  let start_pos = lexbuf.Lexing.lex_curr_p in
  (match config.left_recursion with
  | Run_config.On -> warn_left_recursion diag_state lexbuf Run_config.On
  | Run_config.Auto when config.packrat ->
      warn_left_recursion diag_state lexbuf Run_config.Auto
  | _ -> ());
  let read_token () =
    let token, start_pos, end_pos =
      Core_parse_lex.Api.lexeme pack Lexer.read_token lexbuf
    in
    (match token with
    | Token.EOF -> eof_seen := true
    | _ -> Core_state.mark_consumed core_state);
    (token, start_pos, end_pos)
  in
  let rec loop state checkpoint =
    match checkpoint with
    | I.InputNeeded _ -> (
        try
          let triple = read_token () in
          loop state (I.offer checkpoint triple)
        with Lexer.Lexer_error (msg, _) ->
          let diag = process_lexer_error lexbuf msg in
          Core_reply.err ~id:None ~diagnostic:diag
            ~consumed:(Core_state.consumed state)
            ~committed:(Core_state.committed state))
    | I.Shifting _ | I.AboutToReduce _ -> loop state (I.resume checkpoint)
    | I.Accepted ast ->
        let span =
          Diagnostic.span_of_positions start_pos lexbuf.Lexing.lex_curr_p
        in
        Core_reply.ok ~id:None ~value:ast ~span:(Some span)
          ~consumed:(Core_state.consumed state)
          ~committed:(Core_state.committed state)
    | I.HandlingError _ ->
        let summary =
          Core_stream.expectation_summary_for_checkpoint session checkpoint
        in
        let diag =
          process_parser_error lexbuf "構文エラー: 入力を解釈できません"
            summary
        in
        Core_reply.err ~id:None ~diagnostic:diag
          ~consumed:(Core_state.consumed state)
          ~committed:(Core_state.committed state)
    | I.Rejected ->
        let summary =
          Core_stream.expectation_summary_for_checkpoint session checkpoint
        in
        let diag = process_rejected_error lexbuf summary in
        Core_reply.err ~id:None ~diagnostic:diag
          ~consumed:(Core_state.consumed state)
          ~committed:(Core_state.committed state)
  in
  let checkpoint = Parser.Incremental.compilation_unit lexbuf.Lexing.lex_curr_p in
  let parser state =
    let reply = loop state checkpoint in
    (reply, state)
  in
  let reply, _state =
    Core_parse.rule ~namespace:"menhir" ~name:"compilation_unit" parser
      core_state
  in
  let packrat_stats = Core_stream.packrat_counters session in
  let result =
    match reply with
    | Core_reply.Ok ok ->
        let span =
          match ok.span with
          | Some span -> span
          | None ->
              Diagnostic.span_of_positions start_pos lexbuf.Lexing.lex_curr_p
        in
        Parser_diag_state.record_span_trace diag_state
          ~label:(Some "compilation_unit") ~span;
        finalize_result session ~value:(Some ok.value) ~span:(Some span)
          ~legacy_error:None ~consumed:ok.consumed ~committed:ok.committed
          ~packrat_stats
    | Core_reply.Err err ->
        let diagnostic =
          Core_stream.annotate_core_rule_metadata err.diagnostic err.id
        in
        register_diagnostic session diagnostic ~consumed:err.consumed
          ~committed:err.committed;
        let legacy_error =
          diagnostic_to_parse_error diagnostic ~consumed:err.consumed
            ~committed:err.committed
        in
        finalize_result session ~value:None ~span:None
          ~legacy_error:(Some legacy_error) ~consumed:err.consumed
          ~committed:err.committed ~packrat_stats
  in
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
    register_diagnostic session diag ~consumed:result.consumed
      ~committed:result.committed;
    let legacy_error =
      diagnostic_to_parse_error diag ~consumed:result.consumed
        ~committed:result.committed
    in
    finalize_result session ~value:None ~span:result.span
      ~legacy_error:(Some legacy_error) ~consumed:result.consumed
      ~committed:result.committed ~packrat_stats:result.packrat_stats)
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

module Streaming = struct
  module Stream_cfg = Run_config.Stream

  type demand_action = [ `Pause | `Continue ]

  type demand_hint = {
    action : demand_action;
    min_bytes : int option;
    preferred_bytes : int option;
    resume_hint : string option;
    reason : string option;
  }

  type chunk =
    | Chunk of string
    | Await of demand_hint option
    | Closed
    | Error of string

  type feeder = unit -> chunk

  type stream_meta = {
    bytes_consumed : int;
    chunks_consumed : int;
    await_count : int;
    resume_count : int;
    last_reason : string option;
  }

  type continuation = {
    config : Run_config.t;
    source_name : string;
    buffer : string;
    bytes_consumed : int;
    chunks_consumed : int;
    await_count : int;
    resume_count : int;
    resume_hint : string option;
    demand_min_bytes : int option;
    demand_preferred_bytes : int option;
  }

  type completed = { result : parse_result; meta : stream_meta }

  type pending = {
    continuation : continuation;
    demand : demand_hint;
    meta : stream_meta;
  }

  type outcome = Completed of completed | Pending of pending

  let demand_pause ?min_bytes ?preferred_bytes ?resume_hint ?reason () =
    { action = `Pause; min_bytes; preferred_bytes; resume_hint; reason }

  let demand_continue ?min_bytes ?preferred_bytes ?resume_hint ?reason () =
    { action = `Continue; min_bytes; preferred_bytes; resume_hint; reason }

  let make_meta ~bytes ~chunks ~await ~resume ?reason () =
    {
      bytes_consumed = bytes;
      chunks_consumed = chunks;
      await_count = await;
      resume_count = resume;
      last_reason = reason;
    }

  let fallback first second = match first with Some _ -> first | None -> second

  let merge_hint defaults ?fallback_reason hint =
    let default_min = defaults.demand_min_bytes in
    let default_pref = defaults.demand_preferred_bytes in
    let default_resume = defaults.resume_hint in
    let select opt default =
      match opt with Some _ as value -> value | None -> default
    in
    {
      action = hint.action;
      min_bytes = select hint.min_bytes default_min;
      preferred_bytes = select hint.preferred_bytes default_pref;
      resume_hint = select hint.resume_hint default_resume;
      reason = fallback hint.reason fallback_reason;
    }

  let default_pause defaults ?reason () =
    let default_min = defaults.demand_min_bytes in
    let default_pref = defaults.demand_preferred_bytes in
    let default_resume = defaults.resume_hint in
    {
      action = `Pause;
      min_bytes = default_min;
      preferred_bytes = default_pref;
      resume_hint = default_resume;
      reason;
    }

  let run_stream ?(filename = "<stream>") ?(config = default_run_config)
      ~(feeder : feeder) () : outcome =
    let defaults = Stream_cfg.of_run_config config in
    let buffer = Buffer.create 4096 in
    let rec pump bytes_consumed chunks_consumed await_count =
      match feeder () with
      | Chunk data ->
          Buffer.add_string buffer data;
          let len = String.length data in
          pump (bytes_consumed + len) (chunks_consumed + 1) await_count
      | Await hint_opt ->
          let demand =
            match hint_opt with
            | Some hint ->
                merge_hint defaults ~fallback_reason:(Some "feeder.await") hint
            | None -> default_pause defaults ~reason:(Some "feeder.await") ()
          in
          let continuation =
            {
              config;
              source_name = filename;
              buffer = Buffer.contents buffer;
              bytes_consumed;
              chunks_consumed;
              await_count = await_count + 1;
              resume_count = 0;
              resume_hint = demand.resume_hint;
              demand_min_bytes = demand.min_bytes;
              demand_preferred_bytes = demand.preferred_bytes;
            }
          in
          let meta =
            make_meta ~bytes:bytes_consumed ~chunks:chunks_consumed
              ~await:(await_count + 1) ~resume:0 ~reason:demand.reason ()
          in
          Pending { continuation; demand; meta }
      | Closed ->
          let text = Buffer.contents buffer in
          let result = run_string ~filename ~config text in
          let meta =
            make_meta ~bytes:bytes_consumed ~chunks:chunks_consumed
              ~await:await_count ~resume:0 ~reason:None ()
          in
          Completed { result; meta }
      | Error message ->
          let demand =
            default_pause defaults ~reason:(Some ("stream.error:" ^ message)) ()
          in
          let continuation =
            {
              config;
              source_name = filename;
              buffer = Buffer.contents buffer;
              bytes_consumed;
              chunks_consumed;
              await_count;
              resume_count = 0;
              resume_hint = demand.resume_hint;
              demand_min_bytes = demand.min_bytes;
              demand_preferred_bytes = demand.preferred_bytes;
            }
          in
          let meta =
            make_meta ~bytes:bytes_consumed ~chunks:chunks_consumed ~await:await_count
              ~resume:0 ~reason:demand.reason ()
          in
          Pending { continuation; demand; meta }
    in
    pump 0 0 0

  let resume (continuation : continuation) (input : chunk) : outcome =
    let defaults = Stream_cfg.of_run_config continuation.config in
    let default_min = defaults.demand_min_bytes in
    let default_pref = defaults.demand_preferred_bytes in
    let default_resume = defaults.resume_hint in
    let existing = continuation.buffer in
    let additional_len =
      match input with Chunk s -> String.length s | Await _ | Closed | Error _ -> 0
    in
    let buffer = Buffer.create (String.length existing + additional_len) in
    Buffer.add_string buffer existing;
    let bytes = continuation.bytes_consumed in
    let chunks = continuation.chunks_consumed in
    let await = continuation.await_count in
    let resume_count = continuation.resume_count + 1 in
    match input with
    | Chunk data ->
        Buffer.add_string buffer data;
        let text = Buffer.contents buffer in
        let result =
          run_string ~filename:continuation.source_name
            ~config:continuation.config text
        in
        let meta =
          make_meta ~bytes:(bytes + String.length data) ~chunks:(chunks + 1)
            ~await ~resume:resume_count ~reason:None ()
        in
        Completed { result; meta }
    | Closed ->
        let text = Buffer.contents buffer in
        let result =
          run_string ~filename:continuation.source_name
            ~config:continuation.config text
        in
        let meta =
          make_meta ~bytes ~chunks ~await ~resume:resume_count
            ~reason:(Some "feeder.closed") ()
        in
        Completed { result; meta }
    | Await hint_opt ->
        let fallback_reason = Some "feeder.await" in
        let demand =
          match hint_opt with
          | Some hint -> merge_hint defaults ~fallback_reason hint
          | None ->
              {
                action = `Pause;
                min_bytes = fallback continuation.demand_min_bytes default_min;
                preferred_bytes =
                  fallback continuation.demand_preferred_bytes default_pref;
                resume_hint = fallback continuation.resume_hint default_resume;
                reason = fallback_reason;
              }
        in
        let new_buffer = Buffer.contents buffer in
        let continuation =
          {
            continuation with
            buffer = new_buffer;
            bytes_consumed = bytes;
            chunks_consumed = chunks;
            await_count = await + 1;
            resume_count;
            resume_hint = demand.resume_hint;
            demand_min_bytes = demand.min_bytes;
            demand_preferred_bytes = demand.preferred_bytes;
          }
        in
        let meta =
          make_meta ~bytes ~chunks ~await:(await + 1) ~resume:resume_count
            ~reason:demand.reason ()
        in
        Pending { continuation; demand; meta }
    | Error message ->
        let reason = Some ("stream.error:" ^ message) in
        let demand =
          {
            action = `Pause;
            min_bytes = fallback continuation.demand_min_bytes default_min;
            preferred_bytes =
              fallback continuation.demand_preferred_bytes default_pref;
            resume_hint = fallback continuation.resume_hint default_resume;
            reason;
          }
        in
        let new_buffer = Buffer.contents buffer in
        let continuation =
          {
            continuation with
            buffer = new_buffer;
            resume_count;
            resume_hint = demand.resume_hint;
          }
        in
        let meta =
          make_meta ~bytes ~chunks ~await ~resume:resume_count ~reason ()
        in
        Pending { continuation; demand; meta }
end
