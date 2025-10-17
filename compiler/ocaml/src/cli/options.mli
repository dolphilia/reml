(* Options — コマンドラインオプション定義インターフェース
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、CLI オプションの型安全な管理を提供する。
 *)

(** 出力フォーマット *)
type output_format = Text  (** テキスト形式（デフォルト） *) | Json  (** JSON 形式（LSP 互換） *)

(** メトリクス出力フォーマット *)
type metrics_format = MetricsJson  (** JSON 形式 *) | MetricsCsv  (** CSV 形式 *)

(** カラーモード *)
type color_mode =
  | Auto  (** TTY への出力時のみカラー表示 *)
  | Always  (** 常にカラー表示（パイプ時も） *)
  | Never  (** カラー表示を無効化 *)

(** 型クラス戦略モード *)
type typeclass_mode =
  | TypeclassDictionary  (** 辞書渡し方式 *)
  | TypeclassMonomorph  (** モノモルフィゼーション PoC *)
  | TypeclassBoth  (** 両方式の成果物比較 *)

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
  typeclass_mode : typeclass_mode;  (** 型クラス実装戦略モード *)
  (* デバッグ *)
  trace : bool;  (** コンパイルフェーズのトレースを有効化 *)
  stats : bool;  (** コンパイル統計情報を表示 *)
  verbose : int;  (** ログの詳細度レベル (0-3) *)
  metrics_path : string option;  (** メトリクス出力ファイルパス *)
  metrics_format : metrics_format;  (** メトリクス出力形式 *)
  (* コンパイル *)
  target : string;  (** ターゲットトリプル *)
  link_runtime : bool;  (** ランタイムライブラリとリンクして実行可能ファイルを生成 *)
  runtime_path : string;  (** ランタイムライブラリのパス *)
  verify_ir : bool;  (** 生成された LLVM IR を検証 *)
  effect_stage_override : string option;  (** CLI で指定された Stage 名 *)
  runtime_capabilities_path : string option;  (** Runtime Capability Registry JSON のパス *)
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
