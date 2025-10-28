(* Test_support — Parser RunConfig ヘルパー
 *
 * Phase 2-5 PARSER-002 Step4: テストから RunConfig を明示的に渡すための
 * ユーティリティを提供する。
 *)

module Run_config = Parser_run_config

let legacy_run_config =
  Run_config.Legacy.
    bridge { require_eof = true; legacy_result = true }

let parse_result ?(filename = "<test>") ?(config = legacy_run_config) source =
  Parser_driver.run_string ~filename ~config source

let parse_string ?filename ?config source =
  parse_result ?filename ?config source
  |> Parser_driver.parse_result_to_legacy

let with_run_config ?filename config ~f source =
  let result = parse_result ?filename ~config source in
  f result
