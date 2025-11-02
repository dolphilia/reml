(* unicode_ident_tests.ml — Phase 2-5 SYNTAX-001 Step3
 *
 * 目的:
 *   - Unicode 識別子の受理シナリオを整理し、Phase 2-7 `lexer-unicode` 実装後に即座に回帰検証できるようにする。
 *   - docs/spec/1-1-syntax.md §A.3、docs/spec/1-4-test-unicode-model.md の要件をもとに、
 *     代表的なスクリプト（日本語・ギリシャ語・キリル・ハングル・合成文字）を網羅したケースを準備する。
 *   - Phase 2-5 では ASCII 実装のためテストをスキップし、`REML_ENABLE_UNICODE_TESTS=1` を指定したときだけ実行する。
 *)

open Token

type ident_case = {
  label : string;
  source : string;
  expected_token : Token.t;
  normalized : string option;
}

let ident_case ?normalized:(normalized : string option = None) ~label ~source
    expected_token =
  { label; source; expected_token; normalized }

let acceptance_cases =
  [
    ident_case ~label:"Hiragana (docs/spec/1-1-syntax.md A.3)"
      ~source:"れむる" (IDENT "れむる");
    ident_case ~label:"Katakana with underscore" ~source:"ユーザー_識別子"
      (IDENT "ユーザー_識別子");
    ident_case ~label:"Greek upper identifier" ~source:"Δοκιμή"
      (UPPER_IDENT "Δοκιμή");
    ident_case ~label:"Cyrillic identifier" ~source:"пользователь"
      (IDENT "пользователь");
    ident_case ~label:"Hangul syllables" ~source:"데이터"
      (IDENT "데이터");
    ident_case ~label:"Latin with combining acute"
      ~source:"cafe\u{0301}" ~normalized:(Some "café") (IDENT "café");
  ]

let supplementary_cases =
  [
    ident_case ~label:"Greek module-style upper identifier"
      ~source:"Μέλος" (UPPER_IDENT "Μέλος");
    ident_case ~label:"Han + digits"
      ~source:"識別子123" (IDENT "識別子123");
    ident_case ~label:"Identifier with zero-width joiner"
      ~source:"مثال\u{200D}اختبار"
      (IDENT "مثال\u{200D}اختبار");
  ]

let lex_token_sequence source =
  let lexbuf = Lexing.from_string source in
  let first = Lexer.token lexbuf in
  let second = Lexer.token lexbuf in
  (first, second)

let string_of_token = Token.to_string

let check_identifier_case case =
  let expected_lexeme =
    match case.normalized with Some n -> n | None -> case.source
  in
  let first, second = lex_token_sequence case.source in
  let fail actual =
    Printf.printf "✗ %s\n  Expected: %s (%s)\n  Actual:   %s\n" case.label
      (string_of_token case.expected_token) expected_lexeme
      (string_of_token actual);
    exit 1
  in
  let verify token constructor =
    if second <> EOF then (
      Printf.printf "✗ %s\n  Lexer emitted extra token: %s\n" case.label
        (string_of_token second);
      exit 1);
    match constructor token with
    | Some actual when String.equal actual expected_lexeme ->
        Printf.printf "✓ %s → %s\n" case.label expected_lexeme
    | Some actual ->
        Printf.printf
          "✗ %s\n  Expected lexeme: %s\n  Actual lexeme:   %s\n" case.label
          expected_lexeme actual;
        exit 1
    | None -> fail token
  in
  match (case.expected_token, first) with
  | IDENT _, IDENT _ ->
      verify first (function IDENT s -> Some s | _ -> None)
  | UPPER_IDENT _, UPPER_IDENT _ ->
      verify first (function UPPER_IDENT s -> Some s | _ -> None)
  | _ -> fail first

let fixture_path = Test_support.sample_path "unicode_identifiers.reml"

let () =
  if not (Sys.file_exists fixture_path) then (
    Printf.printf "✗ Missing fixture: %s\n" fixture_path;
    exit 1)

let unicode_tests_enabled =
  match Sys.getenv_opt "REML_ENABLE_UNICODE_TESTS" with
  | Some env ->
      let lowered = String.lowercase_ascii env in
      List.exists (( = ) lowered) [ "1"; "true"; "yes"; "unicode"; "enable" ]
  | None -> false

let () =
  if not unicode_tests_enabled then (
    Printf.printf
      "[unicode_pending] Unicode identifier tests skipped. Set \
       REML_ENABLE_UNICODE_TESTS=1 to execute (SYNTAX-001 Step3).\n";
    exit 0);

  Printf.printf "Unicode identifier acceptance (SYNTAX-001 Step3)\n";
  List.iter check_identifier_case acceptance_cases;
  Printf.printf "\nSupplementary coverage\n";
  List.iter check_identifier_case supplementary_cases;
  Printf.printf "\nFixture verified at %s\n" fixture_path;
  Printf.printf "All Unicode identifier tests passed.\n"
