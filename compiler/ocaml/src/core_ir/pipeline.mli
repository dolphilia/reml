(* Core_ir.Pipeline — Optimization Pipeline Interface *)

open Ir

(** 最適化レベル *)
type opt_level =
  | O0   (** 最適化なし *)
  | O1   (** 基本最適化（定数畳み込み + DCE） *)

(** パイプライン設定 *)
type pipeline_config = {
  opt_level: opt_level;
  enable_const_fold: bool;
  enable_dce: bool;
  max_iterations: int;
  verbose: bool;
  emit_intermediate: bool;
}

(** パイプライン統計 *)
type pipeline_stats = {
  mutable iterations: int;
  mutable total_const_fold_time: float;
  mutable total_dce_time: float;
  mutable total_folded_exprs: int;
  mutable total_removed_bindings: int;
  mutable total_removed_blocks: int;
}

(** デフォルト設定（O0: 最適化なし） *)
val config_o0 : pipeline_config

(** O1 設定（基本最適化） *)
val config_o1 : pipeline_config

(** 設定から最適化レベルを選択 *)
val config_from_level : opt_level -> pipeline_config

(** 統計情報を作成 *)
val create_pipeline_stats : unit -> pipeline_stats

(** 統計情報を表示 *)
val print_stats : pipeline_stats -> unit

(** 関数に対してパイプラインを実行
 *
 * @param config パイプライン設定（デフォルト: config_o1）
 * @param fn 最適化対象の関数
 * @return (最適化後の関数, 統計情報)
 *)
val optimize_function : ?config:pipeline_config -> function_def -> function_def * pipeline_stats

(** モジュール全体に対してパイプラインを実行
 *
 * @param config パイプライン設定（デフォルト: config_o1）
 * @param m 最適化対象のモジュール
 * @return (最適化後のモジュール, 統計情報)
 *)
val optimize_module : ?config:pipeline_config -> module_def -> module_def * pipeline_stats
