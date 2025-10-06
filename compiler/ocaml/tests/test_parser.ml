(* test_parser.ml — Parser ユニットテスト
 *
 * 構文解析の基本機能と成功ケースを検証する。
 *)

open Ast

(* テストヘルパー関数 *)

let parse_string s =
  let lexbuf = Lexing.from_string s in
  try
    Some (Parser.compilation_unit Lexer.token lexbuf)
  with
  | Parser.Error -> None
  | Lexer.Lexer_error _ -> None

let expect_ok desc input =
  match parse_string input with
  | Some _ ->
      Printf.printf "✓ %s\n" desc
  | None ->
      Printf.printf "✗ %s: parse failed\n" desc;
      exit 1

let expect_fail desc input =
  match parse_string input with
  | None ->
      Printf.printf "✓ %s\n" desc
  | Some _ ->
      Printf.printf "✗ %s: expected parse failure but succeeded\n" desc;
      exit 1

let expect_decl_count desc expected input =
  match parse_string input with
  | Some cu ->
      let actual = List.length cu.decls in
      if actual = expected then
        Printf.printf "✓ %s\n" desc
      else begin
        Printf.printf "✗ %s: expected %d decls, got %d\n" desc expected actual;
        exit 1
      end
  | None ->
      Printf.printf "✗ %s: parse failed\n" desc;
      exit 1

let expect_use_count desc expected input =
  match parse_string input with
  | Some cu ->
      let actual = List.length cu.uses in
      if actual = expected then
        Printf.printf "✓ %s\n" desc
      else begin
        Printf.printf "✗ %s: expected %d uses, got %d\n" desc expected actual;
        exit 1
      end
  | None ->
      Printf.printf "✗ %s: parse failed\n" desc;
      exit 1

(* ========== モジュールヘッダテスト ========== *)

let test_module_header () =
  expect_ok "module header: simple" "module test.simple";
  expect_ok "module header: root" "module ::core.parse";
  expect_ok "module header with decls" "module test\n\nlet x = 42"

(* ========== use 宣言テスト ========== *)

let test_use_decls () =
  expect_use_count "use: simple" 1 "use ::Core.Parse";
  expect_use_count "use: alias" 1 "use ::Core.Parse as P";
  expect_use_count "use: brace" 1 "use Core.{Lex, Op}";
  expect_use_count "use: multiple" 2 "use ::Core.Parse\nuse ::Core.Lex";
  expect_ok "use: pub" "pub use ::Core.Parse"

(* ========== let/var 宣言テスト ========== *)

let test_let_var () =
  expect_decl_count "let: simple" 1 "let x = 42";
  expect_decl_count "let: with type" 1 "let x: i64 = 42";
  expect_decl_count "let: pattern tuple" 1 "let (x, y) = (1, 2)";
  expect_decl_count "var: mutable" 1 "var count = 0";
  expect_ok "pub let" "pub let x = 42"

(* ========== 関数宣言テスト ========== *)

let test_fn_decls () =
  expect_decl_count "fn: no params" 1 "fn answer() = 42";
  expect_decl_count "fn: with params" 1 "fn add(x, y) = x + y";
  expect_decl_count "fn: with return type" 1 "fn add(x, y) -> i64 = x + y";
  expect_decl_count "fn: with block" 1 "fn fact(n) { if n <= 1 then 1 else n * fact(n - 1) }";
  expect_ok "fn: generic params" "fn identity<T>(x: T) -> T = x";
  expect_ok "fn: default arg" "fn greet(name = \"World\") = name"

(* ========== 型宣言テスト ========== *)

let test_type_decls () =
  expect_decl_count "type: alias" 1 "type alias UserId = i64";
  expect_decl_count "type: newtype" 1 "type UserId = new i64";
  expect_decl_count "type: sum" 1 "type Option<T> = Some(T) | None";
  expect_ok "type: record variant" "type Point = Point { x: i64, y: i64 }"

(* ========== trait 宣言テスト ========== *)

let test_trait_decls () =
  expect_decl_count "trait: simple" 1 "trait Show { fn show(self) -> Str }";
  expect_ok "trait: generic" "trait Eq<T> { fn eq(self, other: T) -> Bool }";
  expect_ok "trait: where clause" "trait Clone where Self: Sized { fn clone(self) -> Self }"

(* ========== impl 宣言テスト ========== *)

let test_impl_decls () =
  expect_decl_count "impl: inherent" 1 "impl Point { fn new() = Point { x: 0, y: 0 } }";
  expect_ok "impl: trait for type" "impl Show for i64 { fn show(self) = \"int\" }";
  expect_ok "impl: generic" "impl<T> Show for Vec<T> { fn show(self) = \"vec\" }"

(* ========== extern 宣言テスト ========== *)

let test_extern_decls () =
  expect_decl_count "extern: single fn" 1 "extern \"C\" fn puts(s: Str) -> i32;";
  expect_ok "extern: block" "extern \"C\" { fn malloc(size: usize) -> Ptr<u8>; }"

(* ========== 式のテスト ========== *)

let test_exprs () =
  expect_ok "expr: literal int" "let _ = 42";
  expect_ok "expr: literal string" "let _ = \"hello\"";
  expect_ok "expr: binary" "let _ = 1 + 2 * 3";
  expect_ok "expr: pipe" "let _ = x |> f |> g";
  expect_ok "expr: call" "let _ = add(1, 2)";
  expect_ok "expr: call named arg" "let _ = greet(name = \"Alice\")";
  expect_ok "expr: field access" "let _ = point.x";
  expect_ok "expr: tuple access" "let _ = tuple.0";
  expect_ok "expr: index" "let _ = arr[0]";
  expect_ok "expr: propagate" "let _ = try_parse()?";
  expect_ok "expr: if-then-else" "let _ = if x > 0 then x else -x";
  expect_ok "expr: lambda" "let _ = |x, y| x + y";
  expect_ok "expr: match" "let _ = match x with | Some(v) -> v | None -> 0";
  expect_ok "expr: while" "let _ = while cond { body }";
  expect_ok "expr: for" "let _ = for item in list { process(item) }";
  expect_ok "expr: loop" "let _ = loop { break }";
  expect_ok "expr: block" "let _ = { let x = 1; x + 1 }";
  expect_ok "expr: unsafe" "let _ = unsafe { raw_ptr_deref(p) }";
  expect_ok "expr: return" "fn f() { return 42 }";
  expect_ok "expr: defer" "fn f() { defer cleanup(); work() }"

(* ========== パターンマッチテスト ========== *)

let test_patterns () =
  expect_ok "pattern: var" "let x = 42";
  expect_ok "pattern: wildcard" "let _ = 42";
  expect_ok "pattern: tuple" "let (x, y) = (1, 2)";
  expect_ok "pattern: constructor" "match opt with | Some(x) -> x";
  expect_ok "pattern: record" "let { x, y } = point";
  expect_ok "pattern: record rest" "let { x, .. } = point";
  expect_ok "pattern: guard" "match x with | n if n > 0 -> n"

(* ========== 属性テスト ========== *)

let test_attributes () =
  expect_ok "attribute: simple" "@inline fn fast() = 42";
  expect_ok "attribute: with args" "@dsl_export(\"parser\") fn entry() = rule"

(* ========== エラーケーステスト ========== *)

let test_error_cases () =
  expect_fail "error: unclosed paren" "let x = (1 + 2";
  expect_fail "error: missing expr" "let x = ";
  expect_fail "error: invalid token" "let x = @@@";
  expect_fail "error: unclosed string" "let x = \"hello"

(* ========== 統合テスト ========== *)

let test_integration () =
  let simple_program = {|
module test.simple

use ::Core.Parse

let answer = 42

fn add(x, y) -> i64 = x + y

fn fact(n) -> i64 {
  if n <= 1 then 1 else n * fact(n - 1)
}

let result = add(10, 20)
|} in
  expect_ok "integration: simple program" simple_program;
  expect_decl_count "integration: decl count" 4 simple_program;
  expect_use_count "integration: use count" 1 simple_program

(* ========== メイン ========== *)

let () =
  Printf.printf "Running Parser Unit Tests\n";
  Printf.printf "=========================\n\n";

  Printf.printf "Module Headers:\n";
  test_module_header ();
  Printf.printf "\n";

  Printf.printf "Use Declarations:\n";
  test_use_decls ();
  Printf.printf "\n";

  Printf.printf "Let/Var Declarations:\n";
  test_let_var ();
  Printf.printf "\n";

  Printf.printf "Function Declarations:\n";
  test_fn_decls ();
  Printf.printf "\n";

  Printf.printf "Type Declarations:\n";
  test_type_decls ();
  Printf.printf "\n";

  Printf.printf "Trait Declarations:\n";
  test_trait_decls ();
  Printf.printf "\n";

  Printf.printf "Impl Declarations:\n";
  test_impl_decls ();
  Printf.printf "\n";

  Printf.printf "Extern Declarations:\n";
  test_extern_decls ();
  Printf.printf "\n";

  Printf.printf "Expressions:\n";
  test_exprs ();
  Printf.printf "\n";

  Printf.printf "Patterns:\n";
  test_patterns ();
  Printf.printf "\n";

  Printf.printf "Attributes:\n";
  test_attributes ();
  Printf.printf "\n";

  Printf.printf "Error Cases:\n";
  test_error_cases ();
  Printf.printf "\n";

  Printf.printf "Integration Tests:\n";
  test_integration ();
  Printf.printf "\n";

  Printf.printf "=========================\n";
  Printf.printf "All Parser tests passed!\n"
