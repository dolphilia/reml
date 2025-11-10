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
module Json = Yojson.Basic

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
  packrat_cache : Parser_expectation.Packrat.t option;
}

type parse_result_with_rest = {
  result : parse_result;
  rest : string option;
}

let expectation_hint_and_token (expectation : Diagnostic.expectation) =
  match expectation with
  | Diagnostic.Token text -> ("token", text)
  | Diagnostic.Keyword text -> ("keyword", text)
  | Diagnostic.Rule text -> ("rule", text)
  | Diagnostic.Class text -> ("class", text)
  | Diagnostic.Not text -> ("not", text)
  | Diagnostic.Custom text -> ("custom", text)
  | Diagnostic.TypeExpected text -> ("type_expected", text)
  | Diagnostic.TraitBound text -> ("trait_bound", text)
  | Diagnostic.Eof -> ("eof", "EOF")

let expectation_to_json expectation =
  let hint, token = expectation_hint_and_token expectation in
  `Assoc
    [
      ("token", `String token);
      ("label", `String token);
      ("hint", `String hint);
      ("kind", `String hint);
    ]

let recover_extension_payload (summary : Diagnostic.expectation_summary) =
  let tokens =
    List.map expectation_to_json summary.Diagnostic.alternatives
  in
  let fields = ref [ ("expected_tokens", `List tokens) ] in
  (match summary.humanized with
  | Some text when String.trim text <> "" ->
      fields := ("message", `String text) :: !fields
  | _ -> ());
  (match summary.context_note with
  | Some text when String.trim text <> "" ->
      fields := ("context", `String text) :: !fields
  | _ -> ());
  `Assoc (List.rev !fields)

let attach_recover_extension (summary : Diagnostic.expectation_summary option)
    (diag : Diagnostic.t) =
  match summary with
  | None -> diag
  | Some summary ->
      let has_tokens = summary.Diagnostic.alternatives <> [] in
      let has_text =
        match (summary.humanized, summary.context_note) with
        | Some text, _ when String.trim text <> "" -> true
        | _, Some note when String.trim note <> "" -> true
        | _ -> false
      in
      if has_tokens || has_text then
        Diagnostic.set_extension "recover" (recover_extension_payload summary) diag
      else diag

let build_parser_diagnostic ~message ~start_pos ~end_pos ~summary =
  Builder.create
    ~message
    ~primary:(Diagnostic.span_of_positions start_pos end_pos)
    ~domain:Diagnostic.Parser
    ()
  |> Builder.set_expected summary
  |> Builder.build

let process_lexer_error ?streaming_summary lexbuf msg =
  let start_pos = Lexing.lexeme_start_p lexbuf in
  let end_pos = Lexing.lexeme_end_p lexbuf in
  match streaming_summary with
  | Some summary ->
      build_parser_diagnostic ~message:msg ~start_pos ~end_pos ~summary
      |> attach_recover_extension (Some summary)
  | None -> Diagnostic.of_lexer_error ~message:msg ~start_pos ~end_pos

let process_parser_error lexbuf message summary =
  let start_pos = Lexing.lexeme_start_p lexbuf in
  let end_pos = Lexing.lexeme_end_p lexbuf in
  build_parser_diagnostic ~message ~start_pos ~end_pos ~summary
  |> attach_recover_extension (Some summary)

let process_rejected_error lexbuf summary =
  let pos = lexbuf.Lexing.lex_curr_p in
  build_parser_diagnostic
    ~message:"構文エラー: 解析を続行できません"
    ~start_pos:pos ~end_pos:pos ~summary
  |> attach_recover_extension (Some summary)

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
  let packrat_cache = Core_stream.packrat_cache session in
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
    packrat_cache;
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

let run ?(config = default_run_config) ?packrat_cache lexbuf =
  Fun.protect
    ~finally:(fun () -> Parser_flags.set_experimental_effects_enabled false)
    (fun () ->
      Parser_flags.set_experimental_effects_enabled config.experimental_effects;
      let pack, config =
        let pack, config = Core_parse_lex.Bridge.derive config in
        match pack with
        | Core_parse_lex.Pack.{ space_id = Some space_id; _ } ->
            Core_parse_lex.Bridge.with_space_id pack config space_id
        | _ -> (pack, config)
      in
      Core_parse_lex.Api.config_trivia pack lexbuf;
      let session = Core_stream.create_session ?packrat_cache config in
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
      let stream_config = Run_config.Stream.of_run_config config in
      let streaming_expected_summary =
        if stream_config.enabled then
          Some (Parser_expectation.streaming_expression_summary ())
        else None
      in
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
              let diag =
                process_lexer_error ?streaming_summary:streaming_expected_summary
                  lexbuf msg
              in
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
      let checkpoint =
        Parser.Incremental.compilation_unit lexbuf.Lexing.lex_curr_p
      in
      let parser state =
        let reply = loop state checkpoint in
        (reply, state)
      in
      let parse_reply =
        try
          let reply, _state =
            Core_parse.rule ~namespace:"menhir" ~name:"compilation_unit"
              parser core_state
          in
          Ok reply
      with Parser_flags.Experimental_effects_disabled (effect_start, effect_end) ->
          Error (effect_start, effect_end)
      in
      let packrat_stats = Core_stream.packrat_counters session in
      let result =
        match parse_reply with
        | Ok reply -> (
            match reply with
            | Core_reply.Ok ok ->
                let span =
                  match ok.span with
                  | Some span -> span
                  | None ->
                      Diagnostic.span_of_positions start_pos
                        lexbuf.Lexing.lex_curr_p
                in
                Parser_diag_state.record_span_trace diag_state
                  ~label:(Some "compilation_unit") ~span;
                finalize_result session ~value:(Some ok.value)
                  ~span:(Some span) ~legacy_error:None ~consumed:ok.consumed
                  ~committed:ok.committed ~packrat_stats
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
                  ~committed:err.committed ~packrat_stats)
        | Error (effect_start, effect_end) ->
            let primary = Diagnostic.span_of_positions effect_start effect_end in
            let diag =
              Builder.create
                ~message:
                  "効果構文は実験フラグ `-Zalgebraic-effects` が無効なため使用できません"
                ~primary ()
              |> Builder.set_severity Diagnostic.Error
              |> Builder.set_domain Diagnostic.Effect
              |> Builder.set_primary_code "effects.syntax.experimental_disabled"
              |> Builder.add_note
                   "`-Zalgebraic-effects` を有効にして再実行してください"
              |> Builder.build
            in
            let consumed = Core_state.consumed core_state in
            let committed = Core_state.committed core_state in
            register_diagnostic session diag ~consumed ~committed;
            let legacy_error =
              diagnostic_to_parse_error diag ~consumed ~committed
            in
            finalize_result session ~value:None ~span:None
              ~legacy_error:(Some legacy_error) ~consumed ~committed
              ~packrat_stats
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
      else result)

let run_partial ?(config = default_run_config) ?packrat_cache lexbuf =
  let cfg = { config with require_eof = false } in
  { result = run ~config:cfg ?packrat_cache lexbuf; rest = None }

let run_string ?(filename = "<入力>") ?(config = default_run_config)
    ?packrat_cache text =
  let lexbuf = Lexing.from_string text in
  lexbuf.Lexing.lex_curr_p <-
    { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = filename };
  run ~config ?packrat_cache lexbuf

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
  module Packrat = Parser_expectation.Packrat
  module Effects_cfg = Run_config.Effects
  module Bridge_registry = Runtime_bridge_registry

  let normalize_stage stage =
    stage |> String.trim |> String.lowercase_ascii

  let default_stage_required = "beta"
  let fallback_stage_actual = "experimental"

  let stage_rank = function
    | "experimental" -> 0
    | "beta" -> 1
    | "stable" -> 2
    | _ -> 3

  let stage_satisfies ~required ~actual =
    stage_rank (normalize_stage actual) >= stage_rank (normalize_stage required)

  let default_bridge_id = "core.parse.streaming"

  type flow_state = {
    config : Stream_cfg.Flow.t;
    mutable events : int;
  }

  type demand_action = [ `Pause | `Continue ]

  type demand_hint = {
    action : demand_action;
    min_bytes : int option;
    preferred_bytes : int option;
    resume_hint : string option;
    reason : string option;
  }

  type snapshot = {
    expectations : Diagnostic.expectation list;
    summary : Diagnostic.expectation_summary option;
    checkpoint : Diagnostic.span option;
    diagnostic : Diagnostic.t option;
  }

  let empty_snapshot =
    { expectations = []; summary = None; checkpoint = None; diagnostic = None }

  let json_of_option f = function
    | Some value -> f value
    | None -> `Null

  let json_of_string_option = json_of_option (fun s -> `String s)
  let json_of_int_option = json_of_option (fun n -> `Int n)

  let string_list_json entries =
    `List (List.map (fun value -> `String value) entries)

  let span_to_json (span : Diagnostic.span) =
    let open Diagnostic in
    let location_to_json (loc : location) =
      `Assoc
        [
          ("file", `String loc.filename);
          ("line", `Int loc.line);
          ("column", `Int loc.column);
          ("offset", `Int loc.offset);
        ]
    in
    `Assoc
      [
        ("start", location_to_json span.start_pos);
        ("end", location_to_json span.end_pos);
      ]

  let demand_hint_to_json (demand : demand_hint) =
    let action =
      match demand.action with `Pause -> "pause" | `Continue -> "continue"
    in
    `Assoc
      [
        ("action", `String action);
        ("min_bytes", json_of_int_option demand.min_bytes);
        ("preferred_bytes", json_of_int_option demand.preferred_bytes);
        ("resume_hint", json_of_string_option demand.resume_hint);
        ("reason", json_of_string_option demand.reason);
      ]

  let expectations_to_json expectations =
    `List (List.map expectation_to_json expectations)

  let snapshot_for_buffer ~buffer ~config ~source_name =
    if buffer = "" then empty_snapshot
    else
      let preview_config = { config with Run_config.require_eof = true } in
      let preview =
        run_string ~filename:source_name ~config:preview_config buffer
      in
      match preview.diagnostics with
      | diag :: _ ->
          let summary = diag.Diagnostic.expected in
          let expectations =
            match summary with
            | Some summary -> summary.Diagnostic.alternatives
            | None -> []
          in
          {
            expectations;
            summary;
            checkpoint = Some diag.Diagnostic.primary;
            diagnostic = Some diag;
          }
      | [] -> empty_snapshot

  let snapshot_of_diagnostic (diag : Diagnostic.t) =
    let summary = diag.Diagnostic.expected in
    let expectations =
      match summary with
      | Some summary -> summary.Diagnostic.alternatives
      | None -> []
    in
    {
      expectations;
      summary;
      checkpoint = Some diag.Diagnostic.primary;
      diagnostic = Some diag;
    }


  type stream_meta = {
    bytes_consumed : int;
    chunks_consumed : int;
    await_count : int;
    resume_count : int;
    last_reason : string option;
    memo_bytes : int option;
    backpressure_policy : string option;
    backpressure_events : int;
  }

  type continuation_meta = {
    commit_watermark : int;
    buffered_bytes : int;
    resume_hint : demand_hint option;
    expected_tokens : Diagnostic.expectation list;
    last_checkpoint : Diagnostic.span option;
    resume_lineage : string list;
    backpressure_counter : int;
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
    packrat_cache : Packrat.t option;
    meta : continuation_meta;
    flow : flow_state;
    diagnostics : Diagnostic.t list;
  }

  let default_span_for_file filename =
    let open Diagnostic in
    let location = { filename; line = 0; column = 0; offset = 0 } in
    { start_pos = location; end_pos = location }

  let reason_is_backpressure = function
    | Some reason -> String.equal (String.trim reason) "pending.backpressure"
    | None -> false

  let synthesize_streaming_recover ~source_name snapshot =
    let span =
      match snapshot.checkpoint with
      | Some span -> span
      | None -> default_span_for_file source_name
    in
    let summary =
      match snapshot.summary with
      | Some summary -> summary
      | None -> Parser_expectation.streaming_expression_summary ()
    in
    let summary = Parser_expectation.ensure_minimum_alternatives summary
    in
    Builder.create
      ~message:"ストリーミング入力が未完のため、追加トークンを待機しています"
      ~primary:span ()
    |> Builder.set_expected summary
    |> Builder.build

  let diagnostic_with_expected ~snapshot diag =
    match diag.Diagnostic.expected with
    | Some summary when summary.Diagnostic.alternatives <> [] -> diag
    | _ ->
        let summary =
          match snapshot.summary with
          | Some summary -> summary
          | None -> Parser_expectation.streaming_expression_summary ()
        in
        let summary = Parser_expectation.ensure_minimum_alternatives summary
        in
        { diag with expected = Some summary }

  let recover_diagnostics_for_pending ~source_name ~demand ~snapshot =
    let needs_recover = reason_is_backpressure demand.reason in
    match snapshot.diagnostic with
    | Some diag when needs_recover ->
        [ diagnostic_with_expected ~snapshot diag;
          synthesize_streaming_recover ~source_name snapshot ]
    | Some diag -> [ diagnostic_with_expected ~snapshot diag ]
    | None when needs_recover ->
        [ synthesize_streaming_recover ~source_name snapshot ]
    | None -> []

  let build_bridge_stage_diagnostic ~source_name ~config ~defaults:_
      ~(flow_state : flow_state)
      ~demand ~(meta : stream_meta) ~(snapshot : snapshot) =
    match flow_state.config.policy with
    | Stream_cfg.Flow.Manual -> []
    | Stream_cfg.Flow.Auto -> (
        let reason =
          match demand.reason with
          | Some r when String.trim r <> "" -> Some r
          | _ -> meta.last_reason
        in
        match reason with
        | Some reason when String.equal reason "pending.backpressure" ->
            let required_stage = default_stage_required in
            let actual_stage =
              match Effects_cfg.of_run_config config with
              | { Effects_cfg.stage_override = Some stage; _ }
                when String.trim stage <> "" ->
                  normalize_stage stage
              | _ -> fallback_stage_actual
            in
            if stage_satisfies ~required:required_stage ~actual:actual_stage then
              []
            else
              let span =
                match snapshot.checkpoint with
                | Some span -> span
                | None -> default_span_for_file source_name
              in
              let policy_label =
                match flow_state.config.policy with
                | Stream_cfg.Flow.Manual -> "manual"
                | Stream_cfg.Flow.Auto -> "auto"
              in
              let signal : Bridge_registry.stream_signal =
                {
                  bridge_id = default_bridge_id;
                  span;
                  policy = policy_label;
                  reason;
                  demand = demand_hint_to_json demand;
                  await_count = meta.await_count;
                  resume_count = meta.resume_count;
                  backpressure_events = meta.backpressure_events;
                  stage_required = required_stage;
                  stage_actual = actual_stage;
                }
              in
              Bridge_registry.stream_signal signal
        | _ -> [] )

  let make_pending_event ~demand ~(meta : stream_meta)
      ~(continuation : continuation) ~(snapshot : snapshot) =
    let base_metadata =
      [
        ("parser.stream.pending.resume_hint", demand_hint_to_json demand);
        ("parser.stream.pending.last_reason", json_of_string_option meta.last_reason);
        ( "parser.stream.pending.expected_tokens",
          expectations_to_json snapshot.expectations );
        ( "parser.stream.pending.resume_lineage",
          string_list_json continuation.meta.resume_lineage );
        ( "parser.stream.pending.backpressure_events",
          `Int meta.backpressure_events );
      ]
    in
    let metadata =
      match snapshot.checkpoint with
      | Some span ->
          ("parser.stream.pending.last_checkpoint", span_to_json span)
          :: base_metadata
      | None -> base_metadata
    in
    Audit_envelope.make ~category:"parser.stream.pending"
      ~metadata_pairs:(List.rev metadata) ()

  let make_error_event ~demand ~meta ~continuation ~(snapshot : snapshot)
      ~(diagnostic : Diagnostic.t) =
    let demand = (demand : demand_hint option) in
    let meta = (meta : stream_meta option) in
    let continuation = (continuation : continuation option) in
    let expectations =
      if snapshot.expectations <> [] then snapshot.expectations
      else
        match diagnostic.Diagnostic.expected with
        | Some summary -> summary.Diagnostic.alternatives
        | None -> []
    in
    let resume_hint_json =
      match demand with
      | Some hint -> demand_hint_to_json hint
      | None -> `Null
    in
    let last_reason_json =
      match meta with
      | Some stream_meta -> json_of_string_option stream_meta.last_reason
      | None -> `Null
    in
    let base_metadata =
      [
        ( "parser.stream.error.diagnostic",
          Diagnostic_serialization.diagnostic_to_json diagnostic );
        ("parser.stream.error.resume_hint", resume_hint_json);
        ("parser.stream.error.last_reason", last_reason_json);
        ( "parser.stream.error.expected_tokens",
          expectations_to_json expectations );
      ]
    in
    let metadata =
      match snapshot.checkpoint with
      | Some span ->
          ("parser.stream.error.last_checkpoint", span_to_json span)
          :: base_metadata
      | None -> base_metadata
    in
    let metadata =
      match continuation with
      | Some cont ->
          ("parser.stream.error.resume_lineage",
           string_list_json cont.meta.resume_lineage)
          :: metadata
      | None -> metadata
    in
    Audit_envelope.make ~category:"parser.stream.error"
      ~metadata_pairs:(List.rev metadata) ()

  let error_events_from_result ~(meta : stream_meta) (result : parse_result) =
    result.diagnostics
    |> List.filter (fun diag -> diag.Diagnostic.severity = Diagnostic.Error)
    |> List.map (fun diag ->
           let snapshot = snapshot_of_diagnostic diag in
           make_error_event ~demand:None ~meta:(Some meta) ~continuation:None
             ~snapshot ~diagnostic:diag)

  type chunk =
    | Chunk of string
    | Await of demand_hint option
    | Closed
    | Error of string

  type feeder = unit -> chunk

  type completed = {
    result : parse_result;
    meta : stream_meta;
    audit_events : Audit_envelope.event list;
  }

  type pending = {
    continuation : continuation;
    demand : demand_hint;
    meta : stream_meta;
    audit_events : Audit_envelope.event list;
    diagnostics : Diagnostic.t list;
  }

  type outcome = Completed of completed | Pending of pending

  let flow_policy_label (state : flow_state) =
    match state.config.policy with
    | Stream_cfg.Flow.Manual -> None
    | Stream_cfg.Flow.Auto -> Some "auto"

  let init_flow_state (defaults : Stream_cfg.t) : flow_state =
    let open Stream_cfg.Flow in
    let config =
      match defaults.flow with
      | Some flow -> flow
      | None ->
          {
            policy = Manual;
            backpressure =
              { max_lag_bytes = None; debounce_ms = None; throttle_ratio = None };
          }
    in
    { config; events = 0 }

  let apply_backpressure_defaults
      (defaults : Stream_cfg.t)
      (flow_state : flow_state)
      (demand : demand_hint) : demand_hint =
    match flow_state.config.policy with
    | Stream_cfg.Flow.Manual -> demand
    | Stream_cfg.Flow.Auto ->
        flow_state.events <- flow_state.events + 1;
        let max_lag = flow_state.config.backpressure.max_lag_bytes in
        let throttle_ratio =
          match flow_state.config.backpressure.throttle_ratio with
          | Some ratio when Float.is_finite ratio && ratio > 0.0 -> ratio
          | _ -> 1.0
        in
        let clamp_positive value =
          match value with
          | Some v when v > 0 -> Some v
          | Some _ -> Some 1
          | None -> None
        in
        let fallback opt default_value =
          match opt with Some _ -> opt | None -> default_value
        in
        let base_min =
          clamp_positive
            (fallback demand.min_bytes defaults.demand_min_bytes)
        in
        let base_pref =
          clamp_positive
            (fallback demand.preferred_bytes defaults.demand_preferred_bytes)
        in
        let throttled_pref =
          match base_pref with
          | Some pref ->
              let scaled =
                int_of_float
                  (Float.ceil (float_of_int pref *. throttle_ratio))
              in
              Some (max 1 scaled)
          | None -> None
        in
        let min_bytes =
          match (base_min, max_lag) with
          | Some minv, Some cap -> Some (min minv cap)
          | Some minv, None -> Some minv
          | None, Some cap -> Some cap
          | None, None -> None
        in
        let preferred_bytes =
          let capped_pref =
            match (throttled_pref, max_lag) with
            | Some pref, Some cap -> Some (min pref cap)
            | Some pref, None -> Some pref
            | None, Some cap -> Some cap
            | None, None -> None
          in
          match (capped_pref, min_bytes) with
          | Some pref, Some minv when pref < minv -> Some minv
          | Some pref, _ -> Some pref
          | None, Some minv -> Some minv
          | None, None -> None
        in
        {
          demand with
          min_bytes;
          preferred_bytes;
          reason = Some "pending.backpressure";
        }

  let demand_pause ?min_bytes ?preferred_bytes ?resume_hint ?reason () =
    { action = `Pause; min_bytes; preferred_bytes; resume_hint; reason }

  let demand_continue ?min_bytes ?preferred_bytes ?resume_hint ?reason () =
    { action = `Continue; min_bytes; preferred_bytes; resume_hint; reason }

  let packrat_bytes packrat_cache =
    match packrat_cache with
    | None -> None
    | Some cache ->
        let stats = Packrat.metrics cache in
        Some stats.approx_bytes

  let make_meta ~bytes ~chunks ~await ~resume ?reason ?packrat_cache
      (flow_state : flow_state) =
    {
      bytes_consumed = bytes;
      chunks_consumed = chunks;
      await_count = await;
      resume_count = resume;
      last_reason = reason;
      memo_bytes = packrat_bytes packrat_cache;
      backpressure_policy = flow_policy_label flow_state;
      backpressure_events = flow_state.events;
    }

  let build_continuation_meta ~buffer ~demand ~flow_state ?snapshot ?previous ()
      =
    let buffered_bytes = String.length buffer in
    let lineage_base =
      match previous with Some meta -> meta.resume_lineage | None -> []
    in
    let reason =
      match demand.reason with Some r -> r | None -> "stream.pending"
    in
    let snapshot = Option.value ~default:empty_snapshot snapshot in
    {
      commit_watermark = buffered_bytes;
      buffered_bytes;
      resume_hint = Some demand;
      expected_tokens = snapshot.expectations;
      last_checkpoint = snapshot.checkpoint;
      resume_lineage = reason :: lineage_base;
      backpressure_counter = flow_state.events;
    }

  let fallback first second = match first with Some _ -> first | None -> second

  let merge_hint (defaults : Stream_cfg.t) ?fallback_reason
      (hint : demand_hint) : demand_hint =
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

  let default_pause (defaults : Stream_cfg.t) ?(reason : string option = None) ()
      : demand_hint =
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
    let flow_state = init_flow_state defaults in
    let packrat_cache = if config.packrat then Some (Packrat.create ()) else None in
    let buffer = Buffer.create 4096 in
    let rec pump bytes_consumed chunks_consumed await_count packrat_cache =
      match feeder () with
      | Chunk data ->
          Buffer.add_string buffer data;
          let len = String.length data in
          pump (bytes_consumed + len) (chunks_consumed + 1) await_count
            packrat_cache
      | Await hint_opt ->
          let demand =
            match hint_opt with
            | Some hint ->
                merge_hint defaults ~fallback_reason:"feeder.await" hint
            | None -> default_pause defaults ~reason:(Some "feeder.await") ()
          in
          let demand =
            apply_backpressure_defaults defaults flow_state demand
          in
          let buffered = Buffer.contents buffer in
          let snapshot =
            snapshot_for_buffer ~buffer:buffered ~config ~source_name:filename
          in
          let packrat_cache =
            match packrat_cache with
            | Some cache ->
                Packrat.prune_before cache ~offset:(String.length buffered);
                Some cache
            | None -> None
          in
          let pending_recover_diags =
            recover_diagnostics_for_pending ~source_name:filename ~demand
              ~snapshot
          in
          let continuation =
            {
              config;
              source_name = filename;
              buffer = buffered;
              bytes_consumed;
              chunks_consumed;
              await_count = await_count + 1;
              resume_count = 0;
              resume_hint = demand.resume_hint;
              demand_min_bytes = demand.min_bytes;
              demand_preferred_bytes = demand.preferred_bytes;
              packrat_cache;
              meta =
                build_continuation_meta ~buffer:buffered ~demand ~flow_state
                  ~snapshot ();
              flow = flow_state;
              diagnostics = pending_recover_diags;
            }
          in
          let meta =
            make_meta ~bytes:bytes_consumed ~chunks:chunks_consumed
              ~await:(await_count + 1) ~resume:0 ?reason:demand.reason
              ?packrat_cache flow_state
          in
          let diagnostics =
            continuation.diagnostics
            @ build_bridge_stage_diagnostic ~source_name:filename ~config
                ~defaults ~flow_state ~demand ~meta ~snapshot
          in
          let continuation = { continuation with diagnostics } in
          let pending_event =
            make_pending_event ~demand ~meta ~continuation ~snapshot
          in
          let audit_events =
            match snapshot.diagnostic with
            | Some diagnostic ->
                let error_event =
                  make_error_event ~demand:(Some demand) ~meta:(Some meta)
                    ~continuation:(Some continuation) ~snapshot ~diagnostic
                in
                [ pending_event; error_event ]
            | None -> [ pending_event ]
          in
          Pending
            {
              continuation;
              demand;
              meta;
              audit_events;
              diagnostics = continuation.diagnostics;
            }
      | Closed ->
          let text = Buffer.contents buffer in
          let result = run_string ~filename ~config ?packrat_cache text in
          let meta =
            make_meta ~bytes:bytes_consumed ~chunks:chunks_consumed
              ~await:await_count ~resume:0 ?reason:None ?packrat_cache
              flow_state
          in
          let audit_events = error_events_from_result ~meta result in
          Completed { result; meta; audit_events }
      | Error message ->
          let demand =
            default_pause defaults ~reason:(Some ("stream.error:" ^ message)) ()
          in
          let buffered = Buffer.contents buffer in
          let snapshot =
            snapshot_for_buffer ~buffer:buffered ~config ~source_name:filename
          in
        let packrat_cache =
          match packrat_cache with
          | Some cache ->
              Packrat.prune_before cache ~offset:(String.length buffered);
              Some cache
          | None -> None
        in
        let continuation =
          {
            config;
            source_name = filename;
            buffer = buffered;
            bytes_consumed;
            chunks_consumed;
            await_count;
            resume_count = 0;
            resume_hint = demand.resume_hint;
            demand_min_bytes = demand.min_bytes;
            demand_preferred_bytes = demand.preferred_bytes;
            packrat_cache;
            meta =
              build_continuation_meta ~buffer:buffered ~demand ~flow_state
                ~snapshot ();
            flow = flow_state;
            diagnostics = [];
          }
        in
        let meta =
          make_meta ~bytes:bytes_consumed ~chunks:chunks_consumed
            ~await:await_count ~resume:0 ?reason:demand.reason
            ?packrat_cache flow_state
        in
        let diagnostics =
          build_bridge_stage_diagnostic ~source_name:filename ~config ~defaults
            ~flow_state ~demand ~meta ~snapshot
        in
        let continuation = { continuation with diagnostics } in
        let pending_event =
          make_pending_event ~demand ~meta ~continuation ~snapshot
        in
          let audit_events =
            match snapshot.diagnostic with
            | Some diagnostic ->
                let error_event =
                  make_error_event ~demand:(Some demand) ~meta:(Some meta)
                    ~continuation:(Some continuation) ~snapshot ~diagnostic
                in
                [ pending_event; error_event ]
            | None -> [ pending_event ]
          in
          Pending
            {
              continuation;
              demand;
              meta;
              audit_events;
              diagnostics = continuation.diagnostics;
            }
    in
    pump 0 0 0 packrat_cache

  let resume (continuation : continuation) (input : chunk) : outcome =
    let defaults = Stream_cfg.of_run_config continuation.config in
    let flow_state = continuation.flow in
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
    let packrat_cache =
      match continuation.packrat_cache with
      | Some cache ->
          Packrat.prune_before cache
            ~offset:continuation.meta.commit_watermark;
          Some cache
      | None -> None
    in
    match input with
    | Chunk data ->
        Buffer.add_string buffer data;
        let text = Buffer.contents buffer in
        let result =
          run_string ~filename:continuation.source_name
            ~config:continuation.config ?packrat_cache text
        in
        let result =
          if continuation.diagnostics = [] then result
          else
            {
              result with
              diagnostics =
                continuation.diagnostics @ result.diagnostics;
            }
        in
        let meta =
          make_meta ~bytes:(bytes + String.length data) ~chunks:(chunks + 1)
            ~await ~resume:resume_count ?reason:None ?packrat_cache flow_state
        in
        let audit_events = error_events_from_result ~meta result in
        Completed { result; meta; audit_events }
    | Closed ->
        let text = Buffer.contents buffer in
        let result =
          run_string ~filename:continuation.source_name
            ~config:continuation.config ?packrat_cache text
        in
        let result =
          if continuation.diagnostics = [] then result
          else
            {
              result with
              diagnostics =
                continuation.diagnostics @ result.diagnostics;
            }
        in
        let meta =
          make_meta ~bytes ~chunks ~await ~resume:resume_count
            ?reason:(Some "feeder.closed") ?packrat_cache flow_state
        in
        let audit_events = error_events_from_result ~meta result in
        Completed { result; meta; audit_events }
    | Await hint_opt ->
        let fallback_reason = Some "feeder.await" in
        let demand =
          match hint_opt with
          | Some hint ->
              merge_hint defaults
                ~fallback_reason:"feeder.await" hint
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
        let demand = apply_backpressure_defaults defaults flow_state demand in
        let new_buffer = Buffer.contents buffer in
        let snapshot =
          snapshot_for_buffer ~buffer:new_buffer
            ~config:continuation.config
            ~source_name:continuation.source_name
        in
        let previous_meta = continuation.meta in
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
            packrat_cache;
            meta =
              build_continuation_meta ~buffer:new_buffer ~demand ~flow_state
                ~snapshot ~previous:previous_meta ();
            flow = continuation.flow;
          }
        in
        let recover_diags =
          recover_diagnostics_for_pending
            ~source_name:continuation.source_name ~demand ~snapshot
        in
        let meta =
          make_meta ~bytes ~chunks ~await:(await + 1) ~resume:resume_count
            ?reason:demand.reason ?packrat_cache flow_state
        in
        let additional_diagnostics =
          build_bridge_stage_diagnostic
            ~source_name:continuation.source_name ~config:continuation.config
            ~defaults ~flow_state ~demand ~meta ~snapshot
        in
        let diagnostics =
          (continuation.diagnostics @ recover_diags) @ additional_diagnostics
        in
        let continuation = { continuation with diagnostics } in
        let pending_event =
          make_pending_event ~demand ~meta ~continuation ~snapshot
        in
        let audit_events =
          match snapshot.diagnostic with
          | Some diagnostic ->
              let error_event =
                make_error_event ~demand:(Some demand) ~meta:(Some meta)
                  ~continuation:(Some continuation) ~snapshot ~diagnostic
              in
              [ pending_event; error_event ]
          | None -> [ pending_event ]
        in
        Pending
          {
            continuation;
            demand;
            meta;
            audit_events;
            diagnostics = continuation.diagnostics;
          }
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
        let snapshot =
          snapshot_for_buffer ~buffer:new_buffer
            ~config:continuation.config
            ~source_name:continuation.source_name
        in
        let previous_meta = continuation.meta in
        let continuation =
          {
            continuation with
            buffer = new_buffer;
            resume_count;
            resume_hint = demand.resume_hint;
            packrat_cache;
            meta =
              build_continuation_meta ~buffer:new_buffer ~demand ~flow_state
                ~snapshot ~previous:previous_meta ();
            flow = continuation.flow;
          }
        in
        let meta =
          make_meta ~bytes ~chunks ~await ~resume:resume_count ?reason
            ?packrat_cache flow_state
        in
        let additional_diagnostics =
          build_bridge_stage_diagnostic
            ~source_name:continuation.source_name ~config:continuation.config
            ~defaults ~flow_state ~demand ~meta ~snapshot
        in
        let diagnostics =
          continuation.diagnostics @ additional_diagnostics
        in
        let continuation = { continuation with diagnostics } in
        let pending_event =
          make_pending_event ~demand ~meta ~continuation ~snapshot
        in
        let audit_events =
          match snapshot.diagnostic with
          | Some diagnostic ->
              let error_event =
                make_error_event ~demand:(Some demand) ~meta:(Some meta)
                  ~continuation:(Some continuation) ~snapshot ~diagnostic
              in
              [ pending_event; error_event ]
          | None -> [ pending_event ]
        in
        Pending
          {
            continuation;
            demand;
            meta;
            audit_events;
            diagnostics = continuation.diagnostics;
          }
end
