open Diagnostic_serialization

module Json = Yojson.Basic

let publish_diagnostics_notification
    ?version
    ?(transport = Lsp_transport.V2)
    ~uri
    (diagnostics : Diagnostic.t list) =
  let normalized = List.map of_diagnostic diagnostics in
  let params =
    Lsp_transport.encode_publish_diagnostics ~version:transport
      Lsp_transport.{ uri; version; diagnostics = normalized }
  in
  `Assoc
    [
      ("jsonrpc", `String "2.0");
      ("method", `String "textDocument/publishDiagnostics");
      ("params", params);
    ]

let diagnostics_payload ?(transport = Lsp_transport.V2) diagnostics =
  let normalized = List.map of_diagnostic diagnostics in
  Lsp_transport.diagnostics_payload ~version:transport normalized

let write_notification out_channel notification =
  output_string out_channel (Json.to_string notification);
  output_char out_channel '\n';
  flush out_channel
