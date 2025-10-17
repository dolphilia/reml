(* Type_inference_effect — Effect profile resolver for Typer (Phase 2-2)
 *
 * Parser が保持する effect_profile_node から Effect_profile.profile へ正規化し、
 * Stage 要件と実行環境（現状は Stable 固定）の突合を行う。
 *)

open Ast
open Effect_profile

type runtime_stage = {
  default_stage : stage_id;
  capability_stages : (string * stage_id) list;
  stage_trace : stage_trace;
}

let normalize_capability_name name = String.lowercase_ascii name

let runtime_stage_default =
  {
    default_stage = Stable;
    capability_stages = [];
    stage_trace = stage_trace_empty;
  }

let create_runtime_stage ?(capability_stages = []) ?default_stage
    ?(stage_trace = stage_trace_empty) () =
  {
    default_stage =
      (match default_stage with Some stage -> stage | None -> Stable);
    capability_stages =
      List.map
        (fun (name, stage) -> (normalize_capability_name name, stage))
        capability_stages;
    stage_trace;
  }

let stage_for_capability runtime_stage capability_name =
  match capability_name with
  | None -> runtime_stage.default_stage
  | Some raw_name ->
      let key = normalize_capability_name raw_name in
      match
        List.find_opt
          (fun (candidate, _) -> String.equal candidate key)
          runtime_stage.capability_stages
      with
      | Some (_, stage) -> stage
      | None -> runtime_stage.default_stage

let resolve_function_profile ~(runtime_context : runtime_stage)
    ~(function_ident : ident)
    (effect_node : effect_profile_node option) =
  let source_name = Some function_ident.name in
  let capability_name =
    None
    (* TODO Phase 2-3: effect属性から Capability 名を解析して渡す *)
  in
  let current_stage = stage_for_capability runtime_context capability_name in
  let typer_step =
    match capability_name with
    | Some cap ->
        stage_trace_step_of_stage_id_opt ~capability:cap "typer"
          (Some current_stage)
    | None ->
        stage_trace_step_of_stage_id_opt "typer" (Some current_stage)
  in
  let rec split_primary acc = function
    | ( { source; _ } as step ) :: rest
      when String.equal source "cli_option"
           || String.equal source "env_var" ->
        split_primary (step :: acc) rest
    | tail -> (List.rev acc, tail)
  in
  let stage_trace_with_typer base_trace =
    let prefix, suffix = split_primary [] base_trace in
    prefix @ (typer_step :: suffix)
  in
  match effect_node with
  | None ->
      let stage_trace = stage_trace_with_typer runtime_context.stage_trace in
      let stage_trace =
        match stage_trace with
        | [] -> [ typer_step ]
        | trace -> trace
      in
      let profile =
        {
          (default_profile ?source_name ~stage_trace ~span:function_ident.span ())
          with
          resolved_stage = Some current_stage;
          resolved_capability = capability_name;
        }
      in
      Ok profile
  | Some node ->
      let base_trace =
        match stage_trace_with_typer runtime_context.stage_trace with
        | [] -> [ typer_step ]
        | trace -> trace
      in
      let profile =
        profile_of_ast ?source_name ~stage_trace:base_trace node
        |> fun p ->
        {
          p with
          resolved_stage = Some current_stage;
          resolved_capability = capability_name;
        }
      in
      (match profile.diagnostic_payload.invalid_attributes with
      | invalid :: _ ->
          Error
            (Type_error.effect_invalid_attribute_error
               ~function_name:function_ident.name ~profile ~invalid)
      | [] ->
      if stage_requirement_satisfied profile.stage_requirement current_stage then
        Ok profile
      else
        Error
          (Type_error.effect_stage_mismatch_error
             ~function_name:function_ident.name
             ~required_stage:
               (stage_requirement_to_string profile.stage_requirement)
             ~actual_stage:(stage_id_to_string current_stage)
             ~span:profile.source_span ~capability:capability_name
             ~stage_trace:profile.stage_trace))
