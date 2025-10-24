(* parser_diag_state.ml — Parser 診断状態
 *
 * Phase 2-5: `ParseResult` シム導入に合わせ、最遠エラー統計や
 * 期待集合の集約を行う補助モジュール。
 *)

type farthest_snapshot = {
  offset : int;
  expected : Diagnostic.expectation list;
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
  let expected =
    match diagnostic.Diagnostic.expected with
    | Some summary -> summary.alternatives
    | None -> []
  in
  let snapshot =
    { offset; expected; committed; far_consumed = consumed }
  in
  match t.farthest with
  | None -> t.farthest <- Some snapshot
  | Some prev ->
      if snapshot.offset > prev.offset then t.farthest <- Some snapshot
      else if snapshot.offset = prev.offset then
        t.farthest <-
          Some
            {
              snapshot with
              expected = normalize_expectations (prev.expected @ snapshot.expected);
            }

let diagnostics t = List.rev t.diagnostics_rev

let recovered t = t.recovered

let farthest_snapshot t = t.farthest

let farthest_offset t = Option.map (fun entry -> entry.offset) t.farthest
