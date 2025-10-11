(* test_lexer.ml — Lexer ユニットテスト
 *
 * 字句解析の境界ケースと基本機能を検証する。
 *)

open Token

(* テストヘルパー関数 *)

let lex_string s =
  let lexbuf = Lexing.from_string s in
  let rec loop acc =
    match Lexer.token lexbuf with
    | EOF -> List.rev (EOF :: acc)
    | tok -> loop (tok :: acc)
  in
  loop []

let lex_one s =
  let lexbuf = Lexing.from_string s in
  Lexer.token lexbuf

let rec lex_all lexbuf =
  match Lexer.token lexbuf with EOF -> () | _ -> lex_all lexbuf

let expect_tokens desc expected actual =
  if expected = actual then Printf.printf "✓ %s\n" desc
  else (
    Printf.printf "✗ %s\n" desc;
    Printf.printf "  Expected: [%s]\n"
      (String.concat "; " (List.map Token.to_string expected));
    Printf.printf "  Actual:   [%s]\n"
      (String.concat "; " (List.map Token.to_string actual));
    exit 1)

let expect_token desc expected actual =
  if expected = actual then Printf.printf "✓ %s\n" desc
  else (
    Printf.printf "✗ %s\n" desc;
    Printf.printf "  Expected: %s\n" (Token.to_string expected);
    Printf.printf "  Actual:   %s\n" (Token.to_string actual);
    exit 1)

let expect_lexer_error desc input expected_prefix =
  let lexbuf = Lexing.from_string input in
  try
    lex_all lexbuf;
    Printf.printf "✗ %s: expected lexer error but none occurred\n" desc;
    exit 1
  with Lexer.Lexer_error (msg, span) ->
    if expected_prefix = "" || String.starts_with ~prefix:expected_prefix msg
    then Printf.printf "✓ %s (span %d-%d)\n" desc span.start span.end_
    else (
      Printf.printf "✗ %s: unexpected message '%s'\n" desc msg;
      exit 1)

(* ========== キーワードテスト ========== *)

let test_keywords () =
  expect_token "keyword: let" LET (lex_one "let");
  expect_token "keyword: fn" FN (lex_one "fn");
  expect_token "keyword: type" TYPE (lex_one "type");
  expect_token "keyword: module" MODULE (lex_one "module");
  expect_token "keyword: use" USE (lex_one "use");
  expect_token "keyword: if" IF (lex_one "if");
  expect_token "keyword: match" MATCH (lex_one "match");
  expect_token "keyword: while" WHILE (lex_one "while");
  expect_token "keyword: return" RETURN (lex_one "return")

(* ========== 識別子テスト ========== *)

let test_identifiers () =
  expect_token "identifier: simple" (IDENT "foo") (lex_one "foo");
  expect_token "identifier: underscore" (IDENT "_aux") (lex_one "_aux");
  expect_token "identifier: camelCase" (IDENT "parseExpr") (lex_one "parseExpr");
  expect_token "identifier: snake_case" (IDENT "parse_expr")
    (lex_one "parse_expr");
  expect_token "identifier: digits" (IDENT "var123") (lex_one "var123");
  (* Phase 1 では ASCII のみサポート *)
  (* expect_token "identifier: unicode" (IDENT "解析器") (lex_one "解析器") *)
  ()

(* ========== 整数リテラルテスト ========== *)

let test_integers () =
  expect_token "integer: decimal" (INT ("42", Ast.Base10)) (lex_one "42");
  expect_token "integer: binary" (INT ("1010", Ast.Base2)) (lex_one "0b1010");
  expect_token "integer: octal" (INT ("755", Ast.Base8)) (lex_one "0o755");
  expect_token "integer: hex" (INT ("FF", Ast.Base16)) (lex_one "0xFF");
  expect_token "integer: underscore"
    (INT ("1000000", Ast.Base10))
    (lex_one "1_000_000")

(* ========== 浮動小数リテラルテスト ========== *)

let test_floats () =
  expect_token "float: simple" (FLOAT "3.14") (lex_one "3.14");
  expect_token "float: exponent" (FLOAT "1e-9") (lex_one "1e-9");
  expect_token "float: underscore" (FLOAT "2048.0") (lex_one "2_048.0")

(* ========== 文字リテラルテスト ========== *)

let test_chars () =
  expect_token "char: simple" (CHAR "A") (lex_one "'A'");
  expect_token "char: escape newline" (CHAR "\n") (lex_one "'\\n'");
  expect_token "char: escape tab" (CHAR "\t") (lex_one "'\\t'");
  expect_token "char: escape backslash" (CHAR "\\") (lex_one "'\\\\'");
  expect_token "char: escape quote" (CHAR "'") (lex_one "'\\''")

(* ========== 文字列リテラルテスト ========== *)

let test_strings () =
  expect_token "string: normal"
    (STRING ("hello", Ast.Normal))
    (lex_one "\"hello\"");
  expect_token "string: escape"
    (STRING ("line1\nline2", Ast.Normal))
    (lex_one "\"line1\\nline2\"");
  expect_token "string: raw"
    (STRING ("\\n\\t", Ast.Raw))
    (lex_one "r\"\\n\\t\"");
  expect_token "string: multiline"
    (STRING ("line1\nline2", Ast.Multiline))
    (lex_one "\"\"\"line1\nline2\"\"\"")

(* ========== 演算子テスト ========== *)

let test_operators () =
  expect_token "operator: |>" PIPE (lex_one "|>");
  expect_token "operator: ~>" CHANNEL_PIPE (lex_one "~>");
  expect_token "operator: ->" ARROW (lex_one "->");
  expect_token "operator: =>" DARROW (lex_one "=>");
  expect_token "operator: :=" COLONEQ (lex_one ":=");
  expect_token "operator: ==" EQEQ (lex_one "==");
  expect_token "operator: !=" NE (lex_one "!=");
  expect_token "operator: <=" LE (lex_one "<=");
  expect_token "operator: >=" GE (lex_one ">=");
  expect_token "operator: &&" AND (lex_one "&&");
  expect_token "operator: ||" OR (lex_one "||")

(* ========== コメントテスト ========== *)

let test_comments () =
  expect_tokens "line comment"
    [ IDENT "before"; IDENT "after"; EOF ]
    (lex_string "before // comment\nafter");
  expect_tokens "block comment"
    [ IDENT "before"; IDENT "after"; EOF ]
    (lex_string "before /* comment */ after");
  expect_tokens "nested block comment"
    [ IDENT "before"; IDENT "after"; EOF ]
    (lex_string "before /* outer /* inner */ outer */ after")

(* ========== 複合トークン列テスト ========== *)

let test_token_sequence () =
  expect_tokens "let binding"
    [ LET; IDENT "x"; EQ; INT ("42", Ast.Base10); EOF ]
    (lex_string "let x = 42");
  expect_tokens "function call"
    [
      IDENT "add";
      LPAREN;
      INT ("1", Ast.Base10);
      COMMA;
      INT ("2", Ast.Base10);
      RPAREN;
      EOF;
    ]
    (lex_string "add(1, 2)");
  expect_tokens "pipe expression"
    [ IDENT "x"; PIPE; IDENT "f"; PIPE; IDENT "g"; EOF ]
    (lex_string "x |> f |> g")

(* ========== エラーテスト ========== *)

let test_lexer_errors () =
  expect_lexer_error "lexer error: unexpected char" "$invalid"
    "Unexpected character";
  expect_lexer_error "lexer error: unterminated string" "\"hello"
    "Unterminated string";
  expect_lexer_error "lexer error: unterminated comment" "/* not closed"
    "Unterminated block comment"

(* ========== メイン ========== *)

let () =
  Printf.printf "Running Lexer Unit Tests\n";
  Printf.printf "=========================\n\n";

  Printf.printf "Keywords:\n";
  test_keywords ();
  Printf.printf "\n";

  Printf.printf "Identifiers:\n";
  test_identifiers ();
  Printf.printf "\n";

  Printf.printf "Integers:\n";
  test_integers ();
  Printf.printf "\n";

  Printf.printf "Floats:\n";
  test_floats ();
  Printf.printf "\n";

  Printf.printf "Characters:\n";
  test_chars ();
  Printf.printf "\n";

  Printf.printf "Strings:\n";
  test_strings ();
  Printf.printf "\n";

  Printf.printf "Operators:\n";
  test_operators ();
  Printf.printf "\n";

  Printf.printf "Comments:\n";
  test_comments ();
  Printf.printf "\n";

  Printf.printf "Token Sequences:\n";
  test_token_sequence ();
  Printf.printf "\n";

  Printf.printf "Errors:\n";
  test_lexer_errors ();
  Printf.printf "\n";

  Printf.printf "=========================\n";
  Printf.printf "All Lexer tests passed!\n"
