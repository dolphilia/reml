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

let () =
  expect_failure_with_offset "invalid fn definition reports farthest offset"
    "fn bad( =";
  expect_diagnostics_recorded "lexer error surfaces as diagnostic" "fn @@@"
