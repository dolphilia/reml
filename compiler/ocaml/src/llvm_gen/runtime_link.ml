(* Runtime_link — ランタイムライブラリとのリンク支援 (Phase 1-5)
 *
 * このファイルは docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §6.3 に基づき、
 * 生成した LLVM IR とランタイムライブラリをリンクする機能を提供する。
 *
 * 設計方針:
 * - ランタイムライブラリ (libreml_runtime.a) の検索
 * - プラットフォーム検出（macOS / Linux）に基づくリンカー設定
 * - Phase 1 スコープ: 静的リンクのみ対応
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §6.3
 * - runtime/native/Makefile
 *)

(* ========== エラー型 ========== *)

exception LinkError of string

let link_error msg = raise (LinkError msg)
let link_errorf fmt = Printf.ksprintf link_error fmt

(* ========== プラットフォーム検出 ========== *)

type platform = MacOS | Linux | Windows | Unknown

let detect_platform () =
  let uname =
    try
      let ic = Unix.open_process_in "uname -s" in
      let result = input_line ic in
      let _ = Unix.close_process_in ic in
      result
    with _ -> "Unknown"
  in
  match uname with
  | "Darwin" -> MacOS
  | "Linux" -> Linux
  | "Windows_NT" | "CYGWIN" | "MINGW" -> Windows
  | _ -> Unknown

(* ========== ランタイムライブラリの検索 ========== *)

(** ランタイムライブラリのデフォルトパスを取得
 *
 * 優先順位:
 * 1. 環境変数 REML_RUNTIME_PATH が設定されている場合はそれを使用
 * 2. runtime/native/build/libreml_runtime.a（ローカルビルド）
 * 3. /usr/local/lib/reml/libreml_runtime.a（インストール版）
 *)
let find_runtime_library () =
  (* 1. 環境変数チェック *)
  let env_path =
    try Some (Sys.getenv "REML_RUNTIME_PATH") with Not_found -> None
  in
  match env_path with
  | Some path when Sys.file_exists path -> path
  | _ ->
      (* 2. ローカルビルドパス *)
      let local_path = "runtime/native/build/libreml_runtime.a" in
      if Sys.file_exists local_path then local_path
      else
        (* 3. インストール版パス *)
        let installed_path = "/usr/local/lib/reml/libreml_runtime.a" in
        if Sys.file_exists installed_path then installed_path
        else
          link_errorf
            "ランタイムライブラリが見つかりません。\n\
             以下のいずれかを試してください:\n\
             1. runtime/native で 'make runtime' を実行\n\
             2. 環境変数 REML_RUNTIME_PATH を設定"

(* ========== リンカーコマンド生成 ========== *)

(** プラットフォームに応じたリンカーコマンドを生成
 *
 * @param platform 対象プラットフォーム
 * @param obj_file オブジェクトファイルパス (.o)
 * @param runtime_lib ランタイムライブラリパス (.a)
 * @param output_file 出力実行ファイルパス
 * @return リンカーコマンド文字列
 *)
let generate_linker_command platform obj_file runtime_lib output_file =
  match platform with
  | MacOS ->
      (* macOS: clang を使用 *)
      Printf.sprintf "clang %s %s -o %s -lSystem" obj_file runtime_lib
        output_file
  | Linux ->
      (* Linux: clang または gcc を使用 *)
      Printf.sprintf "clang %s %s -o %s -lc -lm" obj_file runtime_lib
        output_file
  | Windows ->
      (* Windows: lld-link を使用（未実装） *)
      link_error "Windows リンクは Phase 2 で対応予定"
  | Unknown -> link_errorf "サポートされていないプラットフォーム: %s" Sys.os_type

(* ========== 統合リンク関数 ========== *)

(** LLVM IR から実行可能ファイルを生成
 *
 * @param ir_file LLVM IR ファイルパス (.ll)
 * @param output_file 出力実行ファイルパス
 *)
let link_with_runtime ir_file output_file =
  let platform = detect_platform () in
  let runtime_lib = find_runtime_library () in

  (* 1. LLVM IR → オブジェクトファイル *)
  let obj_file = Filename.temp_file "reml_" ".o" in
  let llc_cmd = Printf.sprintf "llc -filetype=obj %s -o %s" ir_file obj_file in
  Printf.eprintf "実行: %s\n%!" llc_cmd;
  let llc_status = Sys.command llc_cmd in
  if llc_status <> 0 then link_errorf "llc でのオブジェクトファイル生成に失敗: %d" llc_status;

  (* 2. オブジェクトファイル + ランタイム → 実行ファイル *)
  let linker_cmd =
    generate_linker_command platform obj_file runtime_lib output_file
  in
  Printf.eprintf "実行: %s\n%!" linker_cmd;
  let link_status = Sys.command linker_cmd in

  (* 一時ファイルを削除 *)
  (try Sys.remove obj_file with _ -> ());

  if link_status <> 0 then link_errorf "リンクに失敗: %d" link_status

(** 簡易リンク（オブジェクトファイルが既にある場合）
 *
 * @param obj_file オブジェクトファイルパス (.o)
 * @param output_file 出力実行ファイルパス
 *)
let link_object_with_runtime obj_file output_file =
  let platform = detect_platform () in
  let runtime_lib = find_runtime_library () in

  let linker_cmd =
    generate_linker_command platform obj_file runtime_lib output_file
  in
  Printf.eprintf "実行: %s\n%!" linker_cmd;
  let link_status = Sys.command linker_cmd in

  if link_status <> 0 then link_errorf "リンクに失敗: %d" link_status
