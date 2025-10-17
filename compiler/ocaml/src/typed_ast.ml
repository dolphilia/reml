(* Typed_ast — Typed Abstract Syntax Tree for Reml (Phase 2)
 *
 * このファイルは型推論結果を保持するTyped ASTの定義を提供する。
 * 仕様書 1-2 §C（型推論）に従い、型情報を付与したASTノードを構成する。
 *
 * 設計原則:
 * - 元のASTノードへの参照を保持
 * - 推論された型情報を付与
 * - Span情報を保持してエラー報告に活用
 *)

open Types
open Ast
open Constraint_solver

(* ========== 型付き式 ========== *)

type typed_expr = {
  texpr_kind : typed_expr_kind;
  texpr_ty : ty;  (** 推論された型 *)
  texpr_span : span;  (** 位置情報 *)
}
(** 型付き式ノード
 *
 * 元の式 (Ast.expr) + 推論された型
 *)

and typed_expr_kind =
  | TLiteral of literal
  | TVar of ident * constrained_scheme
      (** 変数参照 + インスタンス化された型スキーム *)
  | TModulePath of module_path * ident
  | TCall of typed_expr * typed_arg list
  | TLambda of typed_param list * ty option * typed_expr
  | TPipe of typed_expr * typed_expr
  | TBinary of binary_op * typed_expr * typed_expr
  | TUnary of unary_op * typed_expr
  | TFieldAccess of typed_expr * ident
  | TTupleAccess of typed_expr * int
  | TIndex of typed_expr * typed_expr
  | TPropagate of typed_expr
  | TIf of typed_expr * typed_expr * typed_expr option
  | TMatch of typed_expr * typed_match_arm list
  | TWhile of typed_expr * typed_expr
  | TFor of
      typed_pattern
      * typed_expr
      * typed_expr
      * dict_ref
      * iterator_dict_info option
  | TLoop of typed_expr
  | TContinue
  | TBlock of typed_stmt list
  | TUnsafe of typed_expr
  | TReturn of typed_expr option
  | TDefer of typed_expr
  | TAssign of typed_expr * typed_expr

(** 型付き関数引数 *)
and typed_arg = TPosArg of typed_expr | TNamedArg of ident * typed_expr

and typed_match_arm = {
  tarm_pattern : typed_pattern;
  tarm_guard : typed_expr option;
  tarm_body : typed_expr;
  tarm_span : span;
}
(** 型付き match アーム *)

and typed_param = {
  tpat : typed_pattern;
  tty : ty;  (** 推論された型 *)
  tdefault : typed_expr option;
  tparam_span : span;
}
(** 型付きパラメータ *)
(* ========== 型付きパターン ========== *)

and typed_pattern = {
  tpat_kind : typed_pattern_kind;
  tpat_ty : ty;  (** パターン全体の型 *)
  tpat_bindings : (string * ty) list;  (** 束縛変数名 → 型のマッピング *)
  tpat_span : span;
}
(** 型付きパターンノード
 *
 * パターンマッチで束縛される変数の型情報を保持
 *)

and typed_pattern_kind =
  | TPatLiteral of literal
  | TPatVar of ident
  | TPatWildcard
  | TPatTuple of typed_pattern list
  | TPatRecord of (ident * typed_pattern option) list * bool
  | TPatConstructor of ident * typed_pattern list
  | TPatGuard of typed_pattern * typed_expr

(* ========== 型付き文 ========== *)
and typed_stmt =
  | TDeclStmt of typed_decl
  | TExprStmt of typed_expr
  | TAssignStmt of typed_expr * typed_expr
  | TDeferStmt of typed_expr
(* ========== 型付き宣言 ========== *)

and typed_decl = {
  tdecl_attrs : attribute list;
  tdecl_vis : visibility;
  tdecl_kind : typed_decl_kind;
  tdecl_scheme : constrained_scheme;  (** 宣言の型スキーム *)
  tdecl_span : span;
}
(** 型付き宣言ノード *)

and typed_decl_kind =
  | TLetDecl of typed_pattern * typed_expr
  | TVarDecl of typed_pattern * typed_expr
  | TFnDecl of typed_fn_decl
  | TTypeDecl of type_decl  (** 型宣言は型推論不要 *)
  | TTraitDecl of trait_decl  (** Phase 2 後半で実装 *)
  | TImplDecl of impl_decl  (** Phase 2 後半で実装 *)
  | TExternDecl of extern_decl  (** 外部宣言 *)
  | TEffectDecl of effect_decl  (** Phase 2 後半で実装 *)
  | THandlerDecl of handler_decl  (** Phase 2 後半で実装 *)
  | TConductorDecl of conductor_decl  (** Phase 2 後半で実装 *)

and typed_fn_decl = {
  tfn_name : ident;
  tfn_generic_params : (ident * type_var) list;  (** ジェネリック型パラメータ → 型変数 *)
  tfn_params : typed_param list;
  tfn_ret_type : ty;  (** 推論された返り値型 *)
  tfn_where_clause : constraint_ list;  (** Phase 2 後半で実装 *)
  tfn_effect_profile : effect_profile_node option;  (** Phase 2 効果統合で利用予定 *)
  tfn_body : typed_fn_body;
}
(** 型付き関数宣言 *)

and typed_fn_body = TFnExpr of typed_expr | TFnBlock of typed_stmt list

(* ========== 型付きコンパイル単位 ========== *)

type typed_compilation_unit = {
  tcu_module_header : module_header option;
  tcu_use_decls : use_decl list;
  tcu_items : typed_decl list;
}
(** 型付きコンパイル単位 *)

(* ========== ユーティリティ関数 ========== *)

(** ダミーのSpan *)
let dummy_span = Ast.dummy_span

(** 型付き式の構築 *)
let make_typed_expr kind ty span =
  { texpr_kind = kind; texpr_ty = ty; texpr_span = span }

(** 型付きパターンの構築 *)
let make_typed_pattern kind ty bindings span =
  { tpat_kind = kind; tpat_ty = ty; tpat_bindings = bindings; tpat_span = span }

(** 型付き宣言の構築 *)
let make_typed_decl attrs vis kind scheme span =
  {
    tdecl_attrs = attrs;
    tdecl_vis = vis;
    tdecl_kind = kind;
    tdecl_scheme = scheme;
    tdecl_span = span;
  }

(* ========== デバッグ用: Typed ASTの文字列表現 ========== *)

(** 型付き式の文字列表現（簡易版） *)
let rec string_of_typed_expr texpr =
  let ty_str = string_of_ty texpr.texpr_ty in
  match texpr.texpr_kind with
  | TLiteral _ -> Printf.sprintf "(Literal : %s)" ty_str
  | TVar (id, _) -> Printf.sprintf "(%s : %s)" id.name ty_str
  | TModulePath (_, id) -> Printf.sprintf "(Path::%s : %s)" id.name ty_str
  | TCall (fn, args) ->
      Printf.sprintf "(Call %s [%d args] : %s)" (string_of_typed_expr fn)
        (List.length args) ty_str
  | TLambda (params, _, body) ->
      Printf.sprintf "(Lambda [%d params] %s : %s)" (List.length params)
        (string_of_typed_expr body)
        ty_str
  | TPipe (e1, e2) ->
      Printf.sprintf "(%s |> %s : %s)" (string_of_typed_expr e1)
        (string_of_typed_expr e2) ty_str
  | TBinary (_, e1, e2) ->
      Printf.sprintf "(Binary %s %s : %s)" (string_of_typed_expr e1)
        (string_of_typed_expr e2) ty_str
  | TUnary (_, e) ->
      Printf.sprintf "(Unary %s : %s)" (string_of_typed_expr e) ty_str
  | TFieldAccess (e, field) ->
      Printf.sprintf "(%s.%s : %s)" (string_of_typed_expr e) field.name ty_str
  | TTupleAccess (e, idx) ->
      Printf.sprintf "(%s.%d : %s)" (string_of_typed_expr e) idx ty_str
  | TIndex (e1, e2) ->
      Printf.sprintf "(%s[%s] : %s)" (string_of_typed_expr e1)
        (string_of_typed_expr e2) ty_str
  | TPropagate e -> Printf.sprintf "(%s? : %s)" (string_of_typed_expr e) ty_str
  | TIf (cond, then_e, else_e) ->
      let else_str =
        match else_e with
        | Some e -> " else " ^ string_of_typed_expr e
        | None -> ""
      in
      Printf.sprintf "(if %s then %s%s : %s)"
        (string_of_typed_expr cond)
        (string_of_typed_expr then_e)
        else_str ty_str
  | TMatch (e, arms) ->
      Printf.sprintf "(match %s [%d arms] : %s)" (string_of_typed_expr e)
        (List.length arms) ty_str
  | TWhile (cond, body) ->
      Printf.sprintf "(while %s %s : %s)"
        (string_of_typed_expr cond)
        (string_of_typed_expr body)
        ty_str
  | TFor (pat, _, body, _, _) ->
      Printf.sprintf "(for %s in ... %s : %s)"
        (string_of_typed_pattern pat)
        (string_of_typed_expr body)
        ty_str
  | TLoop body ->
      Printf.sprintf "(loop %s : %s)" (string_of_typed_expr body) ty_str
  | TContinue -> Printf.sprintf "(continue : %s)" ty_str
  | TBlock stmts ->
      Printf.sprintf "(Block [%d stmts] : %s)" (List.length stmts) ty_str
  | TUnsafe e ->
      Printf.sprintf "(unsafe %s : %s)" (string_of_typed_expr e) ty_str
  | TReturn None -> Printf.sprintf "(return : %s)" ty_str
  | TReturn (Some e) ->
      Printf.sprintf "(return %s : %s)" (string_of_typed_expr e) ty_str
  | TDefer e -> Printf.sprintf "(defer %s : %s)" (string_of_typed_expr e) ty_str
  | TAssign (lhs, rhs) ->
      Printf.sprintf "(%s := %s : %s)" (string_of_typed_expr lhs)
        (string_of_typed_expr rhs) ty_str

(** 型付きパターンの文字列表現（簡易版） *)
and string_of_typed_pattern tpat =
  let ty_str = string_of_ty tpat.tpat_ty in
  let bindings_str =
    String.concat ", "
      (List.map
         (fun (name, ty) -> Printf.sprintf "%s: %s" name (string_of_ty ty))
         tpat.tpat_bindings)
  in
  match tpat.tpat_kind with
  | TPatLiteral _ -> Printf.sprintf "(PatLit : %s)" ty_str
  | TPatVar id -> Printf.sprintf "(%s : %s [%s])" id.name ty_str bindings_str
  | TPatWildcard -> Printf.sprintf "(_ : %s)" ty_str
  | TPatTuple pats ->
      Printf.sprintf "(PatTuple [%d] : %s)" (List.length pats) ty_str
  | TPatRecord (fields, has_rest) ->
      let rest_str = if has_rest then " .." else "" in
      Printf.sprintf "(PatRecord [%d fields%s] : %s)" (List.length fields)
        rest_str ty_str
  | TPatConstructor (id, pats) ->
      Printf.sprintf "(%s [%d] : %s)" id.name (List.length pats) ty_str
  | TPatGuard (pat, _) ->
      Printf.sprintf "(%s if ... : %s)" (string_of_typed_pattern pat) ty_str

(** 型付き宣言の文字列表現（簡易版） *)
let string_of_typed_decl tdecl =
  let scheme_str = string_of_constrained_scheme tdecl.tdecl_scheme in
  match tdecl.tdecl_kind with
  | TLetDecl (pat, _) ->
      Printf.sprintf "let %s = ... : %s"
        (string_of_typed_pattern pat)
        scheme_str
  | TVarDecl (pat, _) ->
      Printf.sprintf "var %s = ... : %s"
        (string_of_typed_pattern pat)
        scheme_str
  | TFnDecl fn ->
      let params_str =
        match fn.tfn_params with
        | [] -> "()"
        | params ->
            params
            |> List.map (fun param ->
                   match param.tpat.tpat_kind with
                   | TPatVar id ->
                       Printf.sprintf "%s: %s" id.name (string_of_ty param.tty)
                   | _ -> string_of_typed_pattern param.tpat)
            |> String.concat ", "
      in
      Printf.sprintf "fn %s(%s) : %s" fn.tfn_name.name params_str scheme_str
  | TTypeDecl _ -> Printf.sprintf "type ... : %s" scheme_str
  | TTraitDecl _ -> Printf.sprintf "trait ... : %s" scheme_str
  | TImplDecl _ -> Printf.sprintf "impl ... : %s" scheme_str
  | TExternDecl _ -> Printf.sprintf "extern ... : %s" scheme_str
  | TEffectDecl _ -> Printf.sprintf "effect ... : %s" scheme_str
  | THandlerDecl _ -> Printf.sprintf "handler ... : %s" scheme_str
  | TConductorDecl _ -> Printf.sprintf "conductor ... : %s" scheme_str

(** 型付きコンパイル単位の文字列表現 *)
let string_of_typed_compilation_unit tcu =
  let items_str =
    String.concat "\n\n" (List.map string_of_typed_decl tcu.tcu_items)
  in
  Printf.sprintf "=== Typed AST ===\n\n%s" items_str
