(** lsp_transport — Phase 2-4 LSP 診断トランスポート層

    `diagnostic_serialization` で正規化した診断を LSP PublishDiagnostics
    互換の JSON へ変換する。V1/V2 の切替と JSON-RPC 実装（`jsonrpc_server.ml`）
    からの再利用を前提とした API を提供する。 *)

type transport_version =
  | V1
  | V2

type publish_params = {
  uri : string;
  version : int option;
  diagnostics : Diagnostic_serialization.normalized_diagnostic list;
}

val encode_publish_diagnostics :
  version:transport_version ->
  publish_params ->
  Yojson.Basic.t
(** `PublishDiagnostics` 通知を JSON へ変換する。 *)

val diagnostics_payload :
  version:transport_version ->
  Diagnostic_serialization.normalized_diagnostic list ->
  Yojson.Basic.t
(** CLI からも利用できる汎用 JSON ペイロード。 *)

val run_config_from_file : string -> Parser_run_config.t
(** LSP 設定ファイルから RunConfig を復元する。 *)

val default_run_config : unit -> Parser_run_config.t
(** `tooling/lsp/config/default.json` に基づく既定 RunConfig。 *)
