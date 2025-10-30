type trivia_kind =
  | Space
  | Newline
  | Line_comment
  | Block_comment
  | Shebang
  | Hash_inline

val consume :
  ?space_id:int ->
  kind:trivia_kind ->
  start_pos:Lexing.position ->
  end_pos:Lexing.position ->
  unit ->
  unit
(** TODO(LEXER-002 Step6): 実際の集計を導入し、lexeme 共有率メトリクスへ反映する。 *)
