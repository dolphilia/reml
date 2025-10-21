(* Audit_envelope — JSON Lines Audit Event Utilities (Phase 2-4 draft)
 *
 * 監査ログの型を仕様 3-6 §1 と同期させるための暫定実装。
 * Diagnostic と共通の `metadata` 語彙を維持しつつ、`audit_id` や
 * `change_set` を保持できるようにする。
 *)

module Json = Yojson.Basic

type metadata = (string * Json.t) list

type envelope = {
  audit_id : string option;
  change_set : Json.t option;
  capability : string option;
  metadata : metadata;
}

type t = envelope

type event = {
  timestamp : string;
  category : string;
  envelope : envelope;
}

let empty_envelope =
  { audit_id = None; change_set = None; capability = None; metadata = [] }

let iso8601_timestamp () =
  let tm = Unix.gmtime (Unix.time ()) in
  Printf.sprintf "%04d-%02d-%02dT%02d:%02d:%02dZ" (tm.Unix.tm_year + 1900)
    (tm.Unix.tm_mon + 1) tm.Unix.tm_mday tm.Unix.tm_hour tm.Unix.tm_min
    tm.Unix.tm_sec

let metadata_of_json = function
  | `Assoc pairs -> pairs
  | `Null -> []
  | json ->
      invalid_arg
        (Printf.sprintf
           "Audit_envelope.metadata_of_json: Assoc 以外の JSON を受け取りました: \
            %s"
           (Json.to_string json))

let metadata_to_json (metadata : metadata) = `Assoc metadata

let make ?timestamp ?audit_id ?change_set ?capability ?metadata
    ?metadata_pairs ~category () =
  let metadata_list =
    match (metadata_pairs, metadata) with
    | Some pairs, _ -> pairs
    | None, Some json -> metadata_of_json json
    | None, None -> []
  in
  let timestamp =
    match timestamp with Some value -> value | None -> iso8601_timestamp ()
  in
  {
    timestamp;
    category;
    envelope =
      { audit_id; change_set; capability; metadata = metadata_list };
  }

let add_metadata (env : envelope) ~key value =
  let filtered = List.filter (fun (k, _) -> not (String.equal k key)) env.metadata in
  { env with metadata = (key, value) :: filtered }

let merge_metadata (env : envelope) entries =
  List.fold_left (fun acc (key, value) -> add_metadata acc ~key value) env entries

let metadata (env : t) = env.metadata
let audit_id (env : t) = env.audit_id
let change_set (env : t) = env.change_set
let capability (env : t) = env.capability

let to_json (event : event) =
  let base =
    [
      ("timestamp", `String event.timestamp);
      ("category", `String event.category);
      ("metadata", metadata_to_json event.envelope.metadata);
    ]
  in
  let base =
    match event.envelope.audit_id with
    | Some id -> ("audit_id", `String id) :: base
    | None -> base
  in
  let base =
    match event.envelope.change_set with
    | Some change -> ("change_set", change) :: base
    | None -> base
  in
  let base =
    match event.envelope.capability with
    | Some cap when String.trim cap <> "" -> ("capability", `String cap) :: base
    | _ -> base
  in
  `Assoc (List.rev base)

let ensure_parent_directory path =
  let dir = Filename.dirname path in
  if dir <> "." && dir <> "" then
    let rec ensure path =
      if path = "." || path = "" then ()
      else if Sys.file_exists path then (
        if not (Sys.is_directory path) then
          invalid_arg (Printf.sprintf "\"%s\" はディレクトリではありません" path))
      else
        let parent = Filename.dirname path in
        if parent <> path then ensure parent;
        Unix.mkdir path 0o755
    in
    ensure dir

let append_events path events =
  match events with
  | [] -> ()
  | _ ->
      ensure_parent_directory path;
      let oc = open_out_gen [ Open_creat; Open_text; Open_append ] 0o644 path in
      List.iter
        (fun event ->
          Json.to_channel oc (to_json event);
          output_char oc '\n')
        events;
      close_out oc
