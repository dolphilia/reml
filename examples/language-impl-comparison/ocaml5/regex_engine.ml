(* 正規表現エンジン：パース + 評価の両方を実装。
 *
 * 対応する正規表現構文（簡易版）：
 * - リテラル: `abc`
 * - 連結: `ab`
 * - 選択: `a|b`
 * - 繰り返し: `a*`, `a+`, `a?`, `a{2,5}`
 * - グループ: `(abc)`
 * - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
 * - アンカー: `^`, `$`
 * - ドット: `.` (任意の1文字)
 *)

(* 正規表現のAST *)
type predefined_class =
  | Digit
  | Word
  | Whitespace
  | NotDigit
  | NotWord
  | NotWhitespace

type charset =
  | CharRange of char * char
  | CharList of char list
  | Predefined of predefined_class
  | Negated of charset
  | Union of charset list

type repeat_kind =
  | ZeroOrMore
  | OneOrMore
  | ZeroOrOne
  | Exactly of int
  | Range of int * int option

type anchor_kind =
  | Start
  | End

type regex =
  | Literal of string
  | CharClass of charset
  | Dot
  | Concat of regex list
  | Alternation of regex list
  | Repeat of regex * repeat_kind
  | Group of regex
  | Anchor of anchor_kind

(* パーサー型 *)
type 'a parser = string -> ('a * string) option

(* パーサーコンビネーター *)
let ok value input = Some (value, input)

let fail message input = None

let bind p f input =
  match p input with
  | Some (value, rest) -> f value rest
  | None -> None

let map f p = bind p (fun value -> ok (f value))

let rec choice parsers input =
  match parsers with
  | [] -> None
  | p :: ps ->
      match p input with
      | Some result -> Some result
      | None -> choice ps input

let rec many p input =
  match p input with
  | Some (value, rest) ->
      (match many p rest with
       | Some (values, final_rest) -> Some (value :: values, final_rest)
       | None -> Some ([value], rest))
  | None -> Some ([], input)

let many1 p =
  bind p (fun first ->
    bind (many p) (fun rest ->
      ok (first :: rest)))

let optional p input =
  match p input with
  | Some (value, rest) -> Some (Some value, rest)
  | None -> Some (None, input)

let char c input =
  if String.length input > 0 && input.[0] = c then
    Some (c, String.sub input 1 (String.length input - 1))
  else
    None

let string s input =
  let len = String.length s in
  if String.length input >= len && String.sub input 0 len = s then
    Some (s, String.sub input len (String.length input - len))
  else
    None

let satisfy pred input =
  if String.length input > 0 && pred input.[0] then
    Some (input.[0], String.sub input 1 (String.length input - 1))
  else
    None

let digit = satisfy (fun c -> c >= '0' && c <= '9')

let integer =
  map (fun digits ->
    List.fold_left (fun acc d ->
      acc * 10 + (Char.code d - Char.code '0')
    ) 0 digits
  ) (many1 digit)

let rec sep_by1 p sep =
  bind p (fun first ->
    bind (many (bind sep (fun _ -> p))) (fun rest ->
      ok (first :: rest)))

(* 正規表現パーサー *)
let rec regex_expr input = alternation_expr input

and alternation_expr input =
  map (fun alts ->
    match alts with
    | [single] -> single
    | _ -> Alternation alts
  ) (sep_by1 concat_expr (string "|")) input

and concat_expr input =
  map (fun terms ->
    match terms with
    | [single] -> single
    | _ -> Concat terms
  ) (many1 postfix_term) input

and postfix_term input =
  bind atom (fun base ->
    map (fun repeat_opt ->
      match repeat_opt with
      | Some kind -> Repeat (base, kind)
      | None -> base
    ) (optional repeat_suffix)
  ) input

and atom input =
  choice [
    (* 括弧グループ *)
    bind (string "(") (fun _ ->
      bind regex_expr (fun inner ->
        bind (string ")") (fun _ ->
          ok (Group inner))));

    (* アンカー *)
    map (fun _ -> Anchor Start) (string "^");
    map (fun _ -> Anchor End) (string "$");

    (* ドット *)
    map (fun _ -> Dot) (string ".");

    (* 文字クラス *)
    char_class;

    (* 定義済みクラス *)
    predefined_class;

    (* エスケープ文字 *)
    escape_char;

    (* 通常のリテラル *)
    map (fun c -> Literal (String.make 1 c))
      (satisfy (fun c ->
        not (List.mem c ['('; ')'; '['; ']'; '{'; '}'; '*'; '+'; '?'; '.'; '|'; '^'; '$'; '\\'])
      ))
  ] input

and escape_char input =
  bind (string "\\") (fun _ ->
    map (fun c ->
      let lit = match c with
        | 'n' -> "\n"
        | 't' -> "\t"
        | 'r' -> "\r"
        | _ -> String.make 1 c
      in
      Literal lit
    ) (satisfy (fun c ->
      List.mem c ['n'; 't'; 'r'; '\\'; '('; ')'; '['; ']'; '{'; '}'; '*'; '+'; '?'; '.'; '|'; '^'; '$']
    ))
  ) input

and predefined_class input =
  bind (string "\\") (fun _ ->
    map (fun cls -> CharClass (Predefined cls))
      (choice [
        map (fun _ -> Digit) (char 'd');
        map (fun _ -> Word) (char 'w');
        map (fun _ -> Whitespace) (char 's');
        map (fun _ -> NotDigit) (char 'D');
        map (fun _ -> NotWord) (char 'W');
        map (fun _ -> NotWhitespace) (char 'S');
      ])
  ) input

and char_class input =
  bind (string "[") (fun _ ->
    bind (optional (string "^")) (fun negated ->
      bind (many1 char_class_item) (fun items ->
        bind (string "]") (fun _ ->
          let union_set = Union items in
          let cs = match negated with
            | Some _ -> Negated union_set
            | None -> union_set
          in
          ok (CharClass cs)
        )
      )
    )
  ) input

and char_class_item input =
  choice [
    (* 範囲 *)
    bind (satisfy (fun c -> c <> ']' && c <> '-')) (fun start ->
      map (fun end_opt ->
        match end_opt with
        | Some end_char -> CharRange (start, end_char)
        | None -> CharList [start]
      ) (optional (bind (string "-") (fun _ ->
        satisfy (fun c -> c <> ']')
      )))
    );

    (* 単一文字 *)
    map (fun c -> CharList [c]) (satisfy (fun c -> c <> ']'))
  ] input

and repeat_suffix input =
  choice [
    map (fun _ -> ZeroOrMore) (string "*");
    map (fun _ -> OneOrMore) (string "+");
    map (fun _ -> ZeroOrOne) (string "?");

    (* {n,m} 形式 *)
    bind (string "{") (fun _ ->
      bind integer (fun n ->
        bind (optional (bind (string ",") (fun _ ->
          optional integer
        ))) (fun range_opt ->
          bind (string "}") (fun _ ->
            ok (match range_opt with
              | None -> Exactly n
              | Some None -> Range (n, None)
              | Some (Some m) -> Range (n, Some m))
          )
        )
      )
    )
  ] input

let parse_regex input =
  match regex_expr input with
  | Some (regex, "") -> Some regex
  | _ -> None

(* マッチングエンジン *)
let rec match_regex regex text =
  match_from_pos regex text 0

and match_from_pos regex text pos =
  match regex with
  | Literal s ->
      let len = String.length s in
      pos + len <= String.length text &&
      String.sub text pos len = s

  | CharClass cs ->
      pos < String.length text &&
      char_matches_class text.[pos] cs

  | Dot ->
      pos < String.length text

  | Concat terms ->
      let rec go terms' current_pos =
        match terms' with
        | [] -> true
        | term :: rest ->
            if match_from_pos term text current_pos then
              go rest (current_pos + 1)
            else
              false
      in
      go terms pos

  | Alternation alts ->
      List.exists (fun alt -> match_from_pos alt text pos) alts

  | Repeat (inner, kind) ->
      (match kind with
       | ZeroOrMore -> match_repeat_loop inner text pos 0 0 999999
       | OneOrMore ->
           if match_from_pos inner text pos then
             match_repeat_loop inner text (pos + 1) 1 1 999999
           else
             false
       | ZeroOrOne -> match_from_pos inner text pos || true
       | Exactly n -> match_repeat_loop inner text pos 0 n n
       | Range (min_count, max_opt) ->
           let max_count = match max_opt with Some m -> m | None -> 999999 in
           match_repeat_loop inner text pos 0 min_count max_count)

  | Group inner ->
      match_from_pos inner text pos

  | Anchor kind ->
      match kind with
      | Start -> pos = 0
      | End -> pos >= String.length text

and char_matches_class c cs =
  match cs with
  | CharRange (start, end_char) ->
      c >= start && c <= end_char

  | CharList chars ->
      List.mem c chars

  | Predefined cls ->
      (match cls with
       | Digit -> c >= '0' && c <= '9'
       | Word ->
           (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
           (c >= '0' && c <= '9') || c = '_'
       | Whitespace -> List.mem c [' '; '\t'; '\n'; '\r']
       | NotDigit -> not (c >= '0' && c <= '9')
       | NotWord ->
           not ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
                (c >= '0' && c <= '9') || c = '_')
       | NotWhitespace -> not (List.mem c [' '; '\t'; '\n'; '\r']))

  | Negated inner ->
      not (char_matches_class c inner)

  | Union sets ->
      List.exists (char_matches_class c) sets

and match_repeat_loop inner text pos count min_count max_count =
  if count = max_count then
    true
  else if count >= min_count && not (match_from_pos inner text pos) then
    true
  else if match_from_pos inner text pos then
    match_repeat_loop inner text (pos + 1) (count + 1) min_count max_count
  else if count >= min_count then
    true
  else
    false

(* テスト例 *)
let test_examples () =
  let examples = [
    ("a+", "aaa", true);
    ("a+", "b", false);
    ("[0-9]+", "123", true);
    ("[0-9]+", "abc", false);
    ("\\d{2,4}", "12", true);
    ("\\d{2,4}", "12345", true);
    ("(abc)+", "abcabc", true);
    ("a|b", "a", true);
    ("a|b", "b", true);
    ("a|b", "c", false);
    ("^hello$", "hello", true);
    ("^hello$", "hello world", false);
  ] in

  List.iter (fun (pattern, text, expected) ->
    match parse_regex pattern with
    | Some regex ->
        let result = match_regex regex text in
        let status = if result = expected then "✓" else "✗" in
        Printf.printf "%s パターン: '%s', テキスト: '%s', 期待: %b, 結果: %b\n"
          status pattern text expected result
    | None ->
        Printf.printf "✗ パーサーエラー: %s\n" pattern
  ) examples

(* 実行 *)
let () = test_examples ()