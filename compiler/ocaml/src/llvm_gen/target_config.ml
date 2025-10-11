(* Target_config — ターゲット固有の設定 (Phase 3)
 *
 * このファイルは docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §3 に基づき、
 * ターゲットアーキテクチャ固有の DataLayout とトリプル設定を提供する。
 *
 * Phase 3 では x86_64 Linux (System V ABI) を主要ターゲットとする。
 * Phase 2 以降で Windows x64、ARM64 への対応を追加予定。
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §3
 * - docs/guides/llvm-integration-notes.md §5.0
 * - docs/notes/llvm-spec-status-survey.md
 *)

(* ========== ターゲットトリプル ========== *)

(** x86_64 Linux (System V ABI) のターゲットトリプル *)
let x86_64_linux_triple = "x86_64-unknown-linux-gnu"

(** x86_64 Windows (MSVC) のターゲットトリプル（将来実装） *)
let x86_64_windows_triple = "x86_64-pc-windows-msvc"

(** ARM64 Linux のターゲットトリプル（将来実装） *)
let aarch64_linux_triple = "aarch64-unknown-linux-gnu"

(* ========== DataLayout 文字列 ========== *)

(** x86_64 Linux (System V ABI) の DataLayout
 *
 * フォーマット: "e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64"
 *
 * 詳細:
 * - e: リトルエンディアン
 * - m:e: ELF マングリング
 * - p:64:64: ポインタは64ビット、アラインメント64ビット
 * - f64:64:64: double は64ビット、アラインメント64ビット
 * - v128:128:128: ベクトル型（SIMD）は128ビット、アラインメント128ビット
 * - a:0:64: 集成体のアラインメントは64ビット
 *
 * 参考: docs/guides/llvm-integration-notes.md §5.0
 *)
let x86_64_linux_datalayout = "e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64"

(** x86_64 Windows (MSVC) の DataLayout（将来実装） *)
let x86_64_windows_datalayout = "e-m:w-p:64:64-f64:64:64-v128:128:128-a:0:64"

(** ARM64 Linux の DataLayout（将来実装） *)
let aarch64_linux_datalayout =
  "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128"

(* ========== 呼び出し規約 ========== *)

(** System V AMD64 呼び出し規約
 *
 * - 整数引数: RDI, RSI, RDX, RCX, R8, R9
 * - 浮動小数引数: XMM0-XMM7
 * - 戻り値: RAX, RDX（構造体の場合）
 * - スタック: 16バイトアラインメント
 *
 * LLVM では `cc ccc`（C calling convention）として自動処理される。
 *)
let calling_convention_sysv = "ccc"

(** Windows x64 呼び出し規約
 *
 * - 整数引数: RCX, RDX, R8, R9
 * - 浮動小数引数: XMM0-XMM3
 * - シャドウスペース: 32バイト
 *
 * LLVM では `cc win64cc` として処理される。
 *)
let calling_convention_win64 = "win64cc"

(* ========== ポインタサイズ ========== *)

(** ターゲット別のポインタサイズ（ビット単位） *)
let pointer_size_bits = function
  | "x86_64-unknown-linux-gnu" -> 64
  | "x86_64-pc-windows-msvc" -> 64
  | "aarch64-unknown-linux-gnu" -> 64
  | _ -> 64 (* デフォルト *)

(** ターゲット別のポインタサイズ（バイト単位） *)
let pointer_size_bytes target_triple = pointer_size_bits target_triple / 8

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
(** プリミティブ型のアラインメント（バイト単位）
 *
 * System V ABI x86_64 の規則に従う。
 *)

let x86_64_linux_alignment =
  {
    i8_align = 1;
    i16_align = 2;
    i32_align = 4;
    i64_align = 8;
    f32_align = 4;
    f64_align = 8;
    ptr_align = 8;
    aggregate_align = 8;
  }

let x86_64_windows_alignment =
  {
    i8_align = 1;
    i16_align = 2;
    i32_align = 4;
    i64_align = 8;
    f32_align = 4;
    f64_align = 8;
    ptr_align = 8;
    aggregate_align = 8;
  }

(** ターゲット別のアラインメント仕様を取得 *)
let get_alignment_spec = function
  | "x86_64-unknown-linux-gnu" -> x86_64_linux_alignment
  | "x86_64-pc-windows-msvc" -> x86_64_windows_alignment
  | _ -> x86_64_linux_alignment
(* デフォルト *)

(* ========== ターゲット設定の取得 ========== *)

type target_config = {
  triple : string;
  datalayout : string;
  calling_conv : string;
  alignment : alignment_spec;
}
(** ターゲット設定 *)

(** デフォルトターゲット（x86_64 Linux） *)
let default_target =
  {
    triple = x86_64_linux_triple;
    datalayout = x86_64_linux_datalayout;
    calling_conv = calling_convention_sysv;
    alignment = x86_64_linux_alignment;
  }

(** ターゲット名からターゲット設定を取得 *)
let get_target_config = function
  | "x86_64-linux" | "linux" | "" ->
      {
        triple = x86_64_linux_triple;
        datalayout = x86_64_linux_datalayout;
        calling_conv = calling_convention_sysv;
        alignment = x86_64_linux_alignment;
      }
  | "x86_64-windows" | "windows" ->
      {
        triple = x86_64_windows_triple;
        datalayout = x86_64_windows_datalayout;
        calling_conv = calling_convention_win64;
        alignment = x86_64_windows_alignment;
      }
  | triple ->
      (* カスタムトリプルの場合はデフォルト設定を使用 *)
      {
        triple;
        datalayout = x86_64_linux_datalayout;
        calling_conv = calling_convention_sysv;
        alignment = x86_64_linux_alignment;
      }

(** LLVM モジュールにターゲット情報を設定 *)
let set_target_config llmodule config =
  Llvm.set_target_triple config.triple llmodule;
  Llvm.set_data_layout config.datalayout llmodule
