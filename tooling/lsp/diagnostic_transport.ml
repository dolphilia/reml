open Diagnostic_serialization

module Json = Yojson.Basic

type publish_params = {
  uri : string;
  version : int option;
  diagnostics : Diagnostic.t list;
}

let publish_params ?version ~uri diagnostics = { uri; version; diagnostics }

let publish_notification_to_json (params : publish_params) =
  let normalized = List.map of_diagnostic params.diagnostics in
  Lsp_transport.encode_publish_diagnostics ~version:Lsp_transport.V2
    { uri = params.uri; version = params.version; diagnostics = normalized }

let diagnostics_to_v2_json diagnostics =
  `List (List.map diagnostic_to_json diagnostics)
