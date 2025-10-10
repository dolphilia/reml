(* Options — コマンドラインオプション定義
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、既存 main.ml の ref 変数ベースの
 * オプション処理をレコード型に集約し、型安全なオプション管理を提供する。
 *)

(** 出力フォーマット *)
type output_format =
  | Text  (** テキスト形式（デフォルト） *)
  | Json  (** JSON 形式（LSP 互換） *)

(** カラーモード *)
type color_mode =
  | Auto    (** TTY への出力時のみカラー表示 *)
  | Always  (** 常にカラー表示（パイプ時も） *)
  | Never   (** カラー表示を無効化 *)

(** コマンドラインオプション設定 *)
type options = {
  (* 入力 *)
  input_file: string;           (** 入力ファイルパス（必須） *)
  use_stdin: bool;              (** 標準入力から読み込むか（Phase 2） *)

  (* 出力 *)
  emit_ast: bool;               (** AST を標準出力に出力 *)
  emit_tast: bool;              (** Typed AST を標準出力に出力 *)
  emit_ir: bool;                (** LLVM IR (.ll) を出力ディレクトリに生成 *)
  emit_bc: bool;                (** LLVM Bitcode (.bc) を出力ディレクトリに生成 *)
  out_dir: string;              (** 出力ディレクトリ *)

  (* 診断 *)
  format: output_format;        (** 診断メッセージの出力形式 *)
  color: color_mode;            (** カラー出力の制御 *)

  (* デバッグ *)
  trace: bool;                  (** コンパイルフェーズのトレースを有効化 *)
  stats: bool;                  (** コンパイル統計情報を表示 *)
  verbose: int;                 (** ログの詳細度レベル (0-3) *)

  (* コンパイル *)
  target: string;               (** ターゲットトリプル *)
  link_runtime: bool;           (** ランタイムライブラリとリンクして実行可能ファイルを生成 *)
  runtime_path: string;         (** ランタイムライブラリのパス *)
  verify_ir: bool;              (** 生成された LLVM IR を検証 *)
}

(** デフォルトオプション *)
let default_options = {
  input_file = "";
  use_stdin = false;

  emit_ast = false;
  emit_tast = false;
  emit_ir = false;
  emit_bc = false;
  out_dir = ".";

  format = Text;
  color = Auto;

  trace = false;
  stats = false;
  verbose = 1;

  target = "x86_64-linux";
  link_runtime = false;
  runtime_path = "runtime/native/build/libreml_runtime.a";
  verify_ir = false;
}

(** 環境変数から color_mode を判定 *)
let color_mode_from_env () =
  try
    let _ = Sys.getenv "NO_COLOR" in
    Never
  with Not_found ->
    Auto

(** 環境変数から verbose レベルを判定 *)
let verbose_level_from_env () =
  try
    match Sys.getenv "REMLC_LOG" with
    | "error" -> 0
    | "warn" -> 1
    | "info" -> 2
    | "debug" -> 3
    | _ -> 1
  with Not_found ->
    1

(** コマンドライン引数を解析してオプションを生成
 *
 * @param argv コマンドライン引数配列
 * @return 解析されたオプション、またはエラーメッセージ
 *)
let parse_args argv =
  let opts = ref default_options in

  (* 環境変数から初期値を設定 *)
  opts := { !opts with
    color = color_mode_from_env ();
    verbose = verbose_level_from_env ();
  };

  (* Arg.parse で使用する ref 変数 *)
  let emit_ast = ref false in
  let emit_tast = ref false in
  let emit_ir = ref false in
  let emit_bc = ref false in
  let out_dir = ref "." in
  let format_str = ref "text" in
  let color_str = ref "auto" in
  let trace = ref false in
  let stats = ref false in
  let verbose = ref 1 in
  let target = ref "x86_64-linux" in
  let link_runtime = ref false in
  let runtime_path = ref "runtime/native/build/libreml_runtime.a" in
  let verify_ir = ref false in
  let input_file = ref "" in

  let usage_msg = "remlc-ocaml [options] <file>" in

  let speclist = [
    (* 出力オプション *)
    ("--emit-ast", Arg.Set emit_ast, "Emit AST to stdout");
    ("--emit-tast", Arg.Set emit_tast, "Emit Typed AST to stdout");
    ("--emit-ir", Arg.Set emit_ir, "Emit LLVM IR (.ll) to output directory");
    ("--emit-bc", Arg.Set emit_bc, "Emit LLVM Bitcode (.bc) to output directory");
    ("--out-dir", Arg.Set_string out_dir, "<dir> Output directory (default: .)");

    (* 診断オプション *)
    ("--format", Arg.Set_string format_str, "<text|json> Output format (default: text)");
    ("--color", Arg.Set_string color_str, "<auto|always|never> Color mode (default: auto)");

    (* デバッグオプション *)
    ("--trace", Arg.Set trace, "Enable phase tracing");
    ("--stats", Arg.Set stats, "Show compilation statistics");
    ("--verbose", Arg.Set_int verbose, "<0-3> Verbosity level (default: 1)");

    (* コンパイルオプション *)
    ("--target", Arg.Set_string target, "<triple> Target triple (default: x86_64-linux)");
    ("--link-runtime", Arg.Set link_runtime, "Link with runtime library to produce executable");
    ("--runtime-path", Arg.Set_string runtime_path, "<path> Path to runtime library");
    ("--verify-ir", Arg.Set verify_ir, "Verify generated LLVM IR");
  ] in

  let anon_fun filename =
    input_file := filename
  in

  Arg.parse_argv argv speclist anon_fun usage_msg;

  (* 入力ファイルのチェック *)
  if !input_file = "" then
    Error "Error: no input file"
  else
    (* format_str を output_format に変換 *)
    let format = match !format_str with
      | "text" -> Text
      | "json" -> Json
      | other ->
          prerr_endline (Printf.sprintf "Warning: unknown format '%s', using 'text'" other);
          Text
    in

    (* color_str を color_mode に変換 *)
    let color = match !color_str with
      | "auto" -> Auto
      | "always" -> Always
      | "never" -> Never
      | other ->
          prerr_endline (Printf.sprintf "Warning: unknown color mode '%s', using 'auto'" other);
          Auto
    in

    (* verbose レベルの範囲チェック *)
    let verbose_level =
      if !verbose < 0 then 0
      else if !verbose > 3 then 3
      else !verbose
    in

    Ok {
      input_file = !input_file;
      use_stdin = false;

      emit_ast = !emit_ast;
      emit_tast = !emit_tast;
      emit_ir = !emit_ir;
      emit_bc = !emit_bc;
      out_dir = !out_dir;

      format = format;
      color = color;

      trace = !trace;
      stats = !stats;
      verbose = verbose_level;

      target = !target;
      link_runtime = !link_runtime;
      runtime_path = !runtime_path;
      verify_ir = !verify_ir;
    }
