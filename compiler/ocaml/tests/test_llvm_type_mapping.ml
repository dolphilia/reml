(* Test_llvm_type_mapping — LLVM 型マッピングのテスト (Phase 3)
 *
 * このファイルは docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §2.3 に基づき、
 * Reml 型から LLVM IR 型への変換が正しく行われることを検証する。
 *
 * テスト方針:
 * - プリミティブ型の全種類をテスト
 * - 複合型（タプル、レコード、配列）のテスト
 * - FAT pointer 構造の検証
 * - Tagged union（ADT）構造の検証
 * - サイズとアラインメントの検証
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §2.3
 * - docs/guides/llvm-integration-notes.md §5.1
 *)

open Types
open Type_mapping
open Target_config

(* ========== テストユーティリティ ========== *)

(** テストカウンタ *)
let test_count = ref 0
let pass_count = ref 0

(** テスト結果の記録 *)
let assert_equal name expected actual =
  test_count := !test_count + 1;
  if expected = actual then begin
    pass_count := !pass_count + 1;
    Printf.printf "✓ %s\n" name
  end else begin
    Printf.printf "✗ %s\n" name;
    Printf.printf "  期待値: %s\n" expected;
    Printf.printf "  実際値: %s\n" actual
  end

(** LLVM 型の文字列表現 *)
let lltype_to_string llty =
  Llvm.string_of_lltype llty

(* ========== プリミティブ型のテスト ========== *)

let test_primitive_types () =
  Printf.printf "\n=== プリミティブ型マッピングテスト ===\n";
  let ctx = create_context "test_primitives" in

  (* Bool → i1 *)
  let bool_ty = reml_type_to_llvm ctx ty_bool in
  assert_equal "Bool → i1"
    "i1"
    (lltype_to_string bool_ty);

  (* Char → i32 *)
  let char_ty = reml_type_to_llvm ctx ty_char in
  assert_equal "Char → i32"
    "i32"
    (lltype_to_string char_ty);

  (* 整数型 *)
  let i8_ty = reml_type_to_llvm ctx ty_i8 in
  assert_equal "i8 → i8"
    "i8"
    (lltype_to_string i8_ty);

  let i16_ty = reml_type_to_llvm ctx ty_i16 in
  assert_equal "i16 → i16"
    "i16"
    (lltype_to_string i16_ty);

  let i32_ty = reml_type_to_llvm ctx ty_i32 in
  assert_equal "i32 → i32"
    "i32"
    (lltype_to_string i32_ty);

  let i64_ty = reml_type_to_llvm ctx ty_i64 in
  assert_equal "i64 → i64"
    "i64"
    (lltype_to_string i64_ty);

  let isize_ty = reml_type_to_llvm ctx ty_isize in
  assert_equal "isize → i64 (x86_64)"
    "i64"
    (lltype_to_string isize_ty);

  (* 符号なし整数型 *)
  let u8_ty = reml_type_to_llvm ctx ty_u8 in
  assert_equal "u8 → i8"
    "i8"
    (lltype_to_string u8_ty);

  let u16_ty = reml_type_to_llvm ctx ty_u16 in
  assert_equal "u16 → i16"
    "i16"
    (lltype_to_string u16_ty);

  let u32_ty = reml_type_to_llvm ctx ty_u32 in
  assert_equal "u32 → i32"
    "i32"
    (lltype_to_string u32_ty);

  let u64_ty = reml_type_to_llvm ctx ty_u64 in
  assert_equal "u64 → i64"
    "i64"
    (lltype_to_string u64_ty);

  let usize_ty = reml_type_to_llvm ctx ty_usize in
  assert_equal "usize → i64 (x86_64)"
    "i64"
    (lltype_to_string usize_ty);

  (* 浮動小数型 *)
  let f32_ty = reml_type_to_llvm ctx ty_f32 in
  assert_equal "f32 → float"
    "float"
    (lltype_to_string f32_ty);

  let f64_ty = reml_type_to_llvm ctx ty_f64 in
  assert_equal "f64 → double"
    "double"
    (lltype_to_string f64_ty);

  (* 単位型 *)
  let unit_ty = reml_type_to_llvm ctx ty_unit in
  assert_equal "() → void"
    "void"
    (lltype_to_string unit_ty)

(* ========== 複合型のテスト ========== *)

let test_composite_types () =
  Printf.printf "\n=== 複合型マッピングテスト ===\n";
  let ctx = create_context "test_composites" in

  (* タプル型 (i64, Bool) *)
  let tuple_ty = reml_type_to_llvm ctx (TTuple [ty_i64; ty_bool]) in
  assert_equal "タプル (i64, Bool) → { i64, i1 }"
    "{ i64, i1 }"
    (lltype_to_string tuple_ty);

  (* タプル型 (i32, f64, Bool) *)
  let tuple3_ty = reml_type_to_llvm ctx (TTuple [ty_i32; ty_f64; ty_bool]) in
  assert_equal "タプル (i32, f64, Bool) → { i32, double, i1 }"
    "{ i32, double, i1 }"
    (lltype_to_string tuple3_ty);

  (* レコード型 { x: i64, y: i64 } *)
  let record_ty = reml_type_to_llvm ctx (TRecord [("x", ty_i64); ("y", ty_i64)]) in
  assert_equal "レコード { x: i64, y: i64 } → { i64, i64 }"
    "{ i64, i64 }"
    (lltype_to_string record_ty);

  (* 配列型 [i32] (スライス) *)
  let slice_ty = reml_type_to_llvm ctx (TArray ty_i32) in
  let slice_str = lltype_to_string slice_ty in
  (* FAT pointer: { ptr, i64 } *)
  assert (String.length slice_str > 0 && String.contains slice_str '{');

  (* 固定長配列 [i64; 5] *)
  let array_ty = reml_type_to_llvm ctx (TSlice (ty_i64, Some 5)) in
  assert_equal "固定長配列 [i64; 5] → [5 x i64]"
    "[5 x i64]"
    (lltype_to_string array_ty)

(* ========== 関数型のテスト ========== *)

let test_function_types () =
  Printf.printf "\n=== 関数型マッピングテスト ===\n";
  let ctx = create_context "test_functions" in

  (* 関数型 i64 -> i64 *)
  let fn_ty = reml_type_to_llvm ctx (TArrow (ty_i64, ty_i64)) in
  assert_equal "関数型 i64 -> i64"
    "i64 (i64)"
    (lltype_to_string fn_ty);

  (* 関数型 Bool -> () *)
  let fn_void_ty = reml_type_to_llvm ctx (TArrow (ty_bool, ty_unit)) in
  assert_equal "関数型 Bool -> ()"
    "void (i1)"
    (lltype_to_string fn_void_ty);

  (* 関数型 (i32, i32) -> i64 *)
  let fn_tuple_ty = reml_type_to_llvm ctx (TArrow (TTuple [ty_i32; ty_i32], ty_i64)) in
  assert_equal "関数型 (i32, i32) -> i64"
    "i64 ({ i32, i32 })"
    (lltype_to_string fn_tuple_ty)

(* ========== FAT pointer のテスト ========== *)

let test_fat_pointer () =
  Printf.printf "\n=== FAT pointer 構造テスト ===\n";
  let ctx = create_context "test_fat_pointer" in

  (* String → { ptr, i64 } *)
  let string_ty = reml_type_to_llvm ctx ty_string in
  let string_str = lltype_to_string string_ty in
  assert (String.length string_str > 0 && String.contains string_str '{');

  (* [Bool] → { ptr, i64 } *)
  let bool_slice_ty = reml_type_to_llvm ctx (TArray ty_bool) in
  let bool_slice_str = lltype_to_string bool_slice_ty in
  assert (String.length bool_slice_str > 0 && String.contains bool_slice_str '{')

(* ========== サイズとアラインメントのテスト ========== *)

let test_size_and_alignment () =
  Printf.printf "\n=== 型サイズとアラインメントテスト ===\n";
  let ctx = create_context "test_sizes" in

  (* プリミティブ型のサイズ *)
  assert_equal "Bool サイズ (1 byte)"
    "1"
    (string_of_int (get_type_size ctx ty_bool));

  assert_equal "i32 サイズ (4 bytes)"
    "4"
    (string_of_int (get_type_size ctx ty_i32));

  assert_equal "i64 サイズ (8 bytes)"
    "8"
    (string_of_int (get_type_size ctx ty_i64));

  assert_equal "f64 サイズ (8 bytes)"
    "8"
    (string_of_int (get_type_size ctx ty_f64));

  (* FAT pointer のサイズ（ptr + len = 16 bytes） *)
  assert_equal "String サイズ (16 bytes: FAT pointer)"
    "16"
    (string_of_int (get_type_size ctx ty_string));

  (* アラインメント *)
  assert_equal "i64 アラインメント (8 bytes)"
    "8"
    (string_of_int (get_type_alignment ctx ty_i64));

  assert_equal "f64 アラインメント (8 bytes)"
    "8"
    (string_of_int (get_type_alignment ctx ty_f64))

(* ========== ターゲット設定のテスト ========== *)

let test_target_config () =
  Printf.printf "\n=== ターゲット設定テスト ===\n";

  (* デフォルトターゲット *)
  let default = Target_config.default_target in
  assert_equal "デフォルトターゲットトリプル"
    "x86_64-unknown-linux-gnu"
    default.triple;

  assert_equal "デフォルト DataLayout"
    "e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64"
    default.datalayout;

  (* ポインタサイズ *)
  assert_equal "x86_64 ポインタサイズ (64 bits)"
    "64"
    (string_of_int (pointer_size_bits "x86_64-unknown-linux-gnu"));

  assert_equal "x86_64 ポインタサイズ (8 bytes)"
    "8"
    (string_of_int (pointer_size_bytes "x86_64-unknown-linux-gnu"));

  (* アラインメント仕様 *)
  let align_spec = get_alignment_spec "x86_64-unknown-linux-gnu" in
  assert_equal "x86_64 i64 アラインメント (8 bytes)"
    "8"
    (string_of_int align_spec.i64_align);

  assert_equal "x86_64 ポインタアラインメント (8 bytes)"
    "8"
    (string_of_int align_spec.ptr_align)

(* ========== メインテスト実行 ========== *)

let () =
  Printf.printf "LLVM 型マッピングテスト開始\n";
  Printf.printf "=====================================\n";

  test_primitive_types ();
  test_composite_types ();
  test_function_types ();
  test_fat_pointer ();
  test_size_and_alignment ();
  test_target_config ();

  Printf.printf "\n=====================================\n";
  Printf.printf "テスト結果: %d/%d 成功\n" !pass_count !test_count;

  if !pass_count = !test_count then begin
    Printf.printf "✅ 全てのテストが成功しました!\n";
    exit 0
  end else begin
    Printf.printf "❌ %d 件のテストが失敗しました\n" (!test_count - !pass_count);
    exit 1
  end
