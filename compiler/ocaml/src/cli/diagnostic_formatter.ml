(* Diagnostic_formatter — ソースコードスニペット表示
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、診断メッセージに
 * ソースコードのコンテキストとポインタを表示する機能を提供する。
 *
 * 設計原則:
 * - 仕様書 3-6-core-diagnostics-audit.md §1 の Diagnostic 構造体に準拠
 * - エラー位置の前後の行を表示してコンテキストを提供
 * - ポインタ（^^^）でエラー箇所を明示
 * - カラー出力に対応
 *)

module Json = Yojson.Basic
module Serialization = Diagnostic_serialization

(** ソース文字列を行の配列に分割 *)
let split_into_lines source = String.split_on_char '\n' source |> Array.of_list

(** ソースコードスニペットを抽出して表示
 *
 * @param source ソースコード文字列
 * @param span エラー位置情報
 * @param color_mode カラーモード
 * @param severity 診断の重要度
 * @return フォーマットされたスニペット文字列
 *)
let format_snippet ~source ~(span : Serialization.normalized_span) ~color_mode
    ~severity =
  let lines = split_into_lines source in
  let total_lines = Array.length lines in
  let start_line = span.start_line + 1 in
  let end_line = span.end_line + 1 in
  let start_col = span.start_col in
  let end_col = span.end_col in

  (* 表示する行の範囲を決定（エラー行の前後1行ずつ） *)
  let context_before = 1 in
  let context_after = 1 in
  let first_line = max 1 (start_line - context_before) in
  let last_line = min total_lines (end_line + context_after) in

  (* スニペット行を構築 *)
  let snippet_lines = ref [] in

  for line_num = first_line to last_line do
    if line_num >= 1 && line_num <= total_lines then
      let line_content = lines.(line_num - 1) in
      let line_num_str = Color.colorize_line_number color_mode line_num in

      (* エラー行かどうかで表示を変える *)
      let is_error_line = line_num >= start_line && line_num <= end_line in

      if is_error_line then (
        (* エラー行: 行番号 + " | " + 行内容 *)
        let prefix = line_num_str ^ " | " in
        snippet_lines := (prefix ^ line_content) :: !snippet_lines;

        (* ポインタ行を追加（エラー箇所を ^^^ で示す） *)
        if line_num = start_line && line_num = end_line then
          (* 単一行エラー *)
          let pointer_offset =
            String.length (Printf.sprintf "%4d | " line_num)
          in
          let pointer_start =
            max 0 (min start_col (String.length line_content))
          in
          let raw_length = end_col - start_col in
          let pointer_length =
            let candidate =
              if raw_length <= 0 then 1 else raw_length
            in
            let available = String.length line_content - pointer_start in
            if available <= 0 then 1 else min candidate available
          in
          let pointer_padding =
            String.make (pointer_offset + pointer_start) ' '
          in
          let pointer = String.make pointer_length '^' in
          let colored_pointer =
            Color.colorize_pointer color_mode severity pointer
          in
          snippet_lines := (pointer_padding ^ colored_pointer) :: !snippet_lines
        else if line_num = start_line then
          (* 複数行エラーの開始行 *)
          let pointer_offset =
            String.length (Printf.sprintf "%4d | " line_num)
          in
          let pointer_start =
            max 0 (min start_col (String.length line_content))
          in
          let pointer_length =
            let available = String.length line_content - pointer_start in
            if available <= 0 then 1 else available
          in
          let pointer_padding =
            String.make (pointer_offset + pointer_start) ' '
          in
          let pointer = String.make pointer_length '^' in
          let colored_pointer =
            Color.colorize_pointer color_mode severity pointer
          in
          snippet_lines := (pointer_padding ^ colored_pointer) :: !snippet_lines
        else if line_num = end_line then
          (* 複数行エラーの終了行 *)
          let pointer_offset =
            String.length (Printf.sprintf "%4d | " line_num)
          in
          let pointer_length =
            let normalized = end_col + 1 in
            let available = String.length line_content in
            let capped = min normalized available in
            if capped <= 0 then 1 else capped
          in
          let pointer_padding = String.make pointer_offset ' ' in
          let pointer = String.make pointer_length '^' in
          let colored_pointer =
            Color.colorize_pointer color_mode severity pointer
          in
          snippet_lines := (pointer_padding ^ colored_pointer) :: !snippet_lines
        else
          (* 複数行エラーの中間行 *)
          let pointer_offset =
            String.length (Printf.sprintf "%4d | " line_num)
          in
          let pointer_length = String.length line_content in
          let pointer_padding = String.make pointer_offset ' ' in
          let pointer = String.make pointer_length '^' in
          let colored_pointer =
            Color.colorize_pointer color_mode severity pointer
          in
          snippet_lines := (pointer_padding ^ colored_pointer) :: !snippet_lines)
      else
        (* コンテキスト行: 行番号 + " | " + 行内容 *)
        let prefix = line_num_str ^ " | " in
        snippet_lines := (prefix ^ line_content) :: !snippet_lines
  done;

  (* 逆順に追加していたので反転 *)
  String.concat "\n" (List.rev !snippet_lines)

let location_string filename line column =
  Printf.sprintf "%s:%d:%d" filename (line + 1) (column + 1)

let format_span (span : Serialization.normalized_span) =
  if span.start_line = span.end_line then
    Printf.sprintf "%s (列 %d-%d)"
      (location_string span.file span.start_line span.start_col)
      (span.start_col + 1) (span.end_col + 1)
  else
    Printf.sprintf "%s - %s"
      (location_string span.file span.start_line span.start_col)
      (location_string span.file span.end_line span.end_col)

let format_fixit = function
  | Serialization.Insert { range; text } ->
      Printf.sprintf "挿入 [%s]: '%s'" (format_span range) text
  | Serialization.Replace { range; text } ->
      Printf.sprintf "置換 [%s]: '%s'" (format_span range) text
  | Serialization.Delete { range } ->
      Printf.sprintf "削除 [%s]" (format_span range)

(** 診断メッセージのヘッダー行を生成
 *
 * @param diag 診断情報
 * @param color_mode カラーモード
 * @return ヘッダー行文字列
 *)
let format_header ~diag ~color_mode =
  let primary = diag.Serialization.primary in
  let loc =
    location_string primary.file primary.start_line primary.start_col
  in
  let severity_label = Diagnostic.severity_label diag.severity in
  let colored_severity =
    Color.colorize_by_severity color_mode diag.severity severity_label
  in
  let code_fragment =
    match diag.codes with
    | [] -> None
    | codes -> Some (String.concat "," codes)
  in
  match (code_fragment, diag.domain) with
  | Some codes, Some domain ->
      let domain_label = Diagnostic.domain_label domain in
      Printf.sprintf "%s: %s[%s] (%s): %s" loc colored_severity codes
        domain_label diag.message
  | Some codes, None ->
      Printf.sprintf "%s: %s[%s]: %s" loc colored_severity codes diag.message
  | None, Some domain ->
      let domain_label = Diagnostic.domain_label domain in
      Printf.sprintf "%s: %s (%s): %s" loc colored_severity domain_label
        diag.message
  | None, None ->
      Printf.sprintf "%s: %s: %s" loc colored_severity diag.message

(** 診断全体をテキスト形式で出力
 *
 * @param source ソースコード文字列（オプション）
 * @param diag 診断情報
 * @param color_mode カラーモード
 * @return フォーマットされた診断文字列
 *)
let format_serialized ~source ~(diag : Serialization.normalized_diagnostic)
    ~color_mode ~include_snippet =
  let header = format_header ~diag ~color_mode in
  let snippet =
    match (include_snippet, source) with
    | true, Some src ->
        "\n"
        ^ format_snippet ~source:src ~span:diag.primary ~color_mode
            ~severity:diag.severity
    | _ -> ""
  in
  let expected_str =
    match diag.expected with
    | None -> ""
    | Some summary ->
        let open Diagnostic in
        let alternatives_str =
          match summary.alternatives with
          | [] -> ""
          | items ->
              let body =
                items |> List.map string_of_expectation |> String.concat ", "
              in
              "\n期待される入力: " ^ body
        in
        let humanized_str =
          match summary.humanized with None -> "" | Some s -> "\n" ^ s
        in
        let context_str =
          match summary.context_note with None -> "" | Some c -> "\n文脈: " ^ c
        in
        alternatives_str ^ humanized_str ^ context_str
  in
  let related_str =
    let lines =
      diag.secondary
      |> List.filter_map (fun (entry : Serialization.normalized_secondary) ->
             let message = Option.value ~default:"" entry.message in
             let loc =
               match entry.span with
               | Some span -> format_span span
               | None -> ""
             in
             match (message, loc) with
             | "", "" -> None
             | "", loc -> Some (Printf.sprintf "  - (%s)" loc)
             | msg, "" -> Some ("  - " ^ msg)
             | msg, loc -> Some (Printf.sprintf "  - %s (%s)" msg loc))
    in
    match lines with
    | [] -> ""
    | _ -> "\n関連情報:\n" ^ String.concat "\n" lines
  in
  let fixits_str =
    match diag.fixits with
    | [] -> ""
    | fixits ->
        fixits
        |> List.map (fun fixit -> "  - " ^ format_fixit fixit)
        |> String.concat "\n"
        |> Printf.sprintf "\n修正候補:\n%s"
  in
  let hints_str =
    let lines =
      diag.hints
      |> List.map (fun (hint : Serialization.normalized_hint) ->
             let head =
               match hint.message with
               | Some msg -> "  - " ^ msg
               | None -> "  - (ヒント)"
             in
             let action_lines =
               hint.actions
               |> List.map (fun fixit -> "    * " ^ format_fixit fixit)
             in
             head :: action_lines)
      |> List.concat
    in
    match lines with
    | [] -> ""
    | _ -> "\nヒント:\n" ^ String.concat "\n" lines
  in
  let extensions_str =
    let entries =
      List.filter
        (fun (key, _) -> not (String.equal key "diagnostic.v2"))
        diag.extensions
    in
    match entries with
    | [] -> ""
    | _ ->
        entries
        |> List.rev
        |> List.map (fun (key, value) ->
               Printf.sprintf "\n拡張[%s]: %s" key (Json.to_string value))
        |> String.concat ""
  in
  let audit_str =
    "\n監査: "
    ^ Json.pretty_to_string (Diagnostic.V2.audit_to_json diag.audit)
  in
  let hint_str =
    match diag.severity_hint with
    | None -> ""
    | Some Diagnostic.Rollback -> "\n推奨アクション: ロールバック"
    | Some Diagnostic.Retry -> "\n推奨アクション: 再試行"
    | Some Diagnostic.Ignore -> "\n推奨アクション: 無視可能"
    | Some Diagnostic.Escalate -> "\n推奨アクション: エスカレーション"
  in
  let timestamp_str = "\nタイムスタンプ: " ^ diag.timestamp in
  [
    header;
    snippet;
    expected_str;
    related_str;
    hints_str;
    fixits_str;
    extensions_str;
    audit_str;
    hint_str;
    timestamp_str;
  ]
  |> List.filter (fun s -> not (String.equal s ""))
  |> String.concat ""

(** 複数の診断をバッチ出力
 *
 * @param source ソースコード文字列（オプション）
 * @param diags 診断情報のリスト
 * @param color_mode カラーモード
 * @return フォーマットされた診断文字列（改行区切り）
 *)
let format_serialized_many ~source ~diags ~color_mode ~include_snippet =
  diags
  |> List.map (fun diag ->
         format_serialized ~source ~diag ~color_mode ~include_snippet)
  |> String.concat "\n\n"

let format_diagnostic ~source ~diag ~color_mode ~include_snippet =
  let serialized = Serialization.of_diagnostic diag in
  format_serialized ~source ~diag:serialized ~color_mode ~include_snippet

let format_diagnostics ~source ~diags ~color_mode ~include_snippet =
  let serialized = List.map Serialization.of_diagnostic diags in
  format_serialized_many ~source ~diags:serialized ~color_mode ~include_snippet
