(** Core.Parse.Lex 橋渡しユーティリティ (LEXER-002 Step3)

    仕様で定義される `ConfigTriviaProfile` と `RunConfig.extensions`
    の間を橋渡しし、`lexeme` / `symbol` などのユーティリティを
    OCaml 実装で呼び出せる形へ露出する。
*)

module Trivia_profile : sig
  type comment_pair =
    Parser_run_config.Lex.Trivia_profile.comment_pair = {
      start : string;
      stop : string;
      nested : bool;
    }

  type t = Parser_run_config.Lex.Trivia_profile.t = {
    line : string list;
    block : comment_pair list;
    shebang : bool;
    hash_inline : bool;
    doc_comment : string option;
  }

  val strict_json : t
  val json_relaxed : t
  val toml_relaxed : t

  val of_profile :
    Parser_run_config.Lex.profile ->
    namespace:Parser_run_config.Extensions.Namespace.t option ->
    t
end

module Pack : sig
  type source =
    | From_config_namespace
    | From_lex_namespace
    | Default

  type t = {
    profile : Parser_run_config.Lex.profile;
    trivia : Trivia_profile.t;
    namespace : Parser_run_config.Extensions.Namespace.t option;
    space_id : int option;
    source : source;
  }
end

module Bridge : sig
  val derive :
    Parser_run_config.run_config ->
    Pack.t * Parser_run_config.run_config

  val with_space_id :
    Pack.t ->
    Parser_run_config.run_config ->
    int ->
    Pack.t * Parser_run_config.run_config
end

module Record = Core_parse_lex_record

module Api : sig
  (** `Lexing.lexbuf -> 'a` を Parser コンビネータになぞらえた疑似型。 *)
  type 'a reader = Lexing.lexbuf -> 'a

  val config_trivia : Pack.t -> unit reader
  (** `RunConfig` から導出したトリビア設定を `Lexer` へ適用し、
      行・ブロックコメント/シェバン設定を同期する。 *)

  val leading : Pack.t -> 'a reader -> 'a reader
  (** 先頭側のトリビアをスキップしてから `reader` を実行する。 *)

  val lexeme : Pack.t -> 'a reader -> 'a reader
  (** `reader` 実行後に後続トリビアをスキップする。 *)

  val trim : Pack.t -> 'a reader -> 'a reader
  (** 前後双方のトリビアをスキップする。 *)

  val symbol : Pack.t -> string -> unit reader
  (** 固定記号を読み取り、後続トリビアをスキップする。 *)

  val token :
    Pack.t ->
    'a reader ->
    ('a * Ast.span) reader
  (** `reader` が消費した直近トークンに `Ast.span` を付与する。 *)
end
