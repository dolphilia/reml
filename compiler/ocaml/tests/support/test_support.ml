(* Test_support — Parser RunConfig ヘルパー
 *
 * Phase 2-5 PARSER-002 Step4: テストから RunConfig を明示的に渡すための
 * ユーティリティを提供する。
 *)

module Run_config = Parser_run_config

let legacy_run_config =
  Run_config.Legacy.
    bridge { require_eof = true; legacy_result = true }

let rec ascend n path =
  if n <= 0 then path else ascend (n - 1) (Filename.dirname path)

let tests_source_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> Filename.concat root "tests"
  | None ->
      let build_dir = Filename.dirname __FILE__ in
      Filename.concat (ascend 4 build_dir) "tests"

let sample_path name =
  Filename.concat tests_source_root ("samples/" ^ name)

let parse_result ?(filename = "<test>") ?(config = legacy_run_config) source =
  Parser_driver.run_string ~filename ~config source

let parse_string ?filename ?config source =
  parse_result ?filename ?config source
  |> Parser_driver.parse_result_to_legacy

let with_run_config ?filename config ~f source =
  let result = parse_result ?filename ~config source in
  f result
