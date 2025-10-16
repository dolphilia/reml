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
        { (default_metadata default_span) with dict_instances }
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

let rec count_dict_calls_in_expr expr =
  match expr.expr_kind with
  | DictMethodCall (dict_expr, _, args, _) ->
      1
      + count_dict_calls_in_expr dict_expr
      + List.fold_left
          (fun acc arg -> acc + count_dict_calls_in_expr arg)
          0 args
  | App (fn_expr, args) ->
      count_dict_calls_in_expr fn_expr
      + List.fold_left
          (fun acc arg -> acc + count_dict_calls_in_expr arg)
          0 args
  | Let (_, bound, body) ->
      count_dict_calls_in_expr bound + count_dict_calls_in_expr body
  | If (cond, then_e, else_e) ->
      count_dict_calls_in_expr cond
      + count_dict_calls_in_expr then_e
      + count_dict_calls_in_expr else_e
  | Match (scrutinee, cases) ->
      count_dict_calls_in_expr scrutinee
      + List.fold_left
          (fun acc case ->
            let acc = acc + count_dict_calls_in_expr case.case_body in
            match case.case_guard with
            | None -> acc
            | Some guard -> acc + count_dict_calls_in_expr guard)
          0 cases
  | Primitive (_, args) | ADTConstruct (_, args) ->
      List.fold_left
        (fun acc arg -> acc + count_dict_calls_in_expr arg)
        0 args
  | ArrayAccess (target, index) ->
      count_dict_calls_in_expr target + count_dict_calls_in_expr index
  | TupleAccess (target, _) | RecordAccess (target, _) | ADTProject (target, _) ->
      count_dict_calls_in_expr target
  | AssignMutable (_, rhs) -> count_dict_calls_in_expr rhs
  | Loop loop_info ->
      let kind_count =
        match loop_info.loop_kind with
        | WhileLoop cond -> count_dict_calls_in_expr cond
        | ForLoop info ->
            let init_count =
              List.fold_left
                (fun acc (_, e) -> acc + count_dict_calls_in_expr e)
                0 info.for_init
            in
            let step_count =
              List.fold_left
                (fun acc (_, e) -> acc + count_dict_calls_in_expr e)
                0 info.for_step
            in
            count_dict_calls_in_expr info.for_source + init_count + step_count
        | InfiniteLoop -> 0
      in
      kind_count + count_dict_calls_in_expr loop_info.loop_body
  | CapabilityCheck _ | DictConstruct _ | DictLookup _ -> 0
  | Closure _ | Literal _ | Var _ | Continue -> 0

let count_dict_calls_in_stmt = function
  | Assign (_, expr)
  | Store (_, expr)
  | Return expr
  | ExprStmt expr ->
      count_dict_calls_in_expr expr
  | Alloca _ -> 0
  | Branch (cond, _, _) -> count_dict_calls_in_expr cond
  | EffectMarker { effect_expr = Some expr; _ } ->
      count_dict_calls_in_expr expr
  | Jump _ | Phi _ | EffectMarker { effect_expr = None; _ } -> 0

let count_dict_calls_in_terminator = function
  | TermReturn expr -> count_dict_calls_in_expr expr
  | TermBranch (cond, _, _) -> count_dict_calls_in_expr cond
  | TermSwitch (scrutinee, _, _) -> count_dict_calls_in_expr scrutinee
  | TermJump _ | TermUnreachable -> 0

let count_dict_calls_in_block block =
  let from_stmts =
    List.fold_left
      (fun acc stmt -> acc + count_dict_calls_in_stmt stmt)
      0 block.stmts
  in
  from_stmts + count_dict_calls_in_terminator block.terminator

let count_dict_calls_in_function fn_def =
  List.fold_left
    (fun acc block -> acc + count_dict_calls_in_block block)
    0 fn_def.fn_blocks

let register_builtin_instance trait ty method_name =
  let methods =
    Monomorph_registry.builtin_methods trait ty
    |> List.filter (fun (name, _) -> String.equal name method_name)
  in
  if methods <> [] then
    Monomorph_registry.record
      Monomorph_registry.
        { trait_name = trait; type_args = [ ty ]; methods }

let make_dict_method_call trait method_name args ret_ty span =
  match args with
  | [] -> None
  | first_arg :: _ ->
      register_builtin_instance trait first_arg.expr_ty method_name;
      Desugar.generate_dict_init trait first_arg.expr_ty span
      |> Option.map (fun dict_expr ->
             make_expr
               (DictMethodCall (dict_expr, method_name, args, None)) ret_ty
               span)

let trait_info_of_primitive op =
  match op with
  | PrimEq -> Some ("Eq", "eq")
  | PrimNe -> Some ("Eq", "ne")
  | PrimLt -> Some ("Ord", "lt")
  | PrimGt -> Some ("Ord", "gt")
  | PrimLe -> Some ("Ord", "le")
  | PrimGe -> Some ("Ord", "ge")
  | _ -> None

let rec convert_primitives_expr expr =
  match expr.expr_kind with
  | Primitive (op, args) -> (
      let args' = List.map convert_primitives_expr args in
      match trait_info_of_primitive op with
      | Some (trait, method_name) -> (
          match make_dict_method_call trait method_name args' expr.expr_ty expr.expr_span with
          | Some dict_call -> dict_call
          | None -> make_expr (Primitive (op, args')) expr.expr_ty expr.expr_span)
      | None -> make_expr (Primitive (op, args')) expr.expr_ty expr.expr_span)
  | DictMethodCall (dict_expr, method_name, method_args, audit) ->
      let dict_expr' = convert_primitives_expr dict_expr in
      let args' = List.map convert_primitives_expr method_args in
      make_expr (DictMethodCall (dict_expr', method_name, args', audit))
        expr.expr_ty expr.expr_span
  | App (fn_expr, args) ->
      let fn_expr' = convert_primitives_expr fn_expr in
      let args' = List.map convert_primitives_expr args in
      make_expr (App (fn_expr', args')) expr.expr_ty expr.expr_span
  | Let (var, bound, body) ->
      let bound' = convert_primitives_expr bound in
      let body' = convert_primitives_expr body in
      make_expr (Let (var, bound', body')) expr.expr_ty expr.expr_span
  | If (cond, then_e, else_e) ->
      let cond' = convert_primitives_expr cond in
      let then' = convert_primitives_expr then_e in
      let else' = convert_primitives_expr else_e in
      make_expr (If (cond', then', else')) expr.expr_ty expr.expr_span
  | Match (scrutinee, cases) ->
      let scrutinee' = convert_primitives_expr scrutinee in
      let cases' =
        List.map
          (fun case ->
            let case_body = convert_primitives_expr case.case_body in
            let guard =
              match case.case_guard with
              | None -> None
              | Some guard_expr -> Some (convert_primitives_expr guard_expr)
            in
            { case with case_body; case_guard = guard })
          cases
      in
      make_expr (Match (scrutinee', cases')) expr.expr_ty expr.expr_span
  | TupleAccess (target, index) ->
      let target' = convert_primitives_expr target in
      make_expr (TupleAccess (target', index)) expr.expr_ty expr.expr_span
  | RecordAccess (target, field) ->
      let target' = convert_primitives_expr target in
      make_expr (RecordAccess (target', field)) expr.expr_ty expr.expr_span
  | ArrayAccess (arr, idx) ->
      let arr' = convert_primitives_expr arr in
      let idx' = convert_primitives_expr idx in
      make_expr (ArrayAccess (arr', idx')) expr.expr_ty expr.expr_span
  | ADTConstruct (ctor, fields) ->
      let fields' = List.map convert_primitives_expr fields in
      make_expr (ADTConstruct (ctor, fields')) expr.expr_ty expr.expr_span
  | ADTProject (target, field_index) ->
      let target' = convert_primitives_expr target in
      make_expr (ADTProject (target', field_index)) expr.expr_ty expr.expr_span
  | AssignMutable (var, rhs) ->
      let rhs' = convert_primitives_expr rhs in
      make_expr (AssignMutable (var, rhs')) expr.expr_ty expr.expr_span
  | Loop loop_info ->
      let loop_kind' =
        match loop_info.loop_kind with
        | WhileLoop cond -> WhileLoop (convert_primitives_expr cond)
        | ForLoop info ->
            let init' =
              List.map
                (fun (var, e) -> (var, convert_primitives_expr e))
                info.for_init
            in
            let step' =
              List.map
                (fun (var, e) -> (var, convert_primitives_expr e))
                info.for_step
            in
            ForLoop
              {
                info with
                for_source = convert_primitives_expr info.for_source;
                for_init = init';
                for_step = step';
              }
        | InfiniteLoop -> InfiniteLoop
      in
      let loop_body' = convert_primitives_expr loop_info.loop_body in
      make_expr
        (Loop
           {
             loop_info with
             loop_kind = loop_kind';
             loop_body = loop_body';
           })
        expr.expr_ty expr.expr_span
  | CapabilityCheck _ | DictConstruct _ | DictLookup _ | Closure _
  | Literal _ | Var _ | Continue ->
      expr

let convert_primitives_stmt stmt =
  match stmt with
  | Assign (var, expr) -> Assign (var, convert_primitives_expr expr)
  | Store (var, expr) -> Store (var, convert_primitives_expr expr)
  | Alloca var -> Alloca var
  | Return expr -> Return (convert_primitives_expr expr)
  | ExprStmt expr -> ExprStmt (convert_primitives_expr expr)
  | Branch (cond, t_lbl, f_lbl) ->
      Branch (convert_primitives_expr cond, t_lbl, f_lbl)
  | EffectMarker info ->
      let effect_expr =
        match info.effect_expr with
        | None -> None
        | Some expr -> Some (convert_primitives_expr expr)
      in
      EffectMarker { info with effect_expr }
  | Jump _ | Phi _ -> stmt

let convert_primitives_terminator terminator =
  match terminator with
  | TermReturn expr -> TermReturn (convert_primitives_expr expr)
  | TermBranch (cond, t_lbl, f_lbl) ->
      TermBranch (convert_primitives_expr cond, t_lbl, f_lbl)
  | TermSwitch (scrutinee, cases, default_lbl) ->
      TermSwitch
        (convert_primitives_expr scrutinee, cases, default_lbl)
  | TermJump _ | TermUnreachable -> terminator

let convert_primitives_block block =
  let stmts = List.map convert_primitives_stmt block.stmts in
  let terminator = convert_primitives_terminator block.terminator in
  { block with stmts; terminator }

let convert_primitives_function fn_def =
  let fn_blocks = List.map convert_primitives_block fn_def.fn_blocks in
  { fn_def with fn_blocks }

let convert_primitives_module module_def =
  let function_defs =
    List.map convert_primitives_function module_def.function_defs
  in
  { module_def with function_defs }

let wrapper_exists wrappers name =
  List.exists (fun fn_def -> String.equal fn_def.fn_name name) wrappers

let method_covers_args (entry : Monomorph_registry.trait_instance)
    (args : expr list) =
  match entry.type_args with
  | [] -> true
  | type_args ->
      List.for_all
        (fun ty_arg ->
          List.exists (fun arg -> type_equal ty_arg arg.expr_ty) args)
        type_args

let find_matching_entry entries method_name args =
  List.find_opt
    (fun (entry : Monomorph_registry.trait_instance) ->
      method_covers_args entry args
      && List.exists
           (fun (registered_method, _) ->
             String.equal registered_method method_name)
           entry.methods)
    entries

let make_wrapper_call wrapper_name args ret_ty span =
  let fn_ty = build_arrow_type (List.map (fun arg -> arg.expr_ty) args) ret_ty in
  let fn_var = VarIdGen.fresh wrapper_name fn_ty span in
  let fn_expr = make_expr (Var fn_var) fn_ty span in
  make_expr (App (fn_expr, args)) ret_ty span

let rec rewrite_expr ~entries ~wrappers expr =
  match expr.expr_kind with
  | DictMethodCall (dict_expr, method_name, args, audit) ->
      let dict_expr' = rewrite_expr ~entries ~wrappers dict_expr in
      let args' = List.map (rewrite_expr ~entries ~wrappers) args in
      let replacement =
        match find_matching_entry entries method_name args' with
        | Some entry ->
            let wrapper_fn_name =
              wrapper_name entry.trait_name entry.type_args method_name
            in
            if wrapper_exists wrappers wrapper_fn_name then
              Some
                (make_wrapper_call wrapper_fn_name args' expr.expr_ty
                   expr.expr_span)
            else None
        | None -> None
      in
      let fallback =
        match replacement with
        | Some _ as value -> value
        | None -> (
            match method_name with
            | "lt" -> Some (make_expr (Primitive (PrimLt, args')) expr.expr_ty expr.expr_span)
            | "gt" -> Some (make_expr (Primitive (PrimGt, args')) expr.expr_ty expr.expr_span)
            | "le" -> Some (make_expr (Primitive (PrimLe, args')) expr.expr_ty expr.expr_span)
            | "ge" -> Some (make_expr (Primitive (PrimGe, args')) expr.expr_ty expr.expr_span)
            | _ -> None)
      in
      Option.value
        ~default:
          (make_expr (DictMethodCall (dict_expr', method_name, args', audit))
             expr.expr_ty expr.expr_span)
        fallback
  | App (fn_expr, args) ->
      let fn_expr' = rewrite_expr ~entries ~wrappers fn_expr in
      let args' = List.map (rewrite_expr ~entries ~wrappers) args in
      make_expr (App (fn_expr', args')) expr.expr_ty expr.expr_span
  | Let (var, bound, body) ->
      let bound' = rewrite_expr ~entries ~wrappers bound in
      let body' = rewrite_expr ~entries ~wrappers body in
      make_expr (Let (var, bound', body')) expr.expr_ty expr.expr_span
  | If (cond, then_e, else_e) ->
      let cond' = rewrite_expr ~entries ~wrappers cond in
      let then' = rewrite_expr ~entries ~wrappers then_e in
      let else' = rewrite_expr ~entries ~wrappers else_e in
      make_expr (If (cond', then', else')) expr.expr_ty expr.expr_span
  | Match (scrutinee, cases) ->
      let scrutinee' = rewrite_expr ~entries ~wrappers scrutinee in
      let cases' =
        List.map
          (fun case ->
            let case_body =
              rewrite_expr ~entries ~wrappers case.case_body
            in
            let guard =
              match case.case_guard with
              | None -> None
              | Some guard_expr ->
                  Some (rewrite_expr ~entries ~wrappers guard_expr)
            in
            { case with case_body; case_guard = guard })
          cases
      in
      make_expr (Match (scrutinee', cases')) expr.expr_ty expr.expr_span
  | Primitive (op, args) ->
      let args' = List.map (rewrite_expr ~entries ~wrappers) args in
      make_expr (Primitive (op, args')) expr.expr_ty expr.expr_span
  | TupleAccess (target, index) ->
      let target' = rewrite_expr ~entries ~wrappers target in
      make_expr (TupleAccess (target', index)) expr.expr_ty expr.expr_span
  | RecordAccess (target, field) ->
      let target' = rewrite_expr ~entries ~wrappers target in
      make_expr (RecordAccess (target', field)) expr.expr_ty expr.expr_span
  | ArrayAccess (arr, idx) ->
      let arr' = rewrite_expr ~entries ~wrappers arr in
      let idx' = rewrite_expr ~entries ~wrappers idx in
      make_expr (ArrayAccess (arr', idx')) expr.expr_ty expr.expr_span
  | ADTConstruct (ctor, fields) ->
      let fields' = List.map (rewrite_expr ~entries ~wrappers) fields in
      make_expr (ADTConstruct (ctor, fields')) expr.expr_ty expr.expr_span
  | ADTProject (target, field_index) ->
      let target' = rewrite_expr ~entries ~wrappers target in
      make_expr (ADTProject (target', field_index)) expr.expr_ty expr.expr_span
  | AssignMutable (var, rhs) ->
      let rhs' = rewrite_expr ~entries ~wrappers rhs in
      make_expr (AssignMutable (var, rhs')) expr.expr_ty expr.expr_span
  | Loop info ->
      let loop_kind' =
        match info.loop_kind with
        | WhileLoop cond ->
            WhileLoop (rewrite_expr ~entries ~wrappers cond)
        | ForLoop for_info ->
            let init' =
              List.map
                (fun (var, e) ->
                  (var, rewrite_expr ~entries ~wrappers e))
                for_info.for_init
            in
            let step' =
              List.map
                (fun (var, e) ->
                  (var, rewrite_expr ~entries ~wrappers e))
                for_info.for_step
            in
            ForLoop
              {
                for_info with
                for_source =
                  rewrite_expr ~entries ~wrappers for_info.for_source;
                for_init = init';
                for_step = step';
              }
        | InfiniteLoop -> InfiniteLoop
      in
      let loop_body' = rewrite_expr ~entries ~wrappers info.loop_body in
      make_expr
        (Loop
           {
             info with
             loop_kind = loop_kind';
             loop_body = loop_body';
           })
        expr.expr_ty expr.expr_span
  | Closure _ | Literal _ | Var _ | DictLookup _ | DictConstruct _
  | CapabilityCheck _ | Continue ->
      expr

let rewrite_effect_info ~entries ~wrappers info =
  let effect_expr =
    match info.effect_expr with
    | None -> None
    | Some expr -> Some (rewrite_expr ~entries ~wrappers expr)
  in
  { info with effect_expr }

let rewrite_stmt ~entries ~wrappers = function
  | Assign (var, expr) -> Assign (var, rewrite_expr ~entries ~wrappers expr)
  | Store (var, expr) -> Store (var, rewrite_expr ~entries ~wrappers expr)
  | Alloca var -> Alloca var
  | Return expr -> Return (rewrite_expr ~entries ~wrappers expr)
  | Jump lbl -> Jump lbl
  | Branch (cond, then_lbl, else_lbl) ->
      Branch (rewrite_expr ~entries ~wrappers cond, then_lbl, else_lbl)
  | Phi _ as phi -> phi
  | EffectMarker info ->
      EffectMarker (rewrite_effect_info ~entries ~wrappers info)
  | ExprStmt expr -> ExprStmt (rewrite_expr ~entries ~wrappers expr)

let rewrite_terminator ~entries ~wrappers = function
  | TermReturn expr -> TermReturn (rewrite_expr ~entries ~wrappers expr)
  | TermJump lbl -> TermJump lbl
  | TermBranch (cond, then_lbl, else_lbl) ->
      TermBranch (rewrite_expr ~entries ~wrappers cond, then_lbl, else_lbl)
  | TermSwitch (scrutinee, cases, default_lbl) ->
      TermSwitch
        (rewrite_expr ~entries ~wrappers scrutinee, cases, default_lbl)
  | TermUnreachable -> TermUnreachable

let rewrite_block ~entries ~wrappers block =
  let stmts =
    List.map (rewrite_stmt ~entries ~wrappers) block.stmts
  in
  let terminator =
    rewrite_terminator ~entries ~wrappers block.terminator
  in
  { block with stmts; terminator }

let rewrite_function ~entries ~wrappers fn_def =
  let fn_blocks =
    List.map (rewrite_block ~entries ~wrappers) fn_def.fn_blocks
  in
  { fn_def with fn_blocks }

let rewrite_module ~entries ~wrappers module_def =
  let function_defs =
    List.map (rewrite_function ~entries ~wrappers) module_def.function_defs
  in
  { module_def with function_defs }

let apply ~(mode : mode) (m : module_def) : module_def =
  Summary.set_mode mode;
  match mode with
  | UseDictionary ->
      Summary.reset ();
      let converted = convert_primitives_module m in
      converted
  | UseMonomorph | UseBoth ->
      let converted = convert_primitives_module m in
      let entries = Monomorph_registry.all () in
      Summary.record entries;
      let wrappers = generate_wrappers converted entries in
      let base_module = rewrite_module ~entries ~wrappers converted in
      if wrappers = [] then base_module
      else
        {
          base_module with
          function_defs = base_module.function_defs @ wrappers;
        }
