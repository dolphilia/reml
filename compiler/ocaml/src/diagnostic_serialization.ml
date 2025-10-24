open Diagnostic

module Json = Yojson.Basic

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
  audit : Audit_envelope.t option;
  timestamp : string option;
}

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
  {
    id = diag.id;
    message = diag.message;
    severity = diag.severity;
    severity_hint = diag.severity_hint;
    domain = diag.domain;
    codes = diag.codes;
    primary = normalize_span diag.primary;
    secondary = List.map normalize_secondary diag.secondary;
    hints = List.map normalize_hint diag.hints;
    fixits = List.map normalize_fixit diag.fixits;
    expected = diag.expected;
    schema_version = Diagnostic.schema_version;
    extensions = diag.extensions;
    audit_metadata = diag.audit_metadata;
    audit = diag.audit;
    timestamp = diag.timestamp;
  }

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
  | Note -> "note"

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
        | Parser -> "parser"
        | Type -> "type"
        | Config -> "config"
        | Runtime -> "runtime"
        | Network -> "network"
        | Data -> "data"
        | Audit -> "audit"
        | Security -> "security"
        | CLI -> "cli"
      in
      Some label

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

let encode_audit = function
  | None -> `Null
  | Some env ->
      let fields = [ ("metadata", Audit_envelope.metadata_to_json (Audit_envelope.metadata env)) ] in
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
  | Note -> 3

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
  let fields =
    match diag.timestamp with
    | Some ts -> ("timestamp", `String ts) :: fields
    | None -> fields
  in
  `Assoc (List.rev fields)

let diagnostic_to_json diag = to_json (of_diagnostic diag)

let diagnostics_to_json diagnostics =
  List.map diagnostic_to_json diagnostics
