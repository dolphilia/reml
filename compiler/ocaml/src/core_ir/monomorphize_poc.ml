open Type_env
open Ir
open Types

(** 実行モード *)
type mode =
  | UseDictionary
  | UseMonomorph
  | UseBoth

(** PoC パスが収集したサマリー情報 *)
module Summary = struct
  type entry = Monomorph_registry.trait_instance

  let last_mode : mode ref = ref UseDictionary
  let recorded_entries : entry list ref = ref []

  let reset () = recorded_entries := []
  let set_mode m = last_mode := m

  let record entries = recorded_entries := entries

  let mode () = !last_mode
  let entries () = !recorded_entries
end

let default_span = Ast.dummy_span

let rec build_arrow_type args ret =
  match args with
  | [] -> ret
  | arg :: rest -> TArrow (arg, build_arrow_type rest ret)

let find_function_def module_def name =
  List.find_opt (fun fn -> String.equal fn.fn_name name) module_def.function_defs

let builtin_signature trait type_args method_name =
  match (trait, type_args, method_name) with
  | "Eq", [ ty ], "eq" -> Some ([ ty; ty ], ty_bool)
  | "Ord", [ ty ], "cmp" -> Some ([ ty; ty ], ty_i32)
  | _ -> None

let wrapper_name trait type_args method_name =
  let suffix =
    match type_args with
    | [ ty ] -> Monomorph_registry.string_of_type_for_symbol ty
    | [] -> "unit"
    | _ ->
        let joined =
          String.concat "_"
            (List.map Monomorph_registry.string_of_type_for_symbol type_args)
        in
        if String.length joined = 0 then "unknown" else joined
  in
  Printf.sprintf "__%s_%s_%s_mono" trait suffix method_name

let make_param index ty =
  let name = Printf.sprintf "__arg%d" index in
  let var = VarIdGen.fresh name ty default_span in
  { param_var = var; param_default = None }

let make_var_expr var =
  make_expr (Var var) var.vty default_span

let create_wrapper ~module_def ~(entry : Monomorph_registry.trait_instance)
    ~(method_name : string) ~(target_fn : string) =
  let base_fn = find_function_def module_def target_fn in
  let params, return_ty =
    match base_fn with
    | Some fn_def ->
        (List.map (fun p -> p.param_var.vty) fn_def.fn_params, fn_def.fn_return_ty)
    | None -> (
        match builtin_signature entry.trait_name entry.type_args method_name with
        | Some signature -> signature
        | None -> ([], ty_unit))
  in
  if params = [] then None
  else
    let wrapper_fn_name = wrapper_name entry.trait_name entry.type_args method_name in
    if List.exists
         (fun fn_def -> String.equal fn_def.fn_name wrapper_fn_name)
         module_def.function_defs
    then
      None
    else
      let fn_params =
        List.mapi (fun idx ty -> make_param idx ty) params
      in
      let target_fn_ty = build_arrow_type params return_ty in
      let target_var = VarIdGen.fresh target_fn target_fn_ty default_span in
      let fn_expr = make_expr (Var target_var) target_fn_ty default_span in
      let arg_exprs =
        List.map (fun param -> make_var_expr param.param_var) fn_params
      in
      let call_expr = make_expr (App (fn_expr, arg_exprs)) return_ty default_span in
      let block =
        make_block "entry" [] [] (TermReturn call_expr) default_span
      in
      let dict_instances =
        match entry.type_args with
        | impl_ty :: _ ->
            [
              {
                trait = entry.trait_name;
                impl_ty;
                methods = [];
              };
            ]
        | [] -> []
      in
      let metadata =
        { default_metadata default_span with dict_instances }
      in
      let fn_def =
        make_function wrapper_fn_name fn_params return_ty [ block ] metadata
      in
      Some fn_def

let generate_wrappers module_def entries =
  let wrappers =
    List.fold_left
      (fun acc entry ->
        List.fold_left
          (fun acc (method_name, target_fn) ->
            match create_wrapper ~module_def ~entry ~method_name ~target_fn with
            | Some fn_def -> fn_def :: acc
            | None -> acc)
          acc entry.methods)
      [] entries
  in
  List.rev wrappers

let apply ~(mode : mode) (m : module_def) : module_def =
  Summary.set_mode mode;
  match mode with
  | UseDictionary ->
      Summary.reset ();
      m
  | UseMonomorph | UseBoth ->
      let entries = Monomorph_registry.all () in
      Summary.record entries;
      let wrappers = generate_wrappers m entries in
      if wrappers = [] then m
      else { m with function_defs = m.function_defs @ wrappers }
