(* Diagnostic_formatter — ソースコードスニペット表示インターフェース
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、診断メッセージに
 * ソースコードのコンテキストとポインタを表示する機能を提供する。
 *)

val format_snippet :
  source:string ->
  span:Diagnostic.span ->
  color_mode:Options.color_mode ->
  severity:Diagnostic.severity ->
  string
(** ソースコードスニペットを抽出して表示
 *
 * エラー位置の前後の行を含めたソースコードスニペットを生成し、
 * エラー箇所にポインタ（^^^）を表示する。
 *
 * 表示形式:
 * ```
 *    1 | fn add(a: i64, b: String) -> i64 = a + b
 *      |                  ^^^^^^
 * ```
 *
 * @param source ソースコード文字列
 * @param span エラー位置情報
 * @param color_mode カラーモード
 * @param severity 診断の重要度（ポインタの色付けに使用）
 * @return フォーマットされたスニペット文字列
 *)

val format_diagnostic :
  source:string option ->
  diag:Diagnostic.t ->
  color_mode:Options.color_mode ->
  include_snippet:bool ->
  string
(** 診断全体をテキスト形式で出力
 *
 * 診断メッセージのヘッダー、ソースコードスニペット、期待値、ノート、
 * 修正提案、重要度ヒントを含む完全な診断文字列を生成する。
 *
 * @param source ソースコード文字列（オプション。None の場合はスニペットを省略）
 * @param diag 診断情報
 * @param color_mode カラーモード
 * @return フォーマットされた診断文字列
 *)

val format_diagnostics :
  source:string option ->
  diags:Diagnostic.t list ->
  color_mode:Options.color_mode ->
  include_snippet:bool ->
  string
(** 複数の診断をバッチ出力
 *
 * 複数の診断を改行で区切って出力する。
 *
 * @param source ソースコード文字列（オプション）
 * @param diags 診断情報のリスト
 * @param color_mode カラーモード
 * @return フォーマットされた診断文字列（改行区切り）
 *)
