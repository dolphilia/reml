(* Runtime_capability_resolver — Resolve runtime Stage/Capability information
 *
 * Phase 2-2: RuntimeCapability JSON / CLI / 環境変数を統合し、
 * 型推論に渡す Stage コンテキストを決定する。
 *
 * 優先度: CLI `--effect-stage` > JSON `stage` > 環境変数 `REMLC_EFFECT_STAGE`
 * フォールバック: 既存の `REML_RUNTIME_STAGE`, 最終的には `Stable`
 *)

open Effect_profile
open Type_inference_effect
module Json = Yojson.Basic

type capability_entry = { name : string; stage : stage_id option }

type registry = {
  stage : stage_id option;
  capabilities : capability_entry list;
  overrides : (string * capability_entry list) list;
}

let empty_registry = { stage = None; capabilities = []; overrides = [] }
let env_stage_var = "REMLC_EFFECT_STAGE"
let legacy_env_stage_var = "REML_RUNTIME_STAGE"
let env_registry_var = "REML_RUNTIME_CAPABILITIES"
let normalize_key value = value |> String.trim |> String.lowercase_ascii

let stage_id_of_string_opt value =
  let trimmed = String.trim value in
  if String.equal trimmed "" then None else Some (stage_id_of_string trimmed)

let capability_entry name stage = { name = String.trim name; stage }

let parse_capability_entries value ~default_stage : capability_entry list =
  (match value with
  | `List entries ->
      List.filter_map
        (function
          | `String name -> Some (capability_entry name None)
          | `Assoc fields -> (
              match List.assoc_opt "name" fields with
              | Some (`String name) ->
                  let stage =
                    match List.assoc_opt "stage" fields with
                    | Some (`String s) -> stage_id_of_string_opt s
                    | _ -> None
                  in
                  Some (capability_entry name stage)
              | _ -> None)
          | _ -> None)
        entries
  | `Assoc fields ->
      fields
      |> List.filter_map (fun (name, data) ->
             match data with
             | `String stage ->
                 Some (capability_entry name (stage_id_of_string_opt stage))
             | `Assoc nested ->
                 let stage =
                   match List.assoc_opt "stage" nested with
                   | Some (`String stage) -> stage_id_of_string_opt stage
                   | _ -> None
                 in
                 Some (capability_entry name stage)
             | `Null -> Some (capability_entry name None)
             | _ -> None)
  | _ -> [])
  |> List.map (fun (entry : capability_entry) ->
         match entry.stage with
         | Some _ -> entry
         | None -> { entry with stage = Some default_stage })

let parse_override_section value ~default_stage =
  match value with
  | `Assoc fields ->
      let stage_override =
        match List.assoc_opt "stage" fields with
        | Some (`String stage) -> stage_id_of_string_opt stage
        | _ -> None
      in
      let capabilities_value =
        match List.assoc_opt "capabilities" fields with
        | Some v -> v
        | None -> `Assoc fields
      in
      let base_stage =
        match stage_override with Some stage -> stage | None -> default_stage
      in
      parse_capability_entries capabilities_value ~default_stage:base_stage
      |> List.map (fun (entry : capability_entry) ->
             match entry.stage with
             | Some _ -> entry
             | None -> { entry with stage = stage_override })
  | _ -> parse_capability_entries value ~default_stage

let load_registry_from_file path =
  try
    match Json.from_file path with
    | `Assoc fields ->
        let stage =
          match List.assoc_opt "stage" fields with
          | Some (`String value) -> stage_id_of_string_opt value
          | _ -> None
        in
        let default_stage =
          match stage with
          | Some s -> s
          | None -> runtime_stage_default.default_stage
        in
        let capabilities =
          match List.assoc_opt "capabilities" fields with
          | Some value -> parse_capability_entries value ~default_stage
          | None -> []
        in
        let overrides =
          match List.assoc_opt "overrides" fields with
          | Some (`Assoc override_entries) ->
              override_entries
              |> List.map (fun (target, value) ->
                     ( normalize_key target,
                       parse_override_section value ~default_stage ))
          | _ -> []
        in
        { stage; capabilities; overrides }
    | _ -> empty_registry
  with Sys_error _ | Yojson.Json_error _ -> empty_registry

let registry_from_path = function
  | Some path when String.trim path <> "" -> load_registry_from_file path
  | _ -> empty_registry

let resolve ~required_capabilities ~cli_override ~registry_path ~target =
  let registry, registry_source_path =
    match registry_path with
    | Some path -> (load_registry_from_file path, Some path)
    | None -> (
        match Sys.getenv_opt env_registry_var with
        | Some path -> (load_registry_from_file path, Some path)
        | None -> (empty_registry, None))
  in
  let stage_trace = ref stage_trace_empty in
  let env_stage =
    match Sys.getenv_opt env_stage_var with
    | Some stage -> stage_id_of_string_opt stage
    | None -> None
  in
  let legacy_env_stage =
    match Sys.getenv_opt legacy_env_stage_var with
    | Some stage -> stage_id_of_string_opt stage
    | None -> None
  in
  let append_step step = stage_trace := !stage_trace @ [ step ] in
  let () =
    match cli_override with
    | Some raw ->
        let normalized = stage_id_to_string (stage_id_of_string raw) in
        append_step
          (make_stage_trace_step ~stage:normalized
             ~note:(Printf.sprintf "--effect-stage %s" raw)
             "cli_option")
    | None ->
        append_step (make_stage_trace_step ~note:"not provided" "cli_option")
  in
  let () =
    match env_stage with
    | Some stage ->
        append_step
          (stage_trace_step_of_stage_id "env_var" stage ~note:env_stage_var)
    | None ->
        append_step
          (make_stage_trace_step
             ~note:(Printf.sprintf "%s not set" env_stage_var)
             "env_var")
  in
  let default_stage =
    match cli_override with
    | Some value -> stage_id_of_string value
    | None -> (
        match registry.stage with
        | Some stage -> stage
        | None -> (
            match env_stage with
            | Some stage -> stage
            | None -> (
                match legacy_env_stage with
                | Some stage -> stage
                | None -> runtime_stage_default.default_stage)))
  in
  let base_capabilities = registry.capabilities in
  let override_capabilities =
    match target with
    | Some triple ->
        let key = normalize_key triple in
        registry.overrides
        |> List.filter_map (fun (candidate, caps) ->
               if String.equal candidate key then Some caps else None)
        |> List.concat
    | None -> []
  in
  let normalized_required_caps =
    required_capabilities
    |> List.map normalize_key
    |> List.sort_uniq String.compare
  in
  let run_config_capabilities =
    normalized_required_caps
    |> List.map (fun name -> capability_entry name None)
  in
  let combined =
    base_capabilities @ override_capabilities @ run_config_capabilities
  in
  let dedup entries =
    List.fold_left
      (fun acc (entry : capability_entry) ->
        let normalized = normalize_key entry.name in
        let filtered =
          List.filter
            (fun (existing : capability_entry) ->
              not (String.equal (normalize_key existing.name) normalized))
            acc
        in
        filtered @ [ entry ])
      [] entries
  in
  let capability_stages =
    dedup combined
    |> List.map (fun (entry : capability_entry) ->
           let stage =
             match entry.stage with
             | Some stage -> stage
             | None -> default_stage
           in
           (entry.name, stage))
  in
  (match normalized_required_caps with
  | [] -> ()
  | required_list ->
      let steps =
        capability_stages
        |> List.filter_map (fun (name, stage) ->
               let normalized = normalize_key name in
               if List.exists (fun required -> String.equal required normalized) required_list then
                 Some
                   (stage_trace_step_of_stage_id ~capability:name
                      ~note:"run_config.effects.required_capabilities"
                      "run_config" stage)
               else None)
      in
      stage_trace := !stage_trace @ steps);
  (match registry_source_path with
  | Some path ->
      let step =
        match registry.stage with
        | Some stage ->
            make_stage_trace_step ~stage:(stage_id_to_string stage) ~file:path
              "capability_json"
        | None -> make_stage_trace_step ~file:path "capability_json"
      in
      append_step step
  | None -> ());
  let override_steps =
    List.map
      (fun (target_key, (entries : capability_entry list)) ->
        let stage_candidate =
          match entries with entry :: _ -> entry.stage | [] -> registry.stage
        in
        let base_step =
          match stage_candidate with
          | Some stage -> stage_trace_step_of_stage_id "runtime_candidate" stage
          | None -> make_stage_trace_step "runtime_candidate"
        in
        let base_step =
          match registry_source_path with
          | Some path -> { base_step with file = Some path }
          | None -> base_step
        in
        { base_step with target = Some target_key })
      registry.overrides
  in
  stage_trace := !stage_trace @ override_steps;
  create_runtime_stage ~default_stage ~capability_stages
    ~stage_trace:!stage_trace ()
