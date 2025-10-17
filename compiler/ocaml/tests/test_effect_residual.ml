(* test_effect_residual.ml — 残余効果診断の統合テスト
 *
 * 型クラス辞書モード／モノモルフィゼーションモードの双方で
 * `effects.contract.residual_leak` 診断が同一になることを検証する。
 *)

open Cli

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path

let golden_path =
  resolve "tests/golden/diagnostics/effects/residual-leak.json.golden"

let write_actual_snapshot name content =
  let actual_dir = resolve "tests/golden/_actual" in
  if not (Sys.file_exists actual_dir) then Unix.mkdir actual_dir 0o755;
  let path = Filename.concat actual_dir (name ^ ".actual.json") in
  Out_channel.with_open_text path (fun oc ->
      output_string oc content;
      if content = "" || content.[String.length content - 1] <> '\n' then
        output_char oc '\n');
  path

let run_with_mode mode =
  let input = resolve "tests/typeclass_effects/effectful_sum.reml" in
  let argv =
    [|
      "remlc";
      "--format=json";
      "--typeclass-mode";
      mode;
      input;
    |]
  in
  let opts =
    match Options.parse_args argv with
    | Ok opts -> opts
    | Error msg -> failwith msg
  in
  let source = In_channel.with_open_text input In_channel.input_all in
  let lexbuf = Lexing.from_string source in
  lexbuf.Lexing.lex_curr_p <-
    { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = input };
  match Parser_driver.parse lexbuf with
  | Error _ -> failwith "Parse error: residual leak fixture is invalid"
  | Ok ast -> (
      let runtime_stage_context =
        Runtime_capability_resolver.resolve
          ~cli_override:opts.Options.effect_stage_override
          ~registry_path:opts.Options.runtime_capabilities_path
          ~target:(Some opts.target)
      in
      let config =
        Type_inference.make_config ~effect_context:runtime_stage_context ()
      in
      match Type_inference.infer_compilation_unit ~config ast with
      | Ok _ -> failwith "Expected residual leak diagnostic but inference succeeded"
      | Error err ->
          let diag = Type_error.to_diagnostic err in
          Cli.Json_formatter.diagnostic_to_json diag)

let compare_with_golden () =
  let json_dictionary = run_with_mode "dictionary" in
  let json_monomorph = run_with_mode "monomorph" in
  if String.trim json_dictionary <> String.trim json_monomorph then (
    let path = write_actual_snapshot "residual-leak-mismatch" json_dictionary in
    failwith
      (Printf.sprintf
         "辞書モードとモノモルフィゼーションモードで診断が一致しません。\n\
         辞書モード出力: %s"
         path));
  if not (Sys.file_exists golden_path) then (
    let path = write_actual_snapshot "residual-leak" json_dictionary in
    failwith
      (Printf.sprintf
         "ゴールデンファイル %s が存在しません。\n現在の出力を %s に書き出しました。"
         golden_path path));
  let expected =
    In_channel.with_open_text golden_path (fun ic ->
        In_channel.input_all ic |> String.trim)
  in
  let actual = String.trim json_dictionary in
  if expected <> actual then (
    let path = write_actual_snapshot "residual-leak" json_dictionary in
    failwith
      (Printf.sprintf
         "residual-leak.json.golden と現在の診断が一致しません。\n\
          ゴールデン: %s\n\
          現在の出力: %s"
         golden_path path))
  else Printf.printf "✓ residual leak diagnostics match golden\n%!"

let () = compare_with_golden ()
