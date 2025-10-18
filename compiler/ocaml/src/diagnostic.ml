(* diagnostic.ml — 診断モデル（仕様書 2-5 準拠）
 *
 * Phase 2: 型推論エラーを含む包括的な診断システム
 *
 * 設計原則:
 * - 仕様書 2-5 §A のデータモデルに準拠
 * - 構文エラーと型エラーの統一的な扱い
 * - LSP連携と多言語対応の基盤
 *)

(* ========== JSON 拡張 ========== *)

module Json = Yojson.Basic

module Extensions = struct
  type t = (string * Json.t) list

  let empty : t = []
  let is_empty = function [] -> true | _ -> false

  let set key value entries =
    let filtered =
      List.filter (fun (k, _) -> not (String.equal k key)) entries
    in
    (key, value) :: filtered

  let to_json entries = `Assoc (List.rev entries)
end

(* ========== 重要度 ========== *)

type severity = Error | Warning | Note

type severity_hint =
  | Rollback (* ロールバック推奨 *)
  | Retry (* 再試行推奨 *)
  | Ignore (* 無視可能 *)
  | Escalate
(* エスカレーション必要 *)

(* ========== エラードメイン ========== *)

(** 診断の責務領域
 *
 * 仕様書 2-5 §A で定義されたドメイン分類
 *)
type error_domain =
  | Parser (* 構文解析 *)
  | Type (* 型システム *)
  | Config (* 設定 *)
  | Runtime (* 実行時 *)
  | Network (* ネットワーク *)
  | Data (* データ *)
  | Audit (* 監査 *)
  | Security (* セキュリティ *)
  | CLI (* コマンドライン *)

(* ========== 位置情報 ========== *)

type location = { filename : string; line : int; column : int; offset : int }
type span = { start_pos : location; end_pos : location }

(* ========== 期待値 ========== *)

(** 期待される構文要素
 *
 * 仕様書 2-5 §A の Expectation
 *)
type expectation =
  | Token of string (* 具体トークン: ")", "if", "+" *)
  | Keyword of string (* キーワード *)
  | Rule of string (* 構文規則: "expression", "pattern" *)
  | Eof (* 入力終端 *)
  | Not of string (* 否定: "直後に英数字が続かないこと" *)
  | Class of string (* 文字クラス: "digit", "identifier" *)
  | Custom of string (* 任意メッセージ *)
  (* 型関連の期待値 *)
  | TypeExpected of string (* 期待される型: "i64", "Bool" *)
  | TraitBound of string
(* トレイト境界: "Eq", "Ord" *)

(* ========== 修正提案 ========== *)

(** IDE用の修正提案
 *
 * 仕様書 2-5 §A の FixIt
 *)
type fixit =
  | Insert of { at : span; text : string }
  | Replace of { at : span; text : string }
  | Delete of { at : span }

(* ========== 期待値サマリ ========== *)

type expectation_summary = {
  message_key : string option; (* LSP/翻訳用キー *)
  locale_args : string list; (* メッセージ引数 *)
  humanized : string option; (* 自然言語フォールバック *)
  context_note : string option; (* 文脈説明 *)
  alternatives : expectation list; (* 候補一覧（優先順） *)
}
(** 期待値の人間可読サマリ
 *
 * 仕様書 2-5 §B-7 の ExpectationSummary
 *)

(* ========== 診断情報 ========== *)

type t = {
  severity : severity;
  severity_hint : severity_hint option;
  domain : error_domain option;
  code : string option; (* 安定ID: "E0001", "E7101" *)
  message : string; (* 1行要約 *)
  span : span; (* 主位置 *)
  expected_summary : expectation_summary option;
  notes : (span option * string) list; (* 追加メモ（位置付き） *)
  fixits : fixit list; (* 修正提案 *)
  extensions : Extensions.t;
  audit_metadata : Extensions.t;
}
(** 診断情報の完全な表現
 *
 * 仕様書 2-5 §A の Diagnostic
 *)

(* ========== ヘルパー関数 ========== *)

(** 重要度ラベル（日本語） *)
let severity_label = function Error -> "エラー" | Warning -> "警告" | Note -> "注記"

(** エラードメインラベル（日本語） *)
let domain_label = function
  | Parser -> "構文解析"
  | Type -> "型システム"
  | Config -> "設定"
  | Runtime -> "実行時"
  | Network -> "ネットワーク"
  | Data -> "データ"
  | Audit -> "監査"
  | Security -> "セキュリティ"
  | CLI -> "CLI"

(** Lexing.position から location への変換 *)
let location_of_pos (pos : Lexing.position) : location =
  let column = pos.pos_cnum - pos.pos_bol + 1 in
  {
    filename = (if pos.pos_fname = "" then "<入力>" else pos.pos_fname);
    line = pos.pos_lnum;
    column;
    offset = pos.pos_cnum;
  }

(** Lexing.position ペアから span への変換 *)
let span_of_positions start_pos end_pos =
  { start_pos = location_of_pos start_pos; end_pos = location_of_pos end_pos }

(* ========== 診断情報の構築 ========== *)

(** 診断情報の構築（Phase 1互換） *)
let make ?(severity = Error) ?severity_hint ?domain ?code
    ?(expected_summary = None) ?(notes = []) ?(fixits = [])
    ?(extensions = Extensions.empty) ?(audit_metadata = Extensions.empty)
    ~message ~start_pos ~end_pos () =
  {
    severity;
    severity_hint;
    domain;
    code;
    message;
    span = span_of_positions start_pos end_pos;
    expected_summary;
    notes = List.map (fun note -> (None, note)) notes;
    fixits;
    extensions;
    audit_metadata;
  }

(** 型エラー用の診断情報を構築 *)
let make_type_error ?(severity = Error) ?severity_hint ?code ?expected_summary
    ?(notes = []) ?(fixits = []) ?(extensions = Extensions.empty)
    ?(audit_metadata = Extensions.empty) ~message ~span () =
  {
    severity;
    severity_hint;
    domain = Some Type;
    code;
    message;
    span;
    expected_summary;
    notes;
    fixits;
    extensions;
    audit_metadata;
  }

(** Lexerエラー用（Phase 1互換） *)
let of_lexer_error ~message ~start_pos ~end_pos =
  make ~domain:Parser ~message ~start_pos ~end_pos ()

(** Parserエラー用（Phase 1互換） *)
let of_parser_error ~message ~start_pos ~end_pos ~expected =
  let expected_summary =
    Some
      {
        message_key = None;
        locale_args = [];
        humanized = None;
        context_note = None;
        alternatives = expected;
      }
  in
  make ~domain:Parser ~expected_summary ~message ~start_pos ~end_pos ()

let set_extension key value diag =
  { diag with extensions = Extensions.set key value diag.extensions }

let set_audit_metadata key value diag =
  { diag with audit_metadata = Extensions.set key value diag.audit_metadata }

let with_effect_stage_extension ?actual_stage ?residual ?provider ?manifest_path
    ?capability_meta ?iterator_fields ?stage_trace ~required_stage ~capability
    diag =
  let stage_fields =
    [
      ("required", `String required_stage);
      ("actual", match actual_stage with Some s -> `String s | None -> `Null);
    ]
  in
  let effect_fields =
    [ ("stage", `Assoc stage_fields); ("capability", `String capability) ]
  in
  let effect_fields =
    match residual with
    | Some value -> ("residual", value) :: effect_fields
    | None -> effect_fields
  in
  let effect_fields =
    match provider with
    | Some value -> ("provider", `String value) :: effect_fields
    | None -> effect_fields
  in
  let effect_fields =
    match manifest_path with
    | Some value -> ("manifest_path", `String value) :: effect_fields
    | None -> effect_fields
  in
  let effect_fields =
    match capability_meta with
    | Some value -> ("metadata", value) :: effect_fields
    | None -> effect_fields
  in
  let effect_fields =
    match iterator_fields with
    | Some fields -> ("iterator", `Assoc fields) :: effect_fields
    | None -> effect_fields
  in
  let effect_fields =
    match stage_trace with
    | Some trace when trace <> [] ->
        ("stage_trace", Effect_profile.stage_trace_to_json trace)
        :: effect_fields
    | _ -> effect_fields
  in
  let payload = `Assoc (List.rev effect_fields) in
  let diag = set_extension "effects" payload diag in
  let diag =
    match stage_trace with
    | Some trace when trace <> [] ->
        set_extension "effect.stage_trace"
          (Effect_profile.stage_trace_to_json trace)
          diag
    | _ -> diag
  in
  let diag =
    set_extension "effect.stage.required" (`String required_stage) diag
  in
  let diag =
    set_extension "effect.stage.actual"
      (match actual_stage with Some s -> `String s | None -> `Null)
      diag
  in
  let diag =
    set_extension "effect.stage.capability" (`String capability) diag
  in
  let diag =
    set_audit_metadata "effect.stage.required" (`String required_stage) diag
  in
  let diag =
    set_audit_metadata "effect.stage.actual"
      (match actual_stage with Some s -> `String s | None -> `Null)
      diag
  in
  let diag = set_audit_metadata "effect.capability" (`String capability) diag in
  let diag =
    match residual with
    | Some value -> set_audit_metadata "effect.residual" value diag
    | None -> diag
  in
  let diag =
    match provider with
    | Some value -> set_audit_metadata "effect.provider" (`String value) diag
    | None -> diag
  in
  let diag =
    match manifest_path with
    | Some value ->
        set_audit_metadata "effect.manifest_path" (`String value) diag
    | None -> diag
  in
  let diag =
    match capability_meta with
    | Some value -> set_audit_metadata "effect.capability_metadata" value diag
    | None -> diag
  in
  let diag =
    match iterator_fields with
    | Some fields ->
        List.fold_left
          (fun acc (key, value) ->
            set_audit_metadata
              (Printf.sprintf "effect.stage.iterator.%s" key)
              value acc)
          diag fields
    | None -> diag
  in
  let diag =
    match stage_trace with
    | Some trace when trace <> [] ->
        set_audit_metadata "stage_trace"
          (Effect_profile.stage_trace_to_json trace)
          diag
    | _ -> diag
  in
  diag

(* ========== 期待値の文字列表現 ========== *)

let string_of_expectation = function
  | Token s -> Printf.sprintf "トークン '%s'" s
  | Keyword s -> Printf.sprintf "キーワード '%s'" s
  | Rule s -> Printf.sprintf "構文 '%s'" s
  | Eof -> "入力終端"
  | Not s -> Printf.sprintf "否定: %s" s
  | Class s -> Printf.sprintf "文字クラス '%s'" s
  | Custom s -> s
  | TypeExpected t -> Printf.sprintf "型 '%s'" t
  | TraitBound t -> Printf.sprintf "トレイト境界 '%s'" t

(* ========== 診断情報の整形出力 ========== *)

let format_location loc =
  Printf.sprintf "%s:%d:%d" loc.filename loc.line loc.column

let format_span span =
  if span.start_pos.line = span.end_pos.line then
    Printf.sprintf "%s (列 %d-%d)"
      (format_location span.start_pos)
      span.start_pos.column span.end_pos.column
  else
    Printf.sprintf "%s - %s"
      (format_location span.start_pos)
      (format_location span.end_pos)

let format_fixit = function
  | Insert { at; text } -> Printf.sprintf "挿入 [%s]: '%s'" (format_span at) text
  | Replace { at; text } -> Printf.sprintf "置換 [%s]: '%s'" (format_span at) text
  | Delete { at } -> Printf.sprintf "削除 [%s]" (format_span at)

(** 診断情報の文字列表現 *)
let to_string diag =
  let loc = format_location diag.span.start_pos in

  (* ヘッダー行 *)
  let header =
    match (diag.code, diag.domain) with
    | Some code, Some domain ->
        Printf.sprintf "%s: %s[%s] (%s): %s" loc
          (severity_label diag.severity)
          code (domain_label domain) diag.message
    | Some code, None ->
        Printf.sprintf "%s: %s[%s]: %s" loc
          (severity_label diag.severity)
          code diag.message
    | None, Some domain ->
        Printf.sprintf "%s: %s (%s): %s" loc
          (severity_label diag.severity)
          (domain_label domain) diag.message
    | None, None ->
        Printf.sprintf "%s: %s: %s" loc
          (severity_label diag.severity)
          diag.message
  in

  (* 期待値サマリ *)
  let expected_str =
    match diag.expected_summary with
    | None -> []
    | Some summary ->
        let alternatives_str =
          match summary.alternatives with
          | [] -> None
          | items ->
              let body =
                items |> List.map string_of_expectation |> String.concat ", "
              in
              Some ("期待される入力: " ^ body)
        in
        let humanized_str =
          match summary.humanized with None -> [] | Some s -> [ s ]
        in
        let context_str =
          match summary.context_note with
          | None -> []
          | Some c -> [ "文脈: " ^ c ]
        in
        (match alternatives_str with None -> [] | Some s -> [ s ])
        @ humanized_str @ context_str
  in

  (* 追加ノート *)
  let notes_str =
    match diag.notes with
    | [] -> []
    | notes ->
        notes
        |> List.map (function
             | None, note -> "補足: " ^ note
             | Some span, note ->
                 Printf.sprintf "補足 [%s]: %s" (format_span span) note)
  in
  let extensions_str =
    match diag.extensions with
    | [] -> []
    | entries ->
        entries |> List.rev
        |> List.map (fun (key, value) ->
               Printf.sprintf "拡張[%s]: %s" key (Json.to_string value))
  in

  (* 修正提案 *)
  let fixits_str =
    match diag.fixits with
    | [] -> []
    | fixits ->
        [ "修正候補:" ] @ (fixits |> List.map (fun f -> "  - " ^ format_fixit f))
  in

  (* 重要度ヒント *)
  let hint_str =
    match diag.severity_hint with
    | None -> []
    | Some Rollback -> [ "推奨アクション: ロールバック" ]
    | Some Retry -> [ "推奨アクション: 再試行" ]
    | Some Ignore -> [ "推奨アクション: 無視可能" ]
    | Some Escalate -> [ "推奨アクション: エスカレーション" ]
  in

  let parts =
    [ header ] @ expected_str @ notes_str @ fixits_str @ extensions_str
    @ hint_str
  in
  String.concat "\n" parts
