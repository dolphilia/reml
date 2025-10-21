(* Json_formatter — LSP 互換 JSON 診断出力
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、診断メッセージを
 * LSP（Language Server Protocol）互換の JSON 形式で出力する機能を提供する。
 *
 * 設計原則:
 * - LSP の Diagnostic 型に準拠
 * - 機械判読可能な構造化データ
 * - CI/CD ツールとの統合を容易にする
 *)

(** 重要度を LSP 互換の整数値に変換
 *
 * LSP DiagnosticSeverity:
 * - 1: Error
 * - 2: Warning
 * - 3: Information
 * - 4: Hint
 *)
let severity_to_lsp_int severity = Diagnostic.V2.severity_to_lsp_int severity

(** 重要度を文字列に変換 *)
let severity_to_string severity = Diagnostic.V2.severity_to_string severity

(** 位置情報を JSON に変換
 *
 * LSP Position:
 * ```json
 * {
 *   "line": 0,     // 0始まり
 *   "character": 0 // 0始まり
 * }
 * ```
 *)
let location_to_json (loc : Diagnostic.location) : Yojson.Basic.t =
  `Assoc
    [
      ("line", `Int (loc.line - 1));
      (* LSP は 0 始まり *)
      ("character", `Int (loc.column - 1)) (* LSP は 0 始まり *);
    ]

(** スパンを LSP Range に変換
 *
 * LSP Range:
 * ```json
 * {
 *   "start": { "line": 0, "character": 0 },
 *   "end": { "line": 0, "character": 5 }
 * }
 * ```
 *)
let span_to_lsp_range (span : Diagnostic.span) : Yojson.Basic.t =
  `Assoc
    [
      ("start", location_to_json span.start_pos);
      ("end", location_to_json span.end_pos);
    ]

(** 期待値を JSON に変換 *)
let expectation_to_json (exp : Diagnostic.expectation) : Yojson.Basic.t =
  match exp with
  | Token s -> `Assoc [ ("type", `String "token"); ("value", `String s) ]
  | Keyword s -> `Assoc [ ("type", `String "keyword"); ("value", `String s) ]
  | Rule s -> `Assoc [ ("type", `String "rule"); ("value", `String s) ]
  | Eof -> `Assoc [ ("type", `String "eof") ]
  | Not s -> `Assoc [ ("type", `String "not"); ("value", `String s) ]
  | Class s -> `Assoc [ ("type", `String "class"); ("value", `String s) ]
  | Custom s -> `Assoc [ ("type", `String "custom"); ("value", `String s) ]
  | TypeExpected t -> `Assoc [ ("type", `String "type"); ("value", `String t) ]
  | TraitBound t -> `Assoc [ ("type", `String "trait"); ("value", `String t) ]

(** 期待値サマリを JSON に変換 *)
let expectation_summary_to_json (summary : Diagnostic.expectation_summary) :
    Yojson.Basic.t =
  let fields =
    [
      ("alternatives", `List (List.map expectation_to_json summary.alternatives));
    ]
  in
  let fields =
    match summary.message_key with
    | Some key -> ("message_key", `String key) :: fields
    | None -> fields
  in
  let fields =
    match summary.humanized with
    | Some h -> ("humanized", `String h) :: fields
    | None -> fields
  in
  let fields =
    match summary.context_note with
    | Some c -> ("context_note", `String c) :: fields
    | None -> fields
  in
  `Assoc fields

(** 修正提案（FixIt）を JSON に変換 *)
let fixit_to_json (fixit : Diagnostic.fixit) : Yojson.Basic.t =
  match fixit with
  | Insert { at; text } ->
      `Assoc
        [
          ("kind", `String "insert");
          ("range", span_to_lsp_range at);
          ("text", `String text);
        ]
  | Replace { at; text } ->
      `Assoc
        [
          ("kind", `String "replace");
          ("range", span_to_lsp_range at);
          ("text", `String text);
        ]
  | Delete { at } ->
      `Assoc [ ("kind", `String "delete"); ("range", span_to_lsp_range at) ]

(** 診断情報を LSP 互換の JSON に変換
 *
 * LSP Diagnostic:
 * ```json
 * {
 *   "range": { "start": {...}, "end": {...} },
 *   "severity": 1,
 *   "code": "E7001",
 *   "source": "remlc",
 *   "message": "型が一致しません",
 *   "relatedInformation": [...]
 * }
 * ```
 *)
let diagnostic_to_lsp_json (diag : Diagnostic.t) : Yojson.Basic.t =
  let severity_v2 = Diagnostic.V2.severity_of_diagnostic diag in
  let fields =
    [
      ("range", span_to_lsp_range diag.Diagnostic.primary);
      ("severity", `Int (severity_to_lsp_int severity_v2));
      ("message", `String diag.Diagnostic.message);
      ("source", `String "remlc");
    ]
  in

  (* コードを追加 *)
  let fields =
    match diag.Diagnostic.codes with
    | code :: _ -> ("code", `String code) :: fields
    | [] -> fields
  in
  let fields =
    match diag.Diagnostic.codes with
    | [] -> fields
    | codes ->
        ("codes", `List (List.map (fun code -> `String code) codes)) :: fields
  in

  (* ドメインを追加 *)
  let fields =
    match diag.Diagnostic.domain with
    | Some domain ->
        let domain_label = Diagnostic.domain_label domain in
        ("domain", `String domain_label) :: fields
    | None -> fields
  in

  (* 関連情報（ノート）を追加 *)
  let fields =
    if diag.Diagnostic.secondary <> [] then
      ( "relatedInformation",
        `List
          (List.map Diagnostic.V2.span_label_to_json diag.Diagnostic.secondary) )
      :: fields
    else fields
  in

  (* 期待値サマリを追加 *)
  let fields =
    match diag.Diagnostic.expected with
    | Some summary ->
        ("expected", expectation_summary_to_json summary) :: fields
    | None -> fields
  in

  (* ヒントを追加 *)
  let fields =
    if diag.Diagnostic.hints <> [] then
      ("hints", `List (List.map Diagnostic.V2.hint_to_json diag.Diagnostic.hints))
      :: fields
    else fields
  in

  (* 修正提案を追加 *)
  let fields =
    if diag.Diagnostic.fixits <> [] then
      ("fixits", `List (List.map fixit_to_json diag.Diagnostic.fixits)) :: fields
    else fields
  in
  let fields =
    if Diagnostic.Extensions.is_empty diag.Diagnostic.extensions then fields
    else
      ( "extensions",
        Diagnostic.Extensions.to_json diag.Diagnostic.extensions )
      :: fields
  in
  let fields =
    match Diagnostic.V2.audit_to_json diag.Diagnostic.audit with
    | `Null -> fields
    | audit_json -> ("audit", audit_json) :: fields
  in
  let fields =
    match diag.Diagnostic.timestamp with
    | Some ts -> ("timestamp", `String ts) :: fields
    | None -> fields
  in

  `Assoc fields

(** 診断情報を Reml 独自の JSON 形式に変換
 *
 * Reml Diagnostic JSON:
 * ```json
 * {
 *   "severity": "error",
 *   "code": "E7001",
 *   "message": "型が一致しません",
 *   "location": {
 *     "file": "/path/to/file.reml",
 *     "line": 1,
 *     "column": 18,
 *     "endLine": 1,
 *     "endColumn": 24
 *   },
 *   "notes": ["期待される型: i64", "実際の型: String"]
 * }
 * ```
 *)
let diagnostic_to_reml_json (diag : Diagnostic.t) : Yojson.Basic.t =
  let severity_v2 = Diagnostic.V2.severity_of_diagnostic diag in
  let fields =
    [
      ("severity", `String (severity_to_string severity_v2));
      ("message", `String diag.Diagnostic.message);
      ( "location",
        `Assoc
          [
            ("file", `String diag.Diagnostic.primary.start_pos.filename);
            ("line", `Int diag.Diagnostic.primary.start_pos.line);
            ("column", `Int diag.Diagnostic.primary.start_pos.column);
            ("endLine", `Int diag.Diagnostic.primary.end_pos.line);
            ("endColumn", `Int diag.Diagnostic.primary.end_pos.column);
          ] );
    ]
  in

  (* コードを追加 *)
  let fields =
    match diag.Diagnostic.codes with
    | code :: _ -> ("code", `String code) :: fields
    | [] -> fields
  in
  let fields =
    match diag.Diagnostic.codes with
    | [] -> fields
    | codes ->
        ("codes", `List (List.map (fun code -> `String code) codes)) :: fields
  in

  (* ドメインを追加 *)
  let fields =
    match diag.Diagnostic.domain with
    | Some domain ->
        let domain_label = Diagnostic.domain_label domain in
        ("domain", `String domain_label) :: fields
    | None -> fields
  in

  (* ノートを追加（簡略版） *)
  let fields =
    if diag.Diagnostic.secondary <> [] then
      let note_messages =
        diag.Diagnostic.secondary
        |> List.filter_map (fun (label : Diagnostic.span_label) ->
               Option.map (fun msg -> `String msg) label.message)
      in
      if note_messages = [] then fields
      else ("notes", `List note_messages) :: fields
    else fields
  in

  (* 期待値を追加 *)
  let fields =
    match diag.Diagnostic.expected with
    | Some summary when summary.alternatives <> [] ->
        let expectations =
          List.map
            (fun exp -> `String (Diagnostic.string_of_expectation exp))
            summary.alternatives
        in
        ("expected", `List expectations) :: fields
    | _ -> fields
  in

  (* 修正提案を追加 *)
  let fields =
    if diag.Diagnostic.fixits <> [] then
      ("fixits", `List (List.map fixit_to_json diag.Diagnostic.fixits)) :: fields
    else fields
  in
  let fields =
    if Diagnostic.Extensions.is_empty diag.Diagnostic.extensions then fields
    else
      ( "extensions",
        Diagnostic.Extensions.to_json diag.Diagnostic.extensions )
      :: fields
  in
  let fields =
    match Diagnostic.V2.audit_to_json diag.Diagnostic.audit with
    | `Null -> fields
    | audit_json -> ("audit", audit_json) :: fields
  in
  let fields =
    match diag.Diagnostic.timestamp with
    | Some ts -> ("timestamp", `String ts) :: fields
    | None -> fields
  in

  let fields =
    if diag.Diagnostic.hints <> [] then
      ("hints", `List (List.map Diagnostic.V2.hint_to_json diag.Diagnostic.hints))
      :: fields
    else fields
  in

  `Assoc fields

(** 複数の診断を JSON 配列に変換
 *
 * @param diags 診断情報のリスト
 * @param lsp_compatible LSP 互換形式を使用するか（デフォルト: false）
 * @return JSON 文字列
 *)
let diagnostics_to_json ?(lsp_compatible = false) (diags : Diagnostic.t list) :
    string =
  let json_converter =
    if lsp_compatible then diagnostic_to_lsp_json else diagnostic_to_reml_json
  in
  let diagnostics_json = `List (List.map json_converter diags) in
  let root = `Assoc [ ("diagnostics", diagnostics_json) ] in
  Yojson.Basic.pretty_to_string root

(** 単一の診断を JSON 文字列に変換
 *
 * @param diag 診断情報
 * @param lsp_compatible LSP 互換形式を使用するか（デフォルト: false）
 * @return JSON 文字列
 *)
let diagnostic_to_json ?(lsp_compatible = false) (diag : Diagnostic.t) : string
    =
  diagnostics_to_json ~lsp_compatible [ diag ]
