(* Target_config — ターゲット固有の設定インターフェース (Phase 3)
 *
 * このモジュールは docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §3 に基づき、
 * ターゲットアーキテクチャ固有の DataLayout とトリプル設定を提供する。
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §3
 * - docs/guides/llvm-integration-notes.md §5.0
 *)

(* ========== ターゲットトリプル ========== *)

val x86_64_linux_triple : string
(** x86_64 Linux (System V ABI) のターゲットトリプル *)

val x86_64_windows_triple : string
(** x86_64 Windows (MSVC) のターゲットトリプル *)

val aarch64_linux_triple : string
(** ARM64 Linux のターゲットトリプル *)

(* ========== DataLayout 文字列 ========== *)

val x86_64_linux_datalayout : string
(** x86_64 Linux (System V ABI) の DataLayout *)

val x86_64_windows_datalayout : string
(** x86_64 Windows (MSVC) の DataLayout *)

val aarch64_linux_datalayout : string
(** ARM64 Linux の DataLayout *)

(* ========== 呼び出し規約 ========== *)

val calling_convention_sysv : string
(** System V AMD64 呼び出し規約 *)

val calling_convention_win64 : string
(** Windows x64 呼び出し規約 *)

(* ========== ポインタサイズ ========== *)

val pointer_size_bits : string -> int
(** ターゲット別のポインタサイズ（ビット単位） *)

val pointer_size_bytes : string -> int
(** ターゲット別のポインタサイズ（バイト単位） *)

(* ========== アラインメント設定 ========== *)

type alignment_spec = {
  i8_align : int;
  i16_align : int;
  i32_align : int;
  i64_align : int;
  f32_align : int;
  f64_align : int;
  ptr_align : int;
  aggregate_align : int;
}
(** アラインメント仕様 *)

val x86_64_linux_alignment : alignment_spec
(** x86_64 Linux のアラインメント仕様 *)

val x86_64_windows_alignment : alignment_spec
(** x86_64 Windows のアラインメント仕様 *)

val get_alignment_spec : string -> alignment_spec
(** ターゲット別のアラインメント仕様を取得 *)

(* ========== ターゲット設定 ========== *)

type target_config = {
  triple : string;
  datalayout : string;
  calling_conv : string;
  alignment : alignment_spec;
}
(** ターゲット設定 *)

val default_target : target_config
(** デフォルトターゲット（x86_64 Linux） *)

val get_target_config : string -> target_config
(** ターゲット名からターゲット設定を取得
 *
 * @param target ターゲット名（"x86_64-linux", "windows" 等）
 * @return ターゲット設定
 *)

val set_target_config : Llvm.llmodule -> target_config -> unit
(** LLVM モジュールにターゲット情報を設定
 *
 * @param llmodule LLVM モジュール
 * @param config ターゲット設定
 *)
