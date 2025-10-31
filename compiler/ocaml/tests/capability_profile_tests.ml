(* capability_profile_tests.ml — EFFECT-003 Step4 テスト
 *
 * 複数 Capability を要求する効果プロファイルの解析結果が
 * StageRequirement::{AtLeast, Exact} の両ケースで配列として保持されることを検証する。
 *)

open Effect_profile
open Type_inference_effect
open Type_error

let () = Printf.printf "\n=== Capability Profile Tests ===\n"

let ident name = Ast.{ name; span = Ast.dummy_span }

let span_from_offsets start_offset end_offset =
  Ast.{ start = start_offset; end_ = end_offset }

let make_effect_node ~stage ~capabilities =
  let stage_annot =
    match stage with
    | `AtLeast name -> Some (Ast.StageAtLeast (ident name))
    | `Exact name -> Some (Ast.StageExact (ident name))
  in
  {
    Ast.effect_declared = List.map ident [ "iter" ];
    effect_residual = [];
    effect_stage = stage_annot;
    effect_capabilities = List.map ident capabilities;
    effect_span = span_from_offsets 10 20;
    effect_invalid_attributes = [];
  }

let runtime_stage ~default ~entries =
  create_runtime_stage ~default_stage:default
    ~capability_stages:entries ()

let assert_stage_equal expected actual =
  let to_string = stage_id_to_string in
  if not (String.equal (to_string actual) expected) then
    failwith
      (Printf.sprintf "Stage mismatch: expected %s, actual %s" expected
         (to_string actual))

let assert_stage_option expected = function
  | Some stage -> assert_stage_equal expected stage
  | None ->
      failwith (Printf.sprintf "Stage was None (expected %s)" expected)

let assert_stage_trace stage_trace expected =
  let actual =
    List.map
      (fun (step : stage_trace_step) ->
        (step.source, step.capability, step.stage))
      stage_trace
  in
  if actual <> expected then
    let render entries =
      String.concat "; "
        (List.map
           (fun (src, cap_opt, stage_opt) ->
             Printf.sprintf "%s:%s:%s" src
               (Option.value ~default:"<none>" cap_opt)
               (Option.value ~default:"<none>" stage_opt))
           entries)
    in
    failwith
      (Printf.sprintf
         "Stage trace mismatch:\n  expected: %s\n  actual:   %s"
         (render expected) (render actual))

let test_stage_at_least_multiple_capabilities () =
  let runtime =
    runtime_stage ~default:Stable
      ~entries:
        [
          ("core.iterator.collect", Stable);
          ("core.iterator.reduce", Stable);
        ]
  in
  let fn_ident = ident "collect_all" in
  let effect_node =
    make_effect_node ~stage:(`AtLeast "beta")
      ~capabilities:[ "core.iterator.collect"; "core.iterator.reduce" ]
  in
  match
    resolve_function_profile ~runtime_context:runtime ~function_ident:fn_ident
      (Some effect_node)
  with
  | Ok profile ->
      let names = capability_names profile.resolved_capabilities in
      assert (
        names
        = [ "core.iterator.collect"; "core.iterator.reduce" ]);
      (match profile.stage_requirement with
      | StageAtLeast Beta -> ()
      | _ -> failwith "StageRequirement (AtLeast beta) が保持されていません");
      profile.resolved_capabilities
      |> List.iter (fun entry ->
             assert_stage_option "stable" entry.capability_stage);
      assert_stage_trace profile.stage_trace
        [
          ("typer", Some "core.iterator.collect", Some "stable");
          ("typer", Some "core.iterator.reduce", Some "stable");
        ];
      Printf.printf
        "✓ StageAtLeast: 複数 Capability が解析・追跡されました\n"
  | Error err ->
      failwith
        (Printf.sprintf
           "StageAtLeast シナリオでエラーが発生しました: %s"
           (string_of_error err))

let test_stage_exact_mismatch_reports_all_capabilities () =
  let runtime =
    runtime_stage ~default:Stable
      ~entries:
        [
          ("core.iterator.collect", Beta);
          ("core.iterator.reduce", Stable);
        ]
  in
  let fn_ident = ident "collect_exact" in
  let effect_node =
    make_effect_node ~stage:(`Exact "stable")
      ~capabilities:[ "core.iterator.collect"; "core.iterator.reduce" ]
  in
  match
    resolve_function_profile ~runtime_context:runtime ~function_ident:fn_ident
      (Some effect_node)
  with
  | Ok _ -> failwith "StageExact ミスマッチが検出されませんでした"
  | Error (EffectStageMismatch details) ->
      let pairs = details.capability_stages in
      let expected =
        [
          ("core.iterator.collect", Some "beta");
          ("core.iterator.reduce", Some "stable");
        ]
      in
      if pairs <> expected then
        failwith
          (Printf.sprintf
             "capability_stages が期待値と異なります。\n  expected: %s\n  actual:   %s"
             (String.concat ", "
                (List.map
                   (fun (name, stage_opt) ->
                     Printf.sprintf "%s:%s" name
                       (Option.value ~default:"<none>" stage_opt))
                   expected))
             (String.concat ", "
                (List.map
                   (fun (name, stage_opt) ->
                     Printf.sprintf "%s:%s" name
                       (Option.value ~default:"<none>" stage_opt))
                   pairs)));
      assert_stage_trace details.stage_trace
        [
          ("typer", Some "core.iterator.collect", Some "beta");
          ("typer", Some "core.iterator.reduce", Some "stable");
        ];
      Printf.printf
        "✓ StageExact: ミスマッチ診断に複数 Capability が含まれました\n"
  | Error err ->
      failwith
        (Printf.sprintf "想定外のエラーが返されました: %s" (string_of_error err))

let () =
  test_stage_at_least_multiple_capabilities ();
  test_stage_exact_mismatch_reports_all_capabilities ();
  Printf.printf "✓ Capability profile tests finished\n"
