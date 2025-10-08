(* Type_mapping — Reml 型から LLVM IR 型へのマッピング (Phase 3)
 *
 * このモジュールは docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §2 に基づき、
 * Reml の型システム (Types.ty) と LLVM IR の型システム (Llvm.lltype) の対応付けを提供する。
 *
 * 設計原則:
 * - すべての Reml 型が LLVM IR 型に変換可能
 * - ABI 互換性を保証（System V ABI、x86_64 Linux）
 * - FAT pointer、tagged union など高レベル型の lowering
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §2
 * - docs/guides/llvm-integration-notes.md §5.1
 * - docs/spec/1-2-types-Inference.md
 *)

(** LLVM コンテキスト *)
type llvm_context = Llvm.llcontext

(** 型マッピングコンテキスト
 *
 * LLVM モジュールと型キャッシュを保持する。
 *)
type type_mapping_context = {
  llctx: llvm_context;                                (** LLVM コンテキスト *)
  llmodule: Llvm.llmodule;                            (** LLVM モジュール *)
  mutable type_cache: (Types.ty, Llvm.lltype) Hashtbl.t; (** 型変換キャッシュ *)
}

(** 型マッピングコンテキストの作成 *)
val create_context : string -> type_mapping_context

(** Reml 型を LLVM 型に変換
 *
 * すべての Reml 型を対応する LLVM IR 型に変換する。
 * 複雑な型（FAT pointer、tagged union）は構造体として表現される。
 *
 * @param ctx 型マッピングコンテキスト
 * @param ty Reml 型
 * @return LLVM IR 型
 *)
val reml_type_to_llvm : type_mapping_context -> Types.ty -> Llvm.lltype

(** 型のサイズを取得（バイト単位）
 *
 * ターゲットアーキテクチャにおける型のサイズを返す。
 * 可変長型（スライス、String）は FAT pointer のサイズを返す。
 *
 * @param ctx 型マッピングコンテキスト
 * @param ty Reml 型
 * @return サイズ（バイト）
 *)
val get_type_size : type_mapping_context -> Types.ty -> int

(** 型のアラインメントを取得（バイト単位）
 *
 * ターゲットアーキテクチャにおける型のアラインメント要件を返す。
 *
 * @param ctx 型マッピングコンテキスト
 * @param ty Reml 型
 * @return アラインメント（バイト）
 *)
val get_type_alignment : type_mapping_context -> Types.ty -> int

(** FAT pointer 型を作成
 *
 * スライス型や String 型で使用される { ptr, i64 } 構造体を生成する。
 *
 * @param ctx 型マッピングコンテキスト
 * @param element_ty 要素型（スライスの場合）
 * @return FAT pointer の LLVM 構造体型
 *)
val make_fat_pointer : type_mapping_context -> Llvm.lltype option -> Llvm.lltype

(** Tagged union 型を作成
 *
 * ADT（代数的データ型）で使用される { i32 tag, payload } 構造体を生成する。
 *
 * @param ctx 型マッピングコンテキスト
 * @param payload_ty ペイロード型
 * @return Tagged union の LLVM 構造体型
 *)
val make_tagged_union : type_mapping_context -> Llvm.lltype -> Llvm.lltype

(** クロージャ型を作成
 *
 * クロージャで使用される { env_ptr*, code_ptr } 構造体を生成する。
 *
 * @param ctx 型マッピングコンテキスト
 * @param fn_ty 関数ポインタ型
 * @return クロージャの LLVM 構造体型
 *)
val make_closure : type_mapping_context -> Llvm.lltype -> Llvm.lltype
