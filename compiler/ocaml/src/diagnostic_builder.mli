(** 診断ビルダー API（ドラフト）

    Phase 2-4 で導入予定の V2 診断フィールドを扱う補助関数を含む。
    実装は `Diagnostic.Builder` に委譲し、互換期間中は旧 API と併用できる。 *)

open Diagnostic

type t

(** 構造化ヒントの分類 *)
type structured_hint_kind =
  | Quick_fix
  | Follow_up
  | Context
  | Documentation
  | Command
  | Link
  | Custom of string

(** 構造化ヒントのペイロード表現 *)
type structured_hint_payload =
  | Command_payload of {
      command : string;
      arguments : Yojson.Basic.t option;
    }
  | Link_payload of {
      href : string;
      title : string option;
    }
  | Replacement_payload of {
      range : span option;
      template : string;
    }
  | Message_payload of string
  | Data_payload of Yojson.Basic.t

(** 構造化ヒント本体 *)
type structured_hint = {
  id : string option;
  title : string option;
  span : span option;
  kind : structured_hint_kind;
  payload : structured_hint_payload;
  actions : fixit list;
}

val create :
  ?id:string ->
  ?severity:severity ->
  ?severity_hint:severity_hint ->
  ?domain:error_domain ->
  ?code:string ->
  ?codes:string list ->
  ?timestamp:string ->
  message:string ->
  primary:span ->
  unit ->
  t

val set_severity : severity -> t -> t
val set_severity_hint : severity_hint option -> t -> t
val set_domain : error_domain -> t -> t

val add_code : string -> t -> t
val set_codes : string list -> t -> t
val push_code : string -> t -> t
val add_codes : string list -> t -> t
val set_primary_code : string -> t -> t

val set_id : string -> t -> t
val clear_id : t -> t

val add_secondary :
  ?span:span ->
  ?message:string ->
  t ->
  t

val merge_secondary : span_label list -> t -> t
val clear_secondary : t -> t

val add_note : ?span:span -> string -> t -> t
val add_notes : (span option * string) list -> t -> t

val set_expected : expectation_summary -> t -> t
val clear_expected : t -> t

val add_fixits : fixit list -> t -> t

val add_hint :
  ?actions:fixit list -> ?message:string -> t -> t

val add_structured_hint :
  ?id:string ->
  ?title:string ->
  ?span:span ->
  ?actions:fixit list ->
  kind:structured_hint_kind ->
  payload:structured_hint_payload ->
  t ->
  t

val merge_structured_hints : structured_hint list -> t -> t
val clear_structured_hints : t -> t

val with_extensions : Extensions.t -> t -> t
val add_extension : string -> Yojson.Basic.t -> t -> t

val with_audit_metadata : Extensions.t -> t -> t
val add_audit_metadata : string -> Yojson.Basic.t -> t -> t

val set_timestamp : string -> t -> t

val build : t -> Diagnostic.t
