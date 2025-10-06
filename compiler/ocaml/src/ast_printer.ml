open Ast

let string_of_ident id = id.name

let string_of_relative_head = function
  | Self -> "self"
  | Super n ->
      if n <= 0 then "self"
      else String.concat "." (List.init n (fun _ -> "super"))
  | PlainIdent id -> string_of_ident id

let string_of_module_path = function
  | Root ids ->
      "::" ^ String.concat "." (List.map string_of_ident ids)
  | Relative (head, tail) ->
      let head_str = string_of_relative_head head in
      match tail with
      | [] -> head_str
      | _ -> head_str ^ "." ^ String.concat "." (List.map string_of_ident tail)

let string_of_int_base = function
  | Base2 -> "base2"
  | Base8 -> "base8"
  | Base10 -> "base10"
  | Base16 -> "base16"

let string_of_string_kind = function
  | Normal -> "normal"
  | Raw -> "raw"
  | Multiline -> "multiline"

let rec string_of_type ty =
  match ty.ty_kind with
  | TyIdent id -> string_of_ident id
  | TyApp (id, args) ->
      let args_str = args |> List.map string_of_type |> String.concat ", " in
      Printf.sprintf "%s<%s>" (string_of_ident id) args_str
  | TyTuple tys ->
      tys |> List.map string_of_type |> String.concat ", " |> Printf.sprintf "(%s)"
  | TyRecord fields ->
      fields
      |> List.map (fun (id, ty) -> Printf.sprintf "%s: %s" (string_of_ident id) (string_of_type ty))
      |> String.concat ", "
      |> Printf.sprintf "{ %s }"
  | TyFn (args, ret) ->
      let args_str = args |> List.map string_of_type |> String.concat " -> " in
      Printf.sprintf "%s -> %s" args_str (string_of_type ret)

let rec string_of_literal = function
  | Int (value, base) -> Printf.sprintf "int(%s:%s)" value (string_of_int_base base)
  | Float f -> Printf.sprintf "float(%s)" f
  | Char c -> Printf.sprintf "char(%S)" c
  | String (s, kind) -> Printf.sprintf "string[%s](%S)" (string_of_string_kind kind) s
  | Bool b -> if b then "bool(true)" else "bool(false)"
  | Unit -> "unit"
  | Tuple exprs ->
      exprs |> List.map string_of_expr |> String.concat ", " |> Printf.sprintf "tuple(%s)"
  | Array exprs ->
      exprs |> List.map string_of_expr |> String.concat ", " |> Printf.sprintf "array[%s]"
  | Record fields ->
      fields
      |> List.map (fun (id, expr) -> Printf.sprintf "%s = %s" (string_of_ident id) (string_of_expr expr))
      |> String.concat ", "
      |> Printf.sprintf "record{%s}"

and string_of_pattern pat =
  match pat.pat_kind with
  | PatLiteral lit -> string_of_literal lit
  | PatVar id -> string_of_ident id
  | PatWildcard -> "_"
  | PatTuple pats ->
      pats |> List.map string_of_pattern |> String.concat ", " |> Printf.sprintf "(%s)"
  | PatRecord (fields, has_rest) ->
      let field_strs =
        fields
        |> List.map (fun (id, pat_opt) ->
               match pat_opt with
               | None -> string_of_ident id
               | Some p -> Printf.sprintf "%s: %s" (string_of_ident id) (string_of_pattern p))
      in
      let rest = if has_rest then "; .." else "" in
      Printf.sprintf "{ %s%s }" (String.concat ", " field_strs) rest
  | PatConstructor (id, args) ->
      if args = [] then string_of_ident id
      else
        args
        |> List.map string_of_pattern
        |> String.concat ", "
        |> Printf.sprintf "%s(%s)" (string_of_ident id)
  | PatGuard (pat, expr) ->
      Printf.sprintf "%s if %s" (string_of_pattern pat) (string_of_expr expr)

and string_of_arg = function
  | PosArg expr -> string_of_expr expr
  | NamedArg (id, expr) -> Printf.sprintf "%s = %s" (string_of_ident id) (string_of_expr expr)

and string_of_stmt = function
  | DeclStmt decl -> "decl:" ^ string_of_decl_kind decl.decl_kind
  | ExprStmt expr -> "expr:" ^ string_of_expr expr
  | AssignStmt (id, expr) -> Printf.sprintf "assign %s := %s" (string_of_ident id) (string_of_expr expr)
  | DeferStmt expr -> Printf.sprintf "defer %s" (string_of_expr expr)

and string_of_binary_op = function
  | Add -> "+"
  | Sub -> "-"
  | Mul -> "*"
  | Div -> "/"
  | Mod -> "%"
  | Pow -> "^"
  | Eq -> "=="
  | Ne -> "!="
  | Lt -> "<"
  | Le -> "<="
  | Gt -> ">"
  | Ge -> ">="
  | And -> "&&"
  | Or -> "||"
  | PipeOp -> "|>"

and string_of_unary_op = function
  | Not -> "!"
  | Neg -> "-"

and string_of_expr expr =
  match expr.expr_kind with
  | Literal lit -> string_of_literal lit
  | Var id -> Printf.sprintf "var(%s)" (string_of_ident id)
  | ModulePath (path, id) -> Printf.sprintf "%s.%s" (string_of_module_path path) (string_of_ident id)
  | Call (fn, args) ->
      let args_str = args |> List.map string_of_arg |> String.concat ", " in
      Printf.sprintf "call(%s)[%s]" (string_of_expr fn) args_str
  | Lambda (params, ret, body) ->
      let params_str = params |> List.map string_of_param |> String.concat ", " in
      let ret_str = match ret with None -> "" | Some ty -> Printf.sprintf " -> %s" (string_of_type ty) in
      Printf.sprintf "lambda(%s%s => %s)" params_str ret_str (string_of_expr body)
  | Pipe (lhs, rhs) -> Printf.sprintf "pipe(%s |> %s)" (string_of_expr lhs) (string_of_expr rhs)
  | Binary (op, lhs, rhs) ->
      Printf.sprintf "binary(%s %s %s)" (string_of_expr lhs) (string_of_binary_op op) (string_of_expr rhs)
  | Unary (op, expr) -> Printf.sprintf "unary(%s%s)" (string_of_unary_op op) (string_of_expr expr)
  | FieldAccess (expr, id) -> Printf.sprintf "%s.%s" (string_of_expr expr) (string_of_ident id)
  | TupleAccess (expr, idx) -> Printf.sprintf "%s.%d" (string_of_expr expr) idx
  | Index (expr, idx) -> Printf.sprintf "%s[%s]" (string_of_expr expr) (string_of_expr idx)
  | Propagate expr -> Printf.sprintf "%s?" (string_of_expr expr)
  | If (cond, then_, else_) ->
      let else_str = match else_ with None -> "none" | Some e -> string_of_expr e in
      Printf.sprintf "if(%s, %s, %s)" (string_of_expr cond) (string_of_expr then_) else_str
  | Match (expr, arms) ->
      let arms_str =
        arms
        |> List.map (fun arm ->
               let guard = match arm.arm_guard with None -> "" | Some g -> " if " ^ string_of_expr g in
               Printf.sprintf "%s%s => %s"
                 (string_of_pattern arm.arm_pattern) guard (string_of_expr arm.arm_body))
        |> String.concat " | "
      in
      Printf.sprintf "match(%s) { %s }" (string_of_expr expr) arms_str
  | While (cond, body) -> Printf.sprintf "while(%s) { %s }" (string_of_expr cond) (string_of_expr body)
  | For (pat, source, body) ->
      Printf.sprintf "for(%s in %s) { %s }" (string_of_pattern pat) (string_of_expr source) (string_of_expr body)
  | Loop body -> Printf.sprintf "loop { %s }" (string_of_expr body)
  | Block stmts ->
      let stmt_str = stmts |> List.map string_of_stmt |> String.concat "; " in
      Printf.sprintf "block[%s]" stmt_str
  | Unsafe body -> Printf.sprintf "unsafe { %s }" (string_of_expr body)
  | Return None -> "return"
  | Return (Some expr) -> Printf.sprintf "return %s" (string_of_expr expr)
  | Defer expr -> Printf.sprintf "defer %s" (string_of_expr expr)
  | Assign (id, expr) -> Printf.sprintf "%s := %s" (string_of_ident id) (string_of_expr expr)

and string_of_param param =
  let pat = string_of_pattern param.pat in
  let ty = match param.ty with None -> "" | Some ty -> ": " ^ string_of_type ty in
  let default =
    match param.default with
    | None -> ""
    | Some expr -> " = " ^ string_of_expr expr
  in
  pat ^ ty ^ default

and string_of_decl_kind = function
  | LetDecl (pat, ty, expr) ->
      let ty_str = match ty with None -> "" | Some ty -> ": " ^ string_of_type ty in
      Printf.sprintf "let %s%s = %s" (string_of_pattern pat) ty_str (string_of_expr expr)
  | VarDecl (pat, ty, expr) ->
      let ty_str = match ty with None -> "" | Some ty -> ": " ^ string_of_type ty in
      Printf.sprintf "var %s%s = %s" (string_of_pattern pat) ty_str (string_of_expr expr)
  | FnDecl fn ->
      let generics =
        match fn.fn_generic_params with
        | [] -> ""
        | params ->
            params |> List.map string_of_ident |> String.concat ", " |> Printf.sprintf "<%s>"
      in
      let params = fn.fn_params |> List.map string_of_param |> String.concat ", " in
      let ret = match fn.fn_ret_type with None -> "" | Some ty -> " -> " ^ string_of_type ty in
      let where_clause =
        match fn.fn_where_clause with
        | [] -> ""
        | clauses ->
            let clause_str =
              clauses
              |> List.map (fun c ->
                     let args = c.constraint_types |> List.map string_of_type |> String.concat ", " in
                     Printf.sprintf "%s(%s)" (string_of_ident c.constraint_trait) args)
              |> String.concat ", "
            in
            " where " ^ clause_str
      in
      let effects =
        match fn.fn_effect_annot with
        | None -> ""
        | Some tags -> tags |> List.map string_of_ident |> String.concat ", " |> Printf.sprintf " !{%s}"
      in
      let body =
        match fn.fn_body with
        | FnExpr expr -> " = " ^ string_of_expr expr
        | FnBlock stmts ->
            let stmt_str = stmts |> List.length |> string_of_int in
            Printf.sprintf " {block %s stmts}" stmt_str
      in
      Printf.sprintf "fn %s%s(%s)%s%s%s%s"
        (string_of_ident fn.fn_name) generics params ret where_clause effects body
  | TypeDecl decl ->
      begin match decl with
      | AliasDecl (name, params, ty) ->
          let params_str =
            match params with
            | [] -> ""
            | ps -> ps |> List.map string_of_ident |> String.concat ", " |> Printf.sprintf "<%s>"
          in
          Printf.sprintf "type alias %s%s = %s" (string_of_ident name) params_str (string_of_type ty)
      | NewtypeDecl (name, params, ty) ->
          let params_str =
            match params with
            | [] -> ""
            | ps -> ps |> List.map string_of_ident |> String.concat ", " |> Printf.sprintf "<%s>"
          in
          Printf.sprintf "type %s%s = new %s" (string_of_ident name) params_str (string_of_type ty)
      | SumDecl (name, params, variants) ->
          let params_str =
            match params with
            | [] -> ""
            | ps -> ps |> List.map string_of_ident |> String.concat ", " |> Printf.sprintf "<%s>"
          in
          let variant_str =
            variants
            |> List.map (fun v ->
                   if v.variant_types = [] then string_of_ident v.variant_name
                   else
                     v.variant_types
                     |> List.map string_of_type
                     |> String.concat ", "
                     |> Printf.sprintf "%s(%s)" (string_of_ident v.variant_name))
            |> String.concat " | "
          in
          Printf.sprintf "type %s%s = %s" (string_of_ident name) params_str variant_str
      end
  | TraitDecl trait ->
      let params =
        match trait.trait_params with
        | [] -> ""
        | ps -> ps |> List.map string_of_ident |> String.concat ", " |> Printf.sprintf "<%s>"
      in
      let where_clause =
        match trait.trait_where with
        | [] -> ""
        | clauses ->
            clauses
            |> List.map (fun c ->
                   let args = c.constraint_types |> List.map string_of_type |> String.concat ", " in
                   Printf.sprintf "%s(%s)" (string_of_ident c.constraint_trait) args)
            |> String.concat ", "
            |> Printf.sprintf " where %s"
      in
      let items = List.length trait.trait_items in
      Printf.sprintf "trait %s%s%s {items=%d}" (string_of_ident trait.trait_name) params where_clause items
  | ImplDecl impl ->
      let params =
        match impl.impl_params with
        | [] -> ""
        | ps -> ps |> List.map string_of_ident |> String.concat ", " |> Printf.sprintf "<%s>"
      in
      let trait_part =
        match impl.impl_trait with
        | None -> ""
        | Some (trait_id, args) ->
            let args_str = args |> List.map string_of_type |> String.concat ", " in
            Printf.sprintf "%s<%s> for " (string_of_ident trait_id) args_str
      in
      let where_clause =
        match impl.impl_where with
        | [] -> ""
        | clauses ->
            clauses
            |> List.map (fun c ->
                   let args = c.constraint_types |> List.map string_of_type |> String.concat ", " in
                   Printf.sprintf "%s(%s)" (string_of_ident c.constraint_trait) args)
            |> String.concat ", "
            |> Printf.sprintf " where %s"
      in
      let items = List.length impl.impl_items in
      Printf.sprintf "impl %s%s%s%s {items=%d}"
        trait_part params (string_of_type impl.impl_type) where_clause items
  | ExternDecl ext ->
      let items = List.length ext.extern_items in
      Printf.sprintf "extern \"%s\" {items=%d}" ext.extern_abi items
  | EffectDecl eff ->
      let ops =
        eff.operations
        |> List.map (fun op -> Printf.sprintf "%s: %s" (string_of_ident op.op_name) (string_of_type op.op_type))
        |> String.concat ", "
      in
      Printf.sprintf "effect %s : %s { %s }" (string_of_ident eff.effect_name) (string_of_ident eff.effect_tag) ops
  | HandlerDecl handler ->
      Printf.sprintf "handler %s = %s" (string_of_ident handler.handler_name) (string_of_expr handler.handler_body)
  | ConductorDecl conductor ->
      let section_count = List.length conductor.conductor_body in
      Printf.sprintf "conductor %s {sections=%d}" (string_of_ident conductor.conductor_name) section_count

let string_of_attribute attr =
  let args =
    match attr.attr_args with
    | [] -> ""
    | args ->
        args
        |> List.map string_of_expr
        |> String.concat ", "
        |> Printf.sprintf "(%s)"
  in
  Printf.sprintf "@%s%s" (string_of_ident attr.attr_name) args

let string_of_visibility = function
  | Public -> "pub "
  | Private -> ""

let rec string_of_use_item item =
  let base =
    match item.item_alias with
    | None -> string_of_ident item.item_name
    | Some alias -> Printf.sprintf "%s as %s" (string_of_ident item.item_name) (string_of_ident alias)
  in
  match item.item_nested with
  | None -> base
  | Some nested ->
      let nested_str = nested |> List.map string_of_use_item |> String.concat ", " in
      Printf.sprintf "%s.{%s}" base nested_str

let string_of_use_tree = function
  | UsePath (path, alias) ->
      let base = string_of_module_path path in
      begin match alias with
      | None -> base
      | Some a -> Printf.sprintf "%s as %s" base (string_of_ident a)
      end
  | UseBrace (path, items) ->
      let items_str = items |> List.map string_of_use_item |> String.concat ", " in
      Printf.sprintf "%s.{%s}" (string_of_module_path path) items_str

let string_of_decl decl =
  let attrs =
    match decl.decl_attrs with
    | [] -> ""
    | attrs -> attrs |> List.map string_of_attribute |> String.concat " " |> (fun s -> s ^ " ")
  in
  attrs ^ string_of_visibility decl.decl_vis ^ string_of_decl_kind decl.decl_kind

let string_of_use_decl use_decl =
  (if use_decl.use_pub then "pub " else "") ^ "use " ^ string_of_use_tree use_decl.use_tree

let string_of_module_header header =
  "module " ^ string_of_module_path header.module_path

let string_of_compilation_unit cu =
  let parts =
    []
    |> (fun acc -> match cu.header with None -> acc | Some h -> string_of_module_header h :: acc)
    |> List.rev
  in
  let parts = parts @ (List.map string_of_use_decl cu.uses) in
  let parts = parts @ (List.map string_of_decl cu.decls) in
  String.concat "\n" parts
