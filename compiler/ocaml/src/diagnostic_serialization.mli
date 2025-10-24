(** Diagnostic_serialization — CLI / LSP / CI 共有の診断シリアライズ層（ドラフト）

    Phase 2-4 のシリアライズ統合タスクに向けた骨格実装。
    現状は Diagnostic.t から共通中間表現と JSON への変換ユーティリティを提供する。 *)

open Diagnostic

(** 正規化済みスパン（0 起算）。 *)
type normalized_span = {
  file : string;
  start_line : int;
  start_col : int;
  end_line : int;
  end_col : int;
}

(** セカンダリラベル。 *)
type normalized_secondary = {
  span : normalized_span option;
  message : string option;
}

(** 修正提案。 *)
type normalized_fixit =
  | Insert of { range : normalized_span; text : string }
  | Replace of { range : normalized_span; text : string }
  | Delete of { range : normalized_span }

(** ヒント情報。 *)
type normalized_hint = {
  message : string option;
  actions : normalized_fixit list;
}

(** 診断の正規化済み中間表現。 *)
type normalized_diagnostic = {
  id : string option;
  message : string;
  severity : severity;
  severity_hint : severity_hint option;
  domain : error_domain option;
  codes : string list;
  primary : normalized_span;
  secondary : normalized_secondary list;
  hints : normalized_hint list;
  fixits : normalized_fixit list;
  expected : expectation_summary option;
  schema_version : string;
  extensions : Extensions.t;
  audit_metadata : Extensions.t;
  audit : Audit_envelope.t option;
  timestamp : string option;
}

(** 共通中間表現へ変換。 *)
val of_diagnostic : Diagnostic.t -> normalized_diagnostic

(** 共通中間表現から JSON へ変換。 *)
val to_json : normalized_diagnostic -> Yojson.Basic.t

(** 単一診断 / 複数診断の JSON 値。 *)
val diagnostic_to_json : Diagnostic.t -> Yojson.Basic.t
val diagnostics_to_json : Diagnostic.t list -> Yojson.Basic.t list

(** 期待値・修正提案などのスカラー変換ユーティリティ。 *)
val span_to_json : normalized_span -> Yojson.Basic.t
val secondary_to_json : normalized_secondary -> Yojson.Basic.t
val fixit_to_json : normalized_fixit -> Yojson.Basic.t
val hint_to_json : normalized_hint -> Yojson.Basic.t
val expectation_summary_to_json : expectation_summary option -> Yojson.Basic.t

val severity_to_string : severity -> string
val severity_hint_to_string : severity_hint -> string
val severity_level_of_severity : severity -> int
val domain_to_string : error_domain option -> string option
