(* parser_diag_state.ml — Parser 診断状態
 *
 * Phase 2-5: `ParseResult` シム導入に合わせ、RunConfig/DiagState の
 * 追加スイッチ（trace / merge_warnings / locale）を保持する。
 *)

module Recover_config = Parser_run_config.Recover

type farthest_snapshot = {
  offset : int;
  expected : Diagnostic.expectation list;
  expected_summary : Diagnostic.expectation_summary option;
  committed : bool;
  far_consumed : bool;
}

type span_trace_entry = {
  label : string option;
  span : Diagnostic.span;
}

type warning_signature = string * string option

type t = {
  mutable farthest : farthest_snapshot option;
  mutable diagnostics_rev : Diagnostic.t list;
  mutable recovered : bool;
  trace_enabled : bool;
  mutable span_trace_rev : span_trace_entry list;
  merge_warnings : bool;
  mutable warning_signatures : warning_signature list;
  locale : string option;
  recover_config : Recover_config.t;
}

let create ?(trace = false) ?(merge_warnings = true) ?locale
    ?(recover = Recover_config.default) () =
  {
    farthest = None;
    diagnostics_rev = [];
    recovered = false;
    trace_enabled = trace;
    span_trace_rev = [];
    merge_warnings;
    warning_signatures = [];
    locale;
    recover_config = recover;
  }

let trace_enabled t = t.trace_enabled
let locale t = t.locale
let recover_config t = t.recover_config
let recover_sync_tokens t = t.recover_config.sync_tokens
let recover_notes_enabled t = t.recover_config.emit_notes

let record_span_trace t ~label ~span =
  if t.trace_enabled then
    t.span_trace_rev <- { label; span } :: t.span_trace_rev

let span_trace t =
  if (not t.trace_enabled) || t.span_trace_rev = [] then None
  else Some (List.rev t.span_trace_rev)

let span_trace_pairs t =
  span_trace t
  |> Option.map (List.map (fun { label; span } -> (label, span)))

let record_recovery t = t.recovered <- true

let normalize_expectations expectations =
  expectations |> List.sort_uniq Stdlib.compare

let primary_code (diag : Diagnostic.t) =
  match diag.Diagnostic.codes with code :: _ -> Some code | [] -> None

let should_record_warning t (diag : Diagnostic.t) =
  if not t.merge_warnings then true
  else
    let signature = (diag.Diagnostic.message, primary_code diag) in
    if List.exists (fun entry -> entry = signature) t.warning_signatures then
      false
    else (
      t.warning_signatures <- signature :: t.warning_signatures;
      true)

let record_diagnostic t ~diagnostic ~committed ~consumed =
  let open Diagnostic in
  let severity = diagnostic.severity in
  let allow_append =
    match severity with
    | Warning -> should_record_warning t diagnostic
    | Error | Info | Hint -> true
  in
  if allow_append then (
    t.diagnostics_rev <- diagnostic :: t.diagnostics_rev;
    if severity = Error then (
      let offset = diagnostic.primary.start_pos.offset in
      let summary = diagnostic.expected in
      let expected =
        match summary with Some summary -> summary.alternatives | None -> []
      in
      let snapshot =
        {
          offset;
          expected;
          expected_summary = summary;
          committed;
          far_consumed = consumed;
        }
      in
      match t.farthest with
      | None -> t.farthest <- Some snapshot
      | Some prev ->
          if snapshot.offset > prev.offset then t.farthest <- Some snapshot
          else if snapshot.offset = prev.offset then
            let merged_expected =
              normalize_expectations (prev.expected @ snapshot.expected)
            in
            let merged_summary =
              match (snapshot.expected_summary, prev.expected_summary) with
              | Some s, _ -> Some { s with alternatives = merged_expected }
              | None, Some prev_summary ->
                  Some { prev_summary with alternatives = merged_expected }
              | None, None -> None
            in
            t.farthest <-
              Some
                {
                  snapshot with
                  expected = merged_expected;
                  expected_summary = merged_summary;
                }))

let record_warning t ~diagnostic =
  record_diagnostic t ~diagnostic ~committed:false ~consumed:false

let diagnostics t = List.rev t.diagnostics_rev

let recovered t = t.recovered

let farthest_snapshot t = t.farthest

let farthest_offset t = Option.map (fun entry -> entry.offset) t.farthest
