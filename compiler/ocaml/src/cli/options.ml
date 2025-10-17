(* Options — コマンドラインオプション定義
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、既存 main.ml の ref 変数ベースの
 * オプション処理をレコード型に集約し、型安全なオプション管理を提供する。
 *)

(** 出力フォーマット *)
type output_format = Text  (** テキスト形式（デフォルト） *) | Json  (** JSON 形式（LSP 互換） *)

(** メトリクス出力フォーマット *)
type metrics_format = MetricsJson  (** JSON 形式 *) | MetricsCsv  (** CSV 形式 *)

(** カラーモード *)
type color_mode =
  | Auto  (** TTY への出力時のみカラー表示 *)
  | Always  (** 常にカラー表示（パイプ時も） *)
  | Never  (** カラー表示を無効化 *)

(** 型クラス戦略モード *)
type typeclass_mode =
  | TypeclassDictionary  (** 辞書渡し方式 *)
  | TypeclassMonomorph  (** モノモルフィゼーション PoC *)
  | TypeclassBoth  (** 両方の成果物を比較出力（PoC） *)

let print_full_help () =
  let lines =
    [
      "Reml OCaml コンパイラ CLI — Phase 1-6 開発者体験整備";
      "";
      "使い方:";
      "  remlc <入力ファイル.reml> [オプション]";
      "";
      "入力:";
      "  <file>              コンパイル対象の Reml ソースファイル";
      "";
      "出力制御:";
      "  --emit-ast          AST を標準出力に書き出す";
      "  --emit-tast         型付き AST を標準出力に書き出す";
      "  --emit-ir           LLVM IR (.ll) を出力ディレクトリに生成";
      "  --emit-bc           LLVM Bitcode (.bc) を出力ディレクトリに生成";
      "  --out-dir <dir>     中間成果物の出力先ディレクトリ（既定: .）";
      "";
      "診断・フォーマット:";
      "  --format <text|json>  診断出力形式（既定: text）";
      "  --color <auto|always|never>";
      "                       カラー表示の制御（既定: auto）";
      "";
      "型クラス戦略（PoC）:";
      "  --typeclass-mode <dictionary|monomorph|both>";
      "                       型クラス実装戦略の切り替え（既定: dictionary）";
      "";
      "トレース・統計:";
      "  --trace             フェーズ別トレースを標準エラーに表示";
      "  --stats             コンパイル統計情報を標準エラーに表示";
      "  --verbose <0-3>     ログ詳細度（環境変数 REMLC_LOG でも指定可）";
      "  --metrics <path>    統計情報をファイルに出力";
      "  --metrics-format <json|csv>";
      "                       メトリクス出力形式（既定: json）";
      "";
      "コンパイル設定:";
      "  --target <triple>   ターゲットトリプル（既定: x86_64-linux）";
      "  --link-runtime      ランタイムライブラリとリンクして実行可能ファイルを生成";
      "  --runtime-path <path>";
      "                       ランタイムライブラリへのパス（既定: \
       runtime/native/build/libreml_runtime.a）";
      "  --verify-ir         生成した LLVM IR を検証";
      "";
      "効果システム:";
      "  --effect-stage <stage>";
      "                       実行時 Stage を明示的に指定（例: experimental/beta/stable）";
      "  --runtime-capabilities <file>";
      "                       Runtime Capability Registry JSON を読み込み、Stage を解決";
      "";
      "例:";
      "  remlc examples/cli/add.reml --emit-ir --trace";
      "  remlc examples/cli/add.reml --link-runtime --out-dir build";
      "  remlc examples/cli/trace_sample.reml --trace --stats";
      "";
      "関連ドキュメント:";
      "  docs/guides/cli-workflow.md    CLI ワークフローガイド";
      "  docs/guides/trace-output.md    トレース・統計出力の詳細";
    ]
  in
  List.iter (fun line -> Printf.printf "%s\n" line) lines

type options = {
  (* 入力 *)
  input_file : string;  (** 入力ファイルパス（必須） *)
  use_stdin : bool;  (** 標準入力から読み込むか（Phase 2） *)
  (* 出力 *)
  emit_ast : bool;  (** AST を標準出力に出力 *)
  emit_tast : bool;  (** Typed AST を標準出力に出力 *)
  emit_ir : bool;  (** LLVM IR (.ll) を出力ディレクトリに生成 *)
  emit_bc : bool;  (** LLVM Bitcode (.bc) を出力ディレクトリに生成 *)
  out_dir : string;  (** 出力ディレクトリ *)
  (* 診断 *)
  format : output_format;  (** 診断メッセージの出力形式 *)
  color : color_mode;  (** カラー出力の制御 *)
  typeclass_mode : typeclass_mode;  (** 型クラス実装戦略モード *)
  (* デバッグ *)
  trace : bool;  (** コンパイルフェーズのトレースを有効化 *)
  stats : bool;  (** コンパイル統計情報を表示 *)
  verbose : int;  (** ログの詳細度レベル (0-3) *)
  metrics_path : string option;  (** メトリクス出力ファイルパス *)
  metrics_format : metrics_format;  (** メトリクス出力形式 *)
  (* コンパイル *)
  target : string;  (** ターゲットトリプル *)
  link_runtime : bool;  (** ランタイムライブラリとリンクして実行可能ファイルを生成 *)
  runtime_path : string;  (** ランタイムライブラリのパス *)
  verify_ir : bool;  (** 生成された LLVM IR を検証 *)
  (* 効果システム / Stage 制御 *)
  effect_stage_override : string option;  (** CLI で指定された Stage 名 *)
  runtime_capabilities_path : string option;  (** Capability Registry JSON のパス *)
}
(** コマンドラインオプション設定 *)

(** デフォルトオプション *)
let default_options =
  {
    input_file = "";
    use_stdin = false;
    emit_ast = false;
    emit_tast = false;
    emit_ir = false;
    emit_bc = false;
    out_dir = ".";
    format = Text;
    color = Auto;
    typeclass_mode = TypeclassDictionary;
    trace = false;
    stats = false;
    verbose = 1;
    metrics_path = None;
    metrics_format = MetricsJson;
    target = "x86_64-linux";
    link_runtime = false;
    runtime_path = "runtime/native/build/libreml_runtime.a";
    verify_ir = false;
    effect_stage_override = None;
    runtime_capabilities_path = None;
  }

(** 環境変数から color_mode を判定 *)
let color_mode_from_env () =
  try
    let _ = Sys.getenv "NO_COLOR" in
    Never
  with Not_found -> Auto

(** 環境変数から verbose レベルを判定 *)
let verbose_level_from_env () =
  try
    match Sys.getenv "REMLC_LOG" with
    | "error" -> 0
    | "warn" -> 1
    | "info" -> 2
    | "debug" -> 3
    | _ -> 1
  with Not_found -> 1

(** コマンドライン引数を解析してオプションを生成
 *
 * @param argv コマンドライン引数配列
 * @return 解析されたオプション、またはエラーメッセージ
 *)
let parse_args argv =
  let opts = ref default_options in

  (* 環境変数から初期値を設定 *)
  opts :=
    {
      !opts with
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
  let metrics_path = ref None in
  let metrics_format_str = ref "json" in
  let target = ref "x86_64-linux" in
  let link_runtime = ref false in
  let runtime_path = ref "runtime/native/build/libreml_runtime.a" in
  let verify_ir = ref false in
  let typeclass_mode_str = ref "dictionary" in
  let input_file = ref "" in
  let effect_stage = ref None in
  let runtime_caps_path = ref None in

  let usage_msg = "remlc-ocaml [options] <file>" in

  let speclist =
    [
      (* 出力オプション *)
      ("--emit-ast", Arg.Set emit_ast, "Emit AST to stdout");
      ("--emit-tast", Arg.Set emit_tast, "Emit Typed AST to stdout");
      ("--emit-ir", Arg.Set emit_ir, "Emit LLVM IR (.ll) to output directory");
      ( "--emit-bc",
        Arg.Set emit_bc,
        "Emit LLVM Bitcode (.bc) to output directory" );
      ( "--out-dir",
        Arg.Set_string out_dir,
        "<dir> Output directory (default: .)" );
      (* 診断オプション *)
      ( "--format",
        Arg.Set_string format_str,
        "<text|json> Output format (default: text)" );
      ( "--color",
        Arg.Set_string color_str,
        "<auto|always|never> Color mode (default: auto)" );
      (* デバッグオプション *)
      ("--trace", Arg.Set trace, "Enable phase tracing");
      ("--stats", Arg.Set stats, "Show compilation statistics");
      ("--verbose", Arg.Set_int verbose, "<0-3> Verbosity level (default: 1)");
      ( "--metrics",
        Arg.String (fun path -> metrics_path := Some path),
        "<path> Output metrics to file" );
      ( "--metrics-format",
        Arg.Set_string metrics_format_str,
        "<json|csv> Metrics output format (default: json)" );
      ( "--typeclass-mode",
        Arg.Set_string typeclass_mode_str,
        "<dictionary|monomorph|both> Type class strategy (default: dictionary)" );
      (* コンパイルオプション *)
      ( "--target",
        Arg.Set_string target,
        "<triple> Target triple (default: x86_64-linux)" );
      ( "--link-runtime",
        Arg.Set link_runtime,
        "Link with runtime library to produce executable" );
      ( "--runtime-path",
        Arg.Set_string runtime_path,
        "<path> Path to runtime library" );
      ("--verify-ir", Arg.Set verify_ir, "Verify generated LLVM IR");
      ( "--effect-stage",
        Arg.String (fun value -> effect_stage := Some value),
        "<stage> Override runtime Stage (experimental|beta|stable|...)" );
      ( "--runtime-capabilities",
        Arg.String (fun value -> runtime_caps_path := Some value),
        "<file> Load Runtime Capability Registry from JSON file" );
      ( "--version",
        Arg.Unit
          (fun () ->
            Version.print_version ();
            exit 0),
        "Show version information" );
      ( "-version",
        Arg.Unit
          (fun () ->
            Version.print_version ();
            exit 0),
        "Show version information" );
      ( "--help",
        Arg.Unit
          (fun () ->
            print_full_help ();
            exit 0),
        "Show detailed help" );
      ( "-help",
        Arg.Unit
          (fun () ->
            print_full_help ();
            exit 0),
        "Show detailed help" );
    ]
  in

  let anon_fun filename = input_file := filename in

  let current = ref 0 in
  try
    Arg.parse_argv ~current argv speclist anon_fun usage_msg;

    (* 入力ファイルのチェック *)
    if !input_file = "" then Error "Error: no input file"
    else
      (* format_str を output_format に変換 *)
      let format =
        match !format_str with
        | "text" -> Text
        | "json" -> Json
        | other ->
            prerr_endline
              (Printf.sprintf "Warning: unknown format '%s', using 'text'" other);
            Text
      in

      (* color_str を color_mode に変換 *)
      let color =
        match !color_str with
        | "auto" -> Auto
        | "always" -> Always
        | "never" -> Never
        | other ->
            prerr_endline
              (Printf.sprintf "Warning: unknown color mode '%s', using 'auto'"
                 other);
            Auto
      in

      (* verbose レベルの範囲チェック *)
      let verbose_level =
        if !verbose < 0 then 0 else if !verbose > 3 then 3 else !verbose
      in

      (* metrics_format_str を metrics_format に変換 *)
      let metrics_fmt =
        match !metrics_format_str with
        | "json" -> MetricsJson
        | "csv" -> MetricsCsv
        | other ->
            prerr_endline
              (Printf.sprintf
                 "Warning: unknown metrics format '%s', using 'json'" other);
            MetricsJson
      in

      (* typeclass_mode_str を typeclass_mode に変換 *)
      let typeclass_mode =
        match String.lowercase_ascii !typeclass_mode_str with
        | "dictionary" -> TypeclassDictionary
        | "monomorph" -> TypeclassMonomorph
        | "both" -> TypeclassBoth
        | other ->
            prerr_endline
              (Printf.sprintf
                 "Warning: unknown typeclass mode '%s', using 'dictionary'"
                 other);
            TypeclassDictionary
      in

      Ok
        {
          input_file = !input_file;
          use_stdin = false;
          emit_ast = !emit_ast;
          emit_tast = !emit_tast;
          emit_ir = !emit_ir;
          emit_bc = !emit_bc;
          out_dir = !out_dir;
          format;
          color;
          typeclass_mode;
          trace = !trace;
          stats = !stats;
          verbose = verbose_level;
          metrics_path = !metrics_path;
          metrics_format = metrics_fmt;
          target = !target;
          link_runtime = !link_runtime;
          runtime_path = !runtime_path;
          verify_ir = !verify_ir;
          effect_stage_override = !effect_stage;
          runtime_capabilities_path = !runtime_caps_path;
        }
  with
  | Arg.Help _ ->
      print_full_help ();
      exit 0
  | Arg.Bad msg -> Error msg
