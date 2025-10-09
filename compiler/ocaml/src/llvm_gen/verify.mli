(* Verify — LLVM IR 検証パイプライン (Phase 3 Week 15-16)
 *
 * このモジュールは生成されたLLVM IRを検証し、エラーを診断形式で報告する。
 *
 * 検証フロー:
 * 1. LLVM IR を一時ファイルに出力
 * 2. scripts/verify_llvm_ir.sh を実行
 * 3. エラー出力を Diagnostic.t へ変換
 *
 * 参考:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §6
 * - docs/spec/3-6-core-diagnostics-audit.md
 *)

(** 検証エラー型 *)
type verification_error =
  | AssembleError of string    (** llvm-as エラー *)
  | VerifyError of string       (** opt -verify エラー *)
  | CodegenError of string      (** llc エラー *)
  | ScriptError of string       (** スクリプト実行エラー *)

(** 検証結果 *)
type verification_result = (unit, verification_error) result

(** LLVM IR を検証
 *
 * @param llmodule LLVM モジュール
 * @return 検証結果（成功時は Ok ()、失敗時は Error エラー詳細）
 *)
val verify_llvm_ir : Llvm.llmodule -> verification_result

(** LLVM IR ファイルを検証
 *
 * @param llvm_ir_path LLVM IR ファイルパス (.ll)
 * @return 検証結果
 *)
val verify_llvm_ir_file : string -> verification_result

(** 検証エラーを診断形式へ変換
 *
 * @param error 検証エラー
 * @param span ソースコード位置（オプション）
 * @return 診断メッセージ
 *)
val error_to_diagnostic : verification_error -> Diagnostic.span option -> Diagnostic.t

(** 検証エラーを文字列化
 *
 * @param error 検証エラー
 * @return エラーメッセージ
 *)
val string_of_error : verification_error -> string
