(* test_parser_expectation.ml — Parser_expectation ユニットテスト *)

open Parser_expectation

let ok desc = Printf.printf "✓ %s\n" desc

let fail desc details =
  Printf.printf "✗ %s\n%s\n" desc details;
  exit 1

let string_of_expectation = function
  | Diagnostic.Keyword kw -> Printf.sprintf "Keyword(%s)" kw
  | Diagnostic.Token sym -> Printf.sprintf "Token(%s)" sym
  | Diagnostic.Eof -> "Eof"
  | Diagnostic.Class name -> Printf.sprintf "Class(%s)" name
  | Diagnostic.Rule name -> Printf.sprintf "Rule(%s)" name
  | Diagnostic.Not text -> Printf.sprintf "Not(%s)" text
  | Diagnostic.Custom text -> Printf.sprintf "Custom(%s)" text
  | Diagnostic.TypeExpected ty -> Printf.sprintf "TypeExpected(%s)" ty
  | Diagnostic.TraitBound trait -> Printf.sprintf "TraitBound(%s)" trait

let expect_equal desc actual expected =
  if Stdlib.compare actual expected = 0 then ok desc
  else
    fail desc
      (Printf.sprintf "  expected: %s\n  actual: %s"
         (string_of_expectation expected)
         (string_of_expectation actual))

let expect_summary desc expectations expected_alternatives expected_key =
  let summary = summarize_with_defaults expectations in
  if summary.Diagnostic.message_key <> Some expected_key then
    fail desc
      (Printf.sprintf "  expected message_key=%s but got %s" expected_key
         (match summary.Diagnostic.message_key with
         | None -> "None"
         | Some k -> k))
  else
    let actual = summary.Diagnostic.alternatives in
    if Stdlib.compare actual expected_alternatives = 0 then ok desc
    else
      let expected_str =
        String.concat ", "
          (List.map string_of_expectation expected_alternatives)
      in
      let actual_str =
        String.concat ", " (List.map string_of_expectation actual)
      in
      fail desc
        (Printf.sprintf "  expected alternatives=[%s]\n  actual   =[%s]"
           expected_str actual_str)

let () =
  expect_equal "keyword token maps to Keyword"
    (expectation_of_token Token.LET)
    (Diagnostic.Keyword "let");
  expect_equal "operator token maps to Token"
    (expectation_of_token Token.PLUS)
    (Diagnostic.Token "+");
  expect_equal "integer literal maps to Class(integer-literal)"
    (expectation_of_token (Token.INT ("0", Ast.Base10)))
    (Diagnostic.Class "integer-literal");
  expect_equal "identifier token maps to Class(identifier)"
    (expectation_of_token (Token.IDENT "value"))
    (Diagnostic.Class "identifier");
  expect_equal "EOF token maps to Eof" (expectation_of_token Token.EOF)
    Diagnostic.Eof;
  expect_equal "nonterminal expectation wraps rule"
    (expectation_of_nonterminal "expression")
    (Diagnostic.Rule "expression");
  expect_equal "expectation_not wraps message"
    (expectation_not "digit")
    (Diagnostic.Not "digit");
  expect_equal "expectation_custom wraps message"
    (expectation_custom "custom-hint")
    (Diagnostic.Custom "custom-hint");
  let expectations =
    [
      expectation_of_token Token.LET;
      expectation_of_token (Token.IDENT "x");
      expectation_of_token Token.PLUS;
      expectation_of_token Token.LET;
    ]
  in
  expect_summary "summary order keyword > token > class" expectations
    [
      Diagnostic.Keyword "let";
      Diagnostic.Token "+";
      Diagnostic.Class "identifier";
    ] "parse.expected";
  (match humanize
           [
             Diagnostic.Keyword "let";
             Diagnostic.Token "+";
             Diagnostic.Class "identifier";
           ]
   with
  | Some text
    when String.equal text "ここで`let`、`+` または identifierのいずれかが必要です" ->
      ok "humanize joins alternatives in Japanese"
  | Some text ->
      fail "humanize joins alternatives in Japanese"
        (Printf.sprintf "  unexpected text: %s" text)
  | None ->
      fail "humanize joins alternatives in Japanese"
        "  expected Some but got None");
  let empty = summarize_with_defaults [] in
  (match empty.Diagnostic.message_key with
  | Some "parse.expected.empty" -> ok "empty summary uses empty message key"
  | other ->
      fail "empty summary uses empty message key"
        (Printf.sprintf "  unexpected message key: %s"
           (match other with None -> "None" | Some k -> k)));
  if empty.Diagnostic.alternatives = [] then
    ok "empty summary has no alternatives"
  else fail "empty summary has no alternatives" "  expected []";
  let ensured = ensure_minimum_alternatives empty in
  (match ensured.Diagnostic.alternatives with
  | [ Diagnostic.Custom label ] when String.equal label "解析継続トークン" ->
      ok "ensure_minimum_alternatives inserts placeholder"
  | other ->
      let actual =
        String.concat ", "
          (List.map string_of_expectation other)
      in
      fail "ensure_minimum_alternatives inserts placeholder"
        (Printf.sprintf "  unexpected alternatives: %s" actual));
  (match ensured.Diagnostic.message_key with
  | Some "parse.expected.empty" ->
      ok "ensure_minimum_alternatives keeps empty message key"
  | other ->
      fail "ensure_minimum_alternatives keeps empty message key"
        (Printf.sprintf "  unexpected message key: %s"
           (match other with None -> "None" | Some key -> key)))
