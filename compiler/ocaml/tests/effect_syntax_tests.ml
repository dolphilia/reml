(* effect_syntax_tests.ml — 効果構文 PoC サマリのゴールデンテスト
 *
 * SYNTAX-003 / EFFECT-002 の PoC 条件を満たすことを確認する。
 * `syntax.effect_construct_acceptance` および
 * `effects.syntax_poison_rate` 指標算出用の JSON を生成し、
 * ゴールデンと比較する。
 *)

open Typed_ast
open Effect_profile

module Run_config = Parser_run_config

let () = Unix.putenv "REMLC_FIXED_TIMESTAMP" "1970-01-01T00:00:00Z"

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path

let golden_path =
  resolve
    "tests/golden/diagnostics/effects/syntax-constructs.json.golden"

let write_actual_snapshot name content =
  let actual_dir = resolve "tests/golden/_actual" in
  if not (Sys.file_exists actual_dir) then Unix.mkdir actual_dir 0o755;
  let path = Filename.concat actual_dir (name ^ ".actual.json") in
  Out_channel.with_open_text path (fun oc ->
      output_string oc content;
      if content <> "" && content.[String.length content - 1] <> '\n' then
        output_char oc '\n');
  path

type expectation = Accept | Reject

type sample = {
  name : string;
  kind : string;
  expectation : expectation;
  source : string;
  expected_codes : string list;
}

let run_config () =
  let base =
    Run_config.Legacy.bridge { require_eof = true; legacy_result = true }
  in
  Run_config.set_experimental_effects base true

let filter_effect_decls ast =
  {
    ast with
    Ast.decls =
      List.filter
        (fun decl ->
          match decl.Ast.decl_kind with Ast.EffectDecl _ -> false | _ -> true)
        ast.Ast.decls;
  }

let parse_and_infer source =
  Diagnostic.reset_audit_sequence ();
  let config = run_config () in
  let parse_result = Parser_driver.run_string ~config source in
  match Parser_driver.parse_result_to_legacy parse_result with
  | Result.Error diag -> Error (`Parse diag)
  | Result.Ok ast -> (
      Type_inference.reset_impl_registry ();
      let filtered_ast = filter_effect_decls ast in
      match Type_inference.infer_compilation_unit filtered_ast with
      | Result.Ok tast -> Ok tast
      | Result.Error err -> Error (`Type err))

let find_function tast name =
  tast.Typed_ast.tcu_items
  |> List.find_map (fun decl ->
         match decl.tdecl_kind with
         | TFnDecl fn when String.equal fn.tfn_name.name name -> Some fn
         | _ -> None)

let json_of_string_list values =
  `List (List.map (fun value -> `String value) values)

let tags_to_json tags =
  tags |> List.map (fun tag -> `String tag.effect_name) |> fun xs -> `List xs

let profile_to_json (profile : profile) =
  let declared =
    tags_to_json profile.effect_set.declared
  in
  let residual =
    tags_to_json profile.effect_set.residual
  in
  let stage =
    `Assoc
      [
        ( "required",
          `String (stage_requirement_to_string profile.stage_requirement) );
        ( "actual",
          match profile.resolved_stage with
          | Some stage -> `String (stage_id_to_string stage)
          | None -> `Null );
      ]
  in
  let capabilities =
    match profile.resolved_capabilities with
    | [] -> `List []
    | entries ->
        `List
          (List.map (fun (entry : capability_resolution) ->
               `String entry.capability_name)
             entries)
  in
  `Assoc
    [
      ("declared", declared);
      ("residual", residual);
      ("stage", stage);
      ("capabilities", capabilities);
    ]

let expectation_to_string = function Accept -> "accept" | Reject -> "reject"

let diagnostics_of_type_error err =
  let diag = Type_error.to_diagnostic err in
  (diag.Diagnostic.codes, diag)

let build_sample_entry sample =
  match parse_and_infer sample.source with
  | Ok tast -> (
      match sample.expectation with
      | Reject ->
          failwith
            (Printf.sprintf
               "サンプル %s はエラーを期待しましたが型推論に成功しました"
               sample.name)
      | Accept ->
          let fn_opt = find_function tast sample.name in
          (match fn_opt with
          | None ->
              failwith
                (Printf.sprintf "サンプル %s の関数が型付き AST に存在しません"
                   sample.name)
          | Some fn ->
              let profile_json = profile_to_json fn.tfn_effect_profile in
              `Assoc
                [
                  ("name", `String sample.name);
                  ("kind", `String sample.kind);
                  ("expectation", `String (expectation_to_string sample.expectation));
                  ("status", `String "ok");
                  ("diagnostics", `List []);
                  ("effect_profile", profile_json);
                ]))
  | Error (`Parse diag) ->
      let codes = diag.Diagnostic.codes in
      if sample.expectation = Accept then
        failwith
          (Printf.sprintf
             "サンプル %s は成功を期待しましたが構文エラーになりました" sample.name);
      if sample.expected_codes <> [] && sample.expected_codes <> codes then
        failwith
          (Printf.sprintf
             "サンプル %s の診断コードが期待と異なります: 期待=%s 実際=%s"
             sample.name
             (String.concat "," sample.expected_codes)
             (String.concat "," codes));
      `Assoc
        [
          ("name", `String sample.name);
          ("kind", `String sample.kind);
          ("expectation", `String (expectation_to_string sample.expectation));
          ("status", `String "error");
          ("diagnostics", json_of_string_list codes);
          ("message", `String diag.Diagnostic.message);
        ]
  | Error (`Type err) ->
      let codes, diag = diagnostics_of_type_error err in
      if sample.expectation = Accept then
        failwith
          (Printf.sprintf
             "サンプル %s は成功を期待しましたが型エラーになりました: %s"
             sample.name diag.Diagnostic.message);
      if sample.expected_codes <> [] && sample.expected_codes <> codes then
        failwith
          (Printf.sprintf
             "サンプル %s の診断コードが期待と異なります: 期待=%s 実際=%s"
             sample.name
             (String.concat "," sample.expected_codes)
             (String.concat "," codes));
      `Assoc
        [
          ("name", `String sample.name);
          ("kind", `String sample.kind);
          ("expectation", `String (expectation_to_string sample.expectation));
          ("status", `String "error");
          ("diagnostics", json_of_string_list codes);
          ("message", `String diag.Diagnostic.message);
        ]

let samples : sample list =
  [
    {
      name = "handled_demo";
      kind = "handle";
      expectation = Accept;
      expected_codes = [];
      source =
        {|
effect Console : io {
  operation log : String -> Unit
}

@allows_effects(Console)
fn handled_demo(msg: String) = {
  handle perform Console.log(msg) with handler ConsoleHandler {
    operation log(value) {
      ()
    }
  }
}
|};
    };
    {
      name = "unhandled_demo";
      kind = "perform";
      expectation = Reject;
      expected_codes = [ "effects.contract.residual_leak" ];
      source =
        {|
effect Console : io {
  operation log : String -> Unit
}

fn unhandled_demo(msg: String) = {
  perform Console.log(msg)
}
|};
    };
    {
      name = "experimental_demo";
      kind = "perform";
      expectation = Reject;
      expected_codes = [];
      source =
        {|
effect Console : io {
  operation log : String -> Unit
}

@requires_capability(stage = "experimental")
@allows_effects(Console)
fn experimental_demo(msg: String) = {
  perform Console.log(msg)
}
|};
    };
  ]

let build_summary () =
  let constructs = List.map build_sample_entry samples in
  `Assoc [ ("effect_syntax", `Assoc [ ("constructs", `List constructs) ]) ]

let () =
  let summary = build_summary () in
  let normalized = Yojson.Basic.to_string summary in
  let pretty = Yojson.Basic.pretty_to_string summary in
  let actual_path = write_actual_snapshot "effect_syntax_constructs" pretty in
  if not (Sys.file_exists golden_path) then (
    Printf.eprintf "ゴールデンが存在しません: %s\n%!" golden_path;
    Printf.eprintf "生成された結果を %s に保存しています。\n%!" actual_path;
    exit 1);
  let golden = Yojson.Basic.from_file golden_path |> Yojson.Basic.to_string in
  if String.equal normalized golden then
    Printf.printf "✓ 効果構文 PoC サマリがゴールデンと一致\n%!"
  else (
    Printf.eprintf
      "効果構文 PoC サマリがゴールデンと一致しません。差分は %s を参照して\
       ください。\n%!"
      actual_path;
    exit 1)
