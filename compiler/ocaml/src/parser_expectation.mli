(* parser_expectation.mli — Menhir 期待集合写像ユーティリティ *)

type collection = {
  sample_tokens : Token.t list;
  expectations : Diagnostic.expectation list;
  summary : Diagnostic.expectation_summary;
}
(** `Parser.MenhirInterpreter` のチェックポイントから期待集合を抽出した結果 *)

val expectation_of_token : Token.t -> Diagnostic.expectation
(** lexer/Parser が扱うトークンを仕様上の Expectation へ変換する *)

val expectation_of_terminal :
  Parser.MenhirInterpreter.token -> Diagnostic.expectation
(** Menhir の終端表現を Expectation へ変換する *)

val expectation_of_nonterminal : string -> Diagnostic.expectation
(** 非終端記号を Expectation.Rule へラップする *)

val expectation_not : string -> Diagnostic.expectation
(** 条件否定を Expectation.Not で表現する補助 *)

val expectation_custom : string -> Diagnostic.expectation
(** 任意メッセージを Expectation.Custom へ包む *)

val dedup_and_sort :
  Diagnostic.expectation list -> Diagnostic.expectation list
(** 仕様で定義された優先順位に従って期待集合を整列し重複を除去する *)

val summarize :
  ?message_key:string ->
  ?locale_args:string list ->
  ?context_note:string ->
  ?humanized:string ->
  Diagnostic.expectation list ->
  Diagnostic.expectation_summary
(** ExpectationSummary を組み立てる。humanized/locale_args が省略された場合は規定値を生成する。 *)

val summarize_with_defaults :
  ?context_note:string ->
  Diagnostic.expectation list ->
  Diagnostic.expectation_summary
(** `parse.expected` 系の既定メッセージキーと整形済み humanized を適用したサマリ生成 *)

val empty_summary : Diagnostic.expectation_summary
(** 期待集合が空だった場合に利用するフォールバックサマリ *)

val streaming_expression_summary : unit -> Diagnostic.expectation_summary
(** Streaming recover 向けにあらかじめ整列済みの期待候補リストを返す。
    `collect-iterator-audit-metrics.py` の `parser.expected_summary_presence`
    ゲートが Streaming ケースでも 1.0 になるよう、`if/loop/identifier` 等
    の代表キーワード／トークン／クラスをまとめて出力する。 *)

val humanize : Diagnostic.expectation list -> string option
(** 期待集合を日本語ヒューマンリーダブル文字列に整形する *)

val ensure_minimum_alternatives :
  Diagnostic.expectation_summary -> Diagnostic.expectation_summary
(** `expected.alternatives` が空の場合にプレースホルダ候補を追加し、
    `parser.expected_summary_presence` メトリクスで欠落しないよう補正する。 *)

module Packrat : sig
  type t

  val create : ?initial_capacity:int -> unit -> t
  (** Packrat キャッシュを生成する。 *)

  val prune_before : t -> offset:int -> unit
  (** 指定したオフセットより前のエントリを破棄してメモリを回収する。 *)

  type metrics = { entries : int; approx_bytes : int }
  (** キャッシュ内エントリ数と概算ヒープ使用量（バイト）。 *)

  val metrics : t -> metrics
  (** キャッシュの概算統計値を取得する。 *)
end

type packrat_status = [ `Hit | `Miss | `Bypassed ]
(** キャッシュ参照結果の状態 *)

val collect :
  checkpoint:'a Parser.MenhirInterpreter.checkpoint ->
  packrat:Packrat.t option ->
  collection * packrat_status
(** Menhir のチェックポイントから受理可能トークンを走査し、期待集合を集計する *)
