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

let get key entries =
  match List.find_opt (fun (k, _) -> String.equal k key) entries with
  | Some (_, value) -> Some value
  | None -> None
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

(* ========== 診断情報 (V2 フィールド) ========== *)

type span_label = { span : span option; message : string option }

type hint = { message : string option; actions : fixit list }

type t = {
  id : string option;
  message : string;
  severity : severity;
  severity_hint : severity_hint option;
  domain : error_domain option;
  codes : string list;
  primary : span;
  secondary : span_label list;
  hints : hint list;
  fixits : fixit list;
  expected : expectation_summary option;
  audit : Audit_envelope.t option;
  timestamp : string option;
  extensions : Extensions.t;
  audit_metadata : Extensions.t;
}
(** 診断情報の完全な表現（仕様 3-6 §1 準拠フィールド） *)

module Legacy = struct
  type t = {
    severity : severity;
    severity_hint : severity_hint option;
    domain : error_domain option;
    code : string option;
    message : string;
    span : span;
    expected_summary : expectation_summary option;
    notes : (span option * string) list;
    fixits : fixit list;
    extensions : Extensions.t;
    audit_metadata : Extensions.t;
  }
end

let legacy_of_diagnostic_internal (diag : t) : Legacy.t =
  let primary_code =
    match diag.codes with code :: _ -> Some code | [] -> None
  in
  Legacy.
    {
      severity = diag.severity;
      severity_hint = diag.severity_hint;
      domain = diag.domain;
      code = primary_code;
      message = diag.message;
      span = diag.primary;
      expected_summary = diag.expected;
      notes =
        List.map
          (fun (label : span_label) ->
            (label.span, Option.value ~default:"" label.message))
          diag.secondary;
      fixits = diag.fixits;
      extensions = diag.extensions;
      audit_metadata = diag.audit_metadata;
    }

let legacy_of_diagnostic[@deprecated "Diagnostic.Legacy を直接生成せず Diagnostic.t を利用してください。"] =
  legacy_of_diagnostic_internal

let diagnostic_of_legacy_internal (legacy : Legacy.t) : t =
  {
    id = None;
    message = legacy.Legacy.message;
    severity = legacy.Legacy.severity;
    severity_hint = legacy.Legacy.severity_hint;
    domain = legacy.Legacy.domain;
    codes =
      (match legacy.Legacy.code with Some code -> [ code ] | None -> []);
    primary = legacy.Legacy.span;
    secondary =
      List.map
        (fun (span_opt, message) ->
          { span = span_opt; message = Some message })
        legacy.Legacy.notes;
    hints = [];
    fixits = legacy.Legacy.fixits;
    expected = legacy.Legacy.expected_summary;
    audit = None;
    timestamp = None;
    extensions = legacy.Legacy.extensions;
    audit_metadata = legacy.Legacy.audit_metadata;
  }

let diagnostic_of_legacy[@deprecated "Diagnostic.Legacy.t からの変換は段階的に廃止予定です。"] =
  diagnostic_of_legacy_internal

let primary_code (diag : t) =
  match diag.codes with code :: _ -> Some code | [] -> None

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

let set_extension key value diag =
  { diag with extensions = Extensions.set key value diag.extensions }

let merge_audit_metadata entries diag =
  match entries with
  | [] -> diag
  | _ ->
      let metadata =
        List.fold_left
          (fun acc (key, value) -> Extensions.set key value acc)
          diag.audit_metadata entries
      in
      let base =
        match diag.audit with
        | Some env -> env
        | None -> Audit_envelope.empty_envelope
      in
      let envelope = Audit_envelope.merge_metadata base entries in
      { diag with audit_metadata = metadata; audit = Some envelope }

let set_audit_metadata key value diag =
  merge_audit_metadata [ (key, value) ] diag

let set_audit_id id diag =
  let base =
    match diag.audit with
    | Some env -> env
    | None -> Audit_envelope.empty_envelope
  in
  let envelope = { base with audit_id = Some id } in
  { diag with audit = Some envelope }

let set_change_set change diag =
  let base =
    match diag.audit with
    | Some env -> env
    | None -> Audit_envelope.empty_envelope
  in
  let envelope = { base with change_set = Some change } in
  { diag with audit = Some envelope }

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

(* ========== Diagnostic v2（仕様同期用ドラフト） ========== *)

type legacy_diagnostic = t
type legacy_severity = severity

module V2 = struct
  type severity = Error | Warning | Info | Hint

  let severity_of_diagnostic (diag : t) =
    match diag.severity with
    | Error -> Error
    | Warning -> Warning
    | Note -> Info

  let severity_to_lsp_int = function
    | Error -> 1
    | Warning -> 2
    | Info -> 3
    | Hint -> 4

  let severity_to_string = function
    | Error -> "error"
    | Warning -> "warning"
    | Info -> "info"
    | Hint -> "hint"

  let span_to_range_json (span : span) =
    `Assoc
      [
        ( "start",
          `Assoc
            [
              ("line", `Int (span.start_pos.line - 1));
              ("character", `Int (span.start_pos.column - 1));
            ] );
        ( "end",
          `Assoc
            [
              ("line", `Int (span.end_pos.line - 1));
              ("character", `Int (span.end_pos.column - 1));
            ] );
      ]

  let span_label_to_json { span; message } =
    let base =
      match span with
      | Some span -> [ ("range", span_to_range_json span) ]
      | None -> []
    in
    let base =
      match message with
      | Some msg -> ("message", `String msg) :: base
      | None -> base
    in
    `Assoc base

  let fixit_action_to_json = function
    | Insert { at; text } ->
        `Assoc
          [
            ("kind", `String "insert");
            ("range", span_to_range_json at);
            ("text", `String text);
          ]
    | Replace { at; text } ->
        `Assoc
          [
            ("kind", `String "replace");
            ("range", span_to_range_json at);
            ("text", `String text);
          ]
    | Delete { at } ->
        `Assoc
          [
            ("kind", `String "delete");
            ("range", span_to_range_json at);
          ]

  let hint_to_json { message; actions } =
    let base =
      match message with
      | Some msg -> [ ("message", `String msg) ]
      | None -> []
    in
    let actions_json =
      match actions with
      | [] -> []
      | xs ->
          [ ("actions", `List (List.map fixit_action_to_json xs)) ]
    in
    `Assoc (base @ actions_json)

  let audit_to_json = function
    | None -> `Null
    | Some envelope ->
        let metadata_json =
          Audit_envelope.metadata_to_json (Audit_envelope.metadata envelope)
        in
        let fields = [ ("metadata", metadata_json) ] in
        let fields =
          match Audit_envelope.audit_id envelope with
          | Some id -> ("audit_id", `String id) :: fields
          | None -> fields
        in
        let fields =
          match Audit_envelope.change_set envelope with
          | Some change -> ("change_set", change) :: fields
          | None -> fields
        in
        let fields =
          match Audit_envelope.capability envelope with
          | Some cap when String.trim cap <> "" -> ("capability", `String cap) :: fields
          | _ -> fields
        in
        `Assoc (List.rev fields)
end

(* ========== Diagnostic Builder ========== *)

module Builder = struct
  type secondary_entry = { span : span option; message : string option }

  type structured_hint_kind =
    | Quick_fix
    | Follow_up
    | Context
    | Documentation
    | Command
    | Link
    | Custom of string

  type structured_hint_payload =
    | Command_payload of {
        command : string;
        arguments : Json.t option;
      }
    | Link_payload of {
        href : string;
        title : string option;
      }
    | Replacement_payload of {
        range : span option;
        template : string;
      }
    | Message_payload of string
    | Data_payload of Json.t

  type structured_hint = {
    id : string option;
    title : string option;
    span : span option;
    kind : structured_hint_kind;
    payload : structured_hint_payload;
    actions : fixit list;
  }

  type t = {
    id : string option;
    severity : severity;
    severity_hint : severity_hint option;
    domain : error_domain option;
    message : string;
    primary : span;
    codes : string list;
    secondary : secondary_entry list;
    expected : expectation_summary option;
    hints : hint list;
    fixits : fixit list;
    extensions : Extensions.t;
    structured_hints : structured_hint list;
    audit_metadata : Extensions.t;
    timestamp : string option;
  }

  let create ?id ?(severity = Error) ?severity_hint ?domain ?code ?(codes = [])
      ?timestamp ~message ~primary () =
    let codes =
      match (code, codes) with
      | Some c, _ when List.mem c codes -> codes
      | Some c, _ -> c :: codes
      | None, _ -> codes
    in
    {
      id;
      severity;
      severity_hint;
      domain;
      message;
      primary;
      codes;
      secondary = [];
      expected = None;
      hints = [];
      fixits = [];
      extensions = Extensions.empty;
      structured_hints = [];
      audit_metadata = Extensions.empty;
      timestamp;
    }

  let json_of_location loc =
    `Assoc
      [
        ("file", `String loc.filename);
        ("line", `Int loc.line);
        ("column", `Int loc.column);
        ("offset", `Int loc.offset);
      ]

  let json_of_span span =
    `Assoc
      [
        ("start", json_of_location span.start_pos);
        ("end", json_of_location span.end_pos);
      ]

  let json_of_fixit = function
    | Insert { at; text } ->
        `Assoc
          [
            ("kind", `String "insert");
            ("range", json_of_span at);
            ("text", `String text);
          ]
    | Replace { at; text } ->
        `Assoc
          [
            ("kind", `String "replace");
            ("range", json_of_span at);
            ("text", `String text);
          ]
    | Delete { at } ->
        `Assoc
          [
            ("kind", `String "delete");
            ("range", json_of_span at);
          ]

  let set_id id builder = { builder with id = Some id }
  let clear_id builder = { builder with id = None }

  let set_severity severity builder = { builder with severity }
  let set_severity_hint severity_hint builder = { builder with severity_hint }

  let set_domain domain builder = { builder with domain = Some domain }

  let add_code code builder =
    if List.mem code builder.codes then builder
    else { builder with codes = builder.codes @ [ code ] }

  let set_codes codes builder = { builder with codes }

  let push_code code builder = add_code code builder

  let add_codes codes builder = List.fold_left (fun acc c -> push_code c acc) builder codes

  let set_primary_code code builder =
    let rest = List.filter (fun existing -> not (String.equal existing code)) builder.codes in
    { builder with codes = code :: rest }

  let add_secondary ?span ?message builder =
    let entry = { span; message } in
    { builder with secondary = builder.secondary @ [ entry ] }

  let merge_secondary entries builder =
    List.fold_left
      (fun acc (label : span_label) ->
        add_secondary ?span:label.span ?message:label.message acc)
      builder entries

  let clear_secondary builder = { builder with secondary = [] }

  let add_note ?span message builder =
    add_secondary ?span ?message:(Some message) builder

  let add_notes notes builder =
    List.fold_left
      (fun acc (span_opt, message) -> add_note ?span:span_opt message acc)
      builder notes

  let set_expected expected builder = { builder with expected = Some expected }

  let clear_expected builder = { builder with expected = None }

  let add_fixits fixits builder =
    { builder with fixits = builder.fixits @ fixits }

  let add_hint ?(actions = []) ?message builder =
    let hint = { message; actions } in
    let builder =
      if actions = [] then builder else add_fixits actions builder
    in
    { builder with hints = builder.hints @ [ hint ] }

  let structured_hint_kind_to_string = function
    | Quick_fix -> "quick_fix"
    | Follow_up -> "follow_up"
    | Context -> "context"
    | Documentation -> "documentation"
    | Command -> "command"
    | Link -> "link"
    | Custom tag -> tag

  let structured_hint_payload_to_json = function
    | Command_payload { command; arguments } ->
        let base = [ ("kind", `String "command"); ("command", `String command) ] in
        let fields =
          match arguments with
          | Some args -> ("arguments", args) :: base
          | None -> base
        in
        `Assoc fields
    | Link_payload { href; title } ->
        let base = [ ("kind", `String "link"); ("href", `String href) ] in
        let fields =
          match title with
          | Some txt -> ("title", `String txt) :: base
          | None -> base
        in
        `Assoc fields
    | Replacement_payload { range; template } ->
        let range_json =
          match range with
          | Some span -> json_of_span span
          | None -> `Null
        in
        `Assoc
          [
            ("kind", `String "replacement");
            ("range", range_json);
            ("template", `String template);
          ]
    | Message_payload message ->
        `Assoc [ ("kind", `String "message"); ("text", `String message) ]
    | Data_payload json -> `Assoc [ ("kind", `String "data"); ("payload", json) ]

  let structured_hint_to_json hint =
    let base =
      [
        ("kind", `String (structured_hint_kind_to_string hint.kind));
        ("payload", structured_hint_payload_to_json hint.payload);
      ]
    in
    let fields =
      match hint.span with
      | Some span -> ("span", json_of_span span) :: base
      | None -> base
    in
    let fields =
      match hint.id with
      | Some id -> ("id", `String id) :: fields
      | None -> fields
    in
    let fields =
      match hint.title with
      | Some title -> ("title", `String title) :: fields
      | None -> fields
    in
    let fields =
      if hint.actions = [] then fields
      else
        ("actions", `List (List.map json_of_fixit hint.actions)) :: fields
    in
    `Assoc fields

  let add_structured_hint ?id ?title ?span ?(actions = []) ~kind ~payload builder =
    let hint = { id; title; span; kind; payload; actions } in
    let builder =
      if actions = [] then builder else add_fixits actions builder
    in
    { builder with structured_hints = builder.structured_hints @ [ hint ] }

  let merge_structured_hints hints builder =
    let builder =
      List.fold_left
        (fun acc hint ->
          let builder =
            if hint.actions = [] then acc else add_fixits hint.actions acc
          in
          { builder with structured_hints = builder.structured_hints @ [ hint ] })
        builder hints
    in
    builder

  let clear_structured_hints builder = { builder with structured_hints = [] }

  let with_extensions extensions builder =
    { builder with extensions }

  let add_extension key value builder =
    { builder with extensions = Extensions.set key value builder.extensions }

  let with_audit_metadata metadata builder =
    { builder with audit_metadata = metadata }

  let add_audit_metadata key value builder =
    {
      builder with
      audit_metadata =
        Extensions.set key value builder.audit_metadata;
    }

  let set_timestamp timestamp builder = { builder with timestamp = Some timestamp }

  let build builder =
    let codes = builder.codes in
    let secondary =
      builder.secondary
      |> List.map (fun (entry : secondary_entry) ->
             ( { span = entry.span; message = entry.message } : span_label ))
    in
    let v2_extension_fields =
      let fields = ref [] in
      (if codes <> [] then
         fields :=
           ("codes", `List (List.map (fun c -> `String c) codes)) :: !fields);
      (if builder.hints <> [] then
         let hints_json =
           builder.hints
           |> List.map (fun (hint : hint) ->
                  let base =
                    match hint.message with
                    | Some msg -> [ ("message", `String msg) ]
                    | None -> []
                  in
                  let fields =
                    match hint.actions with
                    | [] -> base
                    | xs ->
                        ("actions", `List (List.map json_of_fixit xs)) :: base
                  in
                  `Assoc fields)
         in
         fields := ("hints", `List hints_json) :: !fields);
      (if builder.structured_hints <> [] then
         let structured_json =
           List.map structured_hint_to_json builder.structured_hints
         in
         fields :=
           ("structured_hints", `List structured_json) :: !fields);
      (match builder.timestamp with
      | Some ts -> fields := ("timestamp", `String ts) :: !fields
      | None -> ());
      !fields
    in
    let extensions =
      if v2_extension_fields = [] then builder.extensions
      else
        Extensions.set "diagnostic.v2"
          (`Assoc v2_extension_fields)
          builder.extensions
    in
    let audit =
      if Extensions.is_empty builder.audit_metadata then None
      else Some { Audit_envelope.empty_envelope with metadata = builder.audit_metadata }
    in
    {
      id = builder.id;
      message = builder.message;
      severity = builder.severity;
      severity_hint = builder.severity_hint;
      domain = builder.domain;
      codes;
      primary = builder.primary;
      secondary;
      hints = builder.hints;
      fixits = builder.fixits;
      expected = builder.expected;
      audit;
      timestamp = builder.timestamp;
      extensions;
      audit_metadata = builder.audit_metadata;
    }
end

(* ========== 診断情報の構築 ========== *)

(** 診断情報の構築（Phase 1互換） *)
let make ?(severity = Error) ?severity_hint ?domain ?code
    ?(expected_summary = None) ?(notes = []) ?(fixits = [])
    ?(extensions = Extensions.empty) ?(audit_metadata = Extensions.empty)
    ~message ~start_pos ~end_pos () =
  let primary = span_of_positions start_pos end_pos in
  let builder =
    Builder.create ~severity ?severity_hint ?domain ?code ~message ~primary ()
    |> Builder.add_notes (List.map (fun note -> (None, note)) notes)
    |> Builder.add_fixits fixits
    |> Builder.with_extensions extensions
    |> Builder.with_audit_metadata audit_metadata
  in
  let builder =
    match expected_summary with
    | Some summary -> Builder.set_expected summary builder
    | None -> builder
  in
  Builder.build builder

(** 型エラー用の診断情報を構築 *)
let make_type_error ?(severity = Error) ?severity_hint ?code ?expected_summary
    ?(notes = []) ?(fixits = []) ?(extensions = Extensions.empty)
    ?(audit_metadata = Extensions.empty) ~message ~span () =
  let builder =
    Builder.create ~severity ?severity_hint ?code ~message ~primary:span ()
    |> Builder.set_domain Type
    |> Builder.add_notes notes
    |> Builder.add_fixits fixits
    |> Builder.with_extensions extensions
    |> Builder.with_audit_metadata audit_metadata
  in
  let builder =
    match expected_summary with
    | Some summary -> Builder.set_expected summary builder
    | None -> builder
  in
  Builder.build builder

(** Lexerエラー用（Phase 1互換） *)
let of_lexer_error ~message ~start_pos ~end_pos =
  let primary = span_of_positions start_pos end_pos in
  Builder.create ~message ~primary ~domain:Parser () |> Builder.build

(** Parserエラー用（Phase 1互換） *)
let of_parser_error ~message ~start_pos ~end_pos ~expected =
  let expected_summary =
    {
      message_key = None;
      locale_args = [];
      humanized = None;
      context_note = None;
      alternatives = expected;
    }
  in
  Builder.create ~message ~primary:(span_of_positions start_pos end_pos)
    ~domain:Parser ()
  |> Builder.set_expected expected_summary
  |> Builder.build

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
  let loc = format_location diag.primary.start_pos in

  (* ヘッダー行 *)
  let header =
    match (primary_code diag, diag.domain) with
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
    match diag.expected with
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
    match diag.secondary with
    | [] -> []
    | notes ->
        notes
        |> List.filter_map (fun (label : span_label) ->
               match label.message with
               | None -> None
               | Some note ->
                   let text =
                     match label.span with
                     | None -> "補足: " ^ note
                     | Some span ->
                         Printf.sprintf "補足 [%s]: %s" (format_span span) note
                   in
                   Some text)
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
