open Diagnostic_serialization
module D = Diagnostic

module Json = Yojson.Basic

let schema_version = Diagnostic.schema_version
let lsp_source = "reml"

let lsp_uri_of_filename filename =
  if filename = "" || filename = "<入力>" then "reml://embedded"
  else if String.length filename >= 7 && String.sub filename 0 7 = "file://" then
    filename
  else
    "file://" ^ filename

let lsp_position line column =
  `Assoc [ ("line", `Int line); ("character", `Int column) ]

let lsp_range (span : normalized_span) =
  `Assoc
    [
      ("start", lsp_position span.start_line span.start_col);
      ("end", lsp_position span.end_line span.end_col);
    ]

let reml_location (span : normalized_span) =
  `Assoc
    [
      ("file", `String span.file);
      ("line", `Int (span.start_line + 1));
      ("column", `Int (span.start_col + 1));
      ("endLine", `Int (span.end_line + 1));
      ("endColumn", `Int (span.end_col + 1));
    ]

let structured_hints_from_extensions (extensions : Diagnostic.Extensions.t) =
  match List.assoc_opt "diagnostic.v2" extensions with
  | Some (`Assoc fields) -> (
      match List.assoc_opt "structured_hints" fields with
      | Some (`List _ as value) -> value
      | _ -> `List [] )
  | _ -> `List []

let audit_to_json envelope =
  let base =
    [
      ("metadata", Audit_envelope.metadata_to_json (Audit_envelope.metadata envelope));
    ]
  in
  let base =
    match Audit_envelope.audit_id envelope with
    | Some id when String.trim id <> "" -> ("audit_id", `String id) :: base
    | _ -> base
  in
  let base =
    match Audit_envelope.change_set envelope with
    | Some change -> ("change_set", change) :: base
    | None -> base
  in
  let base =
    match Audit_envelope.capability envelope with
    | Some cap when String.trim cap <> "" -> ("capability", `String cap) :: base
    | _ -> base
  in
  `Assoc (List.rev base)

let codes_to_json codes =
  match codes with
  | [] -> `Null
  | xs -> `List (List.map (fun code -> `String code) xs)

let reml_notes (secondary : normalized_secondary list) =
  let rec collect acc = function
    | [] -> List.rev acc
    | (entry : normalized_secondary) :: rest -> (
        match entry.message with
        | Some msg -> collect (`String msg :: acc) rest
        | None -> collect acc rest)
  in
  collect [] secondary

let reml_expected (expected : D.expectation_summary option) =
  expectation_summary_to_json expected

let diagnostic_data_block (diag : normalized_diagnostic) =
  let base =
    [
      ("schema_version", `String schema_version);
      ("message", `String diag.message);
      ("severity", `Int (severity_level_of_severity diag.severity));
      ("severity_label", `String (severity_to_string diag.severity));
      ("primary", span_to_json diag.primary);
      ("codes", codes_to_json diag.codes);
      ("secondary", `List (List.map secondary_to_json diag.secondary));
      ("hints", `List (List.map hint_to_json diag.hints));
      ("fixits", `List (List.map fixit_to_json diag.fixits));
      ("extensions", Diagnostic.Extensions.to_json diag.extensions);
      ("structured_hints", structured_hints_from_extensions diag.extensions);
      ("audit_metadata", Diagnostic.Extensions.to_json diag.audit_metadata);
      ("audit", audit_to_json diag.audit);
      ("expected", expectation_summary_to_json diag.expected);
    ]
  in
  let base =
    match diag.id with Some id -> ("id", `String id) :: base | None -> base
  in
  let base =
    match domain_to_string diag.domain with
    | Some domain -> ("domain", `String domain) :: base
    | None -> base
  in
  let base =
    match diag.severity_hint with
    | Some hint -> ("severity_hint", `String (severity_hint_to_string hint)) :: base
    | None -> base
  in
  let base = ("timestamp", `String diag.timestamp) :: base in
  `Assoc (List.rev base)

let diagnostic_to_lsp_json (normalized : normalized_diagnostic) =
  let primary = normalized.primary in
  let range = lsp_range primary in
  let severity = severity_level_of_severity normalized.severity in
  let data = diagnostic_data_block normalized in
  let base =
    [
      ("range", range);
      ("severity", `Int severity);
      ("message", `String normalized.message);
      ("source", `String lsp_source);
      ("data", data);
    ]
  in
  let base =
    match normalized.codes with
    | code :: _ -> ("code", `String code) :: base
    | [] -> base
  in
  let related =
    normalized.secondary
    |> List.filter_map (fun entry ->
           match (entry.span, entry.message) with
           | Some span, Some message ->
               Some
                 (`Assoc
                    [
                      ( "location",
                        `Assoc
                          [
                            ("uri", `String (lsp_uri_of_filename span.file));
                            ("range", lsp_range span);
                          ] );
                      ("message", `String message);
                    ])
           | _ -> None)
  in
  let base =
    match related with
    | [] -> base
    | infos -> ("relatedInformation", `List infos) :: base
  in
  `Assoc (List.rev base)

let diagnostic_to_reml_json (normalized : normalized_diagnostic) =
  let fields = ref [] in
  let push key value = fields := (key, value) :: !fields in
  push "severity" (`String (severity_to_string normalized.severity));
  push "message" (`String normalized.message);
  push "schema_version" (`String schema_version);
  push "location" (reml_location normalized.primary);
  (match normalized.codes with
  | code :: _ -> push "code" (`String code) | [] -> ());
  if normalized.codes <> [] then
    push "codes" (`List (List.map (fun code -> `String code) normalized.codes));
  (match domain_to_string normalized.domain with
  | Some domain -> push "domain" (`String domain)
  | None -> ());
  (match normalized.id with Some id -> push "id" (`String id) | None -> ());
  push "timestamp" (`String normalized.timestamp);
  (match normalized.severity_hint with
  | Some hint -> push "severity_hint" (`String (severity_hint_to_string hint))
  | None -> ());
  let notes = reml_notes normalized.secondary in
  if notes <> [] then push "notes" (`List notes);
  (match normalized.hints with
  | [] -> ()
  | hints -> push "hints" (`List (List.map hint_to_json hints)));
  (match normalized.fixits with
  | [] -> ()
  | fixits -> push "fixits" (`List (List.map fixit_to_json fixits)));
  (match structured_hints_from_extensions normalized.extensions with
  | `List [] -> ()
  | payload -> push "structured_hints" payload);
  if not (Diagnostic.Extensions.is_empty normalized.extensions) then
    push "extensions" (Diagnostic.Extensions.to_json normalized.extensions);
  if not (Diagnostic.Extensions.is_empty normalized.audit_metadata) then
    push "audit_metadata"
      (Diagnostic.Extensions.to_json normalized.audit_metadata);
  let audit_json = audit_to_json normalized.audit in
  push "audit" audit_json;
  (match reml_expected normalized.expected with
  | `Null -> ()
  | expected -> push "expected" expected);
  `Assoc (List.rev !fields)

let encode_diagnostics ~lsp_compatible diagnostics =
  if lsp_compatible then
    List.map diagnostic_to_lsp_json diagnostics
  else
    List.map diagnostic_to_reml_json diagnostics

let diagnostics_to_json_serialized ~mode ?(lsp_compatible = false)
    (diagnostics : normalized_diagnostic list) : string =
  let json_list = encode_diagnostics ~lsp_compatible diagnostics in
  match mode with
  | Options.JsonPretty ->
      Json.pretty_to_string (`Assoc [ ("diagnostics", `List json_list) ])
  | Options.JsonCompact ->
      Json.to_string (`Assoc [ ("diagnostics", `List json_list) ])
  | Options.JsonLines ->
      json_list |> List.map Json.to_string |> String.concat "\n"

let diagnostic_to_json_serialized ~mode ?(lsp_compatible = false)
    (diagnostic : normalized_diagnostic) : string =
  match mode with
  | Options.JsonLines ->
      encode_diagnostics ~lsp_compatible [ diagnostic ]
      |> List.map Json.to_string |> String.concat "\n"
  | _ -> diagnostics_to_json_serialized ~mode ~lsp_compatible [ diagnostic ]

let diagnostics_to_json ~mode ?(lsp_compatible = false) diagnostics =
  diagnostics
  |> List.map of_diagnostic
  |> diagnostics_to_json_serialized ~mode ~lsp_compatible

let diagnostic_to_json ~mode ?(lsp_compatible = false) diagnostic =
  let serialized = of_diagnostic diagnostic in
  diagnostic_to_json_serialized ~mode ~lsp_compatible serialized
