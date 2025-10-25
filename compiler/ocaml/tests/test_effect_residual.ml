(* test_effect_residual.ml — 残余効果診断の統合テスト
 *
 * 型クラス辞書モード／モノモルフィゼーションモードの双方で
 * `effects.contract.residual_leak` 診断が同一になることを検証する。
 *)

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path

let golden_path =
  resolve "tests/golden/diagnostics/effects/residual-leak.json.golden"

let audit_golden_path =
  resolve "tests/golden/audit/effects-residual.jsonl.golden"

let write_actual_snapshot name content =
  let actual_dir = resolve "tests/golden/_actual" in
  if not (Sys.file_exists actual_dir) then Unix.mkdir actual_dir 0o755;
  let path = Filename.concat actual_dir (name ^ ".actual.json") in
  Out_channel.with_open_text path (fun oc ->
      output_string oc content;
      if content = "" || content.[String.length content - 1] <> '\n' then
        output_char oc '\n');
  path

let write_actual_snapshot_ext name ext content =
  let actual_dir = resolve "tests/golden/_actual" in
  if not (Sys.file_exists actual_dir) then Unix.mkdir actual_dir 0o755;
  let path = Filename.concat actual_dir (name ^ ext) in
  Out_channel.with_open_text path (fun oc ->
      output_string oc content;
      if content <> "" && content.[String.length content - 1] <> '\n' then
        output_char oc '\n');
  path

let json_of_span (span : Ast.span) =
  `Assoc [ ("start", `Int span.start); ("end", `Int span.end_) ]

let json_of_tag tag =
  `Assoc
    [
      ("name", `String tag.Effect_profile.effect_name);
      ("span", json_of_span tag.effect_span);
    ]

let json_of_tag_list tags = `List (List.map json_of_tag tags)

let metadata_for_effect ?symbol ?source_name ~source_span ~stage_requirement
    ~resolved_stage ~resolved_capability ~effect_set ~stage_trace
    ~diagnostic_payload extra_fields : Audit_envelope.metadata =
  let symbol = match symbol with Some s -> s | None -> "<anonymous>" in
  let required = Effect_profile.stage_requirement_to_string stage_requirement in
  let actual =
    match resolved_stage with
    | Some stage -> `String (Effect_profile.stage_id_to_string stage)
    | None -> `Null
  in
  let capability_json =
    match resolved_capability with
    | Some value when String.trim value <> "" -> `String value
    | Some _ -> `Null
    | None -> `Null
  in
  let diagnostic_json =
    Effect_profile.effect_diagnostic_payload_to_json diagnostic_payload
  in
  let residual_leaks =
    diagnostic_payload.Effect_profile.residual_leaks
    |> List.map (fun leak -> `String leak.Effect_profile.leaked_tag.effect_name)
  in
  let base_fields =
    [
      ("symbol", `String symbol);
      ( "effect.source",
        match source_name with Some name -> `String name | None -> `Null );
      ("effect.source.span", json_of_span source_span);
      ("effect.stage.required", `String required);
      ("effect.stage.actual", actual);
      ("effect.stage.capability", capability_json);
      ("effects.declared", json_of_tag_list effect_set.Effect_profile.declared);
      ("effects.residual", json_of_tag_list effect_set.residual);
      ("effects.diagnostic_payload", diagnostic_json);
      ( "effect.residual.leak_count",
        `Int (List.length diagnostic_payload.residual_leaks) );
    ]
  in
  let fields =
    if residual_leaks = [] then base_fields
    else ("effect.residual.missing", `List residual_leaks) :: base_fields
  in
  let fields =
    if stage_trace = [] then fields
    else
      ("stage_trace", Effect_profile.stage_trace_to_json stage_trace) :: fields
  in
  List.rev_append extra_fields fields

let event_of_entry (entry : Constraint_solver.EffectConstraintTable.entry) =
  let metadata =
    metadata_for_effect ~symbol:entry.symbol ?source_name:entry.source_name
      ~source_span:entry.source_span ~stage_requirement:entry.stage_requirement
      ~resolved_stage:entry.resolved_stage
      ~resolved_capability:entry.resolved_capability
      ~effect_set:entry.effect_set ~stage_trace:entry.stage_trace
      ~diagnostic_payload:entry.diagnostic_payload []
  in
  Audit_envelope.make ~category:"effect.stage" ~metadata_pairs:metadata ()

let runtime_stage_event (context : Type_inference_effect.runtime_stage) =
  let capabilities =
    context.Type_inference_effect.capability_stages
    |> List.map (fun (name, stage) ->
           `Assoc
             [
               ("name", `String name);
               ("stage", `String (Effect_profile.stage_id_to_string stage));
             ])
  in
  let metadata =
    [
      ( "effect.stage.default",
        `String (Effect_profile.stage_id_to_string context.default_stage) );
      ("effect.stage.capabilities", `List capabilities);
    ]
  in
  let metadata =
    if context.stage_trace = [] then metadata
    else
      ("stage_trace", Effect_profile.stage_trace_to_json context.stage_trace)
      :: metadata
  in
  Audit_envelope.make ~category:"effect.stage.runtime"
    ~metadata_pairs:metadata ()

let run_with_mode mode =
  Constraint_solver.reset_effect_constraints ();
  let span = Ast.{ start = 0; end_ = 0 } in
  let mk_tag name = { Effect_profile.effect_name = name; effect_span = span } in
  let effect_set =
    { Effect_profile.declared = [ mk_tag "io" ]; residual = [ mk_tag "panic" ] }
  in
  let residual_leak =
    { Effect_profile.leaked_tag = mk_tag "panic"; leak_origin = span }
  in
  let diagnostic_payload =
    {
      Effect_profile.invalid_attributes = [];
      residual_leaks = [ residual_leak ];
    }
  in
  let stage_trace =
    [
      Effect_profile.make_stage_trace_step ~note:"not provided" "cli_option";
      Effect_profile.make_stage_trace_step ~note:"REMLC_EFFECT_STAGE not set"
        "env_var";
      Effect_profile.stage_trace_step_of_stage_id_opt ~capability:"core.runtime"
        "typer" (Some Effect_profile.Stable);
      Effect_profile.make_stage_trace_step
        ~stage:(Effect_profile.stage_id_to_string Effect_profile.Stable)
        ~capability:"core.runtime" "runtime";
      Effect_profile.make_stage_trace_step
        ~stage:(Effect_profile.stage_id_to_string Effect_profile.Stable)
        ~file:"tooling/runtime/capabilities/default.json" "capability_json";
    ]
  in
  let profile =
    Effect_profile.make_profile
      ~stage_requirement:(Effect_profile.StageExact Effect_profile.Stable)
      ~effect_set ~span ~stage_trace ~diagnostic_payload ~source_name:"demo"
      ~resolved_stage:Effect_profile.Stable ~resolved_capability:"core.runtime"
      ()
  in
  Constraint_solver.record_effect_profile ~symbol:"demo" profile;
  let error =
    Type_error.effect_residual_leak_error ~function_name:(Some "demo") ~profile
      ~leaks:[ residual_leak ]
  in
  let fixed_timestamp = "1970-01-01T00:00:00Z" in
  let diag =
    Type_error.to_diagnostic_with_source "" "effectful_sum.reml" error
    |> fun d -> { d with timestamp = Some fixed_timestamp }
  in
  let diag_json =
    Cli.Json_formatter.diagnostic_to_json ~mode:Cli.Options.JsonPretty diag
  in
  let constraint_events =
    Constraint_solver.current_effect_constraints ()
    |> Constraint_solver.EffectConstraintTable.to_list
    |> List.map event_of_entry
  in
  let base_events =
    runtime_stage_event Type_inference_effect.runtime_stage_default
    :: constraint_events
  in
  let audit_events =
    List.map
      (fun (event : Audit_envelope.event) ->
        { event with timestamp = fixed_timestamp })
      base_events
  in
  let audit_temp = Filename.temp_file ("audit-" ^ mode) ".jsonl" in
  Audit_envelope.append_events audit_temp audit_events;
  let audit_output =
    In_channel.with_open_text audit_temp In_channel.input_all
  in
  Sys.remove audit_temp;
  (diag_json, audit_output)

let compare_with_golden () =
  let diag_dictionary, audit_dictionary = run_with_mode "dictionary" in
  let diag_monomorph, audit_monomorph = run_with_mode "monomorph" in
  (if String.trim diag_dictionary <> String.trim diag_monomorph then
     let path =
       write_actual_snapshot "residual-leak-mismatch" diag_dictionary
     in
     failwith
       (Printf.sprintf "辞書モードとモノモルフィゼーションモードで診断が一致しません。\n辞書モード出力: %s" path));
  (if String.trim audit_dictionary <> String.trim audit_monomorph then
     let path =
       write_actual_snapshot_ext "residual-leak-audit-mismatch" ".actual.jsonl"
         audit_dictionary
     in
     failwith
       (Printf.sprintf "辞書モードとモノモルフィゼーションモードで監査出力が一致しません。\n辞書モード出力: %s" path));
  (if not (Sys.file_exists golden_path) then
     let path = write_actual_snapshot "residual-leak" diag_dictionary in
     failwith
       (Printf.sprintf "ゴールデンファイル %s が存在しません。\n現在の出力を %s に書き出しました。" golden_path
          path));
  (if not (Sys.file_exists audit_golden_path) then
     let path =
       write_actual_snapshot_ext "residual-leak-audit" ".actual.jsonl"
         audit_dictionary
     in
     failwith
       (Printf.sprintf "監査ゴールデン %s が存在しません。\n現在の出力を %s に書き出しました。"
          audit_golden_path path));
  let expected_diag =
    In_channel.with_open_text golden_path (fun ic ->
        In_channel.input_all ic |> String.trim)
  in
  let actual_diag = String.trim diag_dictionary in
  (if expected_diag <> actual_diag then
     let path = write_actual_snapshot "residual-leak" diag_dictionary in
     failwith
       (Printf.sprintf
          "residual-leak.json.golden と現在の診断が一致しません。\nゴールデン: %s\n現在の出力: %s"
          golden_path path));
  let expected_audit =
    In_channel.with_open_text audit_golden_path (fun ic ->
        In_channel.input_all ic |> String.trim)
  in
  let actual_audit = String.trim audit_dictionary in
  (if expected_audit <> actual_audit then
     let path =
       write_actual_snapshot_ext "residual-leak-audit" ".actual.jsonl"
         audit_dictionary
     in
     failwith
       (Printf.sprintf
          "effects-residual.jsonl.golden と現在の監査出力が一致しません。\nゴールデン: %s\n現在の出力: %s"
          audit_golden_path path));
  Printf.printf "✓ residual leak diagnostics & audit logs match golden\n%!"

let () = compare_with_golden ()
