(* parser_expectation.ml — Menhir 期待集合写像ユーティリティ
 *
 * 仕様 2-5 §B-7 の優先順位と表記ルールに基づき、Menhir の終端や補助情報を
 * `Diagnostic.expectation` / `ExpectationSummary` へ変換する。
 *)

module I = Parser.MenhirInterpreter

open Diagnostic

type collection = {
  sample_tokens : Token.t list;
  expectations : expectation list;
  summary : expectation_summary;
}

let fallback_placeholder_label = "解析継続トークン"
let fallback_placeholder = Diagnostic.Custom fallback_placeholder_label
let empty_summary_humanized = "ここで解釈可能な構文が見つかりません"

let keyword_to_expectation token =
  match token with
  | Token.MODULE
  | Token.USE
  | Token.AS
  | Token.PUB
  | Token.SELF
  | Token.SUPER
  | Token.LET
  | Token.VAR
  | Token.FN
  | Token.TYPE
  | Token.ALIAS
  | Token.NEW
  | Token.TRAIT
  | Token.IMPL
  | Token.EXTERN
  | Token.EFFECT
  | Token.OPERATION
  | Token.HANDLER
  | Token.CONDUCTOR
  | Token.CHANNELS
  | Token.EXECUTION
  | Token.MONITORING
  | Token.IF
  | Token.THEN
  | Token.ELSE
  | Token.MATCH
  | Token.WITH
  | Token.FOR
  | Token.IN
  | Token.WHILE
  | Token.LOOP
  | Token.RETURN
  | Token.DEFER
  | Token.UNSAFE
  | Token.PERFORM
  | Token.DO
  | Token.HANDLE
  | Token.WHERE
  | Token.TRUE
  | Token.FALSE
  | Token.BREAK
  | Token.CONTINUE ->
      Some (Keyword (Token.to_string token))
  | _ -> None

let operator_to_expectation token =
  match token with
  | Token.PIPE
  | Token.CHANNEL_PIPE
  | Token.DOT
  | Token.COMMA
  | Token.SEMICOLON
  | Token.COLON
  | Token.AT
  | Token.BAR
  | Token.EQ
  | Token.COLONEQ
  | Token.ARROW
  | Token.DARROW
  | Token.LPAREN
  | Token.RPAREN
  | Token.LBRACKET
  | Token.RBRACKET
  | Token.LBRACE
  | Token.RBRACE
  | Token.PLUS
  | Token.MINUS
  | Token.STAR
  | Token.SLASH
  | Token.PERCENT
  | Token.POW
  | Token.EQEQ
  | Token.NE
  | Token.LT
  | Token.LE
  | Token.GT
  | Token.GE
  | Token.AND
  | Token.OR
  | Token.NOT
  | Token.QUESTION
  | Token.DOTDOT
  | Token.UNDERSCORE ->
      Some (Token (Token.to_string token))
  | _ -> None

let literal_to_expectation token =
  match token with
  | Token.INT _ -> Some (Class "integer-literal")
  | Token.FLOAT _ -> Some (Class "float-literal")
  | Token.CHAR _ -> Some (Class "char-literal")
  | Token.STRING _ -> Some (Class "string-literal")
  | _ -> None

let identifier_to_expectation token =
  match token with
  | Token.IDENT _ -> Some (Class "identifier")
  | Token.UPPER_IDENT _ -> Some (Class "upper-identifier")
  | _ -> None

let expectation_of_token token =
  match keyword_to_expectation token with
  | Some value -> value
  | None -> (
      match operator_to_expectation token with
      | Some value -> value
      | None -> (
          match literal_to_expectation token with
          | Some value -> value
          | None -> (
              match identifier_to_expectation token with
              | Some value -> value
              | None ->
                  (match token with
                  | Token.EOF -> Eof
                  | _ ->
                      Custom
                        (Printf.sprintf "unclassified-token:%s"
                           (Token.to_string token))))))

let expectation_of_terminal token = expectation_of_token token

let expectation_of_nonterminal name = Rule name
let expectation_not message = Not message
let expectation_custom message = Custom message

let priority = function
  | Keyword _ -> 0
  | Token _ -> 1
  | Eof -> 1
  | Class _ -> 2
  | TypeExpected _ -> 2
  | Rule _ -> 3
  | TraitBound _ -> 3
  | Not _ -> 4
  | Custom _ -> 5

let raw_label = function
  | Keyword kw -> kw
  | Token sym -> sym
  | Eof -> "EOF"
  | Class name -> name
  | Rule name -> name
  | Not text -> text
  | Custom text -> text
  | TypeExpected ty -> ty
  | TraitBound trait -> trait

let quoted_label = function
  | Keyword kw -> Printf.sprintf "`%s`" kw
  | Token sym -> Printf.sprintf "`%s`" sym
  | Eof -> "入力終端"
  | Class name -> name
  | Rule name -> name
  | Not text -> Printf.sprintf "%s以外" text
  | Custom text -> text
  | TypeExpected ty -> Printf.sprintf "型 %s" ty
  | TraitBound trait -> Printf.sprintf "%s 境界" trait

let compare_expectation a b =
  let pa = priority a in
  let pb = priority b in
  if pa <> pb then Int.compare pa pb
  else String.compare (raw_label a) (raw_label b)

let dedup_and_sort expectations =
  let sorted = List.sort compare_expectation expectations in
  let rec dedup prev acc = function
    | [] -> List.rev acc
    | x :: xs ->
        (match prev with
        | Some prev_value when Stdlib.compare prev_value x = 0 ->
            dedup prev acc xs
        | _ -> dedup (Some x) (x :: acc) xs)
  in
  dedup None [] sorted

let humanize expectations =
  match expectations with
  | [] -> None
  | [ single ] ->
      Some (Printf.sprintf "ここで%sが必要です" (quoted_label single))
  | _ ->
      let labels = List.map quoted_label expectations in
      let body =
        match List.rev labels with
        | last :: rest_rev ->
            let rest = List.rev rest_rev in
            (match rest with
            | [] -> last
            | _ ->
                String.concat "、" rest ^ " または " ^ last)
        | [] -> ""
      in
      Some (Printf.sprintf "ここで%sのいずれかが必要です" body)

let summarize ?message_key ?(locale_args = []) ?context_note ?humanized
    expectations =
  let normalized = dedup_and_sort expectations in
  let inferred_locale_args =
    if locale_args <> [] then locale_args
    else List.map raw_label normalized
  in
  let inferred_humanized =
    match humanized with None -> humanize normalized | some -> some
  in
  {
    message_key;
    locale_args = inferred_locale_args;
    humanized = inferred_humanized;
    context_note;
    alternatives = normalized;
  }

let summarize_with_defaults ?context_note expectations =
  let normalized = dedup_and_sort expectations in
  match normalized with
  | [] ->
      summarize ~message_key:"parse.expected.empty" ~locale_args:[]
        ~humanized:empty_summary_humanized
        ?context_note normalized
  | _ ->
      summarize ~message_key:"parse.expected"
        ~locale_args:(List.map raw_label normalized) ?context_note normalized

let empty_summary = summarize_with_defaults []

module Streaming_expected = struct
  let keyword_labels =
    [
      "continue";
      "defer";
      "do";
      "false";
      "for";
      "handle";
      "if";
      "loop";
      "match";
      "perform";
      "return";
      "self";
      "true";
      "unsafe";
      "while";
    ]

  let token_labels = [ "!"; "("; "-"; "["; "{"; "|" ]

  let class_labels =
    [
      "char-literal";
      "float-literal";
      "identifier";
      "integer-literal";
      "string-literal";
      "upper-identifier";
    ]

  let summary =
    lazy
      (let keyword_expectations =
         List.map (fun label -> Keyword label) keyword_labels
       in
       let token_expectations =
         List.map (fun label -> Token label) token_labels
       in
       let class_expectations =
         List.map (fun label -> Class label) class_labels
       in
       let expectations =
         keyword_expectations @ token_expectations @ class_expectations
       in
       summarize_with_defaults expectations)

  let summary () = Lazy.force summary
end

let streaming_expression_summary () = Streaming_expected.summary ()

let ensure_minimum_alternatives summary =
  if summary.alternatives <> [] then summary
  else
    let humanized =
      match summary.humanized with
      | Some text when String.trim text <> "" -> summary.humanized
      | _ -> Some empty_summary_humanized
    in
    { summary with alternatives = [ fallback_placeholder ]; humanized }

let keyword_samples =
  [
    Token.MODULE;
    Token.USE;
    Token.AS;
    Token.PUB;
    Token.SELF;
    Token.SUPER;
    Token.LET;
    Token.VAR;
    Token.FN;
    Token.TYPE;
    Token.ALIAS;
    Token.NEW;
    Token.TRAIT;
    Token.IMPL;
    Token.EXTERN;
    Token.EFFECT;
    Token.OPERATION;
    Token.HANDLER;
    Token.CONDUCTOR;
    Token.CHANNELS;
    Token.EXECUTION;
    Token.MONITORING;
    Token.IF;
    Token.THEN;
    Token.ELSE;
    Token.MATCH;
    Token.WITH;
    Token.FOR;
    Token.IN;
    Token.WHILE;
    Token.LOOP;
    Token.RETURN;
    Token.DEFER;
    Token.UNSAFE;
    Token.PERFORM;
    Token.DO;
    Token.HANDLE;
    Token.WHERE;
    Token.TRUE;
    Token.FALSE;
    Token.BREAK;
    Token.CONTINUE;
  ]

let operator_samples =
  [
    Token.PIPE;
    Token.CHANNEL_PIPE;
    Token.DOT;
    Token.COMMA;
    Token.SEMICOLON;
    Token.COLON;
    Token.AT;
    Token.BAR;
    Token.EQ;
    Token.COLONEQ;
    Token.ARROW;
    Token.DARROW;
    Token.LPAREN;
    Token.RPAREN;
    Token.LBRACKET;
    Token.RBRACKET;
    Token.LBRACE;
    Token.RBRACE;
    Token.PLUS;
    Token.MINUS;
    Token.STAR;
    Token.SLASH;
    Token.PERCENT;
    Token.POW;
    Token.EQEQ;
    Token.NE;
    Token.LT;
    Token.LE;
    Token.GT;
    Token.GE;
    Token.AND;
    Token.OR;
    Token.NOT;
    Token.QUESTION;
    Token.DOTDOT;
    Token.UNDERSCORE;
  ]

let literal_samples =
  [
    Token.INT ("0", Ast.Base10);
    Token.FLOAT "0.0";
    Token.CHAR "a";
    Token.STRING ("", Ast.Normal);
  ]

let identifier_samples = [ Token.IDENT "value"; Token.UPPER_IDENT "Value" ]

let sentinel_samples = [ Token.EOF ]

let all_samples =
  keyword_samples @ operator_samples @ literal_samples @ identifier_samples
  @ sentinel_samples

module Packrat = struct
  module Key = struct
    type t = { state : int; offset : int }

    let equal a b = a.state = b.state && a.offset = b.offset
    let hash { state; offset } = Hashtbl.hash (state, offset)
  end

  module Table = Hashtbl.Make (Key)

  type t = collection Table.t

  let create ?(initial_capacity = 64) () = Table.create initial_capacity

  let key_of_env env : Key.t option =
    match I.current_state_number env with
    | exception _ -> None
    | state -> (
        try
          let start_pos, _ = I.positions env in
          Some { Key.state; offset = start_pos.Lexing.pos_cnum }
        with _ -> None)

  let find cache key = Table.find_opt cache key
  let store cache key value = Table.replace cache key value

  let prune_before cache ~offset =
    Table.filter_map_inplace
      (fun key value -> if key.offset < offset then None else Some value)
      cache

  type metrics = { entries : int; approx_bytes : int }

  let metrics cache =
    let entries = Table.length cache in
    let approx_bytes =
      let estimate_collection_bytes collection =
        let token_cost = List.length collection.sample_tokens * 16 in
        let expectation_cost = List.length collection.expectations * 24 in
        let summary_cost =
          match collection.summary.humanized with
          | Some text -> 8 + (String.length text * 2)
          | None -> 8
        in
        64 + token_cost + expectation_cost + summary_cost
      in
      let bytes = ref 0 in
      Table.iter
        (fun _key collection ->
          bytes := !bytes + estimate_collection_bytes collection)
        cache;
      !bytes
    in
    { entries; approx_bytes }
end

let env_of_checkpoint = function
  | I.InputNeeded env -> Some env
  | I.HandlingError env -> Some env
  | I.Shifting (_, env, _) -> Some env
  | I.AboutToReduce (env, _) -> Some env
  | _ -> None

type packrat_status = [ `Hit | `Miss | `Bypassed ]

let collect ~checkpoint ~packrat =
  match env_of_checkpoint checkpoint with
  | None ->
      ( { sample_tokens = []; expectations = []; summary = empty_summary },
        `Bypassed )
  | Some env ->
      let basis =
        match checkpoint with
        | I.InputNeeded _ -> checkpoint
        | _ -> I.input_needed env
      in
      let start_pos, _ = I.positions env in
      let compute () =
        let accepted =
          List.filter
            (fun token ->
              try I.acceptable basis token start_pos with _ -> false)
            all_samples
        in
        let summary =
          summarize_with_defaults
            (List.map expectation_of_token accepted)
        in
        {
          sample_tokens = accepted;
          expectations = summary.alternatives;
          summary;
        }
      in
      (match packrat with
      | None -> (compute (), `Bypassed)
      | Some cache -> (
          match Packrat.key_of_env env with
          | None -> (compute (), `Bypassed)
          | Some key -> (
              match Packrat.find cache key with
              | Some cached -> (cached, `Hit)
              | None ->
                  let value = compute () in
                  Packrat.store cache key value;
                  (value, `Miss))))
