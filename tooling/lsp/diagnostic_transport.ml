open Diagnostic_serialization

module Json = Yojson.Basic

type publish_params = {
  uri : string;
  version : int option;
  diagnostics : Diagnostic.t list;
  stream_meta : Json.t option;
}

let publish_params ?version ?stream_meta ~uri diagnostics =
  { uri; version; diagnostics; stream_meta }

let publish_notification_to_json (params : publish_params) =
  let normalized = List.map of_diagnostic params.diagnostics in
  Lsp_transport.encode_publish_diagnostics ~version:Lsp_transport.V2
    {
      uri = params.uri;
      version = params.version;
      diagnostics = normalized;
      stream_meta = params.stream_meta;
    }

let diagnostics_to_v2_json ?stream_meta diagnostics =
  Lsp_transport.diagnostics_payload ~version:Lsp_transport.V2
    ?stream_meta (List.map of_diagnostic diagnostics)
