(* Options — コマンドラインオプション定義インターフェース
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、CLI オプションの型安全な管理を提供する。
 *)

(** 出力フォーマット *)
type output_format = Text  (** テキスト形式（デフォルト） *) | Json  (** JSON 形式（LSP 互換） *)

(** JSON 出力モード *)
type json_mode =
  | JsonPretty  (** 整形済み JSON *)
  | JsonCompact  (** 1 行のコンパクト JSON *)
  | JsonLines  (** JSON Lines 形式 *)

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

(** 監査ログ出力ストアプロファイル *)
type audit_store =
  | AuditStoreTmp  (** 互換モード: tmp/cli-callconv-out 配下へ出力 *)
  | AuditStoreLocal  (** ローカル永続ストア（tooling/audit-store/local） *)
  | AuditStoreCi  (** CI 永続ストア（reports/audit） *)

(** 監査ログ詳細度 *)
type audit_level =
  | AuditLevelSummary  (** 必須キーのみを含むサマリーログ *)
  | AuditLevelFull  (** Phase 2-3 で合意済みのフィールド一式 *)
  | AuditLevelDebug  (** `extensions.*` を含む完全ログ *)

type options = {
  (* 入力 *)
  input_file : string;  (** 入力ファイルパス（必須） *)
  use_stdin : bool;  (** 標準入力から読み込むか（Phase 2） *)
  (* 出力 *)
  emit_ast : bool;  (** AST を標準出力に出力 *)
  emit_tast : bool;  (** Typed AST を標準出力に出力 *)
  emit_ir : bool;  (** LLVM IR (.ll) を出力ディレクトリに生成 *)
  emit_bc : bool;  (** LLVM Bitcode (.bc) を出力ディレクトリに生成 *)
  emit_parse_debug : string option;  (** Parser debug JSON の出力先パス *)
  out_dir : string;  (** 出力ディレクトリ *)
  (* 診断 *)
  format : output_format;  (** 診断メッセージの出力形式 *)
  json_mode : json_mode;  (** JSON 出力モード *)
  include_snippet : bool;  (** テキスト診断にソーススニペットを含めるか *)
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
  audit_enabled : bool;  (** 監査ログ出力を有効化するか *)
  audit_store : audit_store;  (** 監査ログ出力先プロファイル *)
  audit_dir_override : string option;  (** 出力ディレクトリの上書き（任意） *)
  audit_level : audit_level;  (** 監査ログ詳細度 *)
  emit_audit_path : string option;  (** 監査ログ（JSON Lines）出力先 *)
  effect_stage_override : string option;  (** CLI で指定された Stage 名 *)
  runtime_capabilities_path : string option;
      (** Runtime Capability Registry JSON のパス *)
  effects_type_row_mode : string;
      (** TYPE-002 効果行モード (`metadata-only` / `dual-write` / `ty-integrated`) *)
  parser_experimental_effects : bool;
      (** `perform` / `handle` 構文を受理する実験的フラグ（`-Zalgebraic-effects`） *)
  (* Parser RunConfig フラグ *)
  parser_require_eof : bool;  (** RunConfig.require_eof を CLI から制御 *)
  parser_packrat : bool;  (** RunConfig.packrat 切替（Packrat メモ化を有効化、実験的） *)
  parser_left_recursion : Parser_run_config.left_recursion;
      (** RunConfig.left_recursion モード *)
  parser_merge_warnings : bool;
      (** RunConfig.merge_warnings（診断集約の有無） *)
  parser_streaming : bool;  (** ストリーミングランナー PoC を有効化 *)
  stream_checkpoint : string option;  (** RunConfig.extensions.stream.checkpoint *)
  stream_resume_hint : string option;
      (** RunConfig.extensions.stream.resume_hint のトークン *)
  stream_demand_min_bytes : int option;
      (** DemandHint.min_bytes の既定値（省略時は内部既定） *)
  stream_demand_preferred_bytes : int option;
      (** DemandHint.preferred_bytes の既定値（省略時は内部既定） *)
  stream_chunk_size : int option;  (** CLI が読み込むチャンクサイズのヒント（バイト数） *)
  stream_flow_policy : Parser_run_config.Stream.Flow.policy option;
      (** FlowController.policy の明示設定（`auto`/`manual`） *)
  stream_flow_max_lag_bytes : int option;
      (** FlowController.backpressure.max_lag_bytes （チャンク遅延の上限） *)
  stream_flow_debounce_ms : int option;
      (** FlowController.backpressure.debounce_ms （バックプレッシャ反応のデバウンス） *)
  stream_flow_throttle_ratio : float option;
      (** FlowController.backpressure.throttle_ratio （0.0〜1.0 のスロットル率） *)
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

val to_run_config : options -> Parser_run_config.t
(** CLI オプションから RunConfig を構築し、`extensions["config"]` に
    CLI トグルのスナップショットを記録する。 *)
