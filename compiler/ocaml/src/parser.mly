%{
(* Parser — Reml 構文解析器
 *
 * docs/spec/1-1-syntax.md に基づく構文解析を実装する。
 * Menhir で LR(1) パーサを生成。
 *)

open Ast

(* ヘルパー関数 *)

let make_span start_pos end_pos = {
  start = start_pos.Lexing.pos_cnum;
  end_ = end_pos.Lexing.pos_cnum;
}
let tuple_index_from_literal (value, base) =
  match base with
  | Base10 -> (
      try int_of_string value with
      | Failure _ -> failwith "tuple index must be decimal"
    )
  | _ -> failwith "tuple index must be decimal"

let make_qualified_ident parts span =
  make_ident (String.concat "." parts) span

%}

(* トークン定義 *)

(* キーワード *)
%token MODULE USE AS PUB SELF SUPER
%token LET VAR FN TYPE ALIAS NEW TRAIT IMPL EXTERN
%token EFFECT OPERATION HANDLER CONDUCTOR CHANNELS EXECUTION MONITORING
%token IF THEN ELSE MATCH WITH FOR IN WHILE LOOP RETURN DEFER UNSAFE
%token PERFORM DO HANDLE
%token WHERE
%token TRUE FALSE
%token BREAK CONTINUE

(* 演算子・区切り *)
%token PIPE CHANNEL_PIPE
%token DOT COMMA SEMICOLON COLON AT BAR EQ COLONEQ ARROW DARROW
%token LPAREN RPAREN LBRACKET RBRACKET LBRACE RBRACE
%token PLUS MINUS STAR SLASH PERCENT POW
%token EQEQ NE LT LE GT GE
%token AND OR NOT
%token QUESTION DOTDOT UNDERSCORE

(* リテラル *)
%token <string * Ast.int_base> INT
%token <string> FLOAT
%token <string> CHAR
%token <string * Ast.string_kind> STRING
%token <string> IDENT
%token <string> UPPER_IDENT

%token EOF

(* 優先順位と結合性 (仕様 §D.1 に準拠) *)
%left PIPE
%left OR
%left AND
%nonassoc EQEQ NE
%nonassoc LT LE GT GE
%left PLUS MINUS
%left STAR SLASH PERCENT
%right POW
%right UMINUS UNOT  (* 単項演算子 *)
%left DOT LPAREN LBRACKET QUESTION

(* match アーム境界のための特別な優先順位レベル *)
%nonassoc MATCH_ARM

(* 開始シンボル *)
%start <Ast.compilation_unit> compilation_unit

%%

(* ========== コンパイル単位 ========== *)

compilation_unit:
  | header = module_header_opt;
    uses = use_decl_list;
    decls = decl_list;
    EOF
    { { header; uses; decls } }

module_header_opt:
  | (* empty *) { None }
  | MODULE; path = module_path
    {
      let span = make_span $startpos $endpos in
      Some { module_path = path; header_span = span }
    }

use_decl_list:
  | (* empty *) { [] }
  | uses = use_decl_list; u = use_decl { uses @ [u] }

decl_list:
  | (* empty *) { [] }
  | decls = decl_list; d = decl { decls @ [d] }

(* ========== use 宣言 ========== *)

use_decl:
  | pub = pub_opt; USE; tree = use_tree
    {
      let span = make_span $startpos $endpos in
      { use_pub = pub; use_tree = tree; use_span = span }
    }

pub_opt:
  | (* empty *) { false }
  | PUB { true }

use_tree:
  | path = module_path; alias = use_alias_opt
    { UsePath (path, alias) }
  | prefix = use_brace_prefix; DOT; LBRACE; items = use_item_list; RBRACE
    { UseBrace (prefix, items) }

use_alias_opt:
  | (* empty *) { None }
  | AS; id = ident { Some id }

use_brace_prefix:
  | base = use_brace_base { base }
  | prefix = use_brace_prefix; DOT; id = ident
    {
      match prefix with
      | Root ids -> Root (ids @ [id])
      | Relative (head, tail) -> Relative (head, tail @ [id])
    }

use_brace_base:
  | COLON; COLON; ids = ident_list { Root ids }
  | SELF { Relative (Self, []) }
  | count = super_list { Relative (Super count, []) }
  | id = ident { Relative (PlainIdent id, []) }

use_item_list:
  | item = use_item { [item] }
  | items = use_item_list; COMMA; item = use_item { items @ [item] }

use_item:
  | name = ident; alias = use_alias_opt
    {
      { item_name = name; item_alias = alias; item_nested = None }
    }

(* ========== モジュールパス ========== *)

module_path:
  | COLON; COLON; ids = ident_list { Root ids }
  | head = relative_head; tail = relative_tail { Relative (head, tail) }

relative_head:
  | SELF { Self }
  | count = super_list { Super count }
  | id = ident { PlainIdent id }

super_list:
  | SUPER { 1 }
  | count = super_list; DOT; SUPER { count + 1 }

relative_tail:
  | (* empty *) { [] }
  | DOT; ids = ident_list { ids }

(* ========== 宣言 ========== *)

decl:
  | attrs = attribute_list; vis = visibility; kind = decl_kind
    {
      let span = make_span $startpos $endpos in
      { decl_attrs = attrs; decl_vis = vis; decl_kind = kind; decl_span = span }
    }

attribute_list:
  | (* empty *) { [] }
  | attrs = attribute_list; attr = attribute { attrs @ [attr] }

attribute:
  | AT; name = ident; args = attribute_args_opt
    {
      let span = make_span $startpos $endpos in
      { attr_name = name; attr_args = args; attr_span = span }
    }

attribute_args_opt:
  | (* empty *) { [] }
  | LPAREN; args = expr_list; RPAREN { args }

visibility:
  | (* empty *) { Private }
  | PUB { Public }

decl_kind:
  | LET; pat = pattern; ty = type_annot_opt; EQ; e = expr
    { LetDecl (pat, ty, e) }
  | VAR; pat = pattern; ty = type_annot_opt; EQ; e = expr
    { VarDecl (pat, ty, e) }
  | fn = fn_decl { FnDecl fn }
  | TYPE; decl = type_decl { TypeDecl decl }
  | TRAIT; decl = trait_decl { TraitDecl decl }
  | IMPL; decl = impl_decl { ImplDecl decl }
  | EXTERN; decl = extern_decl { ExternDecl decl }
  | EFFECT; decl = effect_decl { EffectDecl decl }
  | HANDLER; decl = handler_decl { HandlerDecl decl }

fn_decl:
  | FN; name = ident; generics = generic_params_opt; params = fn_params;
    ret = return_type_opt; where_clause = where_clause_opt;
    effects = effect_annot_opt; body = fn_body
    {
      {
        fn_name = name;
        fn_generic_params = generics;
        fn_params = params;
        fn_ret_type = ret;
        fn_where_clause = where_clause;
        fn_effect_annot = effects;
        fn_body = body;
      }
    }

fn_params:
  | LPAREN; RPAREN { [] }
  | LPAREN; ps = param_list; RPAREN { ps }

generic_params_opt:
  | (* empty *) { [] }
  | LT; params = generic_param_list; GT { params }

generic_param_list:
  | id = ident { [id] }
  | params = generic_param_list; COMMA; id = ident { params @ [id] }

where_clause_opt:
  | (* empty *) { [] }
  | WHERE; clauses = constraint_list { clauses }

constraint_list:
  | c = constraint_spec { [c] }
  | cs = constraint_list; COMMA; c = constraint_spec { cs @ [c] }

constraint_spec:
  | trait_id = ident; LT; args = type_arg_list; GT
    {
      let span = make_span $startpos $endpos in
      { constraint_trait = trait_id; constraint_types = args; constraint_span = span }
    }

type_arg_list:
  | ty = type_annot { [ty] }
  | tys = type_arg_list; COMMA; ty = type_annot { tys @ [ty] }

effect_annot_opt:
  | (* empty *) { None }
  | NOT; LBRACE; tags = effect_tag_list; RBRACE { Some tags }

effect_tag_list:
  | id = ident { [id] }
  | tags = effect_tag_list; COMMA; id = ident { tags @ [id] }

param_list:
  | p = param { [p] }
  | ps = param_list; COMMA; p = param { ps @ [p] }

param:
  | pat = pattern; ty = type_annot_opt; default = default_expr_opt
    {
      let span = make_span $startpos $endpos in
      { pat; ty; default; param_span = span }
    }

default_expr_opt:
  | (* empty *) { None }
  | EQ; e = expr { Some e }

return_type_opt:
  | (* empty *) { None }
  | ARROW; ty = type_annot { Some ty }

fn_body:
  | EQ; e = expr { FnExpr e }
  | block = block_stmt { FnBlock block }

(* ========== 型宣言 ========== *)

type_decl:
  | ALIAS; name = ident; generics = generic_params_opt; EQ; ty = type_annot
    { AliasDecl (name, generics, ty) }
  | name = ident; generics = generic_params_opt; EQ; NEW; ty = type_annot
    { NewtypeDecl (name, generics, ty) }
  | name = ident; generics = generic_params_opt; EQ; variants = sum_variant_list
    { SumDecl (name, generics, variants) }

sum_variant_list:
  | first = sum_variant { [first] }
  | BAR; first = sum_variant { [first] }
  | variants = sum_variant_list; BAR; v = sum_variant { variants @ [v] }

sum_variant:
  | name = ident; payload = variant_payload_opt
    {
      let span = make_span $startpos $endpos in
      { variant_name = name; variant_types = payload; variant_span = span }
    }

variant_payload_opt:
  | (* empty *) { [] }
  | LPAREN; args = type_arg_list_opt; RPAREN { args }

type_arg_list_opt:
  | (* empty *) { [] }
  | args = type_arg_list { args }

(* ========== トレイト宣言 ========== *)

trait_decl:
  | name = ident; generics = generic_params_opt; where_clause = where_clause_opt; body = trait_body
    { { trait_name = name; trait_params = generics; trait_where = where_clause; trait_items = body } }

trait_body:
  | LBRACE; items = trait_item_list; RBRACE { items }

trait_item_list:
  | (* empty *) { [] }
  | items = trait_item_list; item = trait_item { items @ [item] }

trait_item:
  | attrs = attribute_list; sig_ = fn_signature_only; default = trait_default_opt
    { { item_attrs = attrs; item_sig = sig_; item_default = default } }

fn_signature_only:
  | FN; name = ident; generics = generic_params_opt; params = fn_params;
    ret = return_type_opt; where_clause = where_clause_opt; effects = effect_annot_opt
    {
      {
        sig_name = name;
        sig_params = generics;
        sig_args = params;
        sig_ret = ret;
        sig_where = where_clause;
        sig_effects = effects;
      }
    }

trait_default_opt:
  | (* empty *) { None }
  | EQ; e = expr { Some (FnExpr e) }
  | block = block_stmt { Some (FnBlock block) }

(* ========== impl 宣言 ========== *)

impl_decl:
  | generics = generic_params_opt; target = impl_target; where_clause = where_clause_opt; body = impl_body
    {
      let trait_ref, ty = target in
      { impl_params = generics; impl_trait = trait_ref; impl_type = ty; impl_where = where_clause; impl_items = body }
    }

impl_target:
  | trait_ref = trait_reference; FOR; ty = type_annot { (Some trait_ref, ty) }
  | ty = type_annot { (None, ty) }

trait_reference:
  | name = ident; args = generic_args_opt { (name, args) }

generic_args_opt:
  | (* empty *) { [] }
  | LT; args = type_arg_list; GT { args }

impl_body:
  | LBRACE; items = impl_item_list; RBRACE { items }

impl_item_list:
  | (* empty *) { [] }
  | items = impl_item_list; item = impl_item { items @ [item] }

impl_item:
  | attrs = attribute_list; fn = fn_decl
    { ignore attrs; ImplFn fn }
  | LET; pat = pattern; ty = type_annot_opt; EQ; e = expr
    { ImplLet (pat, ty, e) }
  | VAR; pat = pattern; ty = type_annot_opt; EQ; e = expr
    { ImplLet (pat, ty, e) }

(* ========== extern 宣言 ========== *)

extern_decl:
  | abi = extern_abi; body = extern_body
    { { extern_abi = abi; extern_items = body } }

extern_abi:
  | s = STRING { fst s }

extern_body:
  | sig_ = fn_signature_only; SEMICOLON
    { [ { extern_attrs = []; extern_sig = sig_ } ] }
  | LBRACE; items = extern_item_list; RBRACE { items }

extern_item_list:
  | (* empty *) { [] }
  | items = extern_item_list; item = extern_item { items @ [item] }

extern_item:
  | attrs = attribute_list; sig_ = fn_signature_only; SEMICOLON
    { { extern_attrs = attrs; extern_sig = sig_ } }

(* ========== effect / handler 宣言 ========== *)

effect_decl:
  | name = ident; COLON; tag = ident; body = effect_body
    { { effect_name = name; effect_tag = tag; operations = body } }

effect_body:
  | LBRACE; ops = operation_list; RBRACE { ops }

operation_list:
  | (* empty *) { [] }
  | ops = operation_list; op = operation_decl { ops @ [op] }

operation_decl:
  | attrs = attribute_list; OPERATION; name = ident; COLON; ty = type_annot
    {
      let span = make_span $startpos $endpos in
      ignore attrs;
      { op_name = name; op_type = ty; op_span = span }
    }

handler_decl:
  | name = ident; body = handler_body
    { { handler_name = name; handler_entries = body } }

handler_body:
  | LBRACE; entries = handler_entry_list; RBRACE { entries }

handler_entry_list:
  | entry = handler_entry { [entry] }
  | entries = handler_entry_list; entry = handler_entry { entries @ [entry] }

handler_entry:
  | attrs = attribute_list; OPERATION; name = ident; LPAREN; params = handler_param_list_opt; RPAREN; block = handler_block
    {
      let stmts, _ = block in
      ignore attrs;
      let span = make_span $startpos $endpos in
      HandlerOperation {
        handler_op_name = name;
        handler_op_params = params;
        handler_op_body = stmts;
        handler_op_span = span;
      }
    }
  | attrs = attribute_list; RETURN; value = ident; block = handler_block
    {
      let stmts, _ = block in
      ignore attrs;
      let span = make_span $startpos $endpos in
      HandlerReturn {
        handler_return_name = value;
        handler_return_body = stmts;
        handler_return_span = span;
      }
    }

handler_param_list_opt:
  | (* empty *) { [] }
  | params = param_list { params }

handler_block:
  | LBRACE; stmts = stmt_list; RBRACE
    {
      let span = make_span $startpos $endpos in
      (stmts, span)
    }

(* ========== 式 ========== *)

expr:
  | e = expr_base { e }
  | e = pipe_expr { e }

expr_base:
  | e = postfix_expr { e }
  | e = binary_expr { e }
  | e = unary_expr { e }
  | e = if_expr { e }
  | e = lambda_expr { e }
  | e = match_expr { e }
  | e = while_expr { e }
  | e = for_expr { e }
  | e = loop_expr { e }
  | e = continue_expr { e }
  | e = return_expr { e }
  | e = defer_expr { e }
  | e = unsafe_expr { e }

primary_expr:
  | lit = literal
    {
      let span = make_span $startpos $endpos in
      make_expr (Literal lit) span
    }
  | id = ident
    {
      make_expr (Var id) id.span
    }
  | e = block_expr { e }
  | LPAREN; first = expr; COMMA; rest = tuple_expr_rest; RPAREN
    {
      let elements = first :: rest in
      let span = make_span $startpos $endpos in
      make_expr (Literal (Tuple elements)) span
    }
  | LPAREN; e = expr; RPAREN { e }

literal:
  | i = INT { Int (fst i, snd i) }
  | f = FLOAT { Float f }
  | c = CHAR { Char c }
  | s = STRING { String (fst s, snd s) }
  | TRUE { Bool true }
  | FALSE { Bool false }
  | LPAREN; RPAREN { Unit }
  | LBRACKET; elements = expr_list_opt; RBRACKET { Array elements }
  | LBRACE; fields = record_field_list_opt; RBRACE { Record fields }

(* 後置演算子（関数呼び出し、フィールドアクセス、インデックスなど）
 * Menhir は左再帰を処理できるので、postfix_expr を左再帰で構築 *)
postfix_expr:
  | e = primary_expr { e }
  | func = postfix_expr; LPAREN; args = arg_list_opt; RPAREN
    {
      let span = merge_span func.expr_span (make_span $endpos $endpos) in
      make_expr (Call (func, args)) span
    }
  | target = postfix_expr; DOT; field = ident
    {
      let span = make_span $startpos $endpos in
      make_expr (FieldAccess (target, field)) span
    }
  | target = postfix_expr; DOT; index_lit = INT
    {
      let index = tuple_index_from_literal index_lit in
      let span = make_span $startpos $endpos in
      make_expr (TupleAccess (target, index)) span
    }
  | target = postfix_expr; LBRACKET; idx = expr; RBRACKET
    {
      let span = make_span $startpos $endpos in
      make_expr (Index (target, idx)) span
    }
  | target = postfix_expr; QUESTION
    {
      let span = make_span $startpos $endpos in
      make_expr (Propagate target) span
    }

arg_list_opt:
  | (* empty *) { [] }
  | args = arg_list { args }

arg_list:
  | arg = arg { [arg] }
  | args = arg_list; COMMA; arg = arg { args @ [arg] }

arg:
  | e = expr { PosArg e }
  | id = ident; EQ; e = expr { NamedArg (id, e) }

binary_expr:
  | lhs = expr; PLUS; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Add, lhs, rhs)) span
    }
  | lhs = expr; MINUS; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Sub, lhs, rhs)) span
    }
  | lhs = expr; STAR; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Mul, lhs, rhs)) span
    }
  | lhs = expr; SLASH; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Div, lhs, rhs)) span
    }
  | lhs = expr; PERCENT; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Mod, lhs, rhs)) span
    }
  | lhs = expr; POW; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Pow, lhs, rhs)) span
    }
  | lhs = expr; EQEQ; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Eq, lhs, rhs)) span
    }
  | lhs = expr; NE; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Ne, lhs, rhs)) span
    }
  | lhs = expr; LT; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Lt, lhs, rhs)) span
    }
  | lhs = expr; LE; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Le, lhs, rhs)) span
    }
  | lhs = expr; GT; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Gt, lhs, rhs)) span
    }
  | lhs = expr; GE; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Ge, lhs, rhs)) span
    }
  | lhs = expr; AND; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (And, lhs, rhs)) span
    }
  | lhs = expr; OR; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Or, lhs, rhs)) span
    }

unary_expr:
  | NOT; e = expr %prec UNOT
    {
      let span = make_span $startpos $endpos in
      make_expr (Unary (Not, e)) span
    }
  | MINUS; e = expr %prec UMINUS
    {
      let span = make_span $startpos $endpos in
      make_expr (Unary (Neg, e)) span
    }

pipe_expr:
  | lhs = expr; PIPE; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Pipe (lhs, rhs)) span
    }

if_expr:
  | IF; cond = expr; THEN; then_br = expr; else_br = else_branch_opt
    {
      let span = make_span $startpos $endpos in
      make_expr (If (cond, then_br, else_br)) span
    }

else_branch_opt:
  | (* empty *) { None }
  | ELSE; e = expr { Some e }

lambda_expr:
  | BAR; params = lambda_param_list_opt; BAR; ret = return_type_opt; body = lambda_body
    {
      let span = make_span $startpos $endpos in
      make_expr (Lambda (params, ret, body)) span
    }

lambda_param_list_opt:
  | (* empty *) { [] }
  | params = lambda_param_list { params }

lambda_param_list:
  | p = param { [p] }
  | ps = lambda_param_list; COMMA; p = param { ps @ [p] }

lambda_body:
  | block = block_expr { block }
  | e = expr { e }

match_expr:
  | MATCH; scrutinee = expr; WITH; arms = match_arm_list
    {
      let span = make_span $startpos $endpos in
      make_expr (Match (scrutinee, arms)) span
    }

match_arm_list:
  | arm = match_arm { [arm] }
  | arms = match_arm_list; arm = match_arm { arms @ [arm] }

match_arm:
  | BAR; pat = pattern; guard = match_guard_opt; ARROW; body = expr
    {
      let span = make_span $startpos $endpos in
      { arm_pattern = pat; arm_guard = guard; arm_body = body; arm_span = span }
    }

match_guard_opt:
  | (* empty *) { None }
  | IF; e = expr { Some e }

while_expr:
  | WHILE; cond = expr; body = block_expr
    {
      let span = make_span $startpos $endpos in
      make_expr (While (cond, body)) span
    }

for_expr:
  | FOR; pat = pattern; IN; source = expr; body = block_expr
    {
      let span = make_span $startpos $endpos in
      make_expr (For (pat, source, body)) span
    }

loop_expr:
  | LOOP; body = block_expr
    {
      let span = make_span $startpos $endpos in
      make_expr (Loop body) span
    }

continue_expr:
  | CONTINUE
    {
      let span = make_span $startpos $endpos in
      make_expr Continue span
    }

return_expr:
  | RETURN; value = expr
    {
      let span = make_span $startpos $endpos in
      make_expr (Return (Some value)) span
    }
  | RETURN
    {
      let span = make_span $startpos $endpos in
      make_expr (Return None) span
    }

defer_expr:
  | DEFER; e = expr
    {
      let span = make_span $startpos $endpos in
      make_expr (Defer e) span
    }

unsafe_expr:
  | UNSAFE; body = block_expr
    {
      let span = make_span $startpos $endpos in
      make_expr (Unsafe body) span
    }

block_expr:
  | LBRACE; stmts = stmt_list; RBRACE
    {
      let span = make_span $startpos $endpos in
      make_expr (Block stmts) span
    }

(* ========== 文 ========== *)

block_stmt:
  | LBRACE; stmts = stmt_list; RBRACE { stmts }

stmt_list:
  | (* empty *) { [] }
  | stmts = stmt_list; s = stmt { stmts @ [s] }

stmt:
  | d = decl; SEMICOLON { DeclStmt d }
  | d = decl { DeclStmt d }  (* 最後の宣言はセミコロン省略可 *)
  | lvalue = postfix_expr; COLONEQ; rvalue = expr; SEMICOLON { AssignStmt (lvalue, rvalue) }
  | lvalue = postfix_expr; COLONEQ; rvalue = expr { AssignStmt (lvalue, rvalue) }  (* セミコロン省略可 *)
  | DEFER; value = expr; SEMICOLON { DeferStmt value }
  | e = expr; SEMICOLON { ExprStmt e }
  | e = expr { ExprStmt e }  (* 最後の式はセミコロン省略可 *)

(* ========== パターン ========== *)

pattern:
  | lit = literal
    {
      let span = make_span $startpos $endpos in
      make_pattern (PatLiteral lit) span
    }
  | id = lower_ident
    {
      make_pattern (PatVar id) id.span
    }
  | ctor = upper_ident
    {
      make_pattern (PatConstructor (ctor, [])) ctor.span
    }
  | UNDERSCORE
    {
      let span = make_span $startpos $endpos in
      make_pattern PatWildcard span
    }
  | LPAREN; pat = pattern; RPAREN { pat }
  | LPAREN; first = pattern; COMMA; rest = pattern_list; RPAREN
    {
      let patterns = first :: rest in
      let span = make_span $startpos $endpos in
      make_pattern (PatTuple patterns) span
    }
  | name = upper_ident; LPAREN; args = pattern_arg_list_opt; RPAREN
    {
      let span = make_span $startpos $endpos in
      make_pattern (PatConstructor (name, args)) span
    }
  | name = lower_ident; LPAREN; args = pattern_arg_list_opt; RPAREN
    {
      let span = make_span $startpos $endpos in
      make_pattern (PatConstructor (name, args)) span
    }
  | head = ident; DOT; rest = separated_nonempty_list(DOT, ident)
    {
      let ids = head :: rest in
      let span = make_span $startpos $endpos in
      match List.rev ids with
      | ctor :: rev_prefix ->
          let ctor_ident =
            if rev_prefix = [] then ctor
            else
              let prefix = List.rev rev_prefix |> List.map (fun id -> id.name) in
              make_qualified_ident (prefix @ [ctor.name]) span
          in
          make_pattern (PatConstructor (ctor_ident, [])) span
      | [] -> assert false
    }
  | head = ident; DOT; rest = separated_nonempty_list(DOT, ident); LPAREN; args = pattern_arg_list_opt; RPAREN
    {
      let ids = head :: rest in
      let span = make_span $startpos $endpos in
      match List.rev ids with
      | ctor :: rev_prefix ->
          let ctor_ident =
            if rev_prefix = [] then ctor
            else
              let prefix = List.rev rev_prefix |> List.map (fun id -> id.name) in
              make_qualified_ident (prefix @ [ctor.name]) span
          in
          make_pattern (PatConstructor (ctor_ident, args)) span
      | [] -> assert false
    }
  | LBRACE; body = record_pattern_body; RBRACE
    {
      let fields, has_rest = body in
      let span = make_span $startpos $endpos in
      make_pattern (PatRecord (fields, has_rest)) span
    }

pattern_list:
  | p = pattern { [p] }
  | ps = pattern_list; COMMA; p = pattern { ps @ [p] }

pattern_arg_list_opt:
  | (* empty *) { [] }
  | args = pattern_arg_list { args }

pattern_arg_list:
  | p = pattern { [p] }
  | ps = pattern_arg_list; COMMA; p = pattern { ps @ [p] }

record_pattern_body:
  | DOTDOT { ([], true) }
  | entries = record_pattern_entry_list; rest = record_pattern_rest_opt { (entries, rest) }

record_pattern_entry_list:
  | entry = record_pattern_entry { [entry] }
  | entries = record_pattern_entry_list; COMMA; entry = record_pattern_entry { entries @ [entry] }

record_pattern_entry:
  | name = ident; COLON; pat = pattern { (name, Some pat) }
  | name = ident { (name, None) }

record_pattern_rest_opt:
  | (* empty *) { false }
  | COMMA; DOTDOT { true }

(* ========== 型注釈 ========== *)

type_annot_opt:
  | (* empty *) { None }
  | COLON; ty = type_annot { Some ty }

type_annot:
  | ty = type_primary { ty }
  | lhs = type_primary; ARROW; rhs = type_annot
    {
      let span = make_span $startpos $endpos in
      make_type (TyFn ([lhs], rhs)) span
    }

type_primary:
  | id = ident; args = generic_args_opt
    {
      let span = make_span $startpos $endpos in
      match args with
      | [] -> make_type (TyIdent id) span
      | _ -> make_type (TyApp (id, args)) span
    }
  | LPAREN; ty = type_annot; RPAREN { ty }
  | LPAREN; first = type_annot; COMMA; rest = type_arg_list; RPAREN
    {
      let span = make_span $startpos $endpos in
      make_type (TyTuple (first :: rest)) span
    }
  | LBRACE; fields = type_record_fields; RBRACE
    {
      let span = make_span $startpos $endpos in
      make_type (TyRecord fields) span
    }

type_record_fields:
  | field = type_record_field { [field] }
  | fields = type_record_fields; COMMA; field = type_record_field { fields @ [field] }

type_record_field:
  | name = ident; COLON; ty = type_annot { (name, ty) }

(* ========== ヘルパー ========== *)

lower_ident:
  | id = IDENT
    {
      let span = make_span $startpos $endpos in
      make_ident id span
    }
  | SELF
    {
      let span = make_span $startpos $endpos in
      make_ident "self" span
    }

upper_ident:
  | id = UPPER_IDENT
    {
      let span = make_span $startpos $endpos in
      make_ident id span
    }

ident:
  | id = lower_ident { id }
  | id = upper_ident { id }

ident_list:
  | id = ident { [id] }
  | ids = ident_list; DOT; id = ident { ids @ [id] }

expr_list:
  | e = expr { [e] }
  | es = expr_list; COMMA; e = expr { es @ [e] }

expr_list_opt:
  | (* empty *) { [] }
  | exprs = expr_list { exprs }

record_field_list_opt:
  | (* empty *) { [] }
  | fields = record_field_list { fields }

record_field_list:
  | field = record_field { [field] }
  | fields = record_field_list; COMMA; field = record_field { fields @ [field] }

record_field:
  | name = ident; COLON; value = expr { (name, value) }

tuple_expr_rest:
  | e = expr { [e] }
  | rest = tuple_expr_rest; COMMA; e = expr { rest @ [e] }

(* ========== 仮トークン (未実装部分) ========== *)

%%
