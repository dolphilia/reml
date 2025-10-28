(* parser_run_config.ml — RunConfig 型定義と拡張管理ユーティリティ
 *
 * 仕様書 2-1 §D, 2-6 §B に準拠した設定レコードを OCaml 側へ提供する。
 *)

type left_recursion =
  | Off
  | On
  | Auto

module String_map = Map.Make (String)

module Extensions = struct
  module Namespace = struct
    module Map = Map.Make (String)

    type value =
      | Bool of bool
      | Int of int
      | Float of float
      | String of string
      | Parser_id of int
      | List of value list

    and t = value Map.t

    let empty = Map.empty
    let is_empty = Map.is_empty
    let add = Map.add
    let remove = Map.remove
    let find key map = Map.find_opt key map
    let bindings = Map.bindings
  end

  type value = Namespace.value
  type namespace = Namespace.t
  type t = namespace String_map.t

  let empty = String_map.empty
  let is_empty = String_map.is_empty
  let find_namespace key extensions =
    String_map.find_opt key extensions

  let with_namespace key update extensions =
    let current =
      match find_namespace key extensions with
      | Some namespace -> namespace
      | None -> Namespace.empty
    in
    let updated = update current in
    if Namespace.is_empty updated then String_map.remove key extensions
    else String_map.add key updated extensions

  let remove = String_map.remove
  let bindings = String_map.bindings
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

let default =
  {
    require_eof = false;
    packrat = false;
    left_recursion = Auto;
    trace = false;
    merge_warnings = true;
    legacy_result = false;
    locale = None;
    extensions = Extensions.empty;
  }

let with_extension key update config =
  {
    config with
    extensions = Extensions.with_namespace key update config.extensions;
  }

let find_extension key config =
  Extensions.find_namespace key config.extensions

let set_locale config locale = { config with locale }

module Legacy = struct
  type config = {
    require_eof : bool;
    legacy_result : bool;
  }

  let bridge { require_eof; legacy_result } =
    { default with require_eof; legacy_result }
end
