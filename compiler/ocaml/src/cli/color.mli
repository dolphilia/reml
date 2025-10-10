(* Color — ANSI カラー出力とカラーモード判定インターフェース
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、診断メッセージのカラー出力を提供する。
 *)

(** Unix.isatty のラッパー
 *
 * @param fd ファイルディスクリプタ
 * @return TTY の場合 true
 *)
val is_tty : Unix.file_descr -> bool

(** カラーモードを解決する
 *
 * ユーザーが指定したカラーモード、TTY 判定、環境変数（NO_COLOR, FORCE_COLOR）を
 * 考慮して、実際に使用するカラーモードを決定する。
 *
 * 優先順位:
 * 1. NO_COLOR 環境変数が設定されている場合は常に Never
 * 2. --color=always の場合は Always
 * 3. --color=never の場合は Never
 * 4. --color=auto の場合:
 *    - FORCE_COLOR が設定されている場合は Always
 *    - TTY の場合は Always
 *    - それ以外は Never
 *
 * @param requested ユーザーが指定したカラーモード（--color オプション）
 * @param is_tty 出力先が TTY かどうか
 * @return 実際に使用するカラーモード
 *)
val resolve_color_mode : requested:Options.color_mode -> is_tty:bool -> Options.color_mode

(** 重要度に応じた色を適用する
 *
 * @param mode カラーモード
 * @param severity 診断の重要度
 * @param text 色付けするテキスト
 * @return 色付けされたテキスト（mode が Never の場合は元のテキスト）
 *)
val colorize_by_severity : Options.color_mode -> Diagnostic.severity -> string -> string

(** 赤色を適用する
 *
 * @param mode カラーモード
 * @param text 色付けするテキスト
 * @return 色付けされたテキスト
 *)
val red : Options.color_mode -> string -> string

(** 黄色を適用する
 *
 * @param mode カラーモード
 * @param text 色付けするテキスト
 * @return 色付けされたテキスト
 *)
val yellow : Options.color_mode -> string -> string

(** 青色を適用する
 *
 * @param mode カラーモード
 * @param text 色付けするテキスト
 * @return 色付けされたテキスト
 *)
val blue : Options.color_mode -> string -> string

(** シアン色を適用する
 *
 * @param mode カラーモード
 * @param text 色付けするテキスト
 * @return 色付けされたテキスト
 *)
val cyan : Options.color_mode -> string -> string

(** 太字を適用する
 *
 * @param mode カラーモード
 * @param text 太字にするテキスト
 * @return 太字にされたテキスト
 *)
val bold : Options.color_mode -> string -> string

(** 行番号を色付けする（シアン）
 *
 * @param mode カラーモード
 * @param line_num 行番号
 * @return 色付けされた行番号文字列（4桁、右詰め）
 *)
val colorize_line_number : Options.color_mode -> int -> string

(** ポインタ（^^^）を色付けする（エラー色）
 *
 * @param mode カラーモード
 * @param severity 診断の重要度
 * @param pointer ポインタ文字列
 * @return 色付けされたポインタ
 *)
val colorize_pointer : Options.color_mode -> Diagnostic.severity -> string -> string
