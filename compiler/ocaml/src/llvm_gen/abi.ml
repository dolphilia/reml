(* Abi — LLVM ABI呼び出し規約の実装 (Phase 3 Week 14-15)
 *
 * このファイルは docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §5 に基づき、
 * System V ABI および将来のWindows x64 ABI対応のためのABI判定・属性設定を実装する。
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §5
 * - docs/guides/llvm-integration-notes.md §5.0, §5.2
 * - System V AMD64 ABI Specification
 *)

open Types

(* ========== ABI分類型 ========== *)

type return_classification =
  | DirectReturn
  | SretReturn

type argument_classification =
  | DirectArg
  | ByvalArg of Llvm.lltype

(* ========== 定数定義 ========== *)

(** System V ABI: 構造体のレジスタ渡しサイズ閾値（バイト） *)
let sysv_struct_register_threshold = 16

(** Windows x64 ABI: 構造体のレジスタ渡しサイズ閾値（バイト、Phase 2実装予定） *)
let win64_struct_register_threshold = 8

(* ========== 型サイズ計算 ========== *)

(** 型のサイズをバイト数で取得
 *
 * LLVM の DataLayout を使ってアラインメントを考慮したサイズを計算する。
 *
 * @param llctx LLVM コンテキスト
 * @param llty LLVM型
 * @return サイズ（バイト数）
 *)
let rec get_type_size llctx llty =
  (* LLVM 18+ では Llvm_target.DataLayout が必要だが、
   * 簡易実装として Llvm.size_of を使用 *)
  let size_llvalue = Llvm.size_of llty in
  (* size_of は ConstantInt を返すため、整数値に変換 *)
  match Llvm.int64_of_const size_llvalue with
  | Some n -> Int64.to_int n
  | None ->
      (* フォールバック: 基本型のサイズを推定 *)
      let kind = Llvm.classify_type llty in
      begin match kind with
      | Llvm.TypeKind.Integer ->
          let width = Llvm.integer_bitwidth llty in
          (width + 7) / 8  (* ビット幅をバイトに変換 *)
      | Llvm.TypeKind.Float -> 4
      | Llvm.TypeKind.Double -> 8
      | Llvm.TypeKind.Pointer -> 8  (* 64bit ポインタ前提 *)
      | Llvm.TypeKind.Struct ->
          (* 構造体フィールドサイズの合計を計算 *)
          let element_types = Llvm.struct_element_types llty in
          let total_size = ref 0 in
          Array.iter (fun elem_ty ->
            total_size := !total_size + get_type_size llctx elem_ty
          ) element_types;
          !total_size
      | _ -> 8  (* デフォルト: ポインタサイズ *)
      end

(* ========== 構造体型判定 ========== *)

(** 型が構造体（タプル、レコード）かどうかを判定
 *
 * @param ty Reml型
 * @return true: 構造体型、false: それ以外
 *)
let is_struct_type ty =
  match ty with
  | TTuple _ | TRecord _ -> true
  | _ -> false

(* ========== ABI判定関数 ========== *)

(** 構造体戻り値のABI分類を判定
 *
 * System V ABI: 16バイト以下はレジスタ、超過はsret
 * Windows x64 ABI: 8バイト以下はレジスタ（Phase 2実装予定）
 *
 * @param target ターゲット設定
 * @param ctx 型マッピングコンテキスト
 * @param ty Reml型
 * @return ABI分類（DirectReturn または SretReturn）
 *)
let classify_struct_return target ctx ty =
  (* 構造体型でなければ常に DirectReturn *)
  if not (is_struct_type ty) then
    DirectReturn
  else
    (* LLVM型に変換してサイズを計算 *)
    let llty = Type_mapping.reml_type_to_llvm ctx ty in
    let llctx = Type_mapping.get_llcontext ctx in
    let size = get_type_size llctx llty in

    (* ターゲット別の判定 *)
    let threshold = match target.Target_config.triple with
    | triple when String.starts_with ~prefix:"x86_64-" triple &&
                  String.contains triple 'l' (* linux *) ->
        sysv_struct_register_threshold
    | triple when String.starts_with ~prefix:"x86_64-pc-windows" triple ->
        win64_struct_register_threshold  (* Phase 2 で有効化 *)
    | _ ->
        (* デフォルト: System V ABI *)
        sysv_struct_register_threshold
    in

    if size <= threshold then
      DirectReturn
    else
      SretReturn

(** 構造体引数のABI分類を判定
 *
 * System V ABI: 16バイト以下はレジスタ、超過はbyval
 *
 * @param target ターゲット設定
 * @param ctx 型マッピングコンテキスト
 * @param ty Reml型
 * @return ABI分類（DirectArg または ByvalArg）
 *)
let classify_struct_argument target ctx ty =
  (* 構造体型でなければ常に DirectArg *)
  if not (is_struct_type ty) then
    DirectArg
  else
    (* LLVM型に変換してサイズを計算 *)
    let llty = Type_mapping.reml_type_to_llvm ctx ty in
    let llctx = Type_mapping.get_llcontext ctx in
    let size = get_type_size llctx llty in

    (* ターゲット別の判定 *)
    let threshold = match target.Target_config.triple with
    | triple when String.starts_with ~prefix:"x86_64-" triple &&
                  String.contains triple 'l' (* linux *) ->
        sysv_struct_register_threshold
    | triple when String.starts_with ~prefix:"x86_64-pc-windows" triple ->
        win64_struct_register_threshold  (* Phase 2 で有効化 *)
    | _ ->
        (* デフォルト: System V ABI *)
        sysv_struct_register_threshold
    in

    if size <= threshold then
      DirectArg
    else
      ByvalArg llty

(* ========== LLVM属性設定関数 ========== *)

(** sret属性を関数に追加
 *
 * llvm-ocaml 標準バインディングが公開していない型付き属性 API を補うため、
 * `Llvm_attr.create_sret_attr` で `LLVMCreateTypeAttribute` を直接呼び出す。
 * 何らかの理由で属性種別が解決できない場合は、従来通りの文字列属性にフォールバックする。
 *
 * @param llctx LLVM コンテキスト
 * @param llvm_fn LLVM関数値
 * @param ret_ty 戻り値のLLVM型（構造体戻り値の場合は対象構造体）
 * @param param_index パラメータインデックス（通常0）
 *)
let add_sret_attr llctx llvm_fn ret_ty param_index =
  let attr_kind = Llvm.AttrIndex.Param param_index in
  let sret_attr =
    try Llvm_attr.create_sret_attr llctx ret_ty
    with Llvm.UnknownAttribute _ -> Llvm.create_string_attr llctx "sret" ""
  in
  Llvm.add_function_attr llvm_fn sret_attr attr_kind

(** byval属性を関数引数に追加
 *
 * `Llvm_attr.create_byval_attr` により型付き属性を設定する。
 * 属性種別が取得できないパスでは文字列属性へフォールバックする。
 *
 * @param llctx LLVM コンテキスト
 * @param llvm_fn LLVM関数値
 * @param arg_ty 引数のLLVM型（構造体 byval の場合は対象構造体）
 * @param param_index パラメータインデックス
 *)
let add_byval_attr llctx llvm_fn arg_ty param_index =
  let attr_kind = Llvm.AttrIndex.Param param_index in
  let byval_attr =
    try Llvm_attr.create_byval_attr llctx arg_ty
    with Llvm.UnknownAttribute _ -> Llvm.create_string_attr llctx "byval" ""
  in
  Llvm.add_function_attr llvm_fn byval_attr attr_kind

(* ========== デバッグ・診断関数 ========== *)

(** ABI分類の文字列表現を取得（デバッグ用）
 *
 * @param classification ABI分類
 * @return 文字列表現
 *)
let string_of_return_classification = function
  | DirectReturn -> "DirectReturn (register)"
  | SretReturn -> "SretReturn (memory via sret)"

(** ABI分類の文字列表現を取得（デバッグ用）
 *
 * @param classification ABI分類
 * @return 文字列表現
 *)
let string_of_argument_classification = function
  | DirectArg -> "DirectArg (register)"
  | ByvalArg _ -> "ByvalArg (memory via byval)"
