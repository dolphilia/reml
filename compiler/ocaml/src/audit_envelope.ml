(* Audit_envelope — JSON Lines Audit Event Utilities
 *
 * Phase 2-2 で追加された監査ログ出力用ヘルパ。
 * Diagnostic とは独立に、Runtime/Compiler が収集したメタデータを
 * JSON Lines 形式で書き出す。
 *)

type event = {
  timestamp : string;
  category : string;
  metadata : Yojson.Basic.t;
}

let iso8601_timestamp () =
  let tm = Unix.gmtime (Unix.time ()) in
  Printf.sprintf "%04d-%02d-%02dT%02d:%02d:%02dZ" (tm.Unix.tm_year + 1900)
    (tm.Unix.tm_mon + 1) tm.Unix.tm_mday tm.Unix.tm_hour tm.Unix.tm_min
    tm.Unix.tm_sec

let make ?timestamp ~category ~metadata () =
  let timestamp =
    match timestamp with Some value -> value | None -> iso8601_timestamp ()
  in
  { timestamp; category; metadata }

let to_json (event : event) =
  `Assoc
    [
      ("timestamp", `String event.timestamp);
      ("category", `String event.category);
      ("metadata", event.metadata);
    ]

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
          Yojson.Basic.to_channel oc (to_json event);
          output_char oc '\n')
        events;
      close_out oc
