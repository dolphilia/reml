(* Options — コマンドラインオプション定義
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、既存 main.ml の ref 変数ベースの
 * オプション処理をレコード型に集約し、型安全なオプション管理を提供する。
 *)

module Run_config = Parser_run_config
module Extensions = Parser_run_config.Extensions
module Lex = Parser_run_config.Lex

(** 出力フォーマット *)
type output_format = Text  (** テキスト形式（デフォルト） *) | Json  (** JSON 形式（LSP 互換） *)

(** JSON 出力モード *)
type json_mode =
  | JsonPretty  (** 整形済み JSON *)
  | JsonCompact  (** 1 行 JSON *)
  | JsonLines  (** JSON Lines 形式（1 行1診断） *)

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

(** 監査ログ出力ストアプロファイル *)
type audit_store =
  | AuditStoreTmp  (** 互換モード: tmp/cli-callconv-out 配下へ出力 *)
  | AuditStoreLocal  (** ローカル永続ストア（tooling/audit-store/local） *)
  | AuditStoreCi  (** CI 永続ストア（reports/audit） *)

(** 監査ログ詳細度 *)
type audit_level =
  | AuditLevelSummary  (** 必須キーのみ *)
  | AuditLevelFull  (** Phase 2-3 合意済みフィールド一式 *)
  | AuditLevelDebug  (** 拡張フィールド含む完全ログ *)

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
      "  --emit-parse-debug <path>";
      "                       Parser debug JSON (packrat/span_trace) を指定パスへ出力";
      "  --out-dir <dir>     中間成果物の出力先ディレクトリ（既定: .）";
      "";
      "診断・フォーマット:";
      "  --format <text|json>    診断出力形式（既定: text）";
      "  --json-mode <pretty|compact|lines>";
      "                           JSON 出力モード（既定: pretty）";
      "  --no-snippet           テキスト診断からソーススニペットを除外";
      "  --color <auto|always|never>";
      "                           カラー表示の制御（既定: auto）";
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
      "Parser 実行設定 (PoC):";
      "  --require-eof       RunConfig.require_eof=true を指定";
      "  --packrat           RunConfig.packrat を有効化（実験的）";
      "  --experimental-effects / -Zalgebraic-effects";
      "                       効果構文 PoC を受理（RunConfig.experimental_effects=true）";
      "  --left-recursion <off|on|auto>";
      "                       左再帰処理モード（既定: auto）";
      "  --no-merge-warnings 診断警告を統合せず個別に出力";
      "  --streaming         ストリーミングランナー PoC を有効化";
      "  --stream-chunk-size <bytes>";
      "                       ファイルをチャンク化する際のバイト数";
      "  --stream-checkpoint <token>";
      "                       extensions[\"stream\"].checkpoint を設定";
      "  --stream-resume-hint <hint>";
      "                       extensions[\"stream\"].resume_hint を設定";
      "  --stream-demand-min <bytes>";
      "                       DemandHint.min_bytes の既定値を設定";
      "  --stream-demand-preferred <bytes>";
      "                       DemandHint.preferred_bytes の既定値を設定";
      "  --stream-flow <manual|auto>";
      "                       FlowController.policy を切り替える（既定: manual）";
      "  --stream-flow-max-lag <bytes>";
      "                       FlowController.backpressure.max_lag_bytes を設定";
      "  --stream-flow-debounce-ms <ms>";
      "                       FlowController.backpressure.debounce_ms を設定";
      "  --stream-flow-throttle <ratio>";
      "                       FlowController.backpressure.throttle_ratio (0.0-1.0)";
      "";
      "コンパイル設定:";
      "  --target <triple>   ターゲットトリプル（既定: x86_64-linux）";
      "  --link-runtime      ランタイムライブラリとリンクして実行可能ファイルを生成";
      "  --runtime-path <path>";
      "                       ランタイムライブラリへのパス（既定: \
       runtime/native/build/libreml_runtime.a）";
      "  --verify-ir         生成した LLVM IR を検証";
      "";
      "効果システム・監査:";
      "  --effect-stage <stage>";
      "                       実行時 Stage を明示的に指定（例: experimental/beta/stable）";
      "  --runtime-capabilities <file>";
      "                       Runtime Capability Registry JSON を読み込み、Stage を解決";
      "  --emit-audit <off|on|tmp|local|ci|path>";
      "                       監査ログ出力を制御（既定: on = tmp プロファイル）";
      "  --audit-store <tmp|local|ci>";
      "                       監査ログの出力プロファイルを選択";
      "  --audit-dir <path>   監査ログの出力ディレクトリを上書き";
      "  --audit-level <summary|full|debug>";
      "                       監査ログの詳細度（既定: full）";
      "  --no-emit-audit      監査ログ出力を無効化";
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
  emit_parse_debug : string option;  (** Parser debug JSON をファイル出力 *)
  out_dir : string;  (** 出力ディレクトリ *)
  (* 診断 *)
  format : output_format;  (** 診断メッセージの出力形式 *)
  json_mode : json_mode;  (** JSON 出力モード *)
  include_snippet : bool;  (** テキスト診断へスニペットを含めるか *)
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
  audit_enabled : bool;  (** 監査ログ出力を有効にするか *)
  audit_store : audit_store;  (** 監査ログの出力先プロファイル *)
  audit_dir_override : string option;  (** 出力ディレクトリの上書き（任意） *)
  audit_level : audit_level;  (** 監査ログの詳細度 *)
  emit_audit_path : string option;  (** 監査ログ（JSON Lines）出力先 *)
  (* 効果システム / Stage 制御 *)
  effect_stage_override : string option;  (** CLI で指定された Stage 名 *)
  runtime_capabilities_path : string option;  (** Capability Registry JSON のパス *)
  effects_type_row_mode : string;  (** TYPE-002 効果行モード（metadata-only|dual-write） *)
  parser_experimental_effects : bool;  (** 効果構文 PoC フラグ *)
  (* Parser RunConfig フラグ *)
  parser_require_eof : bool;  (** RunConfig.require_eof を CLI から制御 *)
  parser_packrat : bool;  (** RunConfig.packrat フラグ（Packrat メモ化を有効化、実験的） *)
  parser_left_recursion : Run_config.left_recursion;
      (** RunConfig.left_recursion モード *)
  parser_merge_warnings : bool;
      (** RunConfig.merge_warnings（診断集約の有無） *)
  parser_streaming : bool;  (** ストリーミングランナー PoC を有効化 *)
  stream_checkpoint : string option;  (** ストリーム継続チェックポイント名 *)
  stream_resume_hint : string option;  (** 継続再開ヒントの識別子 *)
  stream_demand_min_bytes : int option;  (** DemandHint.min_bytes 既定値 *)
  stream_demand_preferred_bytes : int option;
      (** DemandHint.preferred_bytes 既定値 *)
  stream_chunk_size : int option;  (** CLI が読み込むチャンクサイズ（バイト単位） *)
  stream_flow_policy : Run_config.Stream.Flow.policy option;
  stream_flow_max_lag_bytes : int option;
  stream_flow_debounce_ms : int option;
  stream_flow_throttle_ratio : float option;
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
  emit_parse_debug = None;
  out_dir = ".";
  format = Text;
  json_mode = JsonPretty;
  include_snippet = true;
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
    audit_enabled = true;
    audit_store = AuditStoreTmp;
    audit_dir_override = None;
    audit_level = AuditLevelFull;
    emit_audit_path = None;
    effect_stage_override = None;
    runtime_capabilities_path = None;
    effects_type_row_mode = "ty-integrated";
    parser_experimental_effects = Run_config.default.experimental_effects;
    parser_require_eof = Run_config.default.require_eof;
    parser_packrat = Run_config.default.packrat;
    parser_left_recursion = Run_config.default.left_recursion;
    parser_merge_warnings = Run_config.default.merge_warnings;
    parser_streaming = false;
    stream_checkpoint = None;
    stream_resume_hint = None;
    stream_demand_min_bytes = None;
    stream_demand_preferred_bytes = None;
    stream_chunk_size = None;
    stream_flow_policy = None;
    stream_flow_max_lag_bytes = None;
    stream_flow_debounce_ms = None;
    stream_flow_throttle_ratio = None;
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
  let emit_parse_debug = ref None in
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
  let audit_enabled = ref true in
  let audit_store_str = ref "tmp" in
  let audit_dir_override = ref None in
  let audit_level_str = ref "full" in
  let legacy_audit_path = ref None in
  let typeclass_mode_str = ref "dictionary" in
  let input_file = ref "" in
  let effect_stage = ref None in
  let json_mode_str = ref "pretty" in
  let include_snippet = ref true in
  let runtime_caps_path = ref None in
  let parser_experimental_effects = ref Run_config.default.experimental_effects in
  let type_row_mode_str = ref "ty-integrated" in
  let parser_require_eof = ref Run_config.default.require_eof in
  let parser_packrat = ref Run_config.default.packrat in
  let parser_left_recursion = ref Run_config.default.left_recursion in
  let parser_merge_warnings = ref Run_config.default.merge_warnings in
  let parser_streaming = ref false in
  let stream_checkpoint = ref None in
  let stream_resume_hint = ref None in
  let stream_demand_min_bytes = ref None in
  let stream_demand_preferred_bytes = ref None in
  let stream_chunk_size = ref None in
  let stream_flow_policy = ref None in
  let stream_flow_max_lag_bytes = ref None in
  let stream_flow_debounce_ms = ref None in
  let stream_flow_throttle_ratio = ref None in

  let usage_msg = "remlc-ocaml [options] <file>" in

  let store_trimmed target value =
    let trimmed = String.trim value in
    if String.length trimmed = 0 then target := None
    else target := Some trimmed
  in
  let set_int_option target option_name text =
    let parsed =
      try
        let value = int_of_string (String.trim text) in
        if value < 0 then None else Some value
      with Failure _ -> None
    in
    match parsed with
    | Some value -> target := Some value
    | None ->
        prerr_endline
          (Printf.sprintf
             "Warning: %s には 0 以上の整数を指定してください（入力値: %s）。"
             option_name text)
  in
  let set_ratio_option target option_name text =
    let parsed =
      try
        let value = float_of_string (String.trim text) in
        if Float.is_nan value || Float.is_infinite value then None else Some value
      with Failure _ -> None
    in
    match parsed with
    | Some value when value >= 0.0 && value <= 1.0 -> target := Some value
    | Some _ ->
        prerr_endline
          (Printf.sprintf
             "Warning: %s には 0.0〜1.0 の範囲で数値を指定してください（入力値: %s）。"
             option_name text)
    | None ->
        prerr_endline
          (Printf.sprintf
             "Warning: %s には 0.0〜1.0 の数値を指定してください（入力値: %s）。"
             option_name text)
  in
  let parse_flow_policy text =
    match String.lowercase_ascii (String.trim text) with
    | "auto" -> Some Run_config.Stream.Flow.Auto
    | "manual" -> Some Run_config.Stream.Flow.Manual
    | _ -> None
  in

  let speclist =
    [
      (* 出力オプション *)
      ("--emit-ast", Arg.Set emit_ast, "Emit AST to stdout");
      ("--emit-tast", Arg.Set emit_tast, "Emit Typed AST to stdout");
      ("--emit-ir", Arg.Set emit_ir, "Emit LLVM IR (.ll) to output directory");
      ( "--emit-bc",
        Arg.Set emit_bc,
        "Emit LLVM Bitcode (.bc) to output directory" );
      ( "--emit-parse-debug",
        Arg.String (fun value -> emit_parse_debug := Some value),
        "<path> Emit parser debug JSON (packrat/span_trace) to file" );
      ( "--out-dir",
        Arg.Set_string out_dir,
        "<dir> Output directory (default: .)" );
      (* 診断オプション *)
      ( "--format",
        Arg.String
          (fun value -> format_str := String.lowercase_ascii value),
        "<text|json> Output format (default: text)" );
      ( "--json-mode",
        Arg.String
          (fun value -> json_mode_str := String.lowercase_ascii value),
        "<pretty|compact|lines> JSON output mode (default: pretty)" );
      ( "--no-snippet",
        Arg.Unit (fun () -> include_snippet := false),
        "Disable source snippets in text diagnostics" );
      ( "--color",
        Arg.String (fun value -> color_str := String.lowercase_ascii value),
        "<auto|always|never> Color mode (default: auto)" );
      (* デバッグオプション *)
      ("--trace", Arg.Set trace, "Enable phase tracing");
      ("--stats", Arg.Set stats, "Show compilation statistics");
      ("--verbose", Arg.Set_int verbose, "<0-3> Verbosity level (default: 1)");
      ( "--metrics",
        Arg.String (fun path -> metrics_path := Some path),
        "<path> Output metrics to file" );
      ( "--metrics-format",
        Arg.String
          (fun value -> metrics_format_str := String.lowercase_ascii value),
        "<json|csv> Metrics output format (default: json)" );
      (* Parser RunConfig オプション *)
      ( "--require-eof",
        Arg.Unit (fun () -> parser_require_eof := true),
        "Require parser to consume entire input (RunConfig.require_eof=true)" );
      ( "--packrat",
        Arg.Unit (fun () -> parser_packrat := true),
        "Enable Packrat memoization shim (RunConfig.packrat=true; experimental)" );
      ( "--experimental-effects",
        Arg.Unit (fun () -> parser_experimental_effects := true),
        "Accept algebraic effects syntax PoC (RunConfig.experimental_effects=true)" );
     ( "-Zalgebraic-effects",
       Arg.Unit (fun () -> parser_experimental_effects := true),
       "Alias of --experimental-effects for Stage PoC builds" );
      ( "--type-row-mode",
        Arg.String
          (fun value ->
            type_row_mode_str :=
              String.lowercase_ascii (String.trim value)),
        "<metadata-only|dual-write|ty-integrated> TYPE-002 effect row integration mode (default: ty-integrated)" );
      ( "--left-recursion",
        Arg.String
          (fun value ->
            let lowered = String.lowercase_ascii value in
            match lowered with
            | "auto" -> parser_left_recursion := Run_config.Auto
            | "on" -> parser_left_recursion := Run_config.On
            | "off" -> parser_left_recursion := Run_config.Off
            | other ->
                prerr_endline
                  (Printf.sprintf
                     "Warning: unknown left recursion mode '%s', using 'auto'"
                     other);
                parser_left_recursion := Run_config.Auto),
        "<off|on|auto> Left recursion handling mode (default: auto)" );
      ( "--no-merge-warnings",
        Arg.Unit (fun () -> parser_merge_warnings := false),
        "Emit all parser warnings without merging (RunConfig.merge_warnings=false)" );
      ( "--streaming",
        Arg.Unit (fun () -> parser_streaming := true),
        "Enable streaming runner PoC (RunConfig.extensions[\"stream\"].enabled)" );
      ( "--stream-checkpoint",
        Arg.String (fun value -> store_trimmed stream_checkpoint value),
        "<token> Set stream checkpoint token" );
      ( "--stream-resume-hint",
        Arg.String (fun value -> store_trimmed stream_resume_hint value),
        "<hint> Set stream resume_hint token" );
      ( "--stream-demand-min",
        Arg.String
          (fun value ->
            set_int_option stream_demand_min_bytes "--stream-demand-min" value),
        "<bytes> Set DemandHint.min_bytes default (>=0)" );
      ( "--stream-demand-preferred",
        Arg.String
          (fun value ->
            set_int_option stream_demand_preferred_bytes
              "--stream-demand-preferred" value),
        "<bytes> Set DemandHint.preferred_bytes default (>=0)" );
      ( "--stream-chunk-size",
        Arg.String
          (fun value ->
            set_int_option stream_chunk_size "--stream-chunk-size" value),
        "<bytes> Configure CLI chunk size for streaming (>=0)" );
      ( "--stream-flow",
        Arg.String
          (fun value ->
            match parse_flow_policy value with
            | Some policy -> stream_flow_policy := Some policy
            | None ->
                prerr_endline
                  (Printf.sprintf
                     "Warning: --stream-flow には manual または auto を指定してください（入力値: %s）。"
                     value)),
        "<manual|auto> Configure FlowController policy" );
      ( "--stream-flow-max-lag",
        Arg.String
          (fun value ->
            set_int_option stream_flow_max_lag_bytes "--stream-flow-max-lag"
              value),
        "<bytes> Configure FlowController.backpressure.max_lag_bytes" );
      ( "--stream-flow-debounce-ms",
        Arg.String
          (fun value ->
            set_int_option stream_flow_debounce_ms "--stream-flow-debounce-ms"
              value),
        "<ms> Configure FlowController.backpressure.debounce_ms" );
      ( "--stream-flow-throttle",
        Arg.String
          (fun value ->
            set_ratio_option stream_flow_throttle_ratio "--stream-flow-throttle"
              value),
        "<ratio> Configure FlowController.backpressure.throttle_ratio (0.0-1.0)" );
      ( "--typeclass-mode",
        Arg.String
          (fun value ->
            typeclass_mode_str := String.lowercase_ascii value),
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
      ( "--emit-audit",
        Arg.String
          (fun value ->
            let lowered = String.lowercase_ascii value in
            match lowered with
            | "off" ->
                audit_enabled := false;
                legacy_audit_path := None
            | "on" ->
                audit_enabled := true;
                legacy_audit_path := None
            | "tmp" | "local" | "ci" as profile ->
                audit_enabled := true;
                audit_store_str := profile;
                legacy_audit_path := None
            | _ ->
                audit_enabled := true;
                legacy_audit_path := Some value),
        "<off|on|tmp|local|ci|path> Control audit logging (default: on)" );
      ( "--no-emit-audit",
        Arg.Unit
          (fun () ->
            audit_enabled := false;
            legacy_audit_path := None),
        "Disable audit logging" );
      ( "--audit-store",
        Arg.String
          (fun value -> audit_store_str := String.lowercase_ascii value),
        "<tmp|local|ci> Select audit log store profile (default: tmp)" );
      ( "--audit-dir",
        Arg.String (fun value -> audit_dir_override := Some value),
        "<path> Override audit output directory" );
      ( "--audit-level",
        Arg.String
          (fun value -> audit_level_str := String.lowercase_ascii value),
        "<summary|full|debug> Control audit log detail level (default: full)" );
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
      let json_mode =
        match !json_mode_str with
        | "pretty" -> JsonPretty
        | "compact" -> JsonCompact
        | "lines" -> JsonLines
        | other ->
            prerr_endline
              (Printf.sprintf
                "Warning: unknown json mode '%s', using 'pretty'" other);
            JsonPretty
      in
      let audit_store =
        match String.lowercase_ascii !audit_store_str with
        | "tmp" -> AuditStoreTmp
        | "local" -> AuditStoreLocal
        | "ci" -> AuditStoreCi
        | other ->
            prerr_endline
              (Printf.sprintf
                 "Warning: unknown audit store '%s', using 'tmp'" other);
            AuditStoreTmp
      in
      let audit_level =
        match !audit_level_str with
        | "summary" -> AuditLevelSummary
        | "full" -> AuditLevelFull
        | "debug" -> AuditLevelDebug
        | other ->
            prerr_endline
              (Printf.sprintf
                 "Warning: unknown audit level '%s', using 'full'" other);
            AuditLevelFull
      in
      let type_row_mode =
        match !type_row_mode_str with
        | "metadata-only" -> "metadata-only"
        | "dual-write" -> "dual-write"
        | "ty-integrated" -> "ty-integrated"
        | other ->
            prerr_endline
              (Printf.sprintf
                 "Warning: unknown type row mode '%s', using 'ty-integrated'" other);
            "ty-integrated"
      in

      let audit_dir_override_value = !audit_dir_override in

      let is_explicit_path path =
        Filename.check_suffix path ".json"
        || Filename.check_suffix path ".jsonl"
      in

      let emit_audit_path =
        if not !audit_enabled then None
        else
          match !legacy_audit_path with
          | Some path -> Some path
          | None -> (
              match audit_dir_override_value with
              | Some dir when is_explicit_path dir -> Some dir
              | _ -> None)
      in

      Ok
        {
          input_file = !input_file;
          use_stdin = false;
          emit_ast = !emit_ast;
          emit_tast = !emit_tast;
          emit_ir = !emit_ir;
          emit_bc = !emit_bc;
          emit_parse_debug =
            (match !emit_parse_debug with
            | Some path when String.trim path <> "" -> Some (String.trim path)
            | _ -> None);
          out_dir = !out_dir;
          format;
          json_mode;
          include_snippet = !include_snippet;
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
          audit_enabled = !audit_enabled;
          audit_store;
          audit_dir_override = audit_dir_override_value;
          audit_level;
          emit_audit_path;
          effect_stage_override = !effect_stage;
          runtime_capabilities_path = !runtime_caps_path;
          effects_type_row_mode = type_row_mode;
          parser_experimental_effects = !parser_experimental_effects;
          parser_require_eof = !parser_require_eof;
          parser_packrat = !parser_packrat;
          parser_left_recursion = !parser_left_recursion;
          parser_merge_warnings = !parser_merge_warnings;
          parser_streaming = !parser_streaming;
          stream_checkpoint = !stream_checkpoint;
          stream_resume_hint = !stream_resume_hint;
          stream_demand_min_bytes = !stream_demand_min_bytes;
          stream_demand_preferred_bytes = !stream_demand_preferred_bytes;
          stream_chunk_size = !stream_chunk_size;
          stream_flow_policy = !stream_flow_policy;
          stream_flow_max_lag_bytes = !stream_flow_max_lag_bytes;
          stream_flow_debounce_ms = !stream_flow_debounce_ms;
          stream_flow_throttle_ratio = !stream_flow_throttle_ratio;
        }
  with
  | Arg.Help _ ->
      print_full_help ();
      exit 0
  | Arg.Bad msg -> Error msg

let string_of_left_recursion = function
  | Run_config.Off -> "off"
  | Run_config.On -> "on"
  | Run_config.Auto -> "auto"

let to_run_config (opts : options) =
  let base =
    {
      Run_config.default with
      require_eof = opts.parser_require_eof;
      packrat = opts.parser_packrat;
      left_recursion = opts.parser_left_recursion;
      trace = opts.trace;
      merge_warnings = opts.parser_merge_warnings;
      experimental_effects = opts.parser_experimental_effects;
      legacy_result = true;
    }
  in
  let module Namespace = Extensions.Namespace in
  let config_namespace =
    Namespace.empty
    |> Namespace.add "source" (Extensions.String "cli")
    |> Namespace.add "require_eof" (Extensions.Bool opts.parser_require_eof)
    |> Namespace.add "packrat" (Extensions.Bool opts.parser_packrat)
    |> Namespace.add "left_recursion"
         (Extensions.String (string_of_left_recursion opts.parser_left_recursion))
    |> Namespace.add "trace" (Extensions.Bool base.trace)
    |> Namespace.add "merge_warnings"
         (Extensions.Bool opts.parser_merge_warnings)
    |> Namespace.add "experimental_effects"
         (Extensions.Bool opts.parser_experimental_effects)
    |> Namespace.add "legacy_result" (Extensions.Bool base.legacy_result)
  in
  let lex_namespace =
    Namespace.empty
    |> Namespace.add "profile"
         (Extensions.String (Lex.profile_symbol Lex.Strict_json))
  in
  Run_config.with_extension "config" (fun _ -> config_namespace) base
  |> Run_config.with_extension "lex" (fun _ -> lex_namespace)
  |> Run_config.Effects.set_stage_override opts.effect_stage_override
  |> Run_config.Effects.set_registry_path opts.runtime_capabilities_path
  |> Run_config.Effects.set_type_row_mode (Some opts.effects_type_row_mode)
  |> Run_config.Stream.set_enabled opts.parser_streaming
  |> Run_config.Stream.set_checkpoint opts.stream_checkpoint
  |> Run_config.Stream.set_resume_hint opts.stream_resume_hint
  |> Run_config.Stream.set_demand_min_bytes opts.stream_demand_min_bytes
  |> Run_config.Stream.set_demand_preferred_bytes
       opts.stream_demand_preferred_bytes
  |> Run_config.Stream.set_chunk_size opts.stream_chunk_size
  |> Run_config.Stream.set_flow_policy opts.stream_flow_policy
  |> Run_config.Stream.set_flow_max_lag_bytes opts.stream_flow_max_lag_bytes
  |> Run_config.Stream.set_flow_debounce_ms opts.stream_flow_debounce_ms
  |> Run_config.Stream.set_flow_throttle_ratio
       opts.stream_flow_throttle_ratio
  (* TODO(LEXER-002 Step5): ParserId を取得したら space_id を設定し、CLI で警告を出す。 *)
