(* Type_mapping — Reml 型から LLVM IR 型へのマッピング実装 (Phase 3)
 *
 * このファイルは docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §2.1-2.3 に基づき、
 * Reml 型システムと LLVM IR 型システムの対応付けを実装する。
 *
 * 実装方針:
 * - プリミティブ型: 直接対応（Bool → i1、i64 → i64 等）
 * - 複合型: 構造体表現（タプル、レコード、FAT pointer、tagged union）
 * - 型キャッシュ: 再帰的型定義に対応するためメモ化を使用
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §2
 * - docs/guides/llvm-integration-notes.md §5.1-5.2
 *)

open Types

(* ========== 型マッピングコンテキスト ========== *)

type llvm_context = Llvm.llcontext

type type_mapping_context = {
  llctx: llvm_context;
  llmodule: Llvm.llmodule;
  mutable type_cache: (ty, Llvm.lltype) Hashtbl.t;
}

let create_context module_name =
  let llctx = Llvm.global_context () in
  let llmodule = Llvm.create_module llctx module_name in
  {
    llctx;
    llmodule;
    type_cache = Hashtbl.create 128;
  }

(* ========== プリミティブ型マッピング ========== *)

(** プリミティブ型を LLVM 型に変換
 *
 * docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §2.1 参照
 *)
let rec reml_primitive_to_llvm ctx = function
  (* Bool → i1 *)
  | TCBool -> Llvm.i1_type ctx.llctx

  (* Char → i32 (Unicode scalar value) *)
  | TCChar -> Llvm.i32_type ctx.llctx

  (* String → FAT pointer { ptr, i64 } *)
  | TCString -> make_fat_pointer ctx None

  (* 整数型 → LLVM 整数型 *)
  | TCInt I8 -> Llvm.i8_type ctx.llctx
  | TCInt I16 -> Llvm.i16_type ctx.llctx
  | TCInt I32 -> Llvm.i32_type ctx.llctx
  | TCInt I64 -> Llvm.i64_type ctx.llctx
  | TCInt Isize -> Llvm.i64_type ctx.llctx  (* x86_64 では 64bit *)

  | TCInt U8 -> Llvm.i8_type ctx.llctx
  | TCInt U16 -> Llvm.i16_type ctx.llctx
  | TCInt U32 -> Llvm.i32_type ctx.llctx
  | TCInt U64 -> Llvm.i64_type ctx.llctx
  | TCInt Usize -> Llvm.i64_type ctx.llctx (* x86_64 では 64bit *)

  (* 浮動小数型 → LLVM 浮動小数型 *)
  | TCFloat F32 -> Llvm.float_type ctx.llctx
  | TCFloat F64 -> Llvm.double_type ctx.llctx

  (* ユーザ定義型（将来拡張） *)
  | TCUser name ->
      (* TODO: 型定義から構造を取得 *)
      (* 現在は opaque struct として扱う *)
      let struct_name = "struct." ^ name in
      Llvm.named_struct_type ctx.llctx struct_name

(** FAT pointer 型を作成
 *
 * スライス型や String 型で使用される { ptr, i64 } 構造体
 * element_ty が None の場合は i8* を使用（String の場合）
 *)
and make_fat_pointer ctx _element_ty_opt =
  (* LLVM 18+ では opaque pointer を使用（型付きポインタは廃止） *)
  let ptr_ty = Llvm.pointer_type ctx.llctx in
  let len_ty = Llvm.i64_type ctx.llctx in
  Llvm.struct_type ctx.llctx [| ptr_ty; len_ty |]

(** Tagged union 型を作成
 *
 * ADT で使用される { i32 tag, payload } 構造体
 *)
and make_tagged_union ctx payload_ty =
  let tag_ty = Llvm.i32_type ctx.llctx in
  Llvm.struct_type ctx.llctx [| tag_ty; payload_ty |]

(** クロージャ型を作成
 *
 * { env_ptr*, code_ptr } 構造体
 *)
and make_closure ctx _fn_ty =
  (* LLVM 18+ では opaque pointer を使用 *)
  let env_ptr_ty = Llvm.pointer_type ctx.llctx in
  let code_ptr_ty = Llvm.pointer_type ctx.llctx in
  Llvm.struct_type ctx.llctx [| env_ptr_ty; code_ptr_ty |]

(* ========== 複合型マッピング ========== *)

(** Reml 型を LLVM 型に変換（メイン関数）
 *
 * docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §2.1-2.2 参照
 *)
let rec reml_type_to_llvm ctx ty =
  (* 型キャッシュをチェック *)
  match Hashtbl.find_opt ctx.type_cache ty with
  | Some llty -> llty
  | None ->
      let llty = reml_type_to_llvm_impl ctx ty in
      Hashtbl.add ctx.type_cache ty llty;
      llty

and reml_type_to_llvm_impl ctx = function
  (* 型変数（エラー：型推論後は存在しないはず） *)
  | TVar tv ->
      failwith (Printf.sprintf "型変数 %s が LLVM IR 生成時に残存"
                  (string_of_type_var tv))

  (* 型定数 *)
  | TCon tc -> reml_primitive_to_llvm ctx tc

  (* 型適用（ジェネリック型） *)
  | TApp (constructor, _arg) ->
      (* TODO: ジェネリック型の展開 *)
      (* 現在は constructor を評価（モノモルフィゼーション前提） *)
      reml_type_to_llvm ctx constructor

  (* 関数型 A -> B *)
  | TArrow (arg_ty, ret_ty) ->
      let arg_llty = reml_type_to_llvm ctx arg_ty in
      let ret_llty = reml_type_to_llvm ctx ret_ty in
      Llvm.function_type ret_llty [| arg_llty |]

  (* タプル型 (T1, T2, ..., Tn) *)
  | TTuple tys ->
      let lltys = Array.of_list (List.map (reml_type_to_llvm ctx) tys) in
      Llvm.struct_type ctx.llctx lltys

  (* レコード型 { x: T1, y: T2, ... } *)
  | TRecord fields ->
      let lltys = Array.of_list (List.map (fun (_, ty) ->
        reml_type_to_llvm ctx ty
      ) fields) in
      Llvm.struct_type ctx.llctx lltys

  (* 配列型 [T] (スライス、動的配列) *)
  | TArray element_ty ->
      let element_llty = reml_type_to_llvm ctx element_ty in
      make_fat_pointer ctx (Some element_llty)

  (* スライス型 [T; N] (固定長配列) *)
  | TSlice (element_ty, n_opt) ->
      let element_llty = reml_type_to_llvm ctx element_ty in
      begin match n_opt with
      | Some n -> Llvm.array_type element_llty n
      | None -> make_fat_pointer ctx (Some element_llty)
      end

  (* 単位型 () → void *)
  | TUnit -> Llvm.void_type ctx.llctx

  (* Never 型 → void (実際には到達不能) *)
  | TNever -> Llvm.void_type ctx.llctx

(* ========== 型サイズとアラインメント ========== *)

(** 型のサイズを取得（バイト単位）
 *
 * ターゲットマシンの DataLayout に基づく。
 * Phase 3 Week 12 では x86_64 固定で実装。
 *)
let rec get_type_size ctx ty =
  let _llty = reml_type_to_llvm ctx ty in
  (* LLVM の DataLayout から型サイズを取得 *)
  (* TODO: Llvm_target.DataLayout.size_of_type を使用 *)
  (* 現在は簡易実装 *)
  match ty with
  | TCon TCBool -> 1
  | TCon TCChar -> 4
  | TCon (TCInt I8) | TCon (TCInt U8) -> 1
  | TCon (TCInt I16) | TCon (TCInt U16) -> 2
  | TCon (TCInt I32) | TCon (TCInt U32) -> 4
  | TCon (TCInt I64) | TCon (TCInt U64) -> 8
  | TCon (TCInt Isize) | TCon (TCInt Usize) -> 8 (* x86_64 *)
  | TCon (TCFloat F32) -> 4
  | TCon (TCFloat F64) -> 8
  | TCon TCString -> 16  (* FAT pointer: { ptr(8), len(8) } *)
  | TArray _ -> 16       (* FAT pointer *)
  | TSlice (_, Some n) ->
      let elem_size = get_type_size ctx (match ty with TSlice (t, _) -> t | _ -> assert false) in
      elem_size * n
  | TSlice (_, None) -> 16  (* FAT pointer *)
  | TUnit -> 0
  | TNever -> 0
  | _ -> 8  (* デフォルト（ポインタサイズ） *)

(** 型のアラインメントを取得（バイト単位）
 *
 * System V ABI x86_64 のアラインメント規則に従う。
 *)
let rec get_type_alignment ctx ty =
  match ty with
  | TCon TCBool -> 1
  | TCon TCChar -> 4
  | TCon (TCInt I8) | TCon (TCInt U8) -> 1
  | TCon (TCInt I16) | TCon (TCInt U16) -> 2
  | TCon (TCInt I32) | TCon (TCInt U32) -> 4
  | TCon (TCInt I64) | TCon (TCInt U64) -> 8
  | TCon (TCInt Isize) | TCon (TCInt Usize) -> 8
  | TCon (TCFloat F32) -> 4
  | TCon (TCFloat F64) -> 8
  | TCon TCString -> 8   (* ポインタアラインメント *)
  | TArray _ -> 8
  | TSlice _ -> 8
  | TTuple tys ->
      (* タプルのアラインメントは最大要素のアラインメント *)
      List.fold_left (fun acc ty ->
        max acc (get_type_alignment ctx ty)
      ) 1 tys
  | TRecord fields ->
      (* レコードのアラインメントは最大フィールドのアラインメント *)
      List.fold_left (fun acc (_, ty) ->
        max acc (get_type_alignment ctx ty)
      ) 1 fields
  | TUnit -> 1
  | TNever -> 1
  | _ -> 8  (* デフォルト（ポインタアラインメント） *)
