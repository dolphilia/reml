(* test_parser.ml — Parser ユニットテスト
 *
 * 構文解析の基本機能と成功ケースを検証する。
 *)

open Ast

(* テストヘルパー関数 *)

let parse_string s = Parser_driver.parse_string s

let expect_ok desc input =
  match parse_string input with
  | Ok _ -> Printf.printf "✓ %s\n" desc
  | Error diag ->
      Printf.printf "✗ %s: parse failed\n" desc;
      Printf.printf "%s\n" (Diagnostic.to_string diag);
      exit 1

let expect_fail desc input =
  match parse_string input with
  | Error _ -> Printf.printf "✓ %s\n" desc
  | Ok _ ->
      Printf.printf "✗ %s: expected parse failure but succeeded\n" desc;
      exit 1

let expect_decl_count desc expected input =
  match parse_string input with
  | Ok cu ->
      let actual = List.length cu.decls in
      if actual = expected then Printf.printf "✓ %s\n" desc
      else (
        Printf.printf "✗ %s: expected %d decls, got %d\n" desc expected actual;
        exit 1)
  | Error diag ->
      Printf.printf "✗ %s: parse failed\n" desc;
      Printf.printf "%s\n" (Diagnostic.to_string diag);
      exit 1

let expect_use_count desc expected input =
  match parse_string input with
  | Ok cu ->
      let actual = List.length cu.uses in
      if actual = expected then Printf.printf "✓ %s\n" desc
      else (
        Printf.printf "✗ %s: expected %d uses, got %d\n" desc expected actual;
        exit 1)
  | Error diag ->
      Printf.printf "✗ %s: parse failed\n" desc;
      Printf.printf "%s\n" (Diagnostic.to_string diag);
      exit 1

let expect_fn_effects desc expected input =
  match parse_string input with
  | Ok cu -> (
      match cu.decls with
      | { decl_kind = FnDecl fn; _ } :: _ ->
          let actual =
            match fn.fn_effect_profile with
            | None -> None
            | Some info ->
                Some (List.map (fun id -> id.name) info.effect_declared)
          in
          if actual = expected then Printf.printf "✓ %s\n" desc
          else (
            let show = function
              | None -> "None"
              | Some tags -> "[" ^ String.concat ", " tags ^ "]"
            in
            Printf.printf "✗ %s: expected %s, got %s\n" desc
              (show expected) (show actual);
            exit 1)
      | _ ->
          Printf.printf "✗ %s: first decl is not a function\n" desc;
          exit 1)
  | Error diag ->
      Printf.printf "✗ %s: parse failed\n" desc;
      Printf.printf "%s\n" (Diagnostic.to_string diag);
      exit 1

let stage_requirement_to_string = function
  | StageExact id -> Printf.sprintf "Exact:%s" id.name
  | StageAtLeast id -> Printf.sprintf "AtLeast:%s" id.name

let expect_fn_stage desc expected input =
  match parse_string input with
  | Ok cu -> (
      match cu.decls with
      | { decl_kind = FnDecl fn; _ } :: _ ->
          let actual =
            match fn.fn_effect_profile with
            | None -> None
            | Some info -> Option.map stage_requirement_to_string info.effect_stage
          in
          if actual = expected then Printf.printf "✓ %s\n" desc
          else (
            let show = function
              | None -> "None"
              | Some v -> v
            in
            Printf.printf "✗ %s: expected %s, got %s\n" desc
              (show expected) (show actual);
            exit 1)
      | _ ->
          Printf.printf "✗ %s: first decl is not a function\n" desc;
          exit 1)
  | Error diag ->
      Printf.printf "✗ %s: parse failed\n" desc;
      Printf.printf "%s\n" (Diagnostic.to_string diag);
      exit 1

let expect_effect_ops desc expected input =
  match parse_string input with
  | Ok cu -> (
      match cu.decls with
      | { decl_kind = EffectDecl eff; _ } :: _ ->
          let ops = List.map (fun op -> op.op_name.name) eff.operations in
          if ops = expected then Printf.printf "✓ %s\n" desc
          else (
            Printf.printf "✗ %s: expected operations [%s], got [%s]\n" desc
              (String.concat ", " expected)
              (String.concat ", " ops);
            exit 1)
      | _ ->
          Printf.printf "✗ %s: first decl is not an effect\n" desc;
          exit 1)
  | Error diag ->
      Printf.printf "✗ %s: parse failed\n" desc;
      Printf.printf "%s\n" (Diagnostic.to_string diag);
      exit 1

type expected_handler_entry =
  | ExpectedOp of string * string list * int
  | ExpectedReturn of string * int

let expect_handler_entries desc expected input =
  match parse_string input with
  | Ok cu -> (
      match cu.decls with
      | { decl_kind = HandlerDecl handler; _ } :: _ ->
          let actual = handler.handler_entries in
          let rec compare expected actual =
            match (expected, actual) with
            | [], [] -> true
            | ( ExpectedOp (name, params, body_len) :: et,
                HandlerOperation op :: at ) ->
                let op_name = op.handler_op_name.name in
                let actual_params =
                  op.handler_op_params
                  |> List.map (fun param ->
                         match param.pat.pat_kind with
                         | PatVar id -> id.name
                         | _ -> "<non-var>")
                in
                if
                  op_name = name && actual_params = params
                  && List.length op.handler_op_body = body_len
                then compare et at
                else false
            | ExpectedReturn (name, body_len) :: et, HandlerReturn ret :: at ->
                let ret_name = ret.handler_return_name.name in
                if
                  ret_name = name
                  && List.length ret.handler_return_body = body_len
                then compare et at
                else false
            | _ -> false
          in
          if compare expected actual then Printf.printf "✓ %s\n" desc
          else (
            Printf.printf "✗ %s: handler entries mismatch\n" desc;
            exit 1)
      | _ ->
          Printf.printf "✗ %s: first decl is not a handler\n" desc;
          exit 1)
  | Error diag ->
      Printf.printf "✗ %s: parse failed\n" desc;
      Printf.printf "%s\n" (Diagnostic.to_string diag);
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
  expect_decl_count "var: mutable" 1 "var count = 0"

(* ========== 関数宣言テスト ========== *)

let test_fn_decls () =
  expect_decl_count "fn: no params" 1 "fn answer() = 42";
  expect_decl_count "fn: with params" 1 "fn add(x, y) = x + y";
  expect_decl_count "fn: with return type" 1 "fn add(x, y) -> i64 = x + y";
  expect_decl_count "fn: with block" 1
    "fn fact(n) { if n <= 1 then 1 else n * fact(n - 1) }";
  expect_ok "fn: generic params" "fn identity<T>(x: T) -> T = x";
  expect_ok "fn: default arg" "fn greet(name = \"World\") = name"

(* ========== 効果注釈テスト ========== *)

let test_effect_annotations () =
  expect_fn_effects "fn: effect annotation with tags"
    (Some [ "io"; "panic" ])
    "fn write_log() !{ io, panic } = panic(\"log\")";
  expect_fn_effects "fn: empty effect annotation" (Some [])
    "fn noop() !{} = 0";
  expect_fn_effects "fn: missing annotation" None "fn pure() = 1";
  expect_fail "fn: unterminated effect list" "fn bad() !{ io, panic = 1";
  expect_fn_stage "fn: requires_capability stage"
    (Some "Exact:experimental")
    "@requires_capability(\"experimental\")\nfn experimental_api() = 0";
  expect_fn_stage "fn: dsl_export default stage"
    (Some "AtLeast:stable")
    "@dsl_export\nfn exported_api() = 0";
  expect_fn_effects "fn: allows_effects attribute"
    (Some [ "io"; "audit" ])
    "@allows_effects(io, audit)\nfn attr_effect() = 0";
  expect_fn_stage "fn: allows_effects stage default"
    (Some "AtLeast:stable")
    "@allows_effects(io)\nfn attr_stage() = 0";
  expect_fn_effects "fn: handles attribute"
    (Some [ "io"; "panic" ])
    "@handles(io, panic)\nfn handle_attr() = 0";
  expect_fn_effects "fn: effect annotation merged with attribute"
    (Some [ "io"; "panic"; "audit" ])
    "@allows_effects(audit)\nfn combined() !{ io, panic } = 0";
  (* TODO: Parser to accept named allows_effects argument via @dsl_export. Currently構文エラーで失敗するため要改善。 *)
  expect_fail "fn: dsl_export named allows_effects"
    "@dsl_export(allows_effects = [io, audit])\nfn export_named() = 0";
  (* TODO: Parser should accept named argument for @handles; currently fails. *)
  expect_fail "fn: handles named argument"
    "@handles(effect = \"panic\")\nfn handle_named() = 0"

(* ========== 型宣言テスト ========== *)

let test_type_decls () =
  expect_decl_count "type: alias" 1 "type alias UserId = i64";
  expect_decl_count "type: newtype" 1 "type UserId = new i64";
  expect_decl_count "type: sum" 1 "type Option<T> = Some(T) | None";
  expect_ok "type: tuple variant" "type Point = Point(i64, i64)"

(* ========== trait 宣言テスト ========== *)

let test_trait_decls () =
  expect_decl_count "trait: simple" 1 "trait Show { fn show(self: Self) -> Str }";
  expect_decl_count "trait: generic" 1
    "trait Eq<T> { fn eq(self: Self, other: T) -> Bool }";
  expect_fail "trait: where clause (todo)"
    "trait Clone where Self: Sized { fn clone(self: Self) -> Self }"

(* ========== impl 宣言テスト ========== *)

let test_impl_decls () =
  expect_decl_count "impl: trait for type" 1
    "impl Show for i64 { fn show(self: i64) -> String = \"int\" }";
  expect_decl_count "impl: inherent" 1
    "impl Point { fn create() -> i64 = 42 }";
  expect_decl_count "impl: generic" 1
    "impl<T> Show for Vec<T> { fn show(self: Vec<T>) -> String = \"vec\" }"

(* ========== extern 宣言テスト ========== *)

let test_extern_decls () =
  expect_decl_count "extern: single fn" 1 "extern \"C\" fn puts(s: Str) -> i32;";
  expect_ok "extern: block"
    "extern \"C\" { fn malloc(size: usize) -> Ptr<u8>; }"

(* ========== 式のテスト ========== *)

let test_exprs () =
  expect_ok "expr: literal int" "let _ = 42";
  expect_ok "expr: literal string" "let _ = \"hello\"";
  expect_ok "expr: binary" "let _ = 1 + 2 * 3";
  expect_ok "expr: pipe" "let _ = x |> f |> g";
  expect_ok "expr: call" "let _ = add(1, 2)";
  expect_ok "expr: call named arg" "let _ = greet(name = \"Alice\")";
  expect_ok "expr: propagate" "let _ = try_parse()?";
  expect_ok "expr: if-then-else" "let _ = if x > 0 then x else -x";
  expect_ok "expr: lambda" "let _ = |x, y| x + y";
  expect_ok "expr: match" "let _ = match x with | Some(v) -> v | None -> 0";
  expect_ok "expr: while" "let _ = while cond { body }";
  expect_ok "expr: for" "let _ = for item in list { process(item) }";
  expect_ok "expr: unsafe" "let _ = unsafe { raw_ptr_deref(p) }";
  expect_ok "expr: return" "fn f() { return 42 }";
  expect_ok "expr: defer" "fn f() { defer cleanup(); work() }";
  expect_ok "expr: field access" "let _ = point.x";
  expect_ok "expr: tuple access" "let _ = tuple.0";
  expect_ok "expr: index" "let _ = arr[0]";
  expect_ok "expr: loop" "let _ = loop { work() }";
  (* ブロック式は `= expr` の形式で使える。関数ブロック本体 `{ ... }` とは異なる *)
  expect_ok "expr: block in fn" "fn f() { let x = 1; x + 1 }";
  expect_ok "expr: block standalone" "fn f() = unsafe { let x = 1; x + 1 }"

(* ========== match/while/for の複雑ケーステスト ========== *)

let test_control_flow_complex () =
  (* match 式の複雑なケース *)
  expect_ok "match: multiple arms"
    {|
let _ = match value with
  | 0 -> "zero"
  | 1 -> "one"
  | 2 -> "two"
  | _ -> "other"
|};
  expect_ok "match: nested patterns"
    {|
let _ = match pair with
  | (Some(x), Some(y)) -> x + y
  | (Some(x), None) -> x
  | (None, Some(y)) -> y
  | (None, None) -> 0
|};
  expect_ok "match: guard conditions"
    {|
let _ = match x with
  | n if n < 0 -> "negative"
  | n if n == 0 -> "zero"
  | n if n > 0 -> "positive"
|};
  expect_ok "match: nested match"
    {|
let _ = match outer with
  | Some(inner) -> match inner with
    | Left(x) -> x
    | Right(y) -> y
  | None -> 0
|};
  (* 単一アームも引き続き動作 *)
  expect_ok "match: single arm" "let _ = match opt with | Some(x) -> x";

  (* while 式の複雑なケース *)
  expect_ok "while: nested"
    {|
fn process() {
  while outer_cond {
    while inner_cond {
      work()
    }
  }
}
|};
  expect_ok "while: with side effects"
    {|
fn count() {
  var i = 0;
  while i < 10 {
    i := i + 1
  }
}
|};

  (* for 式の複雑なケース *)
  expect_ok "for: pattern destructure"
    {|
let _ = for (key, value) in map {
  process(key, value)
}
|};
  expect_ok "for: nested loops"
    {|
fn matrix() {
  for row in rows {
    for cell in row {
      process(cell)
    }
  }
}
|};
  expect_ok "for: option pattern"
    {|
let _ = for Some(x) in list {
  use_value(x)
}
|};

  (* loop 式の基本テスト *)
  expect_ok "loop: basic" "let _ = loop { work() }";
  expect_ok "loop: nested"
    {|
fn run() {
  loop {
    loop {
      inner_work()
    }
  }
}
|}

(* ========== パターンマッチテスト ========== *)

let test_patterns () =
  (* 基本パターン *)
  expect_ok "pattern: var" "let x = 42";
  expect_ok "pattern: wildcard" "let _ = 42";
  expect_ok "pattern: tuple" "let (x, y) = (1, 2)";
  expect_ok "pattern: constructor"
    "let _ = match opt with | Some(x) -> x | None -> 0";
  expect_ok "pattern: qualified constructor"
    {|
let _ = match value with
| Option.None -> 0
| Option.Some(x) -> x
|};
  expect_ok "pattern: DSL uppercase ident"
    {|
let _ = match node with
| DSL.Node(tag) -> process(tag)
| DSL.Leaf -> 0
|};
  expect_ok "pattern: record" "let { x, y } = point";
  expect_ok "pattern: record rest" "let { x, .. } = point";
  expect_ok "pattern: guard" "let _ = match x with | n if n > 0 -> n | _ -> 0";

  (* ネストパターンの追加テスト *)
  expect_ok "pattern: nested Some(Some(x))"
    {|
let _ = match opt with
| Some(Some(x)) -> x
| Some(None) -> 0
| None -> -1
|};
  expect_ok "pattern: nested tuple (Some(x), Some(y))"
    {|
let _ = match pair with
| (Some(x), Some(y)) -> x + y
| _ -> 0
|};
  expect_ok "pattern: nested record { x: Some(v) }"
    {|
let _ = match rec with
| { x: Some(value) } -> value
| { x: None } -> 0
|};

  (* ガード条件の追加テスト *)
  expect_ok "pattern: guard with multiple vars"
    {|
let _ = match pair with
| (x, y) if x > y -> x
| (x, y) -> y
|};
  expect_ok "pattern: guard on nested pattern"
    {|
let _ = match opt with
| Some(Some(x)) if x > 0 -> x
| _ -> 0
|}

(* ========== 属性テスト ========== *)

let test_attributes () =
  expect_ok "attribute: simple" "@inline fn fast() = 42";
  expect_ok "attribute: with args" "@dsl_export(\"parser\") fn entry() = rule"

(* ========== 効果・ハンドラ宣言テスト ========== *)

let test_effects_handlers () =
  let effect_src =
    {|
effect Console : io {
  @requires_capability(Log)
  operation write: Str -> Unit
  operation flush: Unit -> Unit
}
|}
  in
  expect_effect_ops "effect: operation names" [ "write"; "flush" ] effect_src;

  let handler_src =
    {|
handler ConsoleLogger {
  operation write(message, resume) {
    emit(message);
    resume(())
  }
  return value {
    value
  }
}
|}
  in
  expect_handler_entries "handler: operations and return"
    [
      ExpectedOp ("write", [ "message"; "resume" ], 2);
      ExpectedReturn ("value", 1);
    ]
    handler_src

(* ========== エラーケーステスト ========== *)

let test_error_cases () =
  expect_fail "error: unclosed paren" "let x = (1 + 2";
  expect_fail "error: missing expr" "let x = ";
  expect_fail "error: invalid token" "let x = @@@";
  expect_fail "error: unclosed string" "let x = \"hello"

let test_diagnostic_metadata () =
  match Parser_driver.parse_string "let x =" with
  | Result.Error diag ->
      if diag.Diagnostic.span.start_pos.line = 1 then
        Printf.printf "✓ diagnostic: start position captured\n"
      else (
        Printf.printf "✗ diagnostic metadata: unexpected line %d\n"
          diag.Diagnostic.span.start_pos.line;
        exit 1)
  | Result.Ok _ ->
      Printf.printf "✗ diagnostic metadata: expected parse failure\n";
      exit 1

(* ========== 統合テスト ========== *)

let test_integration () =
  let simple_program =
    {|
module test.simple

use ::Core.Parse

let answer = 42

fn add(x, y) -> i64 = x + y

fn fact(n) -> i64 {
  if n <= 1 then 1 else n * fact(n - 1)
}

let result = add(10, 20)
|}
  in
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

  Printf.printf "Effect Annotations:\n";
  test_effect_annotations ();
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

  Printf.printf "Control Flow (Complex):\n";
  test_control_flow_complex ();
  Printf.printf "\n";

  Printf.printf "Patterns:\n";
  test_patterns ();
  Printf.printf "\n";

  Printf.printf "Attributes:\n";
  test_attributes ();
  Printf.printf "\n";

  Printf.printf "Effects & Handlers:\n";
  test_effects_handlers ();
  Printf.printf "\n";

  Printf.printf "Error Cases:\n";
  test_error_cases ();
  Printf.printf "\n";

  Printf.printf "Diagnostics:\n";
  test_diagnostic_metadata ();
  Printf.printf "\n";

  Printf.printf "Integration Tests:\n";
  test_integration ();
  Printf.printf "\n";

  Printf.printf "=========================\n";
  Printf.printf "All Parser tests passed!\n"
