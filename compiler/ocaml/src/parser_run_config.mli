(** parser_run_config.mli — RunConfig 型定義と拡張管理ユーティリティ
    仕様書 2-1 §D, 2-6 §B 相当の設定レコードを OCaml 実装へ導入する。 *)

(** 左再帰シムの挙動 *)
type left_recursion =
  | Off
  | On
  | Auto

module Extensions : sig
  (** 拡張値（仕様では Any） *)
  type value =
    | Bool of bool
    | Int of int
    | Float of float
    | String of string
    | Parser_id of int
    | List of value list

  module Namespace : sig
    type t

    val empty : t
    val is_empty : t -> bool
    val add : string -> value -> t -> t
    val find : string -> t -> value option
    val remove : string -> t -> t
    val bindings : t -> (string * value) list
  end

  type t

  val empty : t
  val is_empty : t -> bool
  val find_namespace : string -> t -> Namespace.t option
  val with_namespace :
    string -> (Namespace.t -> Namespace.t) -> t -> t
  val remove : string -> t -> t
  val bindings : t -> (string * Namespace.t) list
end

type t = {
  require_eof : bool;
  packrat : bool;
  left_recursion : left_recursion;
  trace : bool;
  merge_warnings : bool;
  legacy_result : bool;
  locale : string option;
  extensions : Extensions.t;
}

type run_config = t

val default : t
(** 仕様既定値（require_eof=false ほか） *)

val with_extension :
  string -> (Extensions.Namespace.t -> Extensions.Namespace.t) -> t -> t

val find_extension : string -> t -> Extensions.Namespace.t option

val set_locale : t -> string option -> t

module Legacy : sig
  type config = {
    require_eof : bool;
    legacy_result : bool;
  }

  val bridge : config -> t
end

module Lex : sig
  module Trivia_profile : sig
    type comment_pair = {
      start : string;
      stop : string;
      nested : bool;
    }

    type t = {
      line : string list;
      block : comment_pair list;
      shebang : bool;
      hash_inline : bool;
      doc_comment : string option;
    }

    val strict_json : t
    val json_relaxed : t
    val toml_relaxed : t
  end

  type profile =
    | Strict_json
    | Json_relaxed
    | Toml_relaxed
    | Custom of string

  type t = {
    space_id : int option;
    profile : profile;
    namespace : Extensions.Namespace.t option;
  }

  val default : t
  val of_run_config : run_config -> t
  val profile_symbol : profile -> string
  val profile_of_symbol : string -> profile
  val effective_trivia : t -> Trivia_profile.t
  val set_profile : run_config -> profile -> run_config
  val set_space_id : run_config -> int option -> run_config
end

module Config : sig
  val find : t -> Extensions.Namespace.t option
  val require_eof_override : Extensions.Namespace.t -> bool option
  val trivia_profile : Extensions.Namespace.t -> Lex.profile option
  val with_trivia_profile : t -> Lex.profile -> t
end

module Recover : sig
  type t = {
    sync_tokens : string list;
    emit_notes : bool;
    namespace : Extensions.Namespace.t option;
  }

  val default : t
  val of_run_config : run_config -> t
end

module Stream : sig
  type t = {
    enabled : bool;
    checkpoint : string option;
    resume_hint : string option;
    demand_min_bytes : int option;
    demand_preferred_bytes : int option;
    chunk_size : int option;
    namespace : Extensions.Namespace.t option;
  }

  val default : t
  val of_run_config : run_config -> t
  val set_enabled : bool -> run_config -> run_config
  val set_checkpoint : string option -> run_config -> run_config
  val set_resume_hint : string option -> run_config -> run_config
  val set_demand_min_bytes : int option -> run_config -> run_config
  val set_demand_preferred_bytes : int option -> run_config -> run_config
  val set_chunk_size : int option -> run_config -> run_config
end

module Effects : sig
  type t = {
    stage_override : string option;
    registry_path : string option;
    required_capabilities : string list;
    namespace : Extensions.Namespace.t option;
  }

  val default : t
  val of_run_config : run_config -> t
  val set_stage_override : string option -> run_config -> run_config
  val set_registry_path : string option -> run_config -> run_config
  val set_required_capabilities : string list -> run_config -> run_config
end
