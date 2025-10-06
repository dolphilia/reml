(* Token — 字句トークン定義
 *
 * Reml の字句解析で使用するトークン型を定義する。
 * docs/spec/1-1-syntax.md §A に基づく。
 *)

type t =
  (* キーワード *)
  | MODULE | USE | AS | PUB | SELF | SUPER
  | LET | VAR | FN | TYPE | ALIAS | NEW | TRAIT | IMPL | EXTERN
  | EFFECT | OPERATION | HANDLER | CONDUCTOR | CHANNELS | EXECUTION | MONITORING
  | IF | THEN | ELSE | MATCH | WITH | FOR | IN | WHILE | LOOP | RETURN | DEFER | UNSAFE
  | PERFORM | DO | HANDLE
  | WHERE
  | TRUE | FALSE
  | BREAK | CONTINUE  (* 将来予約 *)

  (* 演算子・区切り *)
  | PIPE           (* |> *)
  | CHANNEL_PIPE   (* ~> *)
  | DOT            (* . *)
  | COMMA          (* , *)
  | SEMICOLON      (* ; *)
  | COLON          (* : *)
  | EQ             (* = *)
  | COLONEQ        (* := *)
  | ARROW          (* -> *)
  | DARROW         (* => *)
  | LPAREN | RPAREN
  | LBRACKET | RBRACKET
  | LBRACE | RBRACE
  | PLUS | MINUS | STAR | SLASH | PERCENT | POW  (* ^ *)
  | EQEQ | NE | LT | LE | GT | GE
  | AND | OR | NOT
  | QUESTION       (* ? *)
  | DOTDOT         (* .. *)

  (* リテラル *)
  | INT of string * Ast.int_base
  | FLOAT of string
  | CHAR of string
  | STRING of string * Ast.string_kind
  | IDENT of string

  (* その他 *)
  | EOF

(** トークンを文字列表現に変換 (デバッグ用) *)
let to_string = function
  | MODULE -> "module"
  | USE -> "use"
  | AS -> "as"
  | PUB -> "pub"
  | SELF -> "self"
  | SUPER -> "super"
  | LET -> "let"
  | VAR -> "var"
  | FN -> "fn"
  | TYPE -> "type"
  | ALIAS -> "alias"
  | NEW -> "new"
  | TRAIT -> "trait"
  | IMPL -> "impl"
  | EXTERN -> "extern"
  | EFFECT -> "effect"
  | OPERATION -> "operation"
  | HANDLER -> "handler"
  | CONDUCTOR -> "conductor"
  | CHANNELS -> "channels"
  | EXECUTION -> "execution"
  | MONITORING -> "monitoring"
  | IF -> "if"
  | THEN -> "then"
  | ELSE -> "else"
  | MATCH -> "match"
  | WITH -> "with"
  | FOR -> "for"
  | IN -> "in"
  | WHILE -> "while"
  | LOOP -> "loop"
  | RETURN -> "return"
  | DEFER -> "defer"
  | UNSAFE -> "unsafe"
  | PERFORM -> "perform"
  | DO -> "do"
  | HANDLE -> "handle"
  | WHERE -> "where"
  | TRUE -> "true"
  | FALSE -> "false"
  | BREAK -> "break"
  | CONTINUE -> "continue"
  | PIPE -> "|>"
  | CHANNEL_PIPE -> "~>"
  | DOT -> "."
  | COMMA -> ","
  | SEMICOLON -> ";"
  | COLON -> ":"
  | EQ -> "="
  | COLONEQ -> ":="
  | ARROW -> "->"
  | DARROW -> "=>"
  | LPAREN -> "("
  | RPAREN -> ")"
  | LBRACKET -> "["
  | RBRACKET -> "]"
  | LBRACE -> "{"
  | RBRACE -> "}"
  | PLUS -> "+"
  | MINUS -> "-"
  | STAR -> "*"
  | SLASH -> "/"
  | PERCENT -> "%"
  | POW -> "^"
  | EQEQ -> "=="
  | NE -> "!="
  | LT -> "<"
  | LE -> "<="
  | GT -> ">"
  | GE -> ">="
  | AND -> "&&"
  | OR -> "||"
  | NOT -> "!"
  | QUESTION -> "?"
  | DOTDOT -> ".."
  | INT (s, _) -> "INT(" ^ s ^ ")"
  | FLOAT s -> "FLOAT(" ^ s ^ ")"
  | CHAR s -> "CHAR(" ^ s ^ ")"
  | STRING (s, _) -> "STRING(\"" ^ String.escaped s ^ "\")"
  | IDENT s -> "IDENT(" ^ s ^ ")"
  | EOF -> "EOF"

(** 予約語テーブル *)
let keyword_table = [
  ("module", MODULE);
  ("use", USE);
  ("as", AS);
  ("pub", PUB);
  ("self", SELF);
  ("super", SUPER);
  ("let", LET);
  ("var", VAR);
  ("fn", FN);
  ("type", TYPE);
  ("alias", ALIAS);
  ("new", NEW);
  ("trait", TRAIT);
  ("impl", IMPL);
  ("extern", EXTERN);
  ("effect", EFFECT);
  ("operation", OPERATION);
  ("handler", HANDLER);
  ("conductor", CONDUCTOR);
  ("channels", CHANNELS);
  ("execution", EXECUTION);
  ("monitoring", MONITORING);
  ("if", IF);
  ("then", THEN);
  ("else", ELSE);
  ("match", MATCH);
  ("with", WITH);
  ("for", FOR);
  ("in", IN);
  ("while", WHILE);
  ("loop", LOOP);
  ("return", RETURN);
  ("defer", DEFER);
  ("unsafe", UNSAFE);
  ("perform", PERFORM);
  ("do", DO);
  ("handle", HANDLE);
  ("where", WHERE);
  ("true", TRUE);
  ("false", FALSE);
  ("break", BREAK);
  ("continue", CONTINUE);
]

(** 識別子を予約語と照合 *)
let keyword_or_ident str =
  match List.assoc_opt str keyword_table with
  | Some tok -> tok
  | None -> IDENT str
