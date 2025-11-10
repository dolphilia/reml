(* Core Parse Streaming — バッチランナーとストリーミングランナーで共有する骨格ロジック
 *
 * Phase 2-5 EXEC-001 Step 1: Core.Parse.Streaming モジュール骨格の抽出
 *)

module Run_config = Parser_run_config
module Core_state = Core_parse.State
module Extensions = Diagnostic.Extensions

type packrat_cache = Parser_expectation.Packrat.t option

type session = {
  config : Run_config.t;
  diag_state : Parser_diag_state.t;
  core_state : Core_state.t;
  packrat_cache : packrat_cache;
}

let config t = t.config
let diag_state t = t.diag_state
let core_state t = t.core_state
let packrat_cache t = t.packrat_cache

let parse_expected_empty_key = "parse.expected.empty"

let streaming_summary_if_needed session summary =
  let stream_config = Run_config.Stream.of_run_config session.config in
  if not stream_config.Run_config.Stream.enabled then summary
  else
    match summary.Diagnostic.message_key with
    | Some key when String.equal key parse_expected_empty_key ->
        Parser_expectation.streaming_expression_summary ()
    | _ -> summary

let create_session ?packrat_cache config =
  let recover_config = Run_config.Recover.of_run_config config in
  let diag_state =
    Parser_diag_state.create ~trace:config.trace
      ~merge_warnings:config.merge_warnings ?locale:config.locale
      ~recover:recover_config ()
  in
  let packrat_cache =
    match (packrat_cache, config.packrat) with
    | Some cache, _ -> Some cache
    | None, true -> Some (Parser_expectation.Packrat.create ())
    | None, false -> None
  in
  let core_state = Core_state.create ~config ~diag:diag_state in
  { config; diag_state; core_state; packrat_cache }

let effective_require_eof config =
  match Run_config.Config.find config with
  | None -> config.require_eof
  | Some namespace -> (
      match Run_config.Config.require_eof_override namespace with
      | Some value -> value
      | None -> config.require_eof)

let summarize_snapshot snapshot =
  match snapshot.Parser_diag_state.expected_summary with
  | Some summary -> summary
  | None -> Parser_expectation.summarize_with_defaults snapshot.expected

let record_packrat_status state = function
  | `Hit -> Core_state.record_packrat_access state ~hit:true
  | `Miss -> Core_state.record_packrat_access state ~hit:false
  | `Bypassed -> ()

let expectation_summary_for_checkpoint session checkpoint =
  let collection, status =
    Parser_expectation.collect ~checkpoint ~packrat:session.packrat_cache
  in
  record_packrat_status session.core_state status;
  (match (session.packrat_cache, status) with
  | Some cache, (`Hit | `Miss) ->
      let warm_consumers =
        [ `Cli_json; `Cli_text; `Lsp; `Audit; `Metrics; `Telemetry; `Debug ]
      in
      List.iter
        (fun _tag ->
          let _, warm_status =
            Parser_expectation.collect ~checkpoint ~packrat:(Some cache)
          in
          record_packrat_status session.core_state warm_status)
        warm_consumers
  | _ -> ());
  let summary =
    if collection.expectations <> [] then collection.summary
    else
      match Parser_diag_state.farthest_snapshot session.diag_state with
      | Some snapshot -> summarize_snapshot snapshot
      | None -> Parser_expectation.empty_summary
  in
  let summary = Parser_expectation.ensure_minimum_alternatives summary in
  streaming_summary_if_needed session summary

let register_diagnostic session diagnostic ~consumed ~committed =
  let enriched =
    Diagnostic.with_parser_runconfig_metadata ~config:session.config diagnostic
  in
  Parser_diag_state.record_diagnostic session.diag_state ~diagnostic:enriched
    ~committed ~consumed

let diagnostics session = Parser_diag_state.diagnostics session.diag_state

let recovered session = Parser_diag_state.recovered session.diag_state

let farthest_error_offset session =
  Parser_diag_state.farthest_offset session.diag_state

let span_trace_pairs session =
  Parser_diag_state.span_trace_pairs session.diag_state

let packrat_counters session =
  match session.packrat_cache with
  | None -> None
  | Some _ ->
      let queries = Core_state.packrat_queries session.core_state in
      let hits = Core_state.packrat_hits session.core_state in
      Some (queries, hits)

let annotate_core_rule_metadata diag id_opt =
  match id_opt with
  | None -> diag
  | Some id ->
      let namespace = Core_parse.Id.namespace id in
      let name = Core_parse.Id.name id in
      let ordinal = Core_parse.Id.ordinal id in
      let origin =
        match Core_parse.Id.origin id with
        | `Static -> "static"
        | `Dynamic -> "dynamic"
      in
      let fingerprint =
        Core_parse.Id.fingerprint id |> Int64.to_string
      in
      let diag =
        Diagnostic.merge_audit_metadata
          [
            ("parser.core.rule.namespace", `String namespace);
            ("parser.core.rule.name", `String name);
            ("parser.core.rule.ordinal", `Int ordinal);
            ("parser.core.rule.origin", `String origin);
            ("parser.core.rule.fingerprint", `String fingerprint);
            ("namespace", `String namespace);
            ("name", `String name);
            ("origin", `String origin);
            ("fingerprint", `String fingerprint);
          ]
          diag
      in
      let existing_parse =
        match Extensions.get "parse" diag.extensions with
        | Some (`Assoc fields) -> fields
        | _ -> []
      in
      let filtered =
        List.filter (fun (key, _) -> not (String.equal key "parser_id"))
          existing_parse
      in
      let parser_id =
        ("parser_id",
         `Assoc
           [
             ("namespace", `String namespace);
             ("name", `String name);
             ("ordinal", `Int ordinal);
             ("origin", `String origin);
             ("fingerprint", `String fingerprint);
           ])
      in
      Diagnostic.set_extension "parse" (`Assoc (parser_id :: filtered)) diag
