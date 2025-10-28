(* run_config_tests.ml — RunConfig の挙動検証ユニットテスト
 *
 * Phase 2-5 PARSER-002 Step5: テスト・検証・メトリクス定着。
 * - require_eof 上書きによる未消費入力検出
 * - merge_warnings フラグの重複警告処理
 * - trace スイッチによる SpanTrace 収集
 * - extensions["lex"] 設定のデコード
 * - Legacy ブリッジ経路の互換性確認
 *)

open Parser_driver

module Run_config = Parser_run_config
module Extensions = Parser_run_config.Extensions
module Namespace = Extensions.Namespace
module Diag_state = Parser_diag_state
module Builder = Diagnostic.Builder
module Trivia = Run_config.Lex.Trivia_profile

let pass desc = Printf.printf "✓ %s\n" desc

let fail desc msg =
  Printf.printf "✗ %s: %s\n" desc msg;
  exit 1

let diagnostic_has_code diagnostics code =
  List.exists
    (fun (diag : Diagnostic.t) ->
      List.exists (fun item -> String.equal item code) diag.Diagnostic.codes)
    diagnostics

let test_require_eof_override () =
  let desc = "RunConfig.require_eof=true が未消費入力を検出する" in
  let config =
    Run_config.with_extension "config"
      (fun ns -> Namespace.add "require_eof" (Extensions.Bool true) ns)
      Run_config.default
  in
  let input = "fn answer() = 42 42" in
  let relaxed = Parser_driver.run_string input in
  let strict = Parser_driver.run_string ~config input in
  let strict_has_eof_diag =
    diagnostic_has_code strict.diagnostics "parser.require_eof.unconsumed_input"
  in
  let relaxed_has_eof_diag =
    diagnostic_has_code relaxed.diagnostics "parser.require_eof.unconsumed_input"
  in
  if strict_has_eof_diag && not relaxed_has_eof_diag then pass desc
  else
    fail desc
      "require_eof 上書きが期待した診断を生成できませんでした"

let test_merge_warnings_toggle () =
  let desc = "merge_warnings=false で警告の重複を許可する" in
  let dummy_pos =
    { Lexing.pos_fname = "<test>"; pos_lnum = 1; pos_bol = 0; pos_cnum = 0 }
  in
  let span = Diagnostic.span_of_positions dummy_pos dummy_pos in
  let warning =
    Builder.create ~message:"dummy warning" ~primary:span ()
    |> Builder.set_severity Diagnostic.Warning
    |> Builder.set_primary_code "parser.runconfig.test_warning"
    |> Builder.build
  in
  let merged =
    let state = Diag_state.create ~merge_warnings:true () in
    Diag_state.record_warning state ~diagnostic:warning;
    Diag_state.record_warning state ~diagnostic:warning;
    Diag_state.diagnostics state
  in
  let full =
    let state = Diag_state.create ~merge_warnings:false () in
    Diag_state.record_warning state ~diagnostic:warning;
    Diag_state.record_warning state ~diagnostic:warning;
    Diag_state.diagnostics state
  in
  match (List.length merged, List.length full) with
  | 1, 2 -> pass desc
  | _ ->
      fail desc
        "merge_warnings フラグの設定に応じた警告件数が確認できませんでした"

let test_trace_span_trace () =
  let desc = "trace=true で SpanTrace が収集される" in
  let input = "fn answer() = 42" in
  let baseline = Parser_driver.run_string input in
  let traced =
    let config = { Run_config.default with trace = true } in
    Parser_driver.run_string ~config input
  in
  match (baseline.span_trace, traced.span_trace) with
  | None, Some ((label, _) :: _)
    when Option.value ~default:"" label = "compilation_unit" ->
      pass desc
  | None, Some _ ->
      fail desc "SpanTrace の先頭ラベルが想定と一致しません"
  | None, None ->
      fail desc "trace=true で SpanTrace が収集されませんでした"
  | Some _, _ ->
      fail desc "trace=false で SpanTrace が収集されています"

let test_lex_extension_profile () =
  let desc = "extensions[\"lex\"] が profile と space_id を復元する" in
  let config =
    Run_config.with_extension "lex"
      (fun ns ->
        ns
        |> Namespace.add "space_id" (Extensions.Parser_id 512)
        |> Namespace.add "profile" (Extensions.String "json_relaxed"))
      Run_config.default
  in
  let lex_config = Run_config.Lex.of_run_config config in
  let trivia = Run_config.Lex.effective_trivia lex_config in
  let has_expected_space = Option.value ~default:0 lex_config.space_id = 512 in
  let has_expected_profile =
    match lex_config.profile with
    | Run_config.Lex.Json_relaxed -> true
    | _ -> false
  in
  let { Trivia.shebang; _ } = trivia in
  if has_expected_space && has_expected_profile && shebang then pass desc
  else fail desc "lex 拡張のデコード結果が仕様と一致しません"

let test_legacy_bridge_compat () =
  let desc = "Legacy ブリッジの互換経路が維持される" in
  let config =
    Run_config.Legacy.
      bridge { require_eof = true; legacy_result = true }
  in
  let ok_input = "fn add(x, y) = x + y" in
  let bad_input = "fn missing(x = x" in
  let ok_result = Parser_driver.run_string ~config ok_input in
  let legacy_ok = Parser_driver.parse_string ok_input in
  let bad_result = Parser_driver.run_string ~config bad_input in
  let legacy_bad = Parser_driver.parse_string bad_input in
  match (ok_result.value, legacy_ok, bad_result.legacy_error, legacy_bad) with
  | Some _, Ok _, Some _, Error _ -> pass desc
  | _ ->
      fail desc "Legacy API と RunConfig ブリッジの挙動が一致しません"

let () =
  let tests =
    [
      test_require_eof_override;
      test_merge_warnings_toggle;
      test_trace_span_trace;
      test_lex_extension_profile;
      test_legacy_bridge_compat;
    ]
  in
  List.iter (fun fn -> fn ()) tests
