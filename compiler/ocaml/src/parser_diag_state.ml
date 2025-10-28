(* parser_diag_state.ml — Parser 診断状態
 *
 * Phase 2-5: `ParseResult` シム導入に合わせ、最遠エラー統計や
 * 期待集合の集約を行う補助モジュール。
 *)

type farthest_snapshot = {
  offset : int;
  expected : Diagnostic.expectation list;
  expected_summary : Diagnostic.expectation_summary option;
  committed : bool;
  far_consumed : bool;
}

type t = {
  mutable farthest : farthest_snapshot option;
  mutable diagnostics_rev : Diagnostic.t list;
  mutable recovered : bool;
}

let create () = { farthest = None; diagnostics_rev = []; recovered = false }

let record_recovery t = t.recovered <- true

let normalize_expectations expectations =
  expectations |> List.sort_uniq Stdlib.compare

let record_diagnostic t ~diagnostic ~committed ~consumed =
  t.diagnostics_rev <- diagnostic :: t.diagnostics_rev;
  let offset = diagnostic.Diagnostic.primary.start_pos.offset in
  let summary = diagnostic.Diagnostic.expected in
  let expected =
    match summary with Some summary -> summary.alternatives | None -> []
  in
  let snapshot =
    { offset; expected; expected_summary = summary; committed; far_consumed = consumed }
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
            }

let diagnostics t = List.rev t.diagnostics_rev

let recovered t = t.recovered

let farthest_snapshot t = t.farthest

let farthest_offset t = Option.map (fun entry -> entry.offset) t.farthest
