(* Core_ir.Ir_printer — Pretty Printer for Core IR (Phase 3)
 *
 * Core IR の人間可読な文字列表現を生成する。
 * デバッグ、テスト、CLI の `--emit-core` オプションで使用。
 *
 * 設計原則:
 * - インデント付き階層表示
 * - 型情報の明示
 * - Span情報の表示（オプション）
 *)

open Types
open Ir

(* ========== ユーティリティ ========== *)

(** インデント文字列生成 *)
let indent n = String.make (n * 2) ' '

(** リスト要素を区切り文字で結合 *)
let join_with sep f xs = String.concat sep (List.map f xs)

(* ========== プリミティブ演算の表示 ========== *)

let string_of_prim_op = function
  | PrimAdd -> "add"
  | PrimSub -> "sub"
  | PrimMul -> "mul"
  | PrimDiv -> "div"
  | PrimMod -> "mod"
  | PrimPow -> "pow"
  | PrimEq -> "eq"
  | PrimNe -> "ne"
  | PrimLt -> "lt"
  | PrimLe -> "le"
  | PrimGt -> "gt"
  | PrimGe -> "ge"
  | PrimAnd -> "and"
  | PrimOr -> "or"
  | PrimNot -> "not"
  | PrimBitAnd -> "bit_and"
  | PrimBitOr -> "bit_or"
  | PrimBitXor -> "bit_xor"
  | PrimBitNot -> "bit_not"
  | PrimShl -> "shl"
  | PrimShr -> "shr"

(* ========== 変数とラベルの表示 ========== *)

let string_of_var_id var =
  Printf.sprintf "%s#%d : %s" var.vname var.vid (string_of_ty var.vty)

let string_of_label lbl = lbl

let string_of_dict_type dict =
  let methods_str =
    match dict.dict_methods with
    | [] -> ""
    | methods ->
        let entries =
          join_with ", "
            (fun (name, ty) -> Printf.sprintf "%s: %s" name (string_of_ty ty))
            methods
        in
        Printf.sprintf " [%s]" entries
  in
  let layout_str =
    match dict.dict_layout_info with
    | None -> ""
    | Some layout ->
        Printf.sprintf " {size=%d align=%d}" layout.vtable_size layout.alignment
  in
  Printf.sprintf "%s for %s%s%s" dict.dict_trait
    (string_of_ty dict.dict_impl_ty)
    methods_str layout_str

(* ========== リテラルの表示 ========== *)

let rec string_of_literal = function
  | Ast.Int (s, _) -> s
  | Ast.Float s -> s
  | Ast.Char s -> Printf.sprintf "'%s'" s
  | Ast.String (s, _) -> Printf.sprintf "\"%s\"" s
  | Ast.Bool true -> "true"
  | Ast.Bool false -> "false"
  | Ast.Unit -> "()"
  | Ast.Tuple exprs ->
      Printf.sprintf "(%s)" (join_with ", " string_of_ast_expr_shallow exprs)
  | Ast.Array exprs ->
      Printf.sprintf "[%s]" (join_with ", " string_of_ast_expr_shallow exprs)
  | Ast.Record fields ->
      let field_strs =
        List.map
          (fun (id, e) ->
            Printf.sprintf "%s: %s" id.Ast.name (string_of_ast_expr_shallow e))
          fields
      in
      Printf.sprintf "{ %s }" (String.concat ", " field_strs)

and string_of_ast_expr_shallow e =
  match e.Ast.expr_kind with
  | Ast.Literal lit -> string_of_literal lit
  | Ast.Var id -> id.Ast.name
  | _ -> "..."

(* ========== 簡略化パターンの表示 ========== *)

let rec string_of_simple_pattern = function
  | PLiteral lit -> string_of_literal lit
  | PVar var -> string_of_var_id var
  | PWildcard -> "_"
  | PConstructor (name, pats) ->
      if List.length pats = 0 then name
      else
        Printf.sprintf "%s(%s)" name
          (join_with ", " string_of_simple_pattern pats)

(* ========== Core IR 式の表示 ========== *)

let rec string_of_expr ?(depth = 0) expr =
  let ind = indent depth in
  let ty_str = string_of_ty expr.expr_ty in

  match expr.expr_kind with
  | Literal lit ->
      Printf.sprintf "%s(lit %s : %s)" ind (string_of_literal lit) ty_str
  | Var var -> Printf.sprintf "%s(var %s)" ind (string_of_var_id var)
  | App (fn, args) ->
      let fn_str = string_of_expr ~depth:(depth + 1) fn in
      let args_str =
        join_with "\n" (fun arg -> string_of_expr ~depth:(depth + 1) arg) args
      in
      Printf.sprintf "%s(app : %s\n%s\n%s)" ind ty_str fn_str args_str
  | Let (var, bound, body) ->
      let bound_str = string_of_expr ~depth:(depth + 1) bound in
      let body_str = string_of_expr ~depth:(depth + 1) body in
      Printf.sprintf "%s(let %s =\n%s\n%sin\n%s)" ind (string_of_var_id var)
        bound_str ind body_str
  | If (cond, then_e, else_e) ->
      let cond_str = string_of_expr ~depth:(depth + 1) cond in
      let then_str = string_of_expr ~depth:(depth + 1) then_e in
      let else_str = string_of_expr ~depth:(depth + 1) else_e in
      Printf.sprintf "%s(if : %s\n%s\n%sthen\n%s\n%selse\n%s)" ind ty_str
        cond_str ind then_str ind else_str
  | Match (scrut, cases) ->
      let scrut_str = string_of_expr ~depth:(depth + 1) scrut in
      let cases_str =
        join_with "\n" (string_of_case ~depth:(depth + 1)) cases
      in
      Printf.sprintf "%s(match : %s\n%s\n%s)" ind ty_str scrut_str cases_str
  | Primitive (op, args) ->
      let args_str =
        join_with " "
          (fun arg -> string_of_expr ~depth:0 arg |> String.trim)
          args
      in
      Printf.sprintf "%s(prim %s %s : %s)" ind (string_of_prim_op op) args_str
        ty_str
  | Closure { env_vars; fn_ref; _ } ->
      let env_str = join_with ", " string_of_var_id env_vars in
      Printf.sprintf "%s(closure %s [%s] : %s)" ind fn_ref env_str ty_str
  | DictLookup { trait_name; type_args; _ } ->
      let ty_args_str = join_with ", " string_of_ty type_args in
      Printf.sprintf "%s(dict %s<%s> : %s)" ind trait_name ty_args_str ty_str
  | DictConstruct dict ->
      Printf.sprintf "%s(dict.construct %s : %s)" ind
        (string_of_dict_type dict) ty_str
  | DictMethodCall (dict_expr, method_name, args) ->
      let dict_str = string_of_expr ~depth:0 dict_expr |> String.trim in
      let args_str =
        join_with ", "
          (fun arg -> string_of_expr ~depth:0 arg |> String.trim)
          args
      in
      Printf.sprintf "%s(dict.call %s.%s(%s) : %s)" ind dict_str method_name
        args_str ty_str
  | CapabilityCheck cap ->
      Printf.sprintf "%s(capability %s : %s)" ind cap.cap_name ty_str
  | TupleAccess (e, idx) ->
      let e_str = string_of_expr ~depth:0 e |> String.trim in
      Printf.sprintf "%s(%s.%d : %s)" ind e_str idx ty_str
  | RecordAccess (e, field) ->
      let e_str = string_of_expr ~depth:0 e |> String.trim in
      Printf.sprintf "%s(%s.%s : %s)" ind e_str field ty_str
  | ArrayAccess (arr, idx) ->
      let arr_str = string_of_expr ~depth:0 arr |> String.trim in
      let idx_str = string_of_expr ~depth:0 idx |> String.trim in
      Printf.sprintf "%s(%s[%s] : %s)" ind arr_str idx_str ty_str
  | ADTConstruct (name, fields) ->
      let fields_str =
        join_with ", "
          (fun f -> string_of_expr ~depth:0 f |> String.trim)
          fields
      in
      Printf.sprintf "%s(%s(%s) : %s)" ind name fields_str ty_str
  | ADTProject (e, idx) ->
      let e_str = string_of_expr ~depth:0 e |> String.trim in
      Printf.sprintf "%s(project %s %d : %s)" ind e_str idx ty_str
  | AssignMutable (var, rhs) ->
      let rhs_str = string_of_expr ~depth:0 rhs |> String.trim in
      Printf.sprintf "%s(set %s := %s : Unit)" ind (string_of_var_id var) rhs_str
  | Loop loop_info ->
      string_of_loop_info ~depth loop_info ty_str

and string_of_case ?(depth = 0) case =
  let ind = indent depth in
  let pat_str = string_of_simple_pattern case.case_pattern in
  let guard_str =
    match case.case_guard with
    | Some g -> " if " ^ (string_of_expr ~depth:0 g |> String.trim)
    | None -> ""
  in
  let body_str = string_of_expr ~depth:(depth + 1) case.case_body in
  Printf.sprintf "%s| %s%s =>\n%s" ind pat_str guard_str body_str

and string_of_loop_kind ?(depth = 0) = function
  | WhileLoop cond ->
      let cond_str = string_of_expr ~depth:(depth + 1) cond in
      Printf.sprintf "%swhile\n%s" (indent depth) cond_str
  | ForLoop _ ->
      Printf.sprintf "%sfor <not-yet-lowered>" (indent depth)
  | InfiniteLoop -> Printf.sprintf "%sloop" (indent depth)

and string_of_loop_source_kind = function
  | LoopSourcePreheader -> "preheader"
  | LoopSourceLatch -> "latch"
  | LoopSourceContinue -> "continue"

and string_of_loop_sources sources =
  match sources with
  | [] -> ""
  | xs ->
      xs
      |> List.map (fun src -> string_of_loop_source_kind src.ls_kind)
      |> String.concat "|"

and string_of_loop_info ?(depth = 0) info ty_str =
  let ind = indent depth in
  let kind_str = string_of_loop_kind ~depth:(depth + 1) info.loop_kind in
  let body_str = string_of_expr ~depth:(depth + 1) info.loop_body in
  let carried =
    match info.loop_carried with
    | [] -> ""
    | vars ->
        let vars_str =
          join_with ", "
            (fun { lc_var; lc_sources } ->
              let srcs = string_of_loop_sources lc_sources in
              if String.equal srcs "" then string_of_var_id lc_var
              else
                Printf.sprintf "%s<-%s" (string_of_var_id lc_var) srcs)
            vars
        in
        Printf.sprintf "\n%sphi [%s]" (indent (depth + 1)) vars_str
  in
  Printf.sprintf "%s(loop : %s\n%s\n%sbody\n%s%s)" ind ty_str kind_str ind
    body_str carried

(* ========== Core IR 文の表示 ========== *)

let string_of_stmt ?(depth = 0) = function
  | Assign (var, expr) ->
      let ind = indent depth in
      let expr_str = string_of_expr ~depth:0 expr |> String.trim in
      Printf.sprintf "%s%s := %s" ind (string_of_var_id var) expr_str
  | Store (var, expr) ->
      let ind = indent depth in
      let expr_str = string_of_expr ~depth:0 expr |> String.trim in
      Printf.sprintf "%s*%s <- %s" ind (string_of_var_id var) expr_str
  | Alloca var ->
      let ind = indent depth in
      Printf.sprintf "%salloca %s" ind (string_of_var_id var)
  | Return expr ->
      let ind = indent depth in
      let expr_str = string_of_expr ~depth:0 expr |> String.trim in
      Printf.sprintf "%sreturn %s" ind expr_str
  | Jump label ->
      let ind = indent depth in
      Printf.sprintf "%sjump %s" ind (string_of_label label)
  | Branch (cond, then_lbl, else_lbl) ->
      let ind = indent depth in
      let cond_str = string_of_expr ~depth:0 cond |> String.trim in
      Printf.sprintf "%sbranch %s ? %s : %s" ind cond_str
        (string_of_label then_lbl) (string_of_label else_lbl)
  | Phi (var, incoming) ->
      let ind = indent depth in
      let incoming_str =
        join_with ", "
          (fun (lbl, v) ->
            Printf.sprintf "[%s: %s]" (string_of_label lbl) (string_of_var_id v))
          incoming
      in
      Printf.sprintf "%s%s := φ(%s)" ind (string_of_var_id var) incoming_str
  | EffectMarker { effect_tag; effect_expr } ->
      let ind = indent depth in
      let expr_str =
        match effect_expr with
        | Some e -> " " ^ (string_of_expr ~depth:0 e |> String.trim)
        | None -> ""
      in
      Printf.sprintf "%seffect %s%s" ind effect_tag.effect_name expr_str
  | ExprStmt expr ->
      let ind = indent depth in
      let expr_str = string_of_expr ~depth:0 expr |> String.trim in
      Printf.sprintf "%s%s" ind expr_str

(* ========== 終端命令の表示 ========== *)

let string_of_terminator ?(depth = 0) = function
  | TermReturn expr ->
      let ind = indent depth in
      let expr_str = string_of_expr ~depth:0 expr |> String.trim in
      Printf.sprintf "%sret %s" ind expr_str
  | TermJump label ->
      let ind = indent depth in
      Printf.sprintf "%sjmp %s" ind (string_of_label label)
  | TermBranch (cond, then_lbl, else_lbl) ->
      let ind = indent depth in
      let cond_str = string_of_expr ~depth:0 cond |> String.trim in
      Printf.sprintf "%sbr %s ? %s : %s" ind cond_str (string_of_label then_lbl)
        (string_of_label else_lbl)
  | TermSwitch (expr, cases, default) ->
      let ind = indent depth in
      let expr_str = string_of_expr ~depth:0 expr |> String.trim in
      let cases_str =
        join_with ", "
          (fun (lit, lbl) ->
            Printf.sprintf "%s => %s" (string_of_literal lit)
              (string_of_label lbl))
          cases
      in
      Printf.sprintf "%sswitch %s { %s | _ => %s }" ind expr_str cases_str
        (string_of_label default)
  | TermUnreachable ->
      let ind = indent depth in
      Printf.sprintf "%sunreachable" ind

(* ========== 基本ブロックの表示 ========== *)

let string_of_block ?(depth = 0) block =
  let ind = indent depth in
  let params_str =
    if List.length block.params = 0 then ""
    else "(" ^ join_with ", " string_of_var_id block.params ^ ")"
  in
  let stmts_str =
    join_with "\n" (string_of_stmt ~depth:(depth + 1)) block.stmts
  in
  let term_str = string_of_terminator ~depth:(depth + 1) block.terminator in

  Printf.sprintf "%s%s%s:\n%s\n%s" ind block.label params_str stmts_str term_str

(* ========== 関数定義の表示 ========== *)

let string_of_param param =
  let default_str =
    match param.param_default with
    | Some e -> " = " ^ (string_of_expr ~depth:0 e |> String.trim)
    | None -> ""
  in
  Printf.sprintf "%s%s" (string_of_var_id param.param_var) default_str

let string_of_function fn =
  let params_str = join_with ", " string_of_param fn.fn_params in
  let ret_ty_str = string_of_ty fn.fn_return_ty in
  let blocks_str = join_with "\n\n" (string_of_block ~depth:1) fn.fn_blocks in

  Printf.sprintf "fn %s(%s) -> %s {\n%s\n}" fn.fn_name params_str ret_ty_str
    blocks_str

(* ========== モジュール定義の表示 ========== *)

let string_of_global global =
  let mutability = if global.global_mutable then "var" else "let" in
  let init_str = string_of_expr ~depth:0 global.global_init |> String.trim in
  Printf.sprintf "%s %s = %s" mutability
    (string_of_var_id global.global_var)
    init_str

let string_of_variant variant =
  if List.length variant.variant_fields = 0 then variant.variant_name
  else
    let fields_str = join_with ", " string_of_ty variant.variant_fields in
    Printf.sprintf "%s(%s)" variant.variant_name fields_str

let string_of_type_def tdef =
  let params_str =
    if List.length tdef.type_params = 0 then ""
    else "<" ^ String.concat ", " tdef.type_params ^ ">"
  in
  let variants_str = join_with "\n  | " string_of_variant tdef.type_variants in
  Printf.sprintf "type %s%s =\n  | %s" tdef.type_name params_str variants_str

let string_of_module mod_def =
  let types_str = join_with "\n\n" string_of_type_def mod_def.type_defs in
  let globals_str = join_with "\n" string_of_global mod_def.global_defs in
  let functions_str =
    join_with "\n\n" string_of_function mod_def.function_defs
  in

  Printf.sprintf "module %s\n\n%s\n\n%s\n\n%s" mod_def.module_name types_str
    globals_str functions_str
