type trivia_kind =
  | Space
  | Newline
  | Line_comment
  | Block_comment
  | Shebang
  | Hash_inline

let consume ?space_id:_ ~kind:_ ~start_pos:_ ~end_pos:_ () = ()
(* TODO(LEXER-002 Step6): 実計測を収集し、lexer.shared_profile_pass_rate へ連携する。 *)
