(* パターンマッチング専用テストスイート
 *
 * Phase 1 の M1 マイルストーンで求められるパターンマッチ機能の
 * 完全性を検証するための包括的テストケース。
 *
 * テスト対象:
 * - ネストパターン（2層・3層以上）
 * - ガード条件（束縛変数の使用、複数条件）
 * - リテラルパターンの網羅性
 * - レコードパターンの rest とネストの組み合わせ
 *)

open Parser_driver

(* ========== テストヘルパー ========== *)

let expect_ok name src =
  match parse_string src with
  | Result.Ok _ -> Printf.printf "✓ %s\n" name
  | Result.Error diag ->
      Printf.printf "✗ %s: %s\n" name diag.Diagnostic.message;
      exit 1

(* 既知の失敗挙動を明示的に検証するヘルパー *)
let _expect_fail name src =
  match parse_string src with
  | Result.Ok _ ->
      Printf.printf "✗ %s: 失敗を期待していましたが成功しました\n" name;
      exit 1
  | Result.Error diag ->
      let open Diagnostic in
      let loc = diag.primary.start_pos in
      Printf.printf "✓ %s (期待通り失敗: %s @ %d:%d)\n" name diag.message loc.line
        loc.column

(* ========== ネストパターンテスト ========== *)

let test_nested_constructors () =
  Printf.printf "Nested Constructor Patterns:\n";

  (* 2層ネスト *)
  expect_ok "nested: Some(Some(x))"
    {|
let _ = match opt with
| Some(Some(x)) -> x
| Some(None) -> 0
| None -> -1
|};

  expect_ok "nested: Ok(Some(x))"
    {|
let _ = match result with
| Ok(Some(value)) -> value
| Ok(None) -> 0
| Err(msg) -> -1
|};

  (* 3層ネスト *)
  expect_ok "nested: Some(Ok(Some(x)))"
    {|
let _ = match triple_opt with
| Some(Ok(Some(x))) -> x
| Some(Ok(None)) -> 0
| Some(Err(_)) -> -1
| None -> -2
|};

  (* 複数引数コンストラクタのネスト *)
  expect_ok "nested: Pair(Some(a), Some(b))"
    {|
let _ = match pair with
| Pair(Some(a), Some(b)) -> a + b
| Pair(Some(a), None) -> a
| Pair(None, Some(b)) -> b
| Pair(None, None) -> 0
|}

(* ========== タプルパターンのネストテスト ========== *)

let test_nested_tuples () =
  Printf.printf "\nNested Tuple Patterns:\n";

  (* タプル内にコンストラクタ *)
  expect_ok "nested tuple: (Some(x), Some(y))"
    {|
let _ = match pair with
| (Some(x), Some(y)) -> x + y
| (Some(x), None) -> x
| (None, Some(y)) -> y
| (None, None) -> 0
|};

  (* 深いタプルネスト *)
  expect_ok "nested tuple: ((a, b), (c, d))"
    {|
let _ = match nested with
| ((a, b), (c, d)) -> a + b + c + d
|};

  expect_ok "nested tuple: (x, (y, (z, w)))"
    {|
let _ = match deep_nested with
| (x, (y, (z, w))) -> x + y + z + w
|};

  (* タプルとコンストラクタの混在 *)
  expect_ok "nested tuple+constructor: ((Some(a), b), c)"
    {|
let _ = match complex with
| ((Some(a), b), c) -> a + b + c
| ((None, b), c) -> b + c
|}

(* ========== レコードパターンのネストテスト ========== *)

let test_nested_records () =
  Printf.printf "\nNested Record Patterns:\n";

  (* レコード内のコンストラクタ *)
  expect_ok "nested record: { x: Some(value) }"
    {|
let _ = match record with
| { x: Some(value) } -> value
| { x: None } -> 0
|};

  (* レコード内のレコード *)
  expect_ok "nested record: { outer: { inner: value } }"
    {|
let _ = match nested_record with
| { outer: { inner: value } } -> value
|};

  (* レコード内のタプル *)
  expect_ok "nested record: { point: (x, y) }"
    {|
let _ = match record with
| { point: (x, y) } -> x + y
|};

  (* rest とネストの組み合わせ *)
  (* Note: 複数アームでのレコードパターン + コンストラクタ + rest の組み合わせは
   * Phase 1 では既知の制限として残す *)
  expect_ok "nested record with rest: { field: Some(x), .. }"
    {|
let _ = match record with
| { important: Some(x), .. } -> x
|}

(* ========== ガード条件の高度なテスト ========== *)

let test_guard_conditions () =
  Printf.printf "\nGuard Conditions:\n";

  (* 単純な束縛変数の使用 *)
  expect_ok "guard: simple binding"
    {|
let _ = match value with
| Some(x) if x > 10 -> "large"
| Some(x) if x > 0 -> "positive"
| Some(x) -> "non-positive"
| None -> "none"
|};

  (* 複数変数の参照 *)
  expect_ok "guard: multiple variables"
    {|
let _ = match pair with
| (x, y) if x > y -> "x bigger"
| (x, y) if x < y -> "y bigger"
| (x, y) -> "equal"
|};

  (* ネストパターン + ガード *)
  expect_ok "guard: nested pattern"
    {|
let _ = match nested with
| Some(Some(x)) if x != 0 -> x
| Some(Some(x)) -> 1
| Some(None) -> 0
| None -> -1
|};

  (* タプル分解 + ガード *)
  expect_ok "guard: tuple destructure"
    {|
let _ = match point with
| (x, y) if x == 0 && y == 0 -> "origin"
| (x, y) if x > 0 && y > 0 -> "quadrant1"
| (x, y) if x < 0 && y > 0 -> "quadrant2"
| (x, y) -> "other"
|};

  (* レコードパターン + ガード *)
  expect_ok "guard: record pattern"
    {|
let _ = match person with
| { age, name } if age >= 18 -> "adult"
| { age, name } -> "minor"
|};

  (* コンストラクタ + ガード (複雑) *)
  expect_ok "guard: constructor complex"
    {|
let _ = match result with
| Ok(value) if value > 100 -> "too large"
| Ok(value) if value >= 0 -> "ok"
| Ok(value) -> "negative"
| Err(msg) if msg == "timeout" -> "retry"
| Err(msg) -> "error"
|}

(* ========== リテラルパターンの網羅性テスト ========== *)

let test_literal_patterns () =
  Printf.printf "\nLiteral Patterns:\n";

  (* 整数リテラル *)
  expect_ok "literal: integer"
    {|
let _ = match code with
| 0 -> "success"
| 1 -> "warning"
| 2 -> "error"
| _ -> "unknown"
|};

  (* 文字列リテラル *)
  expect_ok "literal: string"
    {|
let _ = match status with
| "ok" -> 0
| "error" -> 1
| "pending" -> 2
| _ -> 3
|};

  (* 真偽値リテラル *)
  expect_ok "literal: boolean"
    {|
let _ = match flag with
| true -> 1
| false -> 0
|};

  (* 文字リテラル *)
  expect_ok "literal: char"
    {|
let _ = match ch with
| 'a' -> 1
| 'b' -> 2
| _ -> 0
|};

  (* 混在リテラル + 構造パターン *)
  expect_ok "literal: mixed with structure"
    {|
let _ = match value with
| Some(0) -> "zero"
| Some(1) -> "one"
| Some(n) -> "other"
| None -> "none"
|}

(* ========== 複雑な組み合わせテスト ========== *)

let test_complex_combinations () =
  Printf.printf "\nComplex Combinations:\n";

  (* 深いネスト + ガード + リテラル *)
  expect_ok "complex: deep nest + guard + literal"
    {|
let _ = match data with
| Some((Ok(0), Some("success"))) -> "perfect"
| Some((Ok(n), Some(msg))) if n > 0 -> "ok with value"
| Some((Ok(n), None)) -> "ok no message"
| Some((Err(code), Some(msg))) if code == 404 -> "not found"
| Some((Err(_), _)) -> "error"
| None -> "no data"
|};

  (* レコード + タプル + コンストラクタ *)
  expect_ok "complex: record + tuple + constructor"
    {|
let _ = match response with
| { status: Ok((code, msg)), data: Some(value) } -> value
| { status: Ok((code, msg)), data: None } -> 0
| { status: Err(err), data: _ } -> -1
|};

  (* 複数アーム、それぞれが複雑なパターン *)
  expect_ok "complex: multiple complex arms"
    {|
let _ = match input with
| Some({ x: (a, b), y: Some(c) }) if a + b > c -> "case1"
| Some({ x: (a, b), y: None }) if a > 0 -> "case2"
| Some({ x: _, y: Some(c) }) -> "case3"
| None -> "none"
|};

  (* ネストしたmatch式 *)
  expect_ok "complex: nested match expressions"
    {|
let _ = match outer with
| Some(inner) ->
  match inner with
  | Ok(value) -> value
  | Err(_) -> 0
| None -> -1
|}

(* ========== エッジケース・境界テスト ========== *)

let test_edge_cases () =
  Printf.printf "\nEdge Cases:\n";

  (* 空タプル *)
  expect_ok "edge: unit tuple" {|
let _ = match unit_val with
| () -> 0
|};

  (* ワイルドカードのみ *)
  expect_ok "edge: wildcard only" {|
let _ = match anything with
| _ -> 42
|};

  (* 単一変数束縛 *)
  expect_ok "edge: single variable" {|
let _ = match value with
| x -> x
|};

  (* 多重ワイルドカードとネスト *)
  expect_ok "edge: multiple wildcards nested"
    {|
let _ = match complex with
| Some((_, Some(_))) -> 1
| Some((_, None)) -> 2
| None -> 0
|};

  (* rest パターンの様々な位置 *)
  expect_ok "edge: rest in different positions"
    {|
let _ = match record with
| { x, .. } -> x
|};

  expect_ok "edge: rest with multiple fields"
    {|
let _ = match record with
| { x, y, .. } -> x + y
|}

(* ========== 既知の制限ケース検証 ========== *)

let test_record_pattern_limitations () =
  Printf.printf "\nRecord Pattern 制限ケース:\n";

  (* 成功するケースの再確認 *)
  expect_ok "record: 先頭フィールドが引数付きコンストラクタ"
    {|
let _ = match record with
| { x: Some(value), y } -> value + y
| _ -> 0
|};
  expect_ok "record: 先頭以外なら bare コンストラクタでも成功"
    {|
let _ = match record with
| { y, x: None } -> y
| _ -> 0
|};
  expect_ok "record: 後続フィールドを明示指定すれば成功"
    {|
let _ = match record with
| { x: None, y: value } -> value
| _ -> 0
|};

  (* 失敗するパターンの再現 *)
  expect_ok "record: 先頭 bare コンストラクタ + 短縮フィールド"
    {|
let _ = match record with
| { x: None, y } -> y
| _ -> 0
|};
  expect_ok "record: 先頭 bare コンストラクタ + rest"
    {|
let _ = match record with
| { x: None, .. } -> 0
| _ -> 1
|};
  expect_ok "let bind: 先頭 bare コンストラクタ + 短縮フィールド"
    {|
let { x: None, y } = record
|};
  expect_ok "for loop: 先頭 bare コンストラクタ + 短縮フィールド"
    {|
let _ = for { x: None, y } in records {
  use_value(y)
}
|}

(* ========== メイン実行 ========== *)

let () =
  Printf.printf "Running Pattern Matching Comprehensive Tests\n";
  Printf.printf "==============================================\n\n";

  test_nested_constructors ();
  test_nested_tuples ();
  test_nested_records ();
  test_guard_conditions ();
  test_literal_patterns ();
  test_complex_combinations ();
  test_edge_cases ();
  test_record_pattern_limitations ();

  Printf.printf "\n==============================================\n";
  Printf.printf "✓ All pattern matching tests passed!\n"
