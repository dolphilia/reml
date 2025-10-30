(* Verify — LLVM IR 検証パイプライン (Phase 3 Week 15-16)
 *
 * このモジュールは生成されたLLVM IRを検証し、エラーを診断形式で報告する。
 *)

(* ========== エラー型 ========== *)

type verification_error =
  | AssembleError of string  (** llvm-as エラー *)
  | VerifyError of string  (** opt -verify エラー *)
  | CodegenError of string  (** llc エラー *)
  | ScriptError of string  (** スクリプト実行エラー *)

type verification_result = (unit, verification_error) result

(* ========== ヘルパー関数 ========== *)

(** 一時ディレクトリ取得 *)
let get_temp_dir () = try Sys.getenv "TMPDIR" with Not_found -> "/tmp"

(** 一時ファイルパス生成 *)
let temp_file_path prefix suffix =
  let temp_dir = get_temp_dir () in
  let timestamp = Unix.time () |> int_of_float |> string_of_int in
  let pid = Unix.getpid () |> string_of_int in
  Printf.sprintf "%s/%s_%s_%s%s" temp_dir prefix timestamp pid suffix

(** 検証スクリプトパス取得 *)
let get_verify_script_path () =
  let rec search dir depth =
    if depth > 8 then None
    else
      let candidate_scripts =
        [
          Filename.concat dir "scripts/verify_llvm_ir.sh";
          Filename.concat dir "compiler/ocaml/scripts/verify_llvm_ir.sh";
        ]
      in
      match List.find_opt Sys.file_exists candidate_scripts with
      | Some path -> Some path
      | None ->
          let parent = Filename.dirname dir in
          if parent = dir then None else search parent (depth + 1)
  in
  match search (Sys.getcwd ()) 0 with
  | Some path -> path
  | None -> failwith "検証スクリプトが見つかりません: compiler/ocaml/scripts/verify_llvm_ir.sh"

(* ========== 検証実装 ========== *)

(** LLVM IR ファイルを検証
 *
 * scripts/verify_llvm_ir.sh を実行し、終了コードとエラー出力を解析する。
 *)
let verify_llvm_ir_file llvm_ir_path =
  let script_path = get_verify_script_path () in

  (* スクリプト実行 *)
  let cmd = Printf.sprintf "%s %s 2>&1" script_path llvm_ir_path in
  let ic = Unix.open_process_in cmd in
  let output_lines = ref [] in

  (* 出力読み取り *)
  (try
     while true do
       output_lines := input_line ic :: !output_lines
     done
   with End_of_file -> ());

  let status = Unix.close_process_in ic in
  let output = String.concat "\n" (List.rev !output_lines) in

  (* 終了コード解析 *)
  match status with
  | Unix.WEXITED 0 -> Ok ()
  | Unix.WEXITED 2 -> Error (AssembleError output)
  | Unix.WEXITED 3 -> Error (VerifyError output)
  | Unix.WEXITED 4 -> Error (CodegenError output)
  | Unix.WEXITED code ->
      Error (ScriptError (Printf.sprintf "終了コード %d: %s" code output))
  | Unix.WSIGNALED signal ->
      Error (ScriptError (Printf.sprintf "シグナル %d で終了" signal))
  | Unix.WSTOPPED signal ->
      Error (ScriptError (Printf.sprintf "シグナル %d で停止" signal))

(** LLVM IR を検証
 *
 * LLVM モジュールを一時ファイルに出力し、検証スクリプトを実行する。
 *)
let verify_llvm_ir llmodule =
  let temp_ll = temp_file_path "reml_verify" ".ll" in

  (* LLVM IR をファイル出力 *)
  (try Llvm.print_module temp_ll llmodule
   with e ->
     (* 出力失敗 *)
     let msg = Printf.sprintf "LLVM IR 出力失敗: %s" (Printexc.to_string e) in
     Error (ScriptError msg) |> fun _ -> raise e);

  (* 検証実行 *)
  let result = verify_llvm_ir_file temp_ll in

  (* 一時ファイル削除 *)
  (try Sys.remove temp_ll with _ -> ());

  result

(* ========== ユーティリティ ========== *)

(** List.take 互換実装（OCaml 4.14 対応） *)
let rec take n lst =
  if n <= 0 then []
  else match lst with [] -> [] | x :: xs -> x :: take (n - 1) xs

(* String.starts_with 互換実装（OCaml < 4.13） *)
let string_starts_with ~prefix str =
  let prefix_len = String.length prefix in
  let str_len = String.length str in
  if prefix_len > str_len then false else String.sub str 0 prefix_len = prefix

(* ========== 診断変換 ========== *)

(** 検証エラーを文字列化 *)
let string_of_error = function
  | AssembleError msg -> Printf.sprintf "llvm-as エラー: %s" msg
  | VerifyError msg -> Printf.sprintf "opt -verify エラー: %s" msg
  | CodegenError msg -> Printf.sprintf "llc エラー: %s" msg
  | ScriptError msg -> Printf.sprintf "スクリプトエラー: %s" msg

(** 検証エラーを診断形式へ変換 *)
let error_to_diagnostic error span_opt =
  let severity = Diagnostic.Error in
  let code =
    match error with
    | AssembleError _ -> "E9001"
    | VerifyError _ -> "E9002"
    | CodegenError _ -> "E9003"
    | ScriptError _ -> "E9004"
  in
  let message = string_of_error error in
  let dummy_loc =
    Diagnostic.{ filename = "<unknown>"; line = 0; column = 0; offset = 0 }
  in
  let span =
    match span_opt with
    | Some s -> s
    | None -> { Diagnostic.start_pos = dummy_loc; end_pos = dummy_loc }
  in

  (* LLVM エラー出力から詳細情報を抽出（簡易実装） *)
  let notes =
    match error with
    | VerifyError msg when String.length msg > 0 ->
        let lines = String.split_on_char '\n' msg in
        let relevant_lines =
          List.filter
            (fun line ->
              String.length line > 0
              && not (string_starts_with ~prefix:"[" line))
            lines
        in
        List.map
          (fun line ->
            (Some span, line) (* notes は (span option * string) list 型 *))
          (take 5 relevant_lines)
        (* 最大5行まで *)
    | _ -> []
  in

  Diagnostic.(
    Builder.create ~severity ?severity_hint:None ~domain:Cli ~message
      ~primary:span ()
    |> Builder.set_primary_code code
    |> Builder.add_notes notes
    |> Builder.build)
