(* run_config_loader — LSP 用 RunConfig デコーダ
 *
 * Phase 2-5 PARSER-002 Step4: LSP 設定ファイルから RunConfig を生成し、
 * CLI と同一の拡張ネームスペースを共有する。
 *)

open Yojson.Basic

module Run_config = Parser_run_config
module Extensions = Parser_run_config.Extensions

let member name = function
  | `Assoc fields -> (match List.assoc_opt name fields with Some v -> v | None -> `Null)
  | _ -> `Null

let bool_field ?(default = false) name json =
  match member name json with
  | `Bool value -> value
  | `Null -> default
  | _ -> default

let string_field name json =
  match member name json with
  | `String value when String.trim value <> "" -> Some value
  | _ -> None

let int_field name json =
  match member name json with
  | `Int value -> Some value
  | `Float value -> Some (int_of_float value)
  | `String text -> (try Some (int_of_string text) with Failure _ -> None)
  | _ -> None

let string_list_field name json =
  match member name json with
  | `List values ->
      values
      |> List.filter_map (function `String text -> Some text | _ -> None)
  | `String text -> [ text ]
  | _ -> []

let left_recursion_of_string text default =
  match String.lowercase_ascii (String.trim text) with
  | "on" -> Run_config.On
  | "off" -> Run_config.Off
  | "auto" -> Run_config.Auto
  | _ -> default

let decode_left_recursion json default =
  match string_field "leftRecursion" json with
  | Some text -> left_recursion_of_string text default
  | None -> default

let namespace_add_list name entries namespace =
  match entries with
  | [] -> namespace
  | xs ->
      let values = List.map (fun text -> Extensions.String text) xs in
      Extensions.Namespace.add name (Extensions.List values) namespace

let decode_lex_namespace json =
  let module Namespace = Extensions.Namespace in
  let namespace = Namespace.empty in
  let namespace =
    match string_field "profile" json with
    | Some profile -> Namespace.add "profile" (Extensions.String profile) namespace
    | None -> namespace
  in
  let namespace =
    match int_field "spaceId" json with
    | Some id -> Namespace.add "space_id" (Extensions.Int id) namespace
    | None -> namespace
  in
  namespace

let decode_recover_namespace json =
  let module Namespace = Extensions.Namespace in
  let namespace = Namespace.empty in
  let namespace =
    namespace_add_list "sync_tokens" (string_list_field "syncTokens" json) namespace
  in
  let namespace =
    match member "notes" json with
    | `Bool value -> Namespace.add "notes" (Extensions.Bool value) namespace
    | _ -> namespace
  in
  namespace

let decode_stream_namespace json =
  let module Namespace = Extensions.Namespace in
  let namespace = Namespace.empty in
  let namespace =
    match string_field "checkpoint" json with
    | Some value -> Namespace.add "checkpoint" (Extensions.String value) namespace
    | None -> namespace
  in
  let namespace =
    match string_field "resumeHint" json with
    | Some value -> Namespace.add "resume_hint" (Extensions.String value) namespace
    | None -> namespace
  in
  namespace

let add_extension_if_nonempty key namespace config =
  if Extensions.Namespace.is_empty namespace then config
  else Run_config.with_extension key (fun _ -> namespace) config

let apply_extensions base json =
  let module Namespace = Extensions.Namespace in
  let config_namespace =
    Namespace.empty
    |> Namespace.add "source" (Extensions.String "lsp")
    |> Namespace.add "require_eof" (Extensions.Bool base.Run_config.require_eof)
    |> Namespace.add "packrat" (Extensions.Bool base.Run_config.packrat)
    |> Namespace.add "left_recursion"
         (Extensions.String (match base.Run_config.left_recursion with
           | Run_config.On -> "on"
           | Run_config.Off -> "off"
           | Run_config.Auto -> "auto"))
    |> Namespace.add "trace" (Extensions.Bool base.Run_config.trace)
    |> Namespace.add "merge_warnings" (Extensions.Bool base.Run_config.merge_warnings)
    |> Namespace.add "legacy_result" (Extensions.Bool base.Run_config.legacy_result)
  in
  let config = Run_config.with_extension "config" (fun _ -> config_namespace) base in
  let extensions_json = member "extensions" json in
  let lex_namespace = decode_lex_namespace (member "lex" extensions_json) in
  let recover_namespace = decode_recover_namespace (member "recover" extensions_json) in
  let stream_namespace = decode_stream_namespace (member "stream" extensions_json) in
  config
  |> add_extension_if_nonempty "lex" lex_namespace
  |> add_extension_if_nonempty "recover" recover_namespace
  |> add_extension_if_nonempty "stream" stream_namespace

let of_json json =
  let base =
    {
      Run_config.default with
      require_eof = bool_field ~default:Run_config.default.require_eof "requireEof" json;
      packrat = bool_field ~default:Run_config.default.packrat "packrat" json;
      left_recursion = decode_left_recursion json Run_config.default.left_recursion;
      trace = bool_field ~default:Run_config.default.trace "trace" json;
      merge_warnings = bool_field ~default:Run_config.default.merge_warnings "mergeWarnings" json;
      legacy_result = bool_field ~default:false "legacyResult" json;
      locale = string_field "locale" json;
    }
  in
  apply_extensions base json

let from_file path =
  Yojson.Basic.from_file path |> of_json

let default_config_path = "tooling/lsp/config/default.json"

let default () =
  if Sys.file_exists default_config_path then from_file default_config_path
  else Run_config.with_extension
         "config"
         (fun _ ->
           let module Namespace = Extensions.Namespace in
           Namespace.empty
           |> Namespace.add "source" (Extensions.String "lsp") )
         Run_config.default
