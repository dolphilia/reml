(* Color — ANSI カラー出力とカラーモード判定
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、診断メッセージのカラー出力を提供する。
 *
 * 設計原則:
 * - TTY 判定により自動的にカラーを有効化/無効化
 * - 環境変数（NO_COLOR, FORCE_COLOR）に対応
 * - ANSI エスケープシーケンスによる色付け
 *)

(** ANSI エスケープシーケンス定義 *)
module Ansi = struct
  let reset = "\027[0m"
  let bold = "\027[1m"

  (* 明るい前景色 *)
  let bright_red = "\027[91m"
  let bright_yellow = "\027[93m"
  let bright_blue = "\027[94m"
  let cyan = "\027[36m"
end

(** Unix.isatty のラッパー（Unix モジュールが利用可能な場合のみ使用） *)
let is_tty fd =
  try Unix.isatty fd with _ -> (* Unix モジュールが利用できない環境では false を返す *)
                               false

(** 環境変数から NO_COLOR が設定されているかチェック *)
let has_no_color () =
  try
    let _ = Sys.getenv "NO_COLOR" in
    true
  with Not_found -> false

(** 環境変数から FORCE_COLOR が設定されているかチェック *)
let has_force_color () =
  try
    let _ = Sys.getenv "FORCE_COLOR" in
    true
  with Not_found -> false

(** カラーモードを解決する
 *
 * @param requested ユーザーが指定したカラーモード（--color オプション）
 * @param is_tty 出力先が TTY かどうか
 * @return 実際に使用するカラーモード
 *)
let resolve_color_mode ~requested ~is_tty =
  (* NO_COLOR 環境変数が設定されている場合は常に無効 *)
  if has_no_color () then Options.Never
  else
    match requested with
    | Options.Auto ->
        (* Auto の場合は TTY かどうかで判定 *)
        (* ただし FORCE_COLOR が設定されている場合は強制的に有効 *)
        if has_force_color () then Options.Always
        else if is_tty then Options.Always
        else Options.Never
    | Options.Always -> Options.Always
    | Options.Never -> Options.Never

(** カラーを適用する
 *
 * @param mode カラーモード
 * @param code ANSI カラーコード
 * @param text 色付けするテキスト
 * @return 色付けされたテキスト（または元のテキスト）
 *)
let apply_color mode code text =
  match mode with
  | Options.Never -> text
  | Options.Always | Options.Auto -> code ^ text ^ Ansi.reset

(** 重要度に応じた色を適用する
 *
 * @param mode カラーモード
 * @param severity 診断の重要度
 * @param text 色付けするテキスト
 * @return 色付けされたテキスト
 *)
let colorize_by_severity mode severity text =
  match severity with
  | Diagnostic.Error -> apply_color mode Ansi.bright_red text
  | Diagnostic.Warning -> apply_color mode Ansi.bright_yellow text
  | Diagnostic.Note -> apply_color mode Ansi.bright_blue text

(** 個別の色付けヘルパー関数 *)

let red mode text = apply_color mode Ansi.bright_red text
let yellow mode text = apply_color mode Ansi.bright_yellow text
let blue mode text = apply_color mode Ansi.bright_blue text
let cyan mode text = apply_color mode Ansi.cyan text

let bold mode text =
  match mode with
  | Options.Never -> text
  | Options.Always | Options.Auto -> Ansi.bold ^ text ^ Ansi.reset

(** 行番号を色付けする（シアン） *)
let colorize_line_number mode line_num =
  cyan mode (Printf.sprintf "%4d" line_num)

(** ポインタ（^^^）を色付けする（エラー色） *)
let colorize_pointer mode severity pointer =
  colorize_by_severity mode severity pointer
