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
  type value =
    | Bool of bool
    | Int of int
    | Float of float
    | String of string
    | Parser_id of int
    | List of value list

  module Namespace = struct
    module Map = Map.Make (String)

    type t = value Map.t

    let empty = Map.empty
    let is_empty = Map.is_empty
    let add = Map.add
    let remove = Map.remove
    let find key map = Map.find_opt key map
    let bindings = Map.bindings
  end

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

module Namespace = Extensions.Namespace

let bool_of_value = function Extensions.Bool value -> Some value | _ -> None

let int_of_value = function
  | Extensions.Int value -> Some value
  | Extensions.Parser_id value -> Some value
  | Extensions.String text -> (
      try Some (int_of_string text) with Failure _ -> None)
  | _ -> None

let string_of_value = function Extensions.String value -> Some value | _ -> None

let string_list_of_value = function
  | Extensions.List values ->
      values |> List.filter_map string_of_value
  | Extensions.String value -> [ value ]
  | _ -> []

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

module Lex = struct
  module Trivia_profile = struct
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

    let strict_json =
      {
        line = [ "//" ];
        block = [ { start = "/*"; stop = "*/"; nested = false } ];
        shebang = false;
        hash_inline = false;
        doc_comment = None;
      }

    let json_relaxed =
      {
        line = [ "//" ];
        block = [ { start = "/*"; stop = "*/"; nested = false } ];
        shebang = true;
        hash_inline = false;
        doc_comment = None;
      }

    let toml_relaxed =
      {
        line = [ "#"; "//" ];
        block = [];
        shebang = false;
        hash_inline = true;
        doc_comment = None;
      }
  end

  type profile =
    | Strict_json
    | Json_relaxed
    | Toml_relaxed
    | Custom of string

  type t = {
    space_id : int option;
    profile : profile;
    namespace : Namespace.t option;
  }

  let default =
    { space_id = None; profile = Strict_json; namespace = None }

  let profile_symbol = function
    | Strict_json -> "strict_json"
    | Json_relaxed -> "json_relaxed"
    | Toml_relaxed -> "toml_relaxed"
    | Custom symbol -> symbol

  let profile_of_symbol symbol =
    match String.lowercase_ascii symbol with
    | "strict_json" | "strict-json" -> Strict_json
    | "json_relaxed" | "json-relaxed" -> Json_relaxed
    | "toml_relaxed" | "toml-relaxed" -> Toml_relaxed
    | _other -> Custom symbol

  let decode_space_id namespace =
    match Namespace.find "space_id" namespace with
    | None -> None
    | Some value -> int_of_value value

  let decode_profile namespace =
    match Namespace.find "profile" namespace with
    | Some value -> (
        match string_of_value value with
        | Some symbol -> profile_of_symbol symbol
        | None -> default.profile)
    | None -> default.profile

  let of_run_config config =
    match find_extension "lex" config with
    | None -> default
    | Some namespace ->
        {
          namespace = Some namespace;
          space_id = decode_space_id namespace;
          profile = decode_profile namespace;
        }

  let effective_trivia (config : t) =
    match config.profile with
    | Strict_json -> Trivia_profile.strict_json
    | Json_relaxed -> Trivia_profile.json_relaxed
    | Toml_relaxed -> Trivia_profile.toml_relaxed
    | Custom _ -> Trivia_profile.strict_json

  let encode_profile namespace profile =
    Namespace.add "profile" (Extensions.String (profile_symbol profile)) namespace

  let encode_space_id namespace = function
    | Some id -> Namespace.add "space_id" (Extensions.Parser_id id) namespace
    | None -> Namespace.remove "space_id" namespace

  let set_profile config profile =
    with_extension "lex" (fun namespace -> encode_profile namespace profile) config

  let set_space_id config space_id =
    with_extension "lex" (fun namespace -> encode_space_id namespace space_id) config
end

module Config = struct
  let find config = find_extension "config" config

  let require_eof_override namespace =
    match Namespace.find "require_eof" namespace with
    | Some value -> bool_of_value value
    | None -> None

  let trivia_profile namespace =
    match Namespace.find "trivia" namespace with
    | Some value -> (
        match string_of_value value with
        | Some symbol -> Some (Lex.profile_of_symbol symbol)
        | None -> None)
    | None -> None

  let with_trivia_profile config profile =
    with_extension "config"
      (fun namespace ->
        Namespace.add
          "trivia"
          (Extensions.String (Lex.profile_symbol profile))
          namespace)
      config
end

module Recover = struct
  type t = {
    sync_tokens : string list;
    emit_notes : bool;
    namespace : Namespace.t option;
  }

  let default = { sync_tokens = []; emit_notes = false; namespace = None }

  let decode_sync_tokens namespace =
    match Namespace.find "sync_tokens" namespace with
    | Some value -> string_list_of_value value
    | None -> []

  let decode_emit_notes namespace =
    match Namespace.find "notes" namespace with
    | Some value -> Option.value ~default:default.emit_notes (bool_of_value value)
    | None -> default.emit_notes

  let of_run_config config =
    match find_extension "recover" config with
    | None -> default
    | Some namespace ->
        {
          namespace = Some namespace;
          sync_tokens = decode_sync_tokens namespace;
          emit_notes = decode_emit_notes namespace;
        }
end

module Stream = struct
  type t = {
    checkpoint : string option;
    resume_hint : string option;
    namespace : Namespace.t option;
  }

  let default = { checkpoint = None; resume_hint = None; namespace = None }

  let decode_string namespace key =
    match Namespace.find key namespace with
    | Some value -> string_of_value value
    | None -> None

  let of_run_config config =
    match find_extension "stream" config with
    | None -> default
    | Some namespace ->
        {
          namespace = Some namespace;
          checkpoint = decode_string namespace "checkpoint";
          resume_hint = decode_string namespace "resume_hint";
        }
end
