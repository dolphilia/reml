(* Core_ir.Iterator_audit — Core IR 内のイテレータ監査情報抽出 (Phase 2-2) *)

open Ir

type entry = {
  function_name : string;
  method_name : string;
  required_stage : stage_requirement option;
  actual_stage : Effect.stage_id option;
  capability : string option;
  iterator_kind : string option;
  iterator_source : string option;
  effect_tag : effect_tag;
  effect_row : Types.effect_row option;
}

let entry_from_stage_metadata fn_name method_name span
    (stage : Type_inference.typeclass_stage_metadata)
    ~(effect_row : Types.effect_row option) : entry option =
  let open Typeclass_metadata in
  let required_stage =
    match stage.stage_required with
    | Some (Constraint_solver.IteratorStageExact stage_name) ->
        Some (StageExact (Effect.stage_id_of_string stage_name))
    | Some (Constraint_solver.IteratorStageAtLeast stage_name) ->
        Some (StageAtLeast (Effect.stage_id_of_string stage_name))
    | None -> None
  in
  match required_stage with
  | None -> None
  | Some required_stage ->
      let actual_stage =
        match stage.stage_actual with
        | Some actual ->
            let trimmed = String.trim actual in
            if String.equal trimmed "" then None
            else Some (Effect.stage_id_of_string trimmed)
        | None -> None
      in
      let capability =
        match stage.stage_capability with
        | Some cap ->
            let normalized = String.lowercase_ascii (String.trim cap) in
            if String.equal normalized "" then None else Some normalized
        | None -> None
      in
      let iterator_kind =
        match stage.stage_iterator_kind with
        | Some Constraint_solver.IteratorArrayLike -> Some "array_like"
        | Some Constraint_solver.IteratorCoreIter -> Some "core_iter"
        | Some Constraint_solver.IteratorOptionLike -> Some "option_like"
        | Some Constraint_solver.IteratorResultLike -> Some "result_like"
        | Some (Constraint_solver.IteratorCustom name) ->
            Some (Printf.sprintf "custom:%s" name)
        | None -> None
      in
      let iterator_source =
        match stage.stage_iterator_source with
        | Some src ->
            let trimmed = String.trim src in
            if String.equal trimmed "" then None else Some trimmed
        | None -> None
      in
      let effect_tag : effect_tag =
        {
          effect_name =
            Printf.sprintf "effect.stage.iterator.%s" method_name;
          effect_span = span;
        }
      in
      Some
        {
          function_name = fn_name;
          method_name;
          required_stage = Some required_stage;
          actual_stage;
          capability;
          iterator_kind;
          iterator_source;
          effect_tag;
          effect_row;
        }

let entry_key (entry : entry) =
  let capability = Option.value entry.capability ~default:"<none>" in
  let kind = Option.value entry.iterator_kind ~default:"<unspecified>" in
  Printf.sprintf "%s::%s::%s::%s::%s" entry.function_name entry.method_name
    capability kind entry.effect_tag.effect_name

let make_entry fn_name method_name audit ~(effect_row : Types.effect_row option) =
  {
    function_name = fn_name;
    method_name;
    required_stage = audit.audit_required_stage;
    actual_stage = audit.audit_actual_stage;
    capability =
      Option.map
        (fun cap -> String.lowercase_ascii (String.trim cap.cap_name))
        audit.audit_capability;
    iterator_kind = audit.audit_iterator_kind;
    iterator_source = audit.audit_iterator_source;
    effect_tag = audit.audit_effect;
    effect_row;
  }

let rec collect_expr fn_name effect_row acc expr =
  match expr.expr_kind with
  | Literal _ | Var _ -> acc
  | Primitive (_op, args) ->
      List.fold_left (collect_expr fn_name effect_row) acc args
  | App (fn_expr, args) ->
      let acc = collect_expr fn_name effect_row acc fn_expr in
      List.fold_left (collect_expr fn_name effect_row) acc args
  | Let (_var, bound, body) ->
      let acc = collect_expr fn_name effect_row acc bound in
      collect_expr fn_name effect_row acc body
  | If (cond, then_e, else_e) ->
      let acc = collect_expr fn_name effect_row acc cond in
      let acc = collect_expr fn_name effect_row acc then_e in
      collect_expr fn_name effect_row acc else_e
  | Match (scrutinee, cases) ->
      let acc = collect_expr fn_name effect_row acc scrutinee in
      List.fold_left
        (fun acc case ->
          let acc =
            match case.case_guard with
            | Some guard -> collect_expr fn_name effect_row acc guard
            | None -> acc
          in
          collect_expr fn_name effect_row acc case.case_body)
        acc cases
  | TupleAccess (target, _) | RecordAccess (target, _) ->
      collect_expr fn_name effect_row acc target
  | ArrayAccess (target, index) ->
      let acc = collect_expr fn_name effect_row acc target in
      collect_expr fn_name effect_row acc index
  | ADTConstruct (_ctor, fields) ->
      List.fold_left (collect_expr fn_name effect_row) acc fields
  | ADTProject (target, _) -> collect_expr fn_name effect_row acc target
  | DictConstruct _ -> acc
  | DictLookup _ -> acc
  | CapabilityCheck _ -> acc
  | Closure _ -> acc
  | AssignMutable (_var, rhs) -> collect_expr fn_name effect_row acc rhs
  | Continue -> acc
  | DictMethodCall (dict_expr, method_name, args, audit_opt) -> (
      let acc = collect_expr fn_name effect_row acc dict_expr in
      let acc = List.fold_left (collect_expr fn_name effect_row) acc args in
      match audit_opt with
      | Some audit ->
          make_entry fn_name method_name audit ~effect_row :: acc
      | None -> (
          let fallback_entry =
            match dict_expr.expr_kind with
            | Var var ->
                Option.bind
                  (Type_inference.lookup_typeclass_stage_binding var.vname)
                  (fun stage ->
                    entry_from_stage_metadata fn_name method_name
                      dict_expr.expr_span stage ~effect_row)
            | _ -> None
          in
          match fallback_entry with Some entry -> entry :: acc | None -> acc))
  | Loop loop_info ->
      let acc =
        match loop_info.loop_kind with
        | WhileLoop cond -> collect_expr fn_name effect_row acc cond
        | ForLoop for_info ->
            let acc =
              List.fold_left
                (fun acc (_, init_expr) ->
                  collect_expr fn_name effect_row acc init_expr)
                acc for_info.for_init
            in
            let acc =
              List.fold_left
                (fun acc (_, step_expr) ->
                  collect_expr fn_name effect_row acc step_expr)
                acc for_info.for_step
            in
            collect_expr fn_name effect_row acc for_info.for_source
        | InfiniteLoop -> acc
      in
      collect_expr fn_name effect_row acc loop_info.loop_body

and collect_stmt fn_name effect_row acc = function
  | Assign (_var, expr) -> collect_expr fn_name effect_row acc expr
  | Store (_var, expr) -> collect_expr fn_name effect_row acc expr
  | Alloca _ -> acc
  | Return expr -> collect_expr fn_name effect_row acc expr
  | Jump _ -> acc
  | Branch (cond, _, _) ->
      collect_expr fn_name effect_row acc cond
  | Phi _ -> acc
  | EffectMarker info -> (
      match info.effect_expr with
      | Some expr -> collect_expr fn_name effect_row acc expr
      | None -> acc)
  | ExprStmt expr -> collect_expr fn_name effect_row acc expr

and collect_terminator fn_name effect_row acc = function
  | TermReturn expr -> collect_expr fn_name effect_row acc expr
  | TermJump _ -> acc
  | TermBranch (cond, _, _) ->
      collect_expr fn_name effect_row acc cond
  | TermSwitch (scrutinee, _cases, _default) ->
      collect_expr fn_name effect_row acc scrutinee
  | TermUnreachable -> acc

let collect_block fn_name effect_row acc block =
  let acc =
    List.fold_left (collect_stmt fn_name effect_row) acc block.stmts
  in
  collect_terminator fn_name effect_row acc block.terminator

let collect_function fn_def acc =
  let effect_row = fn_def.fn_metadata.effect_row in
  List.fold_left (collect_block fn_def.fn_name effect_row) acc fn_def.fn_blocks

let collect module_def =
  List.fold_left
    (fun acc fn_def -> collect_function fn_def acc)
    [] module_def.function_defs
