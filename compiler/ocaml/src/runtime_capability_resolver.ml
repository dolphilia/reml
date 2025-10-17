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

type capability_entry = {
  name : string;
  stage : stage_id option;
}

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

let capability_entry name stage =
  { name = String.trim name; stage }

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
  | `Assoc fields -> (
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
             | None -> { entry with stage = stage_override }))
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
          match stage with Some s -> s | None -> runtime_stage_default.default_stage
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
  with
  | Sys_error _ | Yojson.Json_error _ -> empty_registry

let registry_from_path = function
  | Some path when String.trim path <> "" -> load_registry_from_file path
  | _ -> empty_registry

let resolve ~cli_override ~registry_path ~target =
  let registry =
    match registry_path with
    | Some path -> load_registry_from_file path
    | None ->
        registry_from_path (Sys.getenv_opt env_registry_var)
  in
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
  let combined = base_capabilities @ override_capabilities in
  let dedup entries =
    List.fold_left
      (fun acc (entry : capability_entry) ->
        let normalized = normalize_key entry.name in
        let filtered =
          List.filter
            (fun (existing : capability_entry) ->
              not
                (String.equal
                   (normalize_key existing.name)
                   normalized))
            acc
        in
        filtered @ [ entry ])
      [] entries
  in
  let capability_stages =
    dedup combined
    |> List.map (fun (entry : capability_entry) ->
           let stage =
             match entry.stage with Some stage -> stage | None -> default_stage
           in
           (entry.name, stage))
  in
  create_runtime_stage ~default_stage ~capability_stages ()
