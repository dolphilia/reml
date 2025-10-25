(* test_parser_driver.ml — Parser ドライバ `ParseResult` シム検証 *)

open Parser_driver

let expect_run_ok desc input =
  match run_string input with
  | { value = Some _; diagnostics = []; _ } ->
      Printf.printf "✓ %s\n" desc
  | { value = Some _; diagnostics; _ } ->
      Printf.printf "✗ %s: unexpected diagnostics (%d件)\n" desc
        (List.length diagnostics);
      List.iter
        (fun diag -> Printf.printf "  diag: %s\n" (Diagnostic.to_string diag))
        diagnostics;
      exit 1
  | { value = None; diagnostics; _ } ->
      Printf.printf "✗ %s: parser failed (%d diagnostics)\n" desc
        (List.length diagnostics);
      List.iter
        (fun diag -> Printf.printf "  diag: %s\n" (Diagnostic.to_string diag))
        diagnostics;
      exit 1

let expect_legacy_compat desc input =
  match parse_string input with
  | Ok _ -> Printf.printf "✓ %s\n" desc
  | Error diag ->
      Printf.printf "✗ %s: legacy parse failed\n" desc;
      Printf.printf "%s\n" (Diagnostic.to_string diag);
      exit 1

let () =
  expect_run_ok "run_string succeeds with empty uses" "fn answer() = 42";
  expect_run_ok "run_string handles multiple functions"
    {|
fn log(x) = x
fn log_twice(x) = log(log(x))
|};
  expect_legacy_compat "legacy parse API still succeeds" "fn add(x, y) = x + y"
