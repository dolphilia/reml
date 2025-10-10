(* Codegen — Core IR から LLVM IR への変換 (Phase 3 Week 13-14)
 *
 * このファイルは docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §4 に基づき、
 * Core IR から LLVM IR への変換を実装する。
 *
 * 設計方針:
 * - LLVM 18+ の opaque pointer に対応
 * - System V ABI (x86_64 Linux) を既定ターゲットとする
 * - Phase 1 スコープ: 基本式・関数・基本ブロックのみ対応
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §4
 * - docs/guides/llvm-integration-notes.md §5
 * - Core IR 仕様: src/core_ir/ir.ml
 *)

open Core_ir.Ir
open Types

(* ========== エラー型 ========== *)

exception CodegenError of string

let codegen_error msg = raise (CodegenError msg)

let codegen_errorf fmt =
  Printf.ksprintf codegen_error fmt

(* ========== 関数ごとのメタ情報 ========== *)

type function_codegen_info = {
  return_type: Llvm.lltype;                              (** Reml関数の論理的戻り値型 *)
  return_class: Abi.return_classification;               (** ABI上の戻り値分類 *)
  param_classes: Abi.argument_classification list;       (** 引数ごとの ABI 分類 *)
  llvm_fn_type: Llvm.lltype;                             (** LLVM 関数型 *)
}

type current_function_state = {
  info: function_codegen_info;                           (** 関数のメタ情報 *)
  llvm_fn: Llvm.llvalue;                                 (** LLVM 関数値 *)
  fn_def: function_def;                                  (** Core IR 関数定義 *)
  mutable pending_phis: (Llvm.llvalue * (label * var_id) list) list;
                                                        (** 後で解決する φ ノード一覧 *)
  sret_param: Llvm.llvalue option;                      (** sret 用ポインタ引数（必要時） *)
}

(* ========== コードジェネレーションコンテキスト ========== *)

(** コードジェネレーションコンテキスト
 *
 * LLVM モジュール・ビルダー・型マッピング・変数マッピングを管理する。
 *)
type codegen_context = {
  llctx: Llvm.llcontext;                                  (** LLVM コンテキスト *)
  llmodule: Llvm.llmodule;                                (** LLVM モジュール *)
  builder: Llvm.llbuilder;                                (** LLVM IR ビルダー *)
  type_ctx: Type_mapping.type_mapping_context;            (** 型マッピングコンテキスト *)
  target: Target_config.target_config;                    (** ターゲット設定 *)

  (* 関数・変数・ブロックマッピング *)
  mutable fn_map: (string, Llvm.llvalue) Hashtbl.t;       (** 関数名 → LLVM関数 *)
  mutable var_map: (var_id, Llvm.llvalue) Hashtbl.t;      (** 変数ID → LLVM値 *)
  mutable block_map: (label, Llvm.llbasicblock) Hashtbl.t; (** ラベル → LLVM基本ブロック *)
  mutable fn_info_map: (string, function_codegen_info) Hashtbl.t;
                                                          (** 関数名 → コード生成メタ情報 *)
  mutable current_function: current_function_state option; (** 現在生成中の関数状態 *)
}

(* ========== コンテキスト管理 ========== *)

(** コードジェネレーションコンテキストを作成
 *
 * @param module_name モジュール名
 * @param target_name ターゲット名（デフォルト: "x86_64-linux"）
 * @return 初期化されたコンテキスト
 *)
let create_codegen_context module_name ?(target_name="x86_64-linux") () =
  let llctx = Llvm.global_context () in
  let llmodule = Llvm.create_module llctx module_name in
  let builder = Llvm.builder llctx in
  let type_ctx = Type_mapping.create_context module_name in
  let target = Target_config.get_target_config target_name in

  (* ターゲット設定を適用 *)
  Target_config.set_target_config llmodule target;

  {
    llctx;
    llmodule;
    builder;
    type_ctx;
    target;
    fn_map = Hashtbl.create 128;
    var_map = Hashtbl.create 256;
    block_map = Hashtbl.create 128;
    fn_info_map = Hashtbl.create 64;
    current_function = None;
  }

(** LLVM モジュールを取得 *)
let get_llmodule ctx = ctx.llmodule

(** LLVM ビルダーを取得 *)
let get_builder ctx = ctx.builder

let reset_function_context ctx =
  Hashtbl.reset ctx.var_map;
  Hashtbl.reset ctx.block_map

let ensure_function_info ctx fn_name =
  match Hashtbl.find_opt ctx.fn_info_map fn_name with
  | Some info -> info
  | None -> codegen_errorf "関数 %s のコード生成メタ情報が登録されていません" fn_name

let begin_function ctx fn_def llvm_fn =
  reset_function_context ctx;
  let info = ensure_function_info ctx fn_def.fn_name in
  let llvm_params = Llvm.params llvm_fn in
  let param_offset =
    match info.return_class with
    | Abi.SretReturn ->
        if Array.length llvm_params < 1 then
          codegen_errorf "関数 %s の sret 引数が欠落しています" fn_def.fn_name;
        1
    | Abi.DirectReturn -> 0
  in
  let sret_param =
    match info.return_class with
    | Abi.SretReturn -> Some llvm_params.(0)
    | Abi.DirectReturn -> None
  in
  List.iteri (fun i param ->
    let idx = i + param_offset in
    if idx >= Array.length llvm_params then
      codegen_errorf "関数 %s の引数インデックス %d が型と一致しません" fn_def.fn_name idx;
    let llvm_param = llvm_params.(idx) in
    Hashtbl.replace ctx.var_map param.param_var llvm_param
  ) fn_def.fn_params;
  ctx.current_function <- Some { info; llvm_fn; fn_def; pending_phis = []; sret_param }

let current_function_state ctx =
  match ctx.current_function with
  | Some state -> state
  | None -> codegen_error "現在コード生成中の関数が存在しません"

let resolve_pending_phis ctx =
  let state = current_function_state ctx in
  List.iter (fun (phi_node, incoming) ->
    List.iter (fun (label, incoming_var) ->
      let block =
        match Hashtbl.find_opt ctx.block_map label with
        | Some b -> b
        | None -> codegen_errorf "φ ノードが参照する未定義ブロック %s" label
      in
      let value =
        match Hashtbl.find_opt ctx.var_map incoming_var with
        | Some v -> v
        | None -> codegen_errorf "φ ノードが参照する未定義変数 %s" incoming_var.vname
      in
      Llvm.add_incoming (value, block) phi_node
    ) incoming
  ) (List.rev state.pending_phis);
  state.pending_phis <- []

let end_function ctx =
  resolve_pending_phis ctx;
  ctx.current_function <- None;
  reset_function_context ctx

(* ========== ランタイム関数宣言 ========== *)

(** ランタイム関数を宣言
 *
 * mem_alloc, inc_ref, dec_ref, panic を外部リンケージで宣言する。
 * これらの関数は runtime/native で実装される。
 *)
let declare_runtime_functions ctx =
  let ptr_ty = Llvm.pointer_type ctx.llctx in
  let i64_ty = Llvm.i64_type ctx.llctx in
  let void_ty = Llvm.void_type ctx.llctx in

  (* mem_alloc: (i64) -> ptr *)
  let mem_alloc_ty = Llvm.function_type ptr_ty [| i64_ty |] in
  let mem_alloc = Llvm.declare_function "mem_alloc" mem_alloc_ty ctx.llmodule in
  Hashtbl.replace ctx.fn_map "mem_alloc" mem_alloc;

  (* inc_ref: (ptr) -> void *)
  let inc_ref_ty = Llvm.function_type void_ty [| ptr_ty |] in
  let inc_ref = Llvm.declare_function "inc_ref" inc_ref_ty ctx.llmodule in
  Hashtbl.replace ctx.fn_map "inc_ref" inc_ref;

  (* dec_ref: (ptr) -> void *)
  let dec_ref_ty = Llvm.function_type void_ty [| ptr_ty |] in
  let dec_ref = Llvm.declare_function "dec_ref" dec_ref_ty ctx.llmodule in
  Hashtbl.replace ctx.fn_map "dec_ref" dec_ref;

  (* panic: (ptr, i64) -> void (noreturn) *)
  let panic_ty = Llvm.function_type void_ty [| ptr_ty; i64_ty |] in
  let panic = Llvm.declare_function "panic" panic_ty ctx.llmodule in
  Llvm.add_function_attr panic (Llvm.create_enum_attr ctx.llctx "noreturn" 0L) Llvm.AttrIndex.Function;
  Hashtbl.replace ctx.fn_map "panic" panic;

  (* print_i64: (i64) -> void *)
  let print_i64_ty = Llvm.function_type void_ty [| i64_ty |] in
  let print_i64 = Llvm.declare_function "print_i64" print_i64_ty ctx.llmodule in
  Hashtbl.replace ctx.fn_map "print_i64" print_i64;

  (* memcpy: llvm.memcpy.p0.p0.i64(ptr, ptr, i64, i1) -> void *)
  let memcpy_ty = Llvm.function_type void_ty [| ptr_ty; ptr_ty; i64_ty; Llvm.i1_type ctx.llctx |] in
  let memcpy = Llvm.declare_function "llvm.memcpy.p0.p0.i64" memcpy_ty ctx.llmodule in
  Hashtbl.replace ctx.fn_map "memcpy" memcpy

(* ========== ランタイムヘルパー関数 ========== *)

(** memcpy を取得または宣言する *)
let declare_memcpy ctx =
  match Hashtbl.find_opt ctx.fn_map "memcpy" with
  | Some fn -> fn
  | None -> codegen_error "memcpy not declared (should be called after declare_runtime_functions)"

(** mem_alloc を呼び出してヒープメモリを割り当てる
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param size_bytes 割り当てるサイズ（バイト数）
 * @param type_tag 型タグ (REML_TAG_* の値)
 * @return 割り当てられたメモリへのポインタ（ヘッダの直後）
 *)
let call_mem_alloc ctx size_bytes type_tag =
  let i64_ty = Llvm.i64_type ctx.llctx in
  let i32_ty = Llvm.i32_type ctx.llctx in
  let ptr_ty = Llvm.pointer_type ctx.llctx in

  (* mem_alloc を取得 *)
  let mem_alloc =
    match Hashtbl.find_opt ctx.fn_map "mem_alloc" with
    | Some fn -> fn
    | None -> codegen_error "mem_alloc not declared"
  in

  (* mem_alloc(size) を呼び出し *)
  let size_val = Llvm.const_int i64_ty size_bytes in
  let mem_alloc_ty = Llvm.function_type ptr_ty [| i64_ty |] in
  let ptr = Llvm.build_call mem_alloc_ty mem_alloc [| size_val |] "alloc_tmp" ctx.builder in

  (* 型タグを設定（reml_set_type_tag相当の処理）*)
  (* ヘッダは mem_alloc が初期化するが、型タグは呼び出し側で設定 *)
  (* ヘッダ構造: {uint32 refcount, uint32 type_tag} = 8 bytes *)
  (* type_tag オフセット = 4 bytes *)
  let header_ptr = Llvm.build_in_bounds_gep ptr_ty ptr
    [| Llvm.const_int i64_ty (-8) |] "header_ptr" ctx.builder in
  let type_tag_ptr = Llvm.build_in_bounds_gep ptr_ty header_ptr
    [| Llvm.const_int i64_ty 4 |] "type_tag_ptr" ctx.builder in
  let type_tag_val = Llvm.const_int i32_ty type_tag in
  ignore (Llvm.build_store type_tag_val type_tag_ptr ctx.builder);

  ptr

(** inc_ref を呼び出して参照カウントをインクリメント
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param ptr オブジェクトへのポインタ
 *
 * Note: Phase 2 で使用予定（タプル/レコード/クロージャのコピー時）
 *)
let call_inc_ref ctx ptr =
  let ptr_ty = Llvm.pointer_type ctx.llctx in
  let void_ty = Llvm.void_type ctx.llctx in

  let inc_ref =
    match Hashtbl.find_opt ctx.fn_map "inc_ref" with
    | Some fn -> fn
    | None -> codegen_error "inc_ref not declared"
  in

  let inc_ref_ty = Llvm.function_type void_ty [| ptr_ty |] in
  ignore (Llvm.build_call inc_ref_ty inc_ref [| ptr |] "" ctx.builder)
[@@warning "-32"]

(** dec_ref を呼び出して参照カウントをデクリメント
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param ptr オブジェクトへのポインタ
 *
 * Note: Phase 2 で使用予定（スコープ終了時、変数上書き時）
 *)
let call_dec_ref ctx ptr =
  let ptr_ty = Llvm.pointer_type ctx.llctx in
  let void_ty = Llvm.void_type ctx.llctx in

  let dec_ref =
    match Hashtbl.find_opt ctx.fn_map "dec_ref" with
    | Some fn -> fn
    | None -> codegen_error "dec_ref not declared"
  in

  let dec_ref_ty = Llvm.function_type void_ty [| ptr_ty |] in
  ignore (Llvm.build_call dec_ref_ty dec_ref [| ptr |] "" ctx.builder)
[@@warning "-32"] (* Phase 2 で使用予定 *)

(** panic を呼び出してプログラムを異常終了させる
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param msg エラーメッセージ文字列
 *
 * Note: Phase 2 で使用予定（境界チェック失敗、アサーション失敗時）
 *)
let call_panic ctx msg =
  let ptr_ty = Llvm.pointer_type ctx.llctx in
  let i64_ty = Llvm.i64_type ctx.llctx in
  let void_ty = Llvm.void_type ctx.llctx in

  let panic =
    match Hashtbl.find_opt ctx.fn_map "panic" with
    | Some fn -> fn
    | None -> codegen_error "panic not declared"
  in

  (* メッセージ文字列をグローバル定数として定義 *)
  let str_const = Llvm.const_stringz ctx.llctx msg in
  let str_global = Llvm.define_global "panic_msg" str_const ctx.llmodule in

  (* 文字列ポインタを取得 *)
  let zero = Llvm.const_int (Llvm.i32_type ctx.llctx) 0 in
  let str_ptr = Llvm.const_gep ptr_ty str_global [| zero |] in

  (* 長さを取得（NULL終端なので実際には使われないが、シグネチャに必要） *)
  let len = Llvm.const_int i64_ty (String.length msg) in

  (* panic(ptr, len) を呼び出し *)
  let panic_ty = Llvm.function_type void_ty [| ptr_ty; i64_ty |] in
  ignore (Llvm.build_call panic_ty panic [| str_ptr; len |] "" ctx.builder);

  (* unreachable 命令を挿入（panic は決して戻らない） *)
  Llvm.build_unreachable ctx.builder
[@@warning "-32"]

(* ========== 型判定ヘルパー ========== *)

let is_float_type ty =
  match ty with
  | TCon (TCFloat _) -> true
  | _ -> false

let is_unit_type ty =
  match ty with
  | TUnit -> true
  | _ -> false

(* ========== 式のコード生成（前方宣言） ========== *)

(** 式のコード生成（相互再帰のため前方宣言） *)
let rec codegen_expr ctx expr =
  match expr.expr_kind with
  | Literal lit -> codegen_literal ctx lit expr.expr_ty
  | Var var_id -> codegen_var ctx var_id
  | App (fn_expr, arg_exprs) -> codegen_app ctx fn_expr arg_exprs
  | Let (var_id, bound_expr, body_expr) -> codegen_let ctx var_id bound_expr body_expr
  | If (cond_expr, then_expr, else_expr) -> codegen_if ctx cond_expr then_expr else_expr
  | Primitive (op, args) -> codegen_primitive ctx op args
  | TupleAccess (tuple_expr, index) -> codegen_tuple_access ctx tuple_expr index
  | RecordAccess (record_expr, field) -> codegen_record_access ctx record_expr field
  | ArrayAccess (array_expr, index_expr) -> codegen_array_access ctx array_expr index_expr
  | Match _ -> codegen_errorf "Match expression not yet implemented in Phase 1"
  | Closure _ -> codegen_errorf "Closure not yet implemented in Phase 1"
  | DictLookup _ -> codegen_errorf "DictLookup not yet implemented in Phase 1"
  | CapabilityCheck _ -> codegen_errorf "CapabilityCheck not yet implemented in Phase 1"
  | ADTConstruct _ -> codegen_errorf "ADTConstruct not yet implemented in Phase 1"
  | ADTProject _ -> codegen_errorf "ADTProject not yet implemented in Phase 1"

(* ========== リテラルのコード生成 ========== *)

and codegen_literal ctx lit ty =
  match lit with
  | Ast.Int (s, _base) ->
      (* 型に応じて適切な整数定数を生成 *)
      let llvm_ty = Type_mapping.reml_type_to_llvm ctx.type_ctx ty in
      let i = Int64.of_string s in
      Llvm.const_int llvm_ty (Int64.to_int i)

  | Ast.Float s ->
      let llvm_ty = Type_mapping.reml_type_to_llvm ctx.type_ctx ty in
      let f = float_of_string s in
      Llvm.const_float llvm_ty f

  | Ast.Bool b ->
      let i1_ty = Llvm.i1_type ctx.llctx in
      Llvm.const_int i1_ty (if b then 1 else 0)

  | Ast.Char s ->
      let i32_ty = Llvm.i32_type ctx.llctx in
      let c = if String.length s > 0 then String.get s 0 else '\x00' in
      Llvm.const_int i32_ty (Char.code c)

  | Ast.String (s, _kind) ->
      (* FAT pointer { ptr, i64 } を構築 *)
      (* Phase 1-5: ランタイム連携 - mem_alloc でヒープ割り当て *)
      let len = String.length s in
      let i64_ty = Llvm.i64_type ctx.llctx in
      let ptr_ty = Llvm.pointer_type ctx.llctx in

      (* mem_alloc(len + 1) を呼び出し（NULL終端用に+1） *)
      let alloc_size = len + 1 in
      let str_ptr = call_mem_alloc ctx alloc_size 4 (* REML_TAG_STRING *) in

      (* 文字列データをコピー *)
      let str_const = Llvm.const_stringz ctx.llctx s in
      let str_global = Llvm.define_global "str_literal" str_const ctx.llmodule in
      Llvm.set_linkage Llvm.Linkage.Private str_global;
      let zero = Llvm.const_int (Llvm.i32_type ctx.llctx) 0 in
      let src_ptr = Llvm.const_gep ptr_ty str_global [| zero |] in

      (* memcpy(str_ptr, src_ptr, len + 1) を呼び出し *)
      let memcpy_fn = declare_memcpy ctx in
      let size_val = Llvm.const_int i64_ty (alloc_size) in
      let is_volatile = Llvm.const_int (Llvm.i1_type ctx.llctx) 0 in
      ignore (Llvm.build_call
        (Llvm.element_type (Llvm.type_of memcpy_fn))
        memcpy_fn
        [| str_ptr; src_ptr; size_val; is_volatile |]
        "" ctx.builder);

      (* FAT pointer 構造体 { ptr, len } を構築 *)
      let len_const = Llvm.const_int i64_ty len in
      Llvm.const_struct ctx.llctx [| str_ptr; len_const |]

  | Ast.Unit ->
      (* unit は void として扱う（実際には値を返さない） *)
      (* Phase 1: undef を返す *)
      Llvm.undef (Llvm.void_type ctx.llctx)

  | Ast.Tuple _ ->
      (* Phase 1-5: タプルリテラルは糖衣削除で Core IR に変換される前提 *)
      (* Phase 2 で TupleConstruct ノードとして実装予定 *)
      codegen_errorf "Tuple literals not yet implemented in Phase 1 (requires Core IR TupleConstruct node)"

  | Ast.Array _ ->
      codegen_errorf "Array literals not yet implemented in Phase 1"

  | Ast.Record _ ->
      codegen_errorf "Record literals not yet implemented in Phase 1"

(* ========== 変数参照のコード生成 ========== *)

and codegen_var ctx var_id =
  match Hashtbl.find_opt ctx.var_map var_id with
  | Some llvalue -> llvalue
  | None ->
      (* ローカル変数として登録されていない場合はグローバル関数を参照している可能性を考慮 *)
      begin match Hashtbl.find_opt ctx.fn_map var_id.vname with
      | Some fn -> fn
      | None -> codegen_errorf "Undefined variable: %s" var_id.vname
      end

(* ========== 関数適用のコード生成 ========== *)

and codegen_app ctx fn_expr arg_exprs =
  (* 関数式をコード生成 *)
  let fn_value = codegen_expr ctx fn_expr in

  (* 引数をコード生成 *)
  let arg_values = List.map (codegen_expr ctx) arg_exprs in
  let arg_values_array = Array.of_list arg_values in

  (* 関数の型を取得（ポインタの場合は要素型を取り出す） *)
  let fn_ptr_ty = Llvm.type_of fn_value in
  let fn_ty =
    let info_ty =
      match fn_expr.expr_kind with
      | Var var_id ->
          Option.map (fun info -> info.llvm_fn_type)
            (Hashtbl.find_opt ctx.fn_info_map var_id.vname)
      | _ -> None
    in
    match info_ty with
    | Some ty -> ty
    | None ->
        begin match Llvm.classify_type fn_ptr_ty with
        | Llvm.TypeKind.Pointer -> Llvm.element_type fn_ptr_ty
        | Llvm.TypeKind.Function -> fn_ptr_ty
        | _ ->
            codegen_errorf "関数呼び出し対象が関数型ではありません: %s"
              (Llvm.string_of_lltype fn_ptr_ty)
        end
  in

  (* 関数呼び出しを生成 (LLVM 18 opaque pointer 対応) *)
  Llvm.build_call fn_ty fn_value arg_values_array "call_tmp" ctx.builder

(* ========== Let 束縛のコード生成 ========== *)

and codegen_let ctx var_id bound_expr body_expr =
  (* 束縛する値をコード生成 *)
  let bound_value = codegen_expr ctx bound_expr in

  (* 変数マップに登録 *)
  Hashtbl.replace ctx.var_map var_id bound_value;

  (* 本体式をコード生成 *)
  codegen_expr ctx body_expr

(* ========== If 式のコード生成 ========== *)

and codegen_if ctx cond_expr then_expr else_expr =
  (* 条件式をコード生成 *)
  let cond_value = codegen_expr ctx cond_expr in

  (* 現在の関数を取得 *)
  let parent_fn = Llvm.block_parent (Llvm.insertion_block ctx.builder) in

  (* then/else/merge ブロックを作成 *)
  let then_block = Llvm.append_block ctx.llctx "if_then" parent_fn in
  let else_block = Llvm.append_block ctx.llctx "if_else" parent_fn in
  let merge_block = Llvm.append_block ctx.llctx "if_merge" parent_fn in

  (* 条件分岐を生成 *)
  let _ = Llvm.build_cond_br cond_value then_block else_block ctx.builder in

  (* then ブロックをコード生成 *)
  Llvm.position_at_end then_block ctx.builder;
  let then_value = codegen_expr ctx then_expr in
  let _ = Llvm.build_br merge_block ctx.builder in
  let then_end_block = Llvm.insertion_block ctx.builder in

  (* else ブロックをコード生成 *)
  Llvm.position_at_end else_block ctx.builder;
  let else_value = codegen_expr ctx else_expr in
  let _ = Llvm.build_br merge_block ctx.builder in
  let else_end_block = Llvm.insertion_block ctx.builder in

  (* merge ブロックで φ ノードを生成 *)
  Llvm.position_at_end merge_block ctx.builder;
  let phi = Llvm.build_phi [
    (then_value, then_end_block);
    (else_value, else_end_block)
  ] "if_tmp" ctx.builder in

  phi

(* ========== プリミティブ演算のコード生成 ========== *)

and codegen_primitive ctx op args =
  match op, args with
  (* 二項算術演算 *)
  | PrimAdd, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      (* 型に応じて整数加算か浮動小数加算かを判定 *)
      if is_float_type lhs.expr_ty then
        Llvm.build_fadd lhs_val rhs_val "fadd_tmp" ctx.builder
      else
        Llvm.build_add lhs_val rhs_val "add_tmp" ctx.builder

  | PrimSub, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      if is_float_type lhs.expr_ty then
        Llvm.build_fsub lhs_val rhs_val "fsub_tmp" ctx.builder
      else
        Llvm.build_sub lhs_val rhs_val "sub_tmp" ctx.builder

  | PrimMul, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      if is_float_type lhs.expr_ty then
        Llvm.build_fmul lhs_val rhs_val "fmul_tmp" ctx.builder
      else
        Llvm.build_mul lhs_val rhs_val "mul_tmp" ctx.builder

  | PrimDiv, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      if is_float_type lhs.expr_ty then
        Llvm.build_fdiv lhs_val rhs_val "fdiv_tmp" ctx.builder
      else
        Llvm.build_sdiv lhs_val rhs_val "sdiv_tmp" ctx.builder

  | PrimMod, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      Llvm.build_srem lhs_val rhs_val "srem_tmp" ctx.builder

  (* 比較演算 *)
  | PrimEq, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      if is_float_type lhs.expr_ty then
        Llvm.build_fcmp Llvm.Fcmp.Oeq lhs_val rhs_val "fcmp_eq" ctx.builder
      else
        Llvm.build_icmp Llvm.Icmp.Eq lhs_val rhs_val "icmp_eq" ctx.builder

  | PrimNe, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      if is_float_type lhs.expr_ty then
        Llvm.build_fcmp Llvm.Fcmp.One lhs_val rhs_val "fcmp_ne" ctx.builder
      else
        Llvm.build_icmp Llvm.Icmp.Ne lhs_val rhs_val "icmp_ne" ctx.builder

  | PrimLt, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      if is_float_type lhs.expr_ty then
        Llvm.build_fcmp Llvm.Fcmp.Olt lhs_val rhs_val "fcmp_lt" ctx.builder
      else
        Llvm.build_icmp Llvm.Icmp.Slt lhs_val rhs_val "icmp_lt" ctx.builder

  | PrimLe, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      if is_float_type lhs.expr_ty then
        Llvm.build_fcmp Llvm.Fcmp.Ole lhs_val rhs_val "fcmp_le" ctx.builder
      else
        Llvm.build_icmp Llvm.Icmp.Sle lhs_val rhs_val "icmp_le" ctx.builder

  | PrimGt, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      if is_float_type lhs.expr_ty then
        Llvm.build_fcmp Llvm.Fcmp.Ogt lhs_val rhs_val "fcmp_gt" ctx.builder
      else
        Llvm.build_icmp Llvm.Icmp.Sgt lhs_val rhs_val "icmp_gt" ctx.builder

  | PrimGe, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      if is_float_type lhs.expr_ty then
        Llvm.build_fcmp Llvm.Fcmp.Oge lhs_val rhs_val "fcmp_ge" ctx.builder
      else
        Llvm.build_icmp Llvm.Icmp.Sge lhs_val rhs_val "icmp_ge" ctx.builder

  (* 論理演算 *)
  | PrimAnd, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      Llvm.build_and lhs_val rhs_val "and_tmp" ctx.builder

  | PrimOr, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      Llvm.build_or lhs_val rhs_val "or_tmp" ctx.builder

  | PrimNot, [arg] ->
      let arg_val = codegen_expr ctx arg in
      Llvm.build_not arg_val "not_tmp" ctx.builder

  (* ビット演算 *)
  | PrimBitAnd, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      Llvm.build_and lhs_val rhs_val "bitand_tmp" ctx.builder

  | PrimBitOr, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      Llvm.build_or lhs_val rhs_val "bitor_tmp" ctx.builder

  | PrimBitXor, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      Llvm.build_xor lhs_val rhs_val "bitxor_tmp" ctx.builder

  | PrimBitNot, [arg] ->
      let arg_val = codegen_expr ctx arg in
      Llvm.build_not arg_val "bitnot_tmp" ctx.builder

  | PrimShl, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      Llvm.build_shl lhs_val rhs_val "shl_tmp" ctx.builder

  | PrimShr, [lhs; rhs] ->
      let lhs_val = codegen_expr ctx lhs in
      let rhs_val = codegen_expr ctx rhs in
      Llvm.build_ashr lhs_val rhs_val "ashr_tmp" ctx.builder

  | PrimPow, _ ->
      codegen_errorf "PrimPow not yet implemented in Phase 1"

  | _ -> codegen_errorf "Invalid primitive operation or argument count"

(* ========== タプル・レコード・配列アクセス ========== *)

and codegen_tuple_access ctx tuple_expr index =
  let tuple_val = codegen_expr ctx tuple_expr in
  Llvm.build_extractvalue tuple_val index "tuple_access" ctx.builder

and codegen_record_access _ctx _record_expr field =
  (* Phase 1: レコードフィールドアクセスは未実装 *)
  codegen_errorf "Record access not yet implemented in Phase 1: %s" field

and codegen_array_access _ctx _array_expr _index_expr =
  (* Phase 1: 配列アクセスは未実装 *)
  codegen_errorf "Array access not yet implemented in Phase 1"

(* ========== 終端命令のコード生成 ========== *)

let emit_return ctx expr =
  let state = current_function_state ctx in

  (* Phase 1-5: dec_ref 挿入はスキップ *)
  (* 理由: FAT pointer { ptr, i64 } は構造体として渡されるため、 *)
  (*       単純なポインタ判定では正しく処理できない *)
  (* Phase 2: 所有権解析と型情報に基づき、ヒープオブジェクトのみ dec_ref *)
  ignore state.fn_def.fn_params; (* 警告抑制 *)

  if is_unit_type expr.expr_ty then begin
    let _ = codegen_expr ctx expr in
    Llvm.build_ret_void ctx.builder
  end else
    let ret_val = codegen_expr ctx expr in
    match state.info.return_class with
    | Abi.DirectReturn ->
        Llvm.build_ret ret_val ctx.builder
    | Abi.SretReturn ->
        let sret_param =
          match state.sret_param with
          | Some p -> p
          | None ->
              codegen_error "sret パラメータが存在しない状態で sret 戻り値を生成しようとしました"
        in
        let _ = Llvm.build_store ret_val sret_param ctx.builder in
        Llvm.build_ret_void ctx.builder

let codegen_terminator ctx terminator =
  match terminator with
  | TermReturn expr ->
      emit_return ctx expr

  | TermJump label ->
      begin match Hashtbl.find_opt ctx.block_map label with
      | Some target_block ->
          Llvm.build_br target_block ctx.builder
      | None ->
          codegen_errorf "Undefined block label: %s" label
      end

  | TermBranch (cond_expr, then_label, else_label) ->
      let cond_val = codegen_expr ctx cond_expr in
      begin match Hashtbl.find_opt ctx.block_map then_label,
                  Hashtbl.find_opt ctx.block_map else_label with
      | Some then_block, Some else_block ->
          Llvm.build_cond_br cond_val then_block else_block ctx.builder
      | _ ->
          codegen_errorf "Undefined block label in branch"
      end

  | TermSwitch (_expr, _cases, _default_label) ->
      (* Phase 1: Switch は未実装 *)
      codegen_errorf "Switch terminator not yet implemented in Phase 1"

  | TermUnreachable ->
      Llvm.build_unreachable ctx.builder

(* ========== 文のコード生成 ========== *)

let codegen_stmt ctx stmt =
  match stmt with
  | Assign (var_id, expr) ->
      let value = codegen_expr ctx expr in
      Hashtbl.replace ctx.var_map var_id value

  | Return expr ->
      let _ = emit_return ctx expr in
      ()

  | Jump label ->
      begin match Hashtbl.find_opt ctx.block_map label with
      | Some target_block ->
          let _ = Llvm.build_br target_block ctx.builder in
          ()
      | None ->
          codegen_errorf "Undefined block label: %s" label
      end

  | Branch (cond_expr, then_label, else_label) ->
      let cond_val = codegen_expr ctx cond_expr in
      begin match Hashtbl.find_opt ctx.block_map then_label,
                  Hashtbl.find_opt ctx.block_map else_label with
      | Some then_block, Some else_block ->
          let _ = Llvm.build_cond_br cond_val then_block else_block ctx.builder in
          ()
      | _ ->
          codegen_errorf "Undefined block label in branch"
      end

  | Phi (var_id, incoming) ->
      (* φ ノードを生成 *)
      let llvm_ty = Type_mapping.reml_type_to_llvm ctx.type_ctx var_id.vty in
      let phi_node = Llvm.build_empty_phi llvm_ty var_id.vname ctx.builder in
      Hashtbl.replace ctx.var_map var_id phi_node;
      let state = current_function_state ctx in
      state.pending_phis <- (phi_node, incoming) :: state.pending_phis

  | EffectMarker _ ->
      (* Phase 1: 効果マーカーは無視 *)
      ()

  | ExprStmt expr ->
      let _ = codegen_expr ctx expr in
      ()

(* ========== 関数宣言生成 ========== *)

(** 関数宣言を生成
 *
 * Core IR の function_def から LLVM 関数宣言を生成する。
 * Phase 1 では基本的な関数シグネチャのみ対応。
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param fn_def Core IR 関数定義
 * @return LLVM 関数値
 *)
let codegen_function_decl ctx fn_def =
  let open Core_ir.Ir in

  (* パラメータ型を変換 *)
  let param_types = List.map (fun p ->
    Type_mapping.reml_type_to_llvm ctx.type_ctx p.param_var.vty
  ) fn_def.fn_params in
  let param_types_array = Array.of_list param_types in

  (* 返り値型を変換 *)
  let ret_ty = Type_mapping.reml_type_to_llvm ctx.type_ctx fn_def.fn_return_ty in

  (* 戻り値のABI分類を判定 *)
  let return_class = Abi.classify_struct_return ctx.target ctx.type_ctx fn_def.fn_return_ty in

  (* sret の場合は先頭にポインタ引数を追加し、戻り値型を void にする *)
  let actual_ret_ty, actual_param_types_array, sret_offset =
    match return_class with
    | Abi.DirectReturn ->
        ret_ty, param_types_array, 0
    | Abi.SretReturn ->
        let sret_ptr_ty = Llvm.pointer_type ctx.llctx in
        let extended_params =
          Array.init (Array.length param_types_array + 1) (fun i ->
            if i = 0 then sret_ptr_ty else param_types_array.(i - 1)
          )
        in
        Llvm.void_type ctx.llctx, extended_params, 1
  in

  (* 関数型を生成 *)
  let fn_ty = Llvm.function_type actual_ret_ty actual_param_types_array in

  (* 関数を宣言（後続で基本ブロックを追加して定義にする） *)
  let llvm_fn = Llvm.declare_function fn_def.fn_name fn_ty ctx.llmodule in

  (* System V calling convention を設定 *)
  Llvm.set_function_call_conv Llvm.CallConv.c llvm_fn;

  (* sret 属性の付与（必要な場合） *)
  (match return_class with
   | Abi.SretReturn ->
       Abi.add_sret_attr ctx.llctx llvm_fn ret_ty 0
   | Abi.DirectReturn ->
       ());

  (* 各引数のABI分類を判定し、byval 属性を追加 *)
  let param_classes =
    List.mapi (fun i param ->
      let arg_class = Abi.classify_struct_argument ctx.target ctx.type_ctx param.param_var.vty in
      begin match arg_class with
      | Abi.ByvalArg arg_llty ->
          Abi.add_byval_attr ctx.llctx llvm_fn arg_llty (i + sret_offset)
      | Abi.DirectArg -> ()
      end;
      arg_class
    ) fn_def.fn_params
  in

  (* 関数マップに登録 *)
  Hashtbl.replace ctx.fn_map fn_def.fn_name llvm_fn;

  (* パラメータに名前を設定 *)
  (match return_class with
   | Abi.SretReturn ->
       let sret_param = Llvm.param llvm_fn 0 in
       Llvm.set_value_name "__sret_ptr" sret_param
   | Abi.DirectReturn -> ());
  List.iteri (fun i param ->
    let llvm_param = Llvm.param llvm_fn (i + sret_offset) in
    Llvm.set_value_name param.param_var.vname llvm_param
  ) fn_def.fn_params;

  (* 関数メタ情報を記録 *)
  let info = { return_type = ret_ty; return_class; param_classes; llvm_fn_type = fn_ty } in
  Hashtbl.replace ctx.fn_info_map fn_def.fn_name info;

  llvm_fn

(* ========== グローバル変数生成 ========== *)

(** グローバル変数定義を生成
 *
 * Phase 1 では定数初期化のみ対応。
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param global_def Core IR グローバル変数定義
 *)
let codegen_global_def ctx global_def =
  let open Core_ir.Ir in

  (* 型を変換 *)
  let llvm_ty = Type_mapping.reml_type_to_llvm ctx.type_ctx global_def.global_var.vty in

  (* グローバル変数を宣言 *)
  let llvm_global = Llvm.declare_global llvm_ty global_def.global_var.vname ctx.llmodule in

  (* 可変性を設定 *)
  Llvm.set_global_constant (not global_def.global_mutable) llvm_global;

  (* TODO: 初期化式のコード生成（定数のみ）
   * Phase 1 では未実装、Phase 2 で対応
   *)
  ()

(* ========== 基本ブロック生成 ========== *)

(** 基本ブロックを生成
 *
 * Core IR の block リストから LLVM 基本ブロックを生成する。
 * まず全てのブロックを作成し、その後に命令を生成する（前方参照対応）。
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param llvm_fn LLVM 関数値
 * @param blocks Core IR 基本ブロックリスト
 *)
let codegen_blocks ctx llvm_fn blocks =
  (* Phase 1: 全てのブロックを作成 *)
  List.iter (fun block ->
    let llvm_block = Llvm.append_block ctx.llctx block.label llvm_fn in
    Hashtbl.replace ctx.block_map block.label llvm_block
  ) blocks;

  (* Phase 2: 各ブロックの命令を生成 *)
  List.iter (fun block ->
    let llvm_block = Hashtbl.find ctx.block_map block.label in
    Llvm.position_at_end llvm_block ctx.builder;

    (* ブロックパラメータを変数マップに登録 *)
    List.iter (fun _param ->
      (* φノードとして実装される場合は後で処理 *)
      (* Phase 1: 簡易実装 *)
      ()
    ) block.params;

    (* 文を順次コード生成 *)
    List.iter (codegen_stmt ctx) block.stmts;

    (* 終端命令をコード生成 *)
    let _ = codegen_terminator ctx block.terminator in
    ()
  ) blocks

(* ========== モジュール生成 ========== *)

(** モジュール全体を生成
 *
 * Core IR の module_def から LLVM モジュールを生成する。
 *
 * @param module_def Core IR モジュール定義
 * @param target_name ターゲット名（オプション）
 * @return LLVM モジュール
 *)
let codegen_module ?(target_name="x86_64-linux") module_def =
  let ctx = create_codegen_context module_def.module_name ~target_name () in

  (* ランタイム関数を宣言 *)
  declare_runtime_functions ctx;

  (* グローバル変数を生成 *)
  List.iter (codegen_global_def ctx) module_def.global_defs;

  (* 関数宣言を生成 *)
  List.iter (fun fn_def ->
    let _ = codegen_function_decl ctx fn_def in
    ()
  ) module_def.function_defs;

  (* 関数本体を生成 *)
  List.iter (fun fn_def ->
    let llvm_fn = Hashtbl.find ctx.fn_map fn_def.fn_name in
    begin_function ctx fn_def llvm_fn;
    codegen_blocks ctx llvm_fn fn_def.fn_blocks;
    end_function ctx
  ) module_def.function_defs;

  ctx.llmodule

(* ========== LLVM IR 出力 ========== *)

(** LLVM IR をテキスト形式で出力
 *
 * @param llmodule LLVM モジュール
 * @param filename 出力ファイル名
 *)
let emit_llvm_ir llmodule filename =
  Llvm.print_module filename llmodule

(** LLVM IR をビットコード形式で出力
 *
 * @param llmodule LLVM モジュール
 * @param filename 出力ファイル名
 *)
let emit_llvm_bc llmodule filename =
  if not (Llvm_bitwriter.write_bitcode_file llmodule filename) then
    codegen_error ("Failed to write bitcode to " ^ filename)
