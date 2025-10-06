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

let merge_spans s1 s2 = merge_span s1 s2

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
%token DOT COMMA SEMICOLON COLON EQ COLONEQ ARROW DARROW
%token LPAREN RPAREN LBRACKET RBRACKET LBRACE RBRACE
%token PLUS MINUS STAR SLASH PERCENT POW
%token EQEQ NE LT LE GT GE
%token AND OR NOT
%token QUESTION DOTDOT

(* リテラル *)
%token <string * Ast.int_base> INT
%token <string> FLOAT
%token <string> CHAR
%token <string * Ast.string_kind> STRING
%token <string> IDENT

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
  | path = module_path; DOT; LBRACE; items = use_item_list; RBRACE
    { UseBrace (path, items) }

use_alias_opt:
  | (* empty *) { None }
  | AS; id = ident { Some id }

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
  | supers = super_list { Super (List.length supers) }
  | id = ident { PlainIdent id }

super_list:
  | SUPER { [SUPER] }
  | supers = super_list; DOT; SUPER { supers @ [SUPER] }

relative_tail:
  | (* empty *) { [] }
  | DOT; ids = ident_list { ids }

(* ========== 宣言 ========== *)

decl:
  | attrs = attribute_list; vis = visibility; kind = decl_kind
    {
      let span = make_span $startpos $endpos in
      { attrs; vis; kind; span }
    }

attribute_list:
  | (* empty *) { [] }
  | attrs = attribute_list; attr = attribute { attrs @ [attr] }

attribute:
  | AT; name = ident; args = attribute_args_opt
    {
      let span = make_span $startpos $endpos in
      { name; args; attr_span = span }
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
  (* Phase 1 では他の宣言種別は TODO *)

fn_decl:
  | FN; name = ident; params = fn_params; ret = return_type_opt; body = fn_body
    {
      let span = make_span $startpos $endpos in
      {
        name;
        generic_params = [];  (* TODO: Phase 1 では省略 *)
        params;
        ret_type = ret;
        where_clause = [];
        effect_annot = None;
        body;
      }
    }

fn_params:
  | LPAREN; RPAREN { [] }
  | LPAREN; ps = param_list; RPAREN { ps }

param_list:
  | p = param { [p] }
  | ps = param_list; COMMA; p = param { ps @ [p] }

param:
  | pat = pattern; ty = type_annot_opt
    {
      let span = make_span $startpos $endpos in
      { pat; ty; default = None; param_span = span }
    }

return_type_opt:
  | (* empty *) { None }
  | ARROW; ty = type_annot { Some ty }

fn_body:
  | EQ; e = expr { FnExpr e }
  | block = block_stmt { FnBlock block }

(* ========== 式 ========== *)

expr:
  | e = expr_base { e }
  | e = pipe_expr { e }

expr_base:
  | lit = literal
    {
      let span = make_span $startpos $endpos in
      make_expr (Literal lit) span
    }
  | id = ident
    {
      make_expr (Var id) id.span
    }
  | e = call_expr { e }
  | e = binary_expr { e }
  | e = unary_expr { e }
  | e = if_expr { e }
  | e = block_expr { e }
  | LPAREN; e = expr; RPAREN { e }

literal:
  | i = INT { Int (fst i, snd i) }
  | f = FLOAT { Float f }
  | c = CHAR { Char c }
  | s = STRING { String (fst s, snd s) }
  | TRUE { Bool true }
  | FALSE { Bool false }
  | LPAREN; RPAREN { Unit }

call_expr:
  | func = expr_base; LPAREN; args = arg_list_opt; RPAREN
    {
      let span = merge_span func.span (make_span $endpos $endpos) in
      make_expr (Call (func, args)) span
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
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Add, lhs, rhs)) span
    }
  | lhs = expr; MINUS; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Sub, lhs, rhs)) span
    }
  | lhs = expr; STAR; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Mul, lhs, rhs)) span
    }
  | lhs = expr; SLASH; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Div, lhs, rhs)) span
    }
  | lhs = expr; PERCENT; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Mod, lhs, rhs)) span
    }
  | lhs = expr; POW; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Pow, lhs, rhs)) span
    }
  | lhs = expr; EQEQ; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Eq, lhs, rhs)) span
    }
  | lhs = expr; NE; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Ne, lhs, rhs)) span
    }
  | lhs = expr; LT; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Lt, lhs, rhs)) span
    }
  | lhs = expr; LE; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Le, lhs, rhs)) span
    }
  | lhs = expr; GT; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Gt, lhs, rhs)) span
    }
  | lhs = expr; GE; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (Ge, lhs, rhs)) span
    }
  | lhs = expr; AND; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
      make_expr (Binary (And, lhs, rhs)) span
    }
  | lhs = expr; OR; rhs = expr
    {
      let span = merge_span lhs.span rhs.span in
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
      let span = merge_span lhs.span rhs.span in
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
  | d = decl { DeclStmt d }
  | e = expr; SEMICOLON { ExprStmt e }
  | e = expr { ExprStmt e }  (* 最後の式はセミコロン省略可 *)

(* ========== パターン ========== *)

pattern:
  | id = ident
    {
      make_pattern (PatVar id) id.span
    }
  | UNDERSCORE
    {
      let span = make_span $startpos $endpos in
      make_pattern PatWildcard span
    }

(* ========== 型注釈 ========== *)

type_annot_opt:
  | (* empty *) { None }
  | COLON; ty = type_annot { Some ty }

type_annot:
  | id = ident
    {
      make_type (TyIdent id) id.span
    }

(* ========== ヘルパー ========== *)

ident:
  | id = IDENT
    {
      let span = make_span $startpos $endpos in
      make_ident id span
    }

ident_list:
  | id = ident { [id] }
  | ids = ident_list; DOT; id = ident { ids @ [id] }

expr_list:
  | e = expr { [e] }
  | es = expr_list; COMMA; e = expr { es @ [e] }

(* ========== 仮トークン (未実装部分) ========== *)

%token AT UNDERSCORE

%%
