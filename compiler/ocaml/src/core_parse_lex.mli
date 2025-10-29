(** Core.Parse.Lex 橋渡しユーティリティ (LEXER-002 Step2)

    仕様で定義される `ConfigTriviaProfile` と `RunConfig.extensions`
    の間を橋渡しし、後続ステップで `config_trivia` 系 API を実装できるようにする。
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
