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
        ~humanized:"ここで解釈可能な構文が見つかりません"
        ?context_note normalized
  | _ ->
      summarize ~message_key:"parse.expected"
        ~locale_args:(List.map raw_label normalized) ?context_note normalized

let empty_summary = summarize_with_defaults []

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

let env_of_checkpoint = function
  | I.InputNeeded env -> Some env
  | I.HandlingError env -> Some env
  | I.Shifting (_, env, _) -> Some env
  | I.AboutToReduce (env, _) -> Some env
  | _ -> None

let collect ~checkpoint =
  match env_of_checkpoint checkpoint with
  | None ->
      { sample_tokens = []; expectations = []; summary = empty_summary }
  | Some env ->
      let basis =
        match checkpoint with
        | I.InputNeeded _ -> checkpoint
        | _ -> I.input_needed env
      in
      let start_pos, _ = I.positions env in
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
