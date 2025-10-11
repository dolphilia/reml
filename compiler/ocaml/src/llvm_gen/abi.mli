(* Abi — LLVM ABI呼び出し規約の実装 (Phase 3 Week 14-15)
 *
 * このファイルは docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §5 に基づき、
 * System V ABI および将来のWindows x64 ABI対応のためのABI判定・属性設定を提供する。
 *
 * 設計方針:
 * - System V ABI (x86_64 Linux): 16バイト以下の構造体はレジスタ渡し、超過はメモリ経由
 * - LLVM属性: sret（構造体戻り値）、byval（値渡し構造体引数）を適切に設定
 * - Phase 1スコープ: タプル・レコード型のみ対応、ADTは Phase 2 で拡張
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §5
 * - docs/guides/llvm-integration-notes.md §5.0, §5.2
 * - System V AMD64 ABI Specification
 *)

(* ========== ABI分類型 ========== *)

(** 構造体戻り値のABI分類 *)
type return_classification =
  | DirectReturn  (** レジスタ経由で直接返却（16バイト以下） *)
  | SretReturn  (** メモリ経由で返却（sret属性、16バイト超過） *)

(** 構造体引数のABI分類 *)
type argument_classification =
  | DirectArg  (** レジスタ経由で直接渡す（16バイト以下） *)
  | ByvalArg of Llvm.lltype  (** メモリ経由で値渡し（byval属性、16バイト超過） *)

(* ========== ABI判定関数 ========== *)

val classify_struct_return :
  Target_config.target_config ->
  Type_mapping.type_mapping_context ->
  Types.ty ->
  return_classification
(** 構造体戻り値のABI分類を判定
 *
 * System V ABI: 16バイト以下はレジスタ、超過はsret
 * Windows x64: 8バイト以下はレジスタ（Phase 2実装予定）
 *
 * @param target ターゲット設定
 * @param ctx 型マッピングコンテキスト
 * @param ty Reml型
 * @return ABI分類（DirectReturn または SretReturn）
 *)

val classify_struct_argument :
  Target_config.target_config ->
  Type_mapping.type_mapping_context ->
  Types.ty ->
  argument_classification
(** 構造体引数のABI分類を判定
 *
 * System V ABI: 16バイト以下はレジスタ、超過はbyval
 *
 * @param target ターゲット設定
 * @param ctx 型マッピングコンテキスト
 * @param ty Reml型
 * @return ABI分類（DirectArg または ByvalArg）
 *)

val get_type_size : Llvm.llcontext -> Llvm.lltype -> int
(** 型のサイズをバイト数で取得
 *
 * @param llctx LLVM コンテキスト
 * @param llty LLVM型
 * @return サイズ（バイト数）
 *)

(* ========== LLVM属性設定関数 ========== *)

val add_sret_attr : Llvm.llcontext -> Llvm.llvalue -> Llvm.lltype -> int -> unit
(** sret属性を関数に追加
 *
 * 大きな構造体を返す関数の第1引数（隠れた戻り値用ポインタ）にsret属性を設定する。
 *
 * @param llctx LLVM コンテキスト
 * @param llvm_fn LLVM関数値
 * @param ret_ty 戻り値のLLVM型（構造体型）
 * @param param_index パラメータインデックス（通常0）
 *)

val add_byval_attr :
  Llvm.llcontext -> Llvm.llvalue -> Llvm.lltype -> int -> unit
(** byval属性を関数引数に追加
 *
 * 大きな構造体引数を値渡しする際にbyval属性を設定する。
 *
 * @param llctx LLVM コンテキスト
 * @param llvm_fn LLVM関数値
 * @param arg_ty 引数のLLVM型（構造体型）
 * @param param_index パラメータインデックス
 *)

(* ========== デバッグ・診断関数 ========== *)

val string_of_return_classification : return_classification -> string
(** ABI分類の文字列表現を取得（デバッグ用）
 *
 * @param classification ABI分類
 * @return 文字列表現
 *)

val string_of_argument_classification : argument_classification -> string
(** ABI分類の文字列表現を取得（デバッグ用）
 *
 * @param classification ABI分類
 * @return 文字列表現
 *)
