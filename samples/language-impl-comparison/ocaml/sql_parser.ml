(* 簡易SQL Parser - OCaml実装 *)
(* SELECT, WHERE, JOIN, ORDER BY対応 *)
(* Angstromパーサーコンビネーターライブラリを使用 *)

open Angstrom

(* AST定義 *)
type order_direction = Asc | Desc

type join_type =
  | InnerJoin
  | LeftJoin
  | RightJoin
  | FullJoin

type bin_op =
  | Add | Sub | Mul | Div | Mod
  | Eq | Ne | Lt | Le | Gt | Ge
  | And | Or | Like

type un_op =
  | Not
  | IsNull
  | IsNotNull

type literal =
  | IntLit of int
  | FloatLit of float
  | StringLit of string
  | BoolLit of bool
  | NullLit

type expr =
  | Literal of literal
  | Column of string
  | QualifiedColumn of string * string
  | BinaryOp of bin_op * expr * expr
  | UnaryOp of un_op * expr
  | FunctionCall of string * expr list
  | Parenthesized of expr

type column =
  | AllColumns
  | ColumnExpr of expr * string option

type table_ref = {
  table : string;
  alias : string option;
}

type join = {
  join_type : join_type;
  join_table : table_ref;
  on_condition : expr;
}

type order_by = {
  order_columns : (expr * order_direction) list;
}

type query = {
  columns : column list;
  from_table : table_ref;
  where_clause : expr option;
  joins : join list;
  order_by : order_by option;
}

(* パーサー補助関数 *)
let is_whitespace = function
  | ' ' | '\t' | '\n' | '\r' -> true
  | _ -> false

let whitespace = take_while is_whitespace

let line_comment =
  string "--" *> take_till (fun c -> c = '\n') *> return ()

let block_comment =
  string "/*" *> take_till (fun c -> false) *> string "*/" *> return ()
  (* 簡略化: ネストなしのブロックコメント *)

let sc = skip_many (whitespace <|> line_comment <|> block_comment)

let lexeme p = p <* sc

let symbol s = lexeme (string s)

(* キーワード（大文字小文字を区別しない） *)
let keyword kw =
  lexeme (
    string_ci kw >>= fun _ ->
    peek_char >>= function
    | Some c when (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
                  (c >= '0' && c <= '9') || c = '_' ->
        fail "keyword followed by alphanumeric"
    | _ -> return ()
  )

(* 識別子 *)
let is_ident_start c =
  (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c = '_'

let is_ident_cont c =
  is_ident_start c || (c >= '0' && c <= '9')

let identifier =
  let reserved = ["select"; "from"; "where"; "join"; "inner"; "left";
                  "right"; "full"; "on"; "and"; "or"; "not"; "like";
                  "order"; "by"; "asc"; "desc"; "null"; "true"; "false"; "as"] in
  lexeme (
    satisfy is_ident_start >>= fun first ->
    take_while is_ident_cont >>= fun rest ->
    let name = String.make 1 first ^ rest in
    let lower = String.lowercase_ascii name in
    if List.mem lower reserved then
      fail ("reserved word: " ^ name)
    else
      return name
  )

(* リテラル *)
let integer =
  lexeme (take_while1 (fun c -> c >= '0' && c <= '9')) >>= fun s ->
  return (IntLit (int_of_string s))

let float_lit =
  lexeme (
    take_while1 (fun c -> c >= '0' && c <= '9') >>= fun int_part ->
    char '.' >>= fun _ ->
    take_while1 (fun c -> c >= '0' && c <= '9') >>= fun frac_part ->
    return (FloatLit (float_of_string (int_part ^ "." ^ frac_part)))
  )

let string_lit =
  lexeme (
    char '\'' *> take_till (fun c -> c = '\'') <* char '\'' >>= fun s ->
    return (StringLit s)
  )

let literal =
  choice [
    keyword "null" *> return NullLit;
    keyword "true" *> return (BoolLit true);
    keyword "false" *> return (BoolLit false);
    float_lit;
    integer;
    string_lit;
  ]

(* 式パーサー（演算子優先度を考慮した再帰下降パーサー） *)
let rec expr_parser () =
  or_expr ()

and or_expr () =
  and_expr () >>= fun left ->
  many (keyword "or" *> and_expr ()) >>= fun rights ->
  return (List.fold_left (fun acc r -> BinaryOp (Or, acc, r)) left rights)

and and_expr () =
  cmp_expr () >>= fun left ->
  many (keyword "and" *> cmp_expr ()) >>= fun rights ->
  return (List.fold_left (fun acc r -> BinaryOp (And, acc, r)) left rights)

and cmp_expr () =
  add_expr () >>= fun left ->
  option None (
    choice [
      symbol "=" *> return Eq;
      symbol "<>" *> return Ne;
      symbol "!=" *> return Ne;
      symbol "<=" *> return Le;
      symbol ">=" *> return Ge;
      symbol "<" *> return Lt;
      symbol ">" *> return Gt;
      keyword "like" *> return Like;
    ] >>= fun op ->
    add_expr () >>= fun right ->
    return (Some (op, right))
  ) >>= function
  | None -> return left
  | Some (op, right) -> return (BinaryOp (op, left, right))

and add_expr () =
  mul_expr () >>= fun left ->
  many (
    choice [symbol "+" *> return Add; symbol "-" *> return Sub] >>= fun op ->
    mul_expr () >>= fun right ->
    return (op, right)
  ) >>= fun ops ->
  return (List.fold_left (fun acc (op, r) -> BinaryOp (op, acc, r)) left ops)

and mul_expr () =
  unary_expr () >>= fun left ->
  many (
    choice [
      symbol "*" *> return Mul;
      symbol "/" *> return Div;
      symbol "%" *> return Mod;
    ] >>= fun op ->
    unary_expr () >>= fun right ->
    return (op, right)
  ) >>= fun ops ->
  return (List.fold_left (fun acc (op, r) -> BinaryOp (op, acc, r)) left ops)

and unary_expr () =
  choice [
    keyword "not" *> unary_expr () >>= fun e ->
    return (UnaryOp (Not, e));
    postfix_expr ();
  ]

and postfix_expr () =
  primary_expr () >>= fun e ->
  option None (
    keyword "is" *>
    option false (keyword "not" *> return true) >>= fun is_not ->
    keyword "null" *>
    return (if is_not then IsNotNull else IsNull)
  ) >>= function
  | None -> return e
  | Some op -> return (UnaryOp (op, e))

and primary_expr () =
  choice [
    symbol "(" *> expr_parser () <* symbol ")" >>= fun e ->
    return (Parenthesized e);
    function_call ();
    column_ref ();
    literal >>= fun lit -> return (Literal lit);
  ]

and function_call () =
  identifier >>= fun name ->
  symbol "(" *>
  sep_by (symbol ",") (expr_parser ()) <* symbol ")" >>= fun args ->
  return (FunctionCall (name, args))

and column_ref () =
  identifier >>= fun first ->
  option None (symbol "." *> identifier >>= fun col -> return (Some col)) >>= function
  | None -> return (Column first)
  | Some col -> return (QualifiedColumn (first, col))

(* カラムリスト *)
let column_list =
  choice [
    symbol "*" *> return [AllColumns];
    sep_by1 (symbol ",") (
      expr_parser () >>= fun e ->
      option None (option () (keyword "as") *> identifier >>= fun alias -> return (Some alias)) >>= fun alias ->
      return (ColumnExpr (e, alias))
    );
  ]

(* テーブル参照 *)
let table_ref =
  identifier >>= fun table ->
  option None (option () (keyword "as") *> identifier >>= fun alias -> return (Some alias)) >>= fun alias ->
  return { table; alias }

(* JOIN句 *)
let join_type =
  choice [
    keyword "inner" *> keyword "join" *> return InnerJoin;
    keyword "left" *> keyword "join" *> return LeftJoin;
    keyword "right" *> keyword "join" *> return RightJoin;
    keyword "full" *> keyword "join" *> return FullJoin;
    keyword "join" *> return InnerJoin;
  ]

let join_clause =
  join_type >>= fun jt ->
  table_ref >>= fun tbl ->
  keyword "on" *> expr_parser () >>= fun cond ->
  return { join_type = jt; join_table = tbl; on_condition = cond }

(* ORDER BY句 *)
let order_by_clause =
  keyword "order" *> keyword "by" *>
  sep_by1 (symbol ",") (
    expr_parser () >>= fun e ->
    option Asc (
      choice [
        keyword "asc" *> return Asc;
        keyword "desc" *> return Desc;
      ]
    ) >>= fun dir ->
    return (e, dir)
  ) >>= fun cols ->
  return { order_columns = cols }

(* SELECT文 *)
let select_query =
  keyword "select" *> column_list >>= fun cols ->
  keyword "from" *> table_ref >>= fun from ->
  many join_clause >>= fun joins ->
  option None (keyword "where" *> expr_parser () >>= fun e -> return (Some e)) >>= fun where_c ->
  option None (order_by_clause >>= fun ob -> return (Some ob)) >>= fun order ->
  return {
    columns = cols;
    from_table = from;
    where_clause = where_c;
    joins = joins;
    order_by = order;
  }

(* パブリックAPI *)
let parse input =
  parse_string ~consume:All (sc *> select_query <* option () (symbol ";")) input

(* レンダリング関数 *)
let render_literal = function
  | IntLit n -> string_of_int n
  | FloatLit f -> string_of_float f
  | StringLit s -> "'" ^ s ^ "'"
  | BoolLit b -> if b then "TRUE" else "FALSE"
  | NullLit -> "NULL"

let render_binop = function
  | Add -> "+" | Sub -> "-" | Mul -> "*" | Div -> "/" | Mod -> "%"
  | Eq -> "=" | Ne -> "<>" | Lt -> "<" | Le -> "<=" | Gt -> ">" | Ge -> ">="
  | And -> "AND" | Or -> "OR" | Like -> "LIKE"

let rec render_expr = function
  | Literal lit -> render_literal lit
  | Column name -> name
  | QualifiedColumn (tbl, col) -> tbl ^ "." ^ col
  | BinaryOp (op, left, right) ->
      "(" ^ render_expr left ^ " " ^ render_binop op ^ " " ^ render_expr right ^ ")"
  | UnaryOp (Not, e) -> "NOT " ^ render_expr e
  | UnaryOp (IsNull, e) -> render_expr e ^ " IS NULL"
  | UnaryOp (IsNotNull, e) -> render_expr e ^ " IS NOT NULL"
  | FunctionCall (name, args) ->
      name ^ "(" ^ String.concat ", " (List.map render_expr args) ^ ")"
  | Parenthesized e -> "(" ^ render_expr e ^ ")"

let render_column = function
  | AllColumns -> "*"
  | ColumnExpr (e, None) -> render_expr e
  | ColumnExpr (e, Some alias) -> render_expr e ^ " AS " ^ alias

let render_query q =
  let cols = String.concat ", " (List.map render_column q.columns) in
  let from = "FROM " ^ q.from_table.table ^
    (match q.from_table.alias with None -> "" | Some a -> " AS " ^ a) in
  let joins = String.concat " " (List.map (fun j ->
    let jt = match j.join_type with
      | InnerJoin -> "INNER JOIN"
      | LeftJoin -> "LEFT JOIN"
      | RightJoin -> "RIGHT JOIN"
      | FullJoin -> "FULL JOIN" in
    jt ^ " " ^ j.join_table.table ^ " ON " ^ render_expr j.on_condition
  ) q.joins) in
  let where = match q.where_clause with
    | None -> ""
    | Some e -> " WHERE " ^ render_expr e in
  let order = match q.order_by with
    | None -> ""
    | Some ob ->
        let cols = String.concat ", " (List.map (fun (e, dir) ->
          render_expr e ^ " " ^ (if dir = Asc then "ASC" else "DESC")
        ) ob.order_columns) in
        " ORDER BY " ^ cols in
  "SELECT " ^ cols ^ " " ^ from ^ " " ^ joins ^ where ^ order

(* テスト *)
let () =
  print_endline "=== OCaml SQL Parser テスト ===";
  let test_sql = "SELECT * FROM users WHERE id = 1" in
  match parse test_sql with
  | Ok q ->
      print_endline ("パース成功: " ^ test_sql);
      print_endline ("レンダリング: " ^ render_query q)
  | Error msg ->
      print_endline ("パースエラー: " ^ msg)