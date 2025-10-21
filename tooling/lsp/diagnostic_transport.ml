(* tooling/lsp/diagnostic_transport.ml — Diagnostic.t から LSP PublishDiagnostics への変換
 *
 * Stage B 要件:
 * - 診断 V2 スキーマ（tooling/json-schema/diagnostic-v2.schema.json）と整合する JSON 生成
 * - secondary/hints/audit/timestamp を LSP data / relatedInformation へ直接マッピング
 * - CI 側で参照可能な schema.version を公開
 *)

open Diagnostic

module Json = Yojson.Basic

let schema_version = "2.0.0-draft"

let lsp_source = "reml"

let lsp_uri_of_filename filename =
  if filename = "" || filename = "<入力>" then "reml://embedded"
  else if String.length filename >= 7 && String.sub filename 0 7 = "file://" then
    filename
  else
    "file://" ^ filename

let lsp_position_of_location (loc : location) =
  `Assoc
    [
      ("line", `Int (max 0 (loc.line - 1)));
      ("character", `Int (max 0 (loc.column - 1)));
    ]

let span_to_range (span : span) =
  `Assoc
    [
      ("start", lsp_position_of_location span.start_pos);
      ("end", lsp_position_of_location span.end_pos);
    ]

let span_to_object (span : span) =
  `Assoc
    [
      ("file", `String span.start_pos.filename);
      ("start_line", `Int (max 0 (span.start_pos.line - 1)));
      ("start_col", `Int (max 0 (span.start_pos.column - 1)));
      ("end_line", `Int (max 0 (span.end_pos.line - 1)));
      ("end_col", `Int (max 0 (span.end_pos.column - 1)));
    ]

let string_of_severity_hint = function
  | Rollback -> "rollback"
  | Retry -> "retry"
  | Ignore -> "ignore"
  | Escalate -> "escalate"

let string_of_domain = function
  | Parser -> "parser"
  | Type -> "type"
  | Config -> "config"
  | Runtime -> "runtime"
  | Network -> "network"
  | Data -> "data"
  | Audit -> "audit"
  | Security -> "security"
  | CLI -> "cli"

let encode_codes = function
  | [] -> `Null
  | codes -> `List (List.map (fun code -> `String code) codes)

let encode_expectation = function
  | None -> `Null
  | Some summary ->
      let base =
        [
          ("message_key", match summary.message_key with Some key -> `String key | None -> `Null);
          ("humanized", match summary.humanized with Some msg -> `String msg | None -> `Null);
          ("context_note", match summary.context_note with Some note -> `String note | None -> `Null);
          ( "alternatives",
            `List
              (List.map
                 (function
                   | Token s -> `Assoc [ ("kind", `String "token"); ("value", `String s) ]
                   | Keyword s -> `Assoc [ ("kind", `String "keyword"); ("value", `String s) ]
                   | Rule s -> `Assoc [ ("kind", `String "rule"); ("value", `String s) ]
                   | Eof -> `Assoc [ ("kind", `String "eof") ]
                   | Not s -> `Assoc [ ("kind", `String "not"); ("value", `String s) ]
                   | Class s -> `Assoc [ ("kind", `String "class"); ("value", `String s) ]
                   | Custom s -> `Assoc [ ("kind", `String "custom"); ("value", `String s) ]
                   | TypeExpected s -> `Assoc [ ("kind", `String "type"); ("value", `String s) ]
                   | TraitBound s -> `Assoc [ ("kind", `String "trait"); ("value", `String s) ])
                 summary.alternatives) );
          ("locale_args", `List (List.map (fun arg -> `String arg) summary.locale_args));
        ]
      in
      `Assoc base

let encode_extensions extensions = Extensions.to_json extensions

let encode_secondary secondary =
  `List
    (List.map
       (fun (label : span_label) ->
         let fields =
           match label.span with
           | None -> []
           | Some span -> [ ("span", span_to_object span) ]
         in
         let fields =
           match label.message with
           | None -> fields
           | Some msg -> ("message", `String msg) :: fields
         in
         `Assoc fields)
       secondary)

let encode_hints hints =
  `List (List.map Diagnostic.V2.hint_to_json hints)

let extract_structured_hints_from_extensions extensions =
  match Extensions.get "diagnostic.v2" extensions with
  | Some (`Assoc fields) -> (
      match List.find_opt (fun (key, _) -> String.equal key "structured_hints") fields with
      | Some (_, (`List _ as value)) -> value
      | _ -> `List [])
  | _ -> `List []

let encode_audit audit = Diagnostic.V2.audit_to_json audit

let encode_audit_metadata metadata = Extensions.to_json metadata

let severity_to_lsp (diag : t) =
  Diagnostic.V2.severity_of_diagnostic diag |> Diagnostic.V2.severity_to_lsp_int

let primary_code = function [] -> None | code :: _ -> Some code

let diagnostic_data_block (diag : t) =
  let fields = ref [] in
  (match diag.id with Some id -> fields := ("id", `String id) :: !fields | None -> ());
  fields :=
    ("codes", encode_codes diag.codes) :: !fields;
  (match diag.domain with
  | Some domain -> fields := ("domain", `String (string_of_domain domain)) :: !fields
  | None -> ());
  (match diag.severity_hint with
  | Some hint ->
      fields := ("severity_hint", `String (string_of_severity_hint hint)) :: !fields
  | None -> ());
  fields := ("extensions", encode_extensions diag.extensions) :: !fields;
  fields := ("audit_metadata", encode_audit_metadata diag.audit_metadata) :: !fields;
  fields := ("timestamp", match diag.timestamp with Some ts -> `String ts | None -> `Null) :: !fields;
  fields := ("audit", encode_audit diag.audit) :: !fields;
  fields := ("schema_version", `String schema_version) :: !fields;
  fields := ("hints", encode_hints diag.hints) :: !fields;
  fields := ("secondary", encode_secondary diag.secondary) :: !fields;
  fields := ("expected", encode_expectation diag.expected) :: !fields;
  fields := ("primary", span_to_object diag.primary) :: !fields;
  fields := ("severity", `Int (severity_to_lsp diag)) :: !fields;
  fields := ("message", `String diag.message) :: !fields;
  fields := ("structured_hints", extract_structured_hints_from_extensions diag.extensions) :: !fields;
  `Assoc (List.rev !fields)

let to_v2_json (diag : t) =
  let base =
    [
      ("schema_version", `String schema_version);
      ("id", (match diag.id with Some id -> `String id | None -> `Null));
      ("message", `String diag.message);
      ("severity", `Int (severity_to_lsp diag));
      ("domain", (match diag.domain with Some d -> `String (string_of_domain d) | None -> `Null));
      ("codes", encode_codes diag.codes);
      ("primary", span_to_object diag.primary);
      ("secondary", encode_secondary diag.secondary);
      ("expected", encode_expectation diag.expected);
      ("hints", encode_hints diag.hints);
      ("structured_hints", extract_structured_hints_from_extensions diag.extensions);
      ("extensions", encode_extensions diag.extensions);
      ("audit_metadata", encode_audit_metadata diag.audit_metadata);
      ("audit", encode_audit diag.audit);
      ("timestamp", (match diag.timestamp with Some ts -> `String ts | None -> `Null));
    ]
  in
  `Assoc base

let related_information_of_secondary (secondary : span_label list) =
  secondary
  |> List.filter_map (fun (label : span_label) ->
         match (label.span, label.message) with
         | Some span, Some msg ->
             Some
               (`Assoc
                  [
                    ("location", `Assoc [ ("uri", `String (lsp_uri_of_filename span.start_pos.filename)); ("range", span_to_range span) ]);
                    ("message", `String msg);
                  ])
         | _ -> None)

let diagnostic_to_lsp_json (diag : t) =
  let base =
    [
      ("range", span_to_range diag.primary);
      ("severity", `Int (severity_to_lsp diag));
      ("message", `String diag.message);
      ("source", `String lsp_source);
      ("data", diagnostic_data_block diag);
    ]
  in
  let base =
    match primary_code diag.codes with
    | Some code -> ("code", `String code) :: base
    | None -> base
  in
  let base =
    match related_information_of_secondary diag.secondary with
    | [] -> base
    | infos -> ("relatedInformation", `List infos) :: base
  in
  `Assoc base

type publish_params = {
  uri : string;
  version : int option;
  diagnostics : t list;
}

let publish_params ?version ~uri diagnostics = { uri; version; diagnostics }

let publish_notification_to_json (params : publish_params) =
  let diagnostics =
    params.diagnostics |> List.map diagnostic_to_lsp_json |> fun list -> `List list
  in
  let fields = [ ("uri", `String params.uri); ("diagnostics", diagnostics) ] in
  let fields =
    match params.version with Some value -> ("version", `Int value) :: fields | None -> fields
  in
  `Assoc fields

let diagnostics_to_v2_json diagnostics =
  `List (List.map to_v2_json diagnostics)
