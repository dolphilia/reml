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
