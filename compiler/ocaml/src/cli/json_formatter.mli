(* Json_formatter — LSP 互換 JSON 診断出力インターフェース
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、診断メッセージを
 * LSP（Language Server Protocol）互換の JSON 形式で出力する機能を提供する。
 *)

val diagnostics_to_json :
  mode:Options.json_mode ->
  ?lsp_compatible:bool ->
  Diagnostic.t list ->
  string
(** 複数の診断を JSON 配列に変換
 *
 * 出力形式（Reml 独自形式、デフォルト）:
 * ```json
 * {
 *   "diagnostics": [
 *     {
 *       "severity": "error",
 *       "code": "E7001",
 *       "message": "型が一致しません",
 *       "location": {
 *         "file": "/path/to/file.reml",
 *         "line": 1,
 *         "column": 18,
 *         "endLine": 1,
 *         "endColumn": 24
 *       },
 *       "notes": ["期待される型: i64", "実際の型: String"]
 *     }
 *   ]
 * }
 * ```
 *
 * LSP 互換形式（`lsp_compatible=true` の場合）:
 * ```json
 * {
 *   "diagnostics": [
 *     {
 *       "range": {
 *         "start": { "line": 0, "character": 17 },
 *         "end": { "line": 0, "character": 23 }
 *       },
 *       "severity": 1,
 *       "code": "E7001",
 *       "source": "remlc",
 *       "message": "型が一致しません",
 *       "relatedInformation": [...]
 *     }
 *   ]
 * }
 * ```
 *
 * @param diags 診断情報のリスト
 * @param lsp_compatible LSP 互換形式を使用するか（デフォルト: false）
 * @return JSON 文字列（整形済み）
 *)

val diagnostic_to_json :
  mode:Options.json_mode ->
  ?lsp_compatible:bool ->
  Diagnostic.t ->
  string
(** 単一の診断を JSON 文字列に変換
 *
 * @param diag 診断情報
 * @param lsp_compatible LSP 互換形式を使用するか（デフォルト: false）
 * @return JSON 文字列（整形済み）
 *)
