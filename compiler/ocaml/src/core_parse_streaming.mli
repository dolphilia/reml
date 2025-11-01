(* Core Parse Streaming — バッチランナーとストリーミングランナーで共有する骨格ロジック
 *
 * Phase 2-5 EXEC-001 Step 1: Core.Parse.Streaming モジュール骨格の抽出
 *)

module Run_config = Parser_run_config

type packrat_cache = Parser_expectation.Packrat.t option

type session

val create_session : Run_config.t -> session

val config : session -> Run_config.t
val diag_state : session -> Parser_diag_state.t
val core_state : session -> Core_parse.State.t
val packrat_cache : session -> packrat_cache

val effective_require_eof : Run_config.t -> bool

val expectation_summary_for_checkpoint :
  session ->
  'a Parser.MenhirInterpreter.checkpoint ->
  Diagnostic.expectation_summary

val register_diagnostic :
  session ->
  Diagnostic.t ->
  consumed:bool ->
  committed:bool ->
  unit

val diagnostics : session -> Diagnostic.t list
val recovered : session -> bool
val farthest_error_offset : session -> int option
val span_trace_pairs :
  session -> (string option * Diagnostic.span) list option

val packrat_counters : session -> (int * int) option

val annotate_core_rule_metadata :
  Diagnostic.t ->
  Core_parse.Id.t option ->
  Diagnostic.t
