(* test_parse_result_state.ml — DiagState と最遠エラー統計の検証 *)

open Parser_driver

let expect_failure_with_offset desc input =
  match run_string input with
  | {
      value = None;
      farthest_error_offset = Some offset;
      diagnostics = _ :: _;
      _;
    } ->
      if offset >= 0 then Printf.printf "✓ %s (offset=%d)\n" desc offset
      else (
        Printf.printf "✗ %s: invalid offset %d\n" desc offset;
        exit 1)
  | { value = None; farthest_error_offset = None; _ } ->
      Printf.printf "✗ %s: missing farthest offset\n" desc;
      exit 1
  | { value = Some _; _ } ->
      Printf.printf "✗ %s: expected failure but parser succeeded\n" desc;
      exit 1
  | _ ->
      Printf.printf "✗ %s: parser state did not match expectations\n" desc;
      exit 1

let expect_diagnostics_recorded desc input =
  match run_string input with
  | { value = None; diagnostics; _ } when diagnostics <> [] ->
      Printf.printf "✓ %s (diagnostics=%d)\n" desc (List.length diagnostics)
  | _ ->
      Printf.printf "✗ %s: diagnostics were not produced\n" desc;
      exit 1

let expect_legacy_expected desc input =
  match run_string input with
  | {
      value = None;
      diagnostics = diag :: _;
      legacy_error = Some legacy;
      _;
    } -> (
      match diag.Diagnostic.expected with
      | Some summary ->
          if legacy.expected <> [] && summary.Diagnostic.alternatives <> [] then
            Printf.printf "✓ %s\n" desc
          else (
            Printf.printf "✗ %s: 期待集合が空のままです\n" desc;
            exit 1)
      | None ->
          Printf.printf "✗ %s: diagnostic expected is None\n" desc;
          exit 1)
  | { value = None; legacy_error = None; _ } ->
      Printf.printf "✗ %s: legacy error was not populated\n" desc;
      exit 1
  | _ ->
      Printf.printf "✗ %s: parser succeeded unexpectedly\n" desc;
      exit 1

let () =
  expect_failure_with_offset "invalid fn definition reports farthest offset"
    "fn bad( =";
  expect_diagnostics_recorded "lexer error surfaces as diagnostic" "fn @@@";
  expect_legacy_expected
    "legacy API でも診断の期待集合が維持される"
    "fn broken( ->"
