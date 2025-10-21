open Diagnostic_serialization

module Json = Yojson.Basic

type transport_version =
  | V1
  | V2

type publish_params = {
  uri : string;
  version : int option;
  diagnostics : normalized_diagnostic list;
}

let lsp_position line column =
  `Assoc [ ("line", `Int line); ("character", `Int column) ]

let lsp_range span =
  `Assoc
    [
      ("start", lsp_position span.start_line span.start_col);
      ("end", lsp_position span.end_line span.end_col);
    ]

let structured_hints_from_extensions extensions =
  match List.assoc_opt "diagnostic.v2" extensions with
  | Some (`Assoc fields) -> (
      match List.assoc_opt "structured_hints" fields with
      | Some (`List _ as value) -> value
      | _ -> `List [] )
  | _ -> `List []

let audit_to_json = function
  | None -> `Null
  | Some envelope ->
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

let data_block diag =
  let base =
    [
      ("schema_version", `String "2.0.0-draft");
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
  let base =
    match diag.timestamp with
    | Some ts -> ("timestamp", `String ts) :: base
    | None -> base
  in
  `Assoc (List.rev base)

let encode_v2_diagnostic (diag : normalized_diagnostic) =
  let base =
    [
      ("range", lsp_range diag.primary);
      ("severity", `Int (severity_level_of_severity diag.severity));
      ("message", `String diag.message);
      ("data", data_block diag);
    ]
  in
  let base =
    match diag.codes with
    | code :: _ -> ("code", `String code) :: base
    | [] -> base
  in
  let related =
    diag.secondary
    |> List.filter_map (fun entry ->
           match (entry.span, entry.message) with
           | Some span, Some message ->
               Some
                 (`Assoc
                    [
                      ( "location",
                        `Assoc
                          [
                            ("uri", `String span.file);
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

let encode_v1_diagnostic (diag : normalized_diagnostic) =
  Diagnostic_v1.to_v1_json diag

let diagnostics_payload ~version diagnostics =
  match version with
  | V2 -> `List (List.map encode_v2_diagnostic diagnostics)
  | V1 -> `List (List.map encode_v1_diagnostic diagnostics)

let encode_publish_diagnostics ~version (params : publish_params) =
  let diagnostics = diagnostics_payload ~version params.diagnostics in
  let fields = [ ("uri", `String params.uri); ("diagnostics", diagnostics) ] in
  let fields =
    match params.version with Some v -> ("version", `Int v) :: fields | None -> fields
  in
  `Assoc fields
