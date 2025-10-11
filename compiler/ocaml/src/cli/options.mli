(* Options — コマンドラインオプション定義インターフェース
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、CLI オプションの型安全な管理を提供する。
 *)

(** 出力フォーマット *)
type output_format = Text  (** テキスト形式（デフォルト） *) | Json  (** JSON 形式（LSP 互換） *)

(** カラーモード *)
type color_mode =
  | Auto  (** TTY への出力時のみカラー表示 *)
  | Always  (** 常にカラー表示（パイプ時も） *)
  | Never  (** カラー表示を無効化 *)

type options = {
  (* 入力 *)
  input_file : string;  (** 入力ファイルパス（必須） *)
  use_stdin : bool;  (** 標準入力から読み込むか（Phase 2） *)
  (* 出力 *)
  emit_ast : bool;  (** AST を標準出力に出力 *)
  emit_tast : bool;  (** Typed AST を標準出力に出力 *)
  emit_ir : bool;  (** LLVM IR (.ll) を出力ディレクトリに生成 *)
  emit_bc : bool;  (** LLVM Bitcode (.bc) を出力ディレクトリに生成 *)
  out_dir : string;  (** 出力ディレクトリ *)
  (* 診断 *)
  format : output_format;  (** 診断メッセージの出力形式 *)
  color : color_mode;  (** カラー出力の制御 *)
  (* デバッグ *)
  trace : bool;  (** コンパイルフェーズのトレースを有効化 *)
  stats : bool;  (** コンパイル統計情報を表示 *)
  verbose : int;  (** ログの詳細度レベル (0-3) *)
  (* コンパイル *)
  target : string;  (** ターゲットトリプル *)
  link_runtime : bool;  (** ランタイムライブラリとリンクして実行可能ファイルを生成 *)
  runtime_path : string;  (** ランタイムライブラリのパス *)
  verify_ir : bool;  (** 生成された LLVM IR を検証 *)
}
(** コマンドラインオプション設定 *)

val default_options : options
(** デフォルトオプション *)

val parse_args : string array -> (options, string) result
(** コマンドライン引数を解析してオプションを生成
 *
 * @param argv コマンドライン引数配列
 * @return 解析されたオプション、またはエラーメッセージ
 *)
