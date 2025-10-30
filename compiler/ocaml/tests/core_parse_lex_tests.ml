(* core_parse_lex_tests.ml — Core.Parse.Lex 橋渡しユニットテスト
 *
 * LEXER-002 Step5:
 * - プロファイル設定（strict_json / json_relaxed / toml_relaxed）が
 *   `Lexer` へ正しく適用されること
 * - `Api.symbol` が前後トリビアを読み飛ばし、記号ミスマッチ時に
 *   `Lexer_error` を送出すること
 * - `leading` 経由で `json_relaxed` が shebang を許容し、
 *   `toml_relaxed` が `#` コメントを共有できること
 *)

open Parser_run_config

module Lex_profile = Lex
module Trivia = Lex.Trivia_profile

let pass desc = Printf.printf "✓ %s\n" desc

let fail desc msg =
  Printf.printf "✗ %s: %s\n" desc msg;
  exit 1

let expect predicate desc =
  if predicate () then pass desc else fail desc "期待した条件を満たしませんでした"

let expect_token desc expected actual =
  if expected = actual then pass desc
  else
    fail desc
      (Printf.sprintf "期待: %s / 実際: %s" (Token.to_string expected)
         (Token.to_string actual))

let expect_lexer_error desc thunk =
  try
    let _ = thunk () in
    fail desc "Lexer_error が発生しませんでした"
  with
  | Lexer.Lexer_error _ -> pass desc
  | exn ->
      fail desc
        (Printf.sprintf "想定外の例外: %s" (Printexc.to_string exn))

let reset_profile () = Lexer.set_trivia_profile Trivia.strict_json

let pack_for_profile profile =
  let config = Lex.set_profile default profile in
  let pack, _ = Core_parse_lex.Bridge.derive config in
  pack

let read_token lexbuf =
  let token, _, _ = Lexer.read_token lexbuf in
  token

let test_config_trivia_profiles () =
  let desc = "config_trivia が Trivia_profile を適用する" in
  reset_profile ();
  let verify profile expected =
    let pack = pack_for_profile profile in
    let lexbuf = Lexing.from_string "" in
    Core_parse_lex.Api.config_trivia pack lexbuf;
    let actual = Lexer.current_trivia_profile () in
    Bool.equal actual.Trivia.shebang expected.Trivia.shebang
    && Bool.equal actual.Trivia.hash_inline expected.Trivia.hash_inline
    && actual.Trivia.line = expected.Trivia.line
  in
  let results =
    [
      ( Lex.Strict_json,
        Trivia.strict_json,
        "strict_json プロファイル適用" );
      ( Lex.Json_relaxed,
        Trivia.json_relaxed,
        "json_relaxed プロファイル適用" );
      ( Lex.Toml_relaxed,
        Trivia.toml_relaxed,
        "toml_relaxed プロファイル適用" );
    ]
  in
  List.iter
    (fun (profile, expected, label) ->
      expect
        (fun () -> verify profile expected)
        (Printf.sprintf "%s" label))
    results;
  reset_profile ();
  pass desc

let test_strict_json_rejects_shebang () =
  reset_profile ();
  let pack, _ = Core_parse_lex.Bridge.derive default in
  let lexbuf = Lexing.from_string "#!/usr/bin/env reml\n42" in
  expect_lexer_error "strict_json で shebang を拒否する" (fun () ->
      let _ = Core_parse_lex.Api.leading pack read_token lexbuf in
      ())

let test_json_relaxed_allows_shebang () =
  reset_profile ();
  let pack = pack_for_profile Lex.Json_relaxed in
  let lexbuf = Lexing.from_string "#!/usr/bin/env reml\n42" in
  let token, _, _ =
    Core_parse_lex.Api.leading pack Lexer.read_token lexbuf
  in
  expect_token "json_relaxed で shebang 後の INT を取得する"
    (Token.INT ("42", Ast.Base10))
    token

let test_toml_relaxed_skips_hash_comment () =
  reset_profile ();
  let pack = pack_for_profile Lex.Toml_relaxed in
  let lexbuf = Lexing.from_string "# comment\nvalue" in
  let token, _, _ =
    Core_parse_lex.Api.leading pack Lexer.read_token lexbuf
  in
  expect_token "toml_relaxed が # コメントを読み飛ばす" (Token.IDENT "value")
    token

let test_symbol_consumes_trivia () =
  reset_profile ();
  let pack, _ = Core_parse_lex.Bridge.derive default in
  let lexbuf = Lexing.from_string "   =  value" in
  Core_parse_lex.Api.symbol pack "=" lexbuf;
  let next_token, _, _ = Lexer.read_token lexbuf in
  expect_token "symbol が後続トリビアを消費する" (Token.IDENT "value")
    next_token

let test_symbol_mismatch_raises () =
  reset_profile ();
  let pack, _ = Core_parse_lex.Bridge.derive default in
  let lexbuf = Lexing.from_string " +" in
  expect_lexer_error "symbol ミスマッチで Lexer_error が発生する" (fun () ->
      Core_parse_lex.Api.symbol pack ";" lexbuf)

let () =
  let tests =
    [
      test_config_trivia_profiles;
      test_strict_json_rejects_shebang;
      test_json_relaxed_allows_shebang;
      test_toml_relaxed_skips_hash_comment;
      test_symbol_consumes_trivia;
      test_symbol_mismatch_raises;
    ]
  in
  List.iter (fun fn -> fn ()) tests
