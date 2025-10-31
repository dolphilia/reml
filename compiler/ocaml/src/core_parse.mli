(* core_parse.mli — Core Parse ブリッジ層 PoC インターフェース
 *
 * Phase 2-5 PARSER-003 Step3:
 *   Menhir 依存のパース処理へコアコンビネーター層を挿入するための
 *   足場として、State/Reply/Id と基本コンビネーターを定義する。
 *)

module Id : sig
  type origin = [ `Static | `Dynamic ]

  type t

  val namespace : t -> string
  val name : t -> string
  val ordinal : t -> int
  val fingerprint : t -> int64
  val origin : t -> origin
end

module Reply : sig
  type 'a t =
    | Ok of {
        id : Id.t option;
        value : 'a;
        span : Diagnostic.span option;
        consumed : bool;
        committed : bool;
      }
    | Err of {
        id : Id.t option;
        diagnostic : Diagnostic.t;
        consumed : bool;
        committed : bool;
      }

  val ok :
    ?id:Id.t ->
    value:'a ->
    span:Diagnostic.span option ->
    consumed:bool ->
    committed:bool ->
    'a t

  val err :
    ?id:Id.t ->
    diagnostic:Diagnostic.t ->
    consumed:bool ->
    committed:bool ->
    'a t
end

module State : sig
  type t

  val create :
    config:Parser_run_config.t ->
    diag:Parser_diag_state.t ->
    t

  val config : t -> Parser_run_config.t
  val diag : t -> Parser_diag_state.t
  val consumed : t -> bool
  val committed : t -> bool
  val mark_consumed : t -> unit
  val mark_committed : t -> unit
  val with_consumed : t -> bool -> unit
  val with_committed : t -> bool -> unit
end

type 'a parser = State.t -> 'a Reply.t * State.t

val rule :
  namespace:string ->
  name:string ->
  'a parser ->
  State.t ->
  'a Reply.t * State.t

val label : printable:string -> 'a parser -> 'a parser
val cut : 'a parser -> 'a parser
val cut_here : unit parser
val attempt : 'a parser -> 'a parser
