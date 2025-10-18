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
}

let entry_key (entry : entry) =
  let capability = Option.value entry.capability ~default:"<none>" in
  let kind = Option.value entry.iterator_kind ~default:"<unspecified>" in
  Printf.sprintf "%s::%s::%s::%s::%s" entry.function_name entry.method_name
    capability kind entry.effect_tag.effect_name

let make_entry fn_name method_name audit =
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
  }

let rec collect_expr fn_name acc expr =
  match expr.expr_kind with
  | Literal _ | Var _ -> acc
  | Primitive (_op, args) -> List.fold_left (collect_expr fn_name) acc args
  | App (fn_expr, args) ->
      let acc = collect_expr fn_name acc fn_expr in
      List.fold_left (collect_expr fn_name) acc args
  | Let (_var, bound, body) ->
      let acc = collect_expr fn_name acc bound in
      collect_expr fn_name acc body
  | If (cond, then_e, else_e) ->
      let acc = collect_expr fn_name acc cond in
      let acc = collect_expr fn_name acc then_e in
      collect_expr fn_name acc else_e
  | Match (scrutinee, cases) ->
      let acc = collect_expr fn_name acc scrutinee in
      List.fold_left
        (fun acc case ->
          let acc =
            match case.case_guard with
            | Some guard -> collect_expr fn_name acc guard
            | None -> acc
          in
          collect_expr fn_name acc case.case_body)
        acc cases
  | TupleAccess (target, _) | RecordAccess (target, _) ->
      collect_expr fn_name acc target
  | ArrayAccess (target, index) ->
      let acc = collect_expr fn_name acc target in
      collect_expr fn_name acc index
  | ADTConstruct (_ctor, fields) ->
      List.fold_left (collect_expr fn_name) acc fields
  | ADTProject (target, _) -> collect_expr fn_name acc target
  | DictConstruct _ -> acc
  | DictLookup _ -> acc
  | CapabilityCheck _ -> acc
  | Closure _ -> acc
  | AssignMutable (_var, rhs) -> collect_expr fn_name acc rhs
  | Continue -> acc
  | DictMethodCall (dict_expr, method_name, args, audit_opt) -> (
      let acc = collect_expr fn_name acc dict_expr in
      let acc = List.fold_left (collect_expr fn_name) acc args in
      match audit_opt with
      | Some audit -> make_entry fn_name method_name audit :: acc
      | None -> acc)
  | Loop loop_info ->
      let acc =
        match loop_info.loop_kind with
        | WhileLoop cond -> collect_expr fn_name acc cond
        | ForLoop for_info ->
            let acc =
              List.fold_left
                (fun acc (_, init_expr) -> collect_expr fn_name acc init_expr)
                acc for_info.for_init
            in
            let acc =
              List.fold_left
                (fun acc (_, step_expr) -> collect_expr fn_name acc step_expr)
                acc for_info.for_step
            in
            collect_expr fn_name acc for_info.for_source
        | InfiniteLoop -> acc
      in
      collect_expr fn_name acc loop_info.loop_body

and collect_stmt fn_name acc = function
  | Assign (_var, expr) -> collect_expr fn_name acc expr
  | Store (_var, expr) -> collect_expr fn_name acc expr
  | Alloca _ -> acc
  | Return expr -> collect_expr fn_name acc expr
  | Jump _ -> acc
  | Branch (cond, _, _) -> collect_expr fn_name acc cond
  | Phi _ -> acc
  | EffectMarker info -> (
      match info.effect_expr with
      | Some expr -> collect_expr fn_name acc expr
      | None -> acc)
  | ExprStmt expr -> collect_expr fn_name acc expr

and collect_terminator fn_name acc = function
  | TermReturn expr -> collect_expr fn_name acc expr
  | TermJump _ -> acc
  | TermBranch (cond, _, _) -> collect_expr fn_name acc cond
  | TermSwitch (scrutinee, _cases, _default) ->
      collect_expr fn_name acc scrutinee
  | TermUnreachable -> acc

let collect_block fn_name acc block =
  let acc = List.fold_left (collect_stmt fn_name) acc block.stmts in
  collect_terminator fn_name acc block.terminator

let collect_function fn_def acc =
  List.fold_left (collect_block fn_def.fn_name) acc fn_def.fn_blocks

let collect module_def =
  List.fold_left
    (fun acc fn_def -> collect_function fn_def acc)
    [] module_def.function_defs
