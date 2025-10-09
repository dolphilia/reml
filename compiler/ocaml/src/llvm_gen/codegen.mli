(* Codegen — Core IR から LLVM IR への変換 (Phase 3 Week 13-14)
 *
 * このファイルは codegen.ml の公開インターフェースを定義する。
 *)

(** コードジェネレーションエラー *)
exception CodegenError of string

(** コードジェネレーションコンテキスト
 *
 * LLVM モジュール・ビルダー・型マッピング・変数マッピングを管理する。
 * 内部実装の詳細は隠蔽される。
 *)
type codegen_context

(** コードジェネレーションコンテキストを作成
 *
 * @param module_name モジュール名
 * @param target_name ターゲット名（デフォルト: "x86_64-linux"）
 * @return 初期化されたコンテキスト
 *)
val create_codegen_context : string -> ?target_name:string -> unit -> codegen_context

(** LLVM モジュールを取得
 *
 * @param ctx コードジェネレーションコンテキスト
 * @return LLVM モジュール
 *)
val get_llmodule : codegen_context -> Llvm.llmodule

(** LLVM ビルダーを取得
 *
 * @param ctx コードジェネレーションコンテキスト
 * @return LLVM IR ビルダー
 *)
val get_builder : codegen_context -> Llvm.llbuilder

(** ランタイム関数を宣言
 *
 * mem_alloc, inc_ref, dec_ref, panic を外部リンケージで宣言する。
 *
 * @param ctx コードジェネレーションコンテキスト
 *)
val declare_runtime_functions : codegen_context -> unit

(** 関数宣言を生成
 *
 * Core IR の function_def から LLVM 関数宣言を生成する。
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param fn_def Core IR 関数定義
 * @return LLVM 関数値
 *)
val codegen_function_decl : codegen_context -> Core_ir.Ir.function_def -> Llvm.llvalue

(** グローバル変数定義を生成
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param global_def Core IR グローバル変数定義
 *)
val codegen_global_def : codegen_context -> Core_ir.Ir.global_def -> unit

(** 基本ブロックを生成
 *
 * Core IR の block リストから LLVM 基本ブロックを生成する。
 *
 * @param ctx コードジェネレーションコンテキスト
 * @param llvm_fn LLVM 関数値
 * @param blocks Core IR 基本ブロックリスト
 *)
val codegen_blocks : codegen_context -> Llvm.llvalue -> Core_ir.Ir.block list -> unit

(** モジュール全体を生成
 *
 * Core IR の module_def から LLVM モジュールを生成する。
 *
 * @param module_def Core IR モジュール定義
 * @param target_name ターゲット名（オプション）
 * @return LLVM モジュール
 *)
val codegen_module : ?target_name:string -> Core_ir.Ir.module_def -> Llvm.llmodule

(** LLVM IR をテキスト形式で出力
 *
 * @param llmodule LLVM モジュール
 * @param filename 出力ファイル名
 *)
val emit_llvm_ir : Llvm.llmodule -> string -> unit

(** LLVM IR をビットコード形式で出力
 *
 * @param llmodule LLVM モジュール
 * @param filename 出力ファイル名
 *)
val emit_llvm_bc : Llvm.llmodule -> string -> unit
