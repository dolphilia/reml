open Diagnostic

module Json = Yojson.Basic

let other_domain_extension_key = "domain.other"

type normalized_span = {
  file : string;
  start_line : int;
  start_col : int;
  end_line : int;
  end_col : int;
}

type normalized_secondary = {
  span : normalized_span option;
  message : string option;
}

type normalized_fixit =
  | Insert of { range : normalized_span; text : string }
  | Replace of { range : normalized_span; text : string }
  | Delete of { range : normalized_span }

type normalized_hint = {
  message : string option;
  actions : normalized_fixit list;
}

let normalize_domain_and_extensions domain extensions =
  match domain with
  | None -> (None, Extensions.remove other_domain_extension_key extensions)
  | Some (Other raw) ->
      let sanitized = Domain.sanitize_other raw in
      let normalized_domain = Some (Other sanitized) in
      let updated_extensions =
        Extensions.set other_domain_extension_key (`String sanitized) extensions
      in
      (normalized_domain, updated_extensions)
  | Some value ->
      (Some value, Extensions.remove other_domain_extension_key extensions)

type normalized_diagnostic = {
  id : string option;
  message : string;
  severity : severity;
  severity_hint : severity_hint option;
  domain : error_domain option;
  codes : string list;
  primary : normalized_span;
  secondary : normalized_secondary list;
  hints : normalized_hint list;
  fixits : normalized_fixit list;
  expected : expectation_summary option;
  schema_version : string;
  extensions : Extensions.t;
  audit_metadata : Extensions.t;
  audit : Audit_envelope.t;
  timestamp : string;
}

let assoc_find key fields =
  List.find_map
    (fun (candidate, value) ->
      if String.equal candidate key then Some value else None)
    fields

let streaming_enabled_in_extensions extensions =
  match Extensions.get "runconfig" extensions with
  | Some (`Assoc run_fields) -> (
      match assoc_find "extensions" run_fields with
      | Some (`Assoc ext_fields) -> (
          match assoc_find "stream" ext_fields with
          | Some (`Assoc stream_fields) -> (
              match assoc_find "enabled" stream_fields with
              | Some (`Bool flag) -> flag
              | _ -> false)
          | _ -> false)
      | _ -> false)
  | _ -> false

let has_parse_extension extensions =
  match Extensions.get "parse" extensions with
  | Some (`Assoc fields) -> (
      match assoc_find "parser_id" fields with
      | Some (`Assoc id_fields) -> assoc_find "namespace" id_fields <> None
      | _ -> false)
  | _ -> false

let has_parser_core_rule metadata =
  match Extensions.get "parser.core.rule" metadata with
  | Some (`Assoc fields) -> fields <> []
  | _ -> false

let streaming_placeholder_namespace = "core.parse.streaming"
let streaming_placeholder_name = "streaming_recover"
let streaming_placeholder_origin = "synthetic"
let streaming_placeholder_fingerprint = "streaming-recover-placeholder"

let parser_id_placeholder () =
  `Assoc
    [
      ("namespace", `String streaming_placeholder_namespace);
      ("name", `String streaming_placeholder_name);
      ("ordinal", `Int 0);
      ("origin", `String streaming_placeholder_origin);
      ("fingerprint", `String streaming_placeholder_fingerprint);
    ]

let parser_rule_placeholder () =
  `Assoc
    [
      ("namespace", `String streaming_placeholder_namespace);
      ("name", `String streaming_placeholder_name);
      ("ordinal", `Int 0);
      ("origin", `String streaming_placeholder_origin);
      ("fingerprint", `String streaming_placeholder_fingerprint);
    ]

let ensure_streaming_parser_metadata
    (diag : normalized_diagnostic) : normalized_diagnostic =
  if not (streaming_enabled_in_extensions diag.extensions) then diag
  else
    let extensions =
      if has_parse_extension diag.extensions then diag.extensions
      else
        let parse_value = `Assoc [ ("parser_id", parser_id_placeholder ()) ] in
        Extensions.set "parse" parse_value diag.extensions
    in
    let needs_audit_update = not (has_parser_core_rule diag.audit_metadata) in
    let audit_metadata =
      if not needs_audit_update then diag.audit_metadata
      else
        diag.audit_metadata
        |> Extensions.set "parser.core.rule" (parser_rule_placeholder ())
        |> Extensions.set "parser.core.rule.namespace"
             (`String streaming_placeholder_namespace)
        |> Extensions.set "parser.core.rule.name"
             (`String streaming_placeholder_name)
        |> Extensions.set "parser.core.rule.ordinal" (`Int 0)
        |> Extensions.set "parser.core.rule.origin"
             (`String streaming_placeholder_origin)
        |> Extensions.set "parser.core.rule.fingerprint"
             (`String streaming_placeholder_fingerprint)
    in
    let audit =
      if needs_audit_update then
        Audit_envelope.merge_metadata diag.audit audit_metadata
      else diag.audit
    in
    { diag with extensions; audit_metadata; audit }

let normalize_span (span : span) =
  {
    file = span.start_pos.filename;
    start_line = max 0 (span.start_pos.line - 1);
    start_col = max 0 (span.start_pos.column - 1);
    end_line = max 0 (span.end_pos.line - 1);
    end_col = max 0 (span.end_pos.column - 1);
  }

let normalize_secondary ({ span; message } : span_label) =
  let span =
    match span with Some value -> Some (normalize_span value) | None -> None
  in
  { span; message }

let normalize_fixit = function
  | Diagnostic.Insert { at; text } ->
      Insert { range = normalize_span at; text }
  | Diagnostic.Replace { at; text } ->
      Replace { range = normalize_span at; text }
  | Diagnostic.Delete { at } -> Delete { range = normalize_span at }

let normalize_hint ({ message; actions } : hint) =
  { message; actions = List.map normalize_fixit actions }

let of_diagnostic (diag : Diagnostic.t) : normalized_diagnostic =
  let missing_audit = Audit_envelope.missing_required_keys diag.audit in
  let domain, extensions =
    normalize_domain_and_extensions diag.domain diag.extensions
  in
  if missing_audit <> [] then (
    let message =
      Printf.sprintf "Diagnostic.audit に必須メタデータが不足しています: %s"
        (String.concat ", " missing_audit)
    in
    prerr_endline ("[diagnostic_serialization] " ^ message);
    invalid_arg message);
  if String.trim diag.timestamp = "" then (
    let message = "Diagnostic.timestamp が空です" in
    prerr_endline ("[diagnostic_serialization] " ^ message);
    invalid_arg message);
  let normalized =
    {
      id = diag.id;
      message = diag.message;
      severity = diag.severity;
      severity_hint = diag.severity_hint;
      domain;
      codes = diag.codes;
      primary = normalize_span diag.primary;
      secondary = List.map normalize_secondary diag.secondary;
      hints = List.map normalize_hint diag.hints;
      fixits = List.map normalize_fixit diag.fixits;
      expected = diag.expected;
      schema_version = Diagnostic.schema_version;
      extensions;
      audit_metadata = diag.audit_metadata;
      audit = diag.audit;
      timestamp = diag.timestamp;
    }
  in
  ensure_streaming_parser_metadata normalized

let span_to_json (span : normalized_span) =
  `Assoc
    [
      ("file", `String span.file);
      ("start_line", `Int span.start_line);
      ("start_col", `Int span.start_col);
      ("end_line", `Int span.end_line);
      ("end_col", `Int span.end_col);
    ]

let severity_to_string = function
  | Error -> "error"
  | Warning -> "warning"
  | Info -> "info"
  | Hint -> "hint"

let severity_hint_to_string = function
  | Rollback -> "rollback"
  | Retry -> "retry"
  | Ignore -> "ignore"
  | Escalate -> "escalate"
let domain_to_string = function
  | None -> None
  | Some domain ->
      let label =
        match domain with
        | Effect -> "effect"
        | Target -> "target"
        | Plugin -> "plugin"
        | Lsp -> "lsp"
        | Parser -> "parser"
        | Type -> "type"
        | Config -> "config"
        | Runtime -> "runtime"
        | Network -> "network"
        | Data -> "data"
        | Audit -> "audit"
        | Security -> "security"
        | Cli -> "cli"
        | Other _ -> "other"
      in
      Some label

let[@warning "-32"] domain_of_json ?extensions json =
  match json with
  | `Null -> None
  | `String value ->
      let trimmed = String.trim value in
      if String.equal trimmed "" then None
      else
        let normalized = String.lowercase_ascii trimmed in
        let to_other label =
          let sanitized = Domain.sanitize_other label in
          Some (Other sanitized)
        in
        (match normalized with
        | "effect" -> Some Effect
        | "target" -> Some Target
        | "plugin" -> Some Plugin
        | "lsp" -> Some Lsp
        | "parser" -> Some Parser
        | "type" -> Some Type
        | "config" -> Some Config
        | "runtime" -> Some Runtime
        | "network" -> Some Network
        | "data" -> Some Data
        | "audit" -> Some Audit
        | "security" -> Some Security
        | "cli" -> Some Cli
        | "other" -> (
            match extensions with
            | Some ext -> (
                match Extensions.get other_domain_extension_key ext with
                | Some (`String label) when String.trim label <> "" ->
                    to_other label
                | _ -> to_other trimmed)
            | None -> to_other trimmed)
        | _ -> to_other trimmed)
  | _ -> None

let expectation_to_json : expectation -> Json.t = function
  | Token s -> `Assoc [ ("kind", `String "token"); ("value", `String s) ]
  | Keyword s -> `Assoc [ ("kind", `String "keyword"); ("value", `String s) ]
  | Rule s -> `Assoc [ ("kind", `String "rule"); ("value", `String s) ]
  | Eof -> `Assoc [ ("kind", `String "eof") ]
  | Not s -> `Assoc [ ("kind", `String "not"); ("value", `String s) ]
  | Class s -> `Assoc [ ("kind", `String "class"); ("value", `String s) ]
  | Custom s -> `Assoc [ ("kind", `String "custom"); ("value", `String s) ]
  | TypeExpected s -> `Assoc [ ("kind", `String "type"); ("value", `String s) ]
  | TraitBound s -> `Assoc [ ("kind", `String "trait"); ("value", `String s) ]

let expectation_summary_to_json = function
  | None -> `Null
  | Some summary ->
      let fields =
        [
          ( "alternatives",
            `List (List.map expectation_to_json summary.alternatives) );
        ]
      in
      let fields =
        match summary.message_key with
        | Some key -> ("message_key", `String key) :: fields
        | None -> fields
      in
      let fields =
        match summary.humanized with
        | Some text -> ("humanized", `String text) :: fields
        | None -> fields
      in
      let fields =
        match summary.context_note with
        | Some text -> ("context_note", `String text) :: fields
        | None -> fields
      in
      let fields =
        match summary.locale_args with
        | [] -> fields
        | args ->
            ("locale_args", `List (List.map (fun arg -> `String arg) args))
            :: fields
      in
      `Assoc fields

let fixit_to_json = function
  | Insert { range; text } ->
      `Assoc
        [
          ("kind", `String "insert");
          ("range", span_to_json range);
          ("text", `String text);
        ]
  | Replace { range; text } ->
      `Assoc
        [
          ("kind", `String "replace");
          ("range", span_to_json range);
          ("text", `String text);
        ]
  | Delete { range } ->
      `Assoc [ ("kind", `String "delete"); ("range", span_to_json range) ]

let hint_to_json { message; actions } =
  let base =
    match message with Some msg -> [ ("message", `String msg) ] | None -> []
  in
  let base =
    match actions with
    | [] -> base
    | xs -> ("actions", `List (List.map fixit_to_json xs)) :: base
  in
  `Assoc (List.rev base)

let secondary_to_json { span; message } =
  let base =
    match span with Some s -> [ ("span", span_to_json s) ] | None -> []
  in
  let base =
    match message with Some msg -> ("message", `String msg) :: base | None -> base
  in
  `Assoc (List.rev base)

let encode_extensions entries = Extensions.to_json entries

let encode_audit_metadata entries = Extensions.to_json entries

let encode_audit env =
  let fields =
    [
      ( "metadata",
        Audit_envelope.metadata_to_json (Audit_envelope.metadata env) );
    ]
  in
  let fields =
    match Audit_envelope.audit_id env with
    | Some id when String.trim id <> "" -> ("audit_id", `String id) :: fields
    | _ -> fields
  in
  let fields =
    match Audit_envelope.change_set env with
    | Some change -> ("change_set", change) :: fields
    | None -> fields
  in
  let fields =
    match Audit_envelope.capability env with
    | Some cap when String.trim cap <> "" -> ("capability", `String cap) :: fields
    | _ -> fields
  in
  `Assoc (List.rev fields)

let severity_level_of_severity = function
  | Error -> 1
  | Warning -> 2
  | Info -> 3
  | Hint -> 4

let to_json (diag : normalized_diagnostic) : Json.t =
  let fields =
    [
      ("message", `String diag.message);
      ("severity", `String (severity_to_string diag.severity));
      ("severity_level", `Int (severity_level_of_severity diag.severity));
      ("primary", span_to_json diag.primary);
      ("codes", `List (List.map (fun code -> `String code) diag.codes));
      ("secondary", `List (List.map secondary_to_json diag.secondary));
      ("hints", `List (List.map hint_to_json diag.hints));
      ("fixits", `List (List.map fixit_to_json diag.fixits));
      ("schema_version", `String diag.schema_version);
      ("extensions", encode_extensions diag.extensions);
      ("audit_metadata", encode_audit_metadata diag.audit_metadata);
      ("audit", encode_audit diag.audit);
      ("expected", expectation_summary_to_json diag.expected);
    ]
  in
  let fields =
    match diag.id with Some id -> ("id", `String id) :: fields | None -> fields
  in
  let fields =
    match domain_to_string diag.domain with
    | Some domain -> ("domain", `String domain) :: fields
    | None -> fields
  in
  let fields =
    match diag.severity_hint with
    | Some hint -> ("severity_hint", `String (severity_hint_to_string hint)) :: fields
    | None -> fields
  in
  let fields = ("timestamp", `String diag.timestamp) :: fields in
  `Assoc (List.rev fields)

let diagnostic_to_json diag = to_json (of_diagnostic diag)

let diagnostics_to_json diagnostics =
  List.map diagnostic_to_json diagnostics
