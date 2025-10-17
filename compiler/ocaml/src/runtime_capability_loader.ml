(* Runtime_capability_loader — Resolve runtime Stage/Capability information *)

open Effect_profile
open Type_inference_effect

module Json = Yojson.Basic

type registry = {
  default_stage : stage_id option;
  capability_stages : (string * stage_id) list;
}

let empty_registry = { default_stage = None; capability_stages = [] }

let env_stage_var = "REML_RUNTIME_STAGE"
let env_registry_var = "REML_RUNTIME_CAPABILITIES"

let stage_id_of_string_opt value =
  let trimmed = String.trim value in
  if String.equal trimmed "" then None
  else Some (stage_id_of_string trimmed)

let parse_capabilities json =
  match json with
  | `Assoc fields ->
      fields
      |> List.filter_map (fun (name, value) ->
             match value with
             | `String stage -> Some (name, stage_id_of_string stage)
             | _ -> None)
  | _ -> []

let load_registry_from_file path =
  try
    match Json.from_file path with
    | `Assoc fields ->
        let default_stage =
          match List.assoc_opt "default_stage" fields with
          | Some (`String stage) -> Some (stage_id_of_string stage)
          | _ -> None
        in
        let capability_stages =
          match List.assoc_opt "capabilities" fields with
          | Some value -> parse_capabilities value
          | None -> []
        in
        {
          default_stage;
          capability_stages =
            List.map
              (fun (name, stage) -> (String.lowercase_ascii name, stage))
              capability_stages;
        }
    | _ -> empty_registry
  with
  | Sys_error _ | Json.Json_error _ -> empty_registry

let registry_from_path path =
  match path with
  | Some p -> load_registry_from_file p
  | None -> empty_registry

let resolve ~cli_override ~registry_path =
  let env_registry =
    registry_from_path
      (match Sys.getenv_opt env_registry_var with
      | Some path -> Some path
      | None -> None)
  in
  let registry =
    let direct = registry_from_path registry_path in
    if direct != empty_registry then direct else env_registry
  in
  let env_stage =
    match Sys.getenv_opt env_stage_var with
    | Some value -> stage_id_of_string_opt value
    | None -> None
  in
  let default_stage =
    match cli_override with
    | Some stage -> stage_id_of_string stage
    | None -> (
        match registry.default_stage with
        | Some stage -> stage
        | None -> (
            match env_stage with
            | Some stage -> stage
            | None ->
                runtime_stage_default.default_stage))
  in
  let capability_stages = registry.capability_stages in
  Type_inference_effect.create_runtime_stage ~default_stage
    ~capability_stages ()
