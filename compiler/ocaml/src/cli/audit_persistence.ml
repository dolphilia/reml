open Audit_path_resolver

module Json = Yojson.Basic

type outcome =
  | Success
  | Failure

let outcome_to_string = function Success -> "success" | Failure -> "failure"

let rec ensure_directory path =
  if path = "" || path = "." then ()
  else if Sys.file_exists path then (
    if not (Sys.is_directory path) then
      invalid_arg (Printf.sprintf "\"%s\" はディレクトリではありません" path))
  else (
    let parent = Filename.dirname path in
    if parent <> path then ensure_directory parent;
    Unix.mkdir path 0o755)

let ensure_parent_directory path =
  let dir = Filename.dirname path in
  if dir <> path then ensure_directory dir

let option_iter f = function Some value -> f value | None -> ()

let find_assoc key entries =
  let rec loop = function
    | [] -> None
    | (k, v) :: rest -> if String.equal k key then Some v else loop rest
  in
  loop entries

let json_list member =
  match member with
  | `List lst -> lst
  | _ -> []

let string_member json key =
  match json with
  | `Assoc entries -> (
      match find_assoc key entries with
      | Some (`String value) -> Some value
      | _ -> None)
  | _ -> None

let remove_entries_with_build_id build_id entries =
  let predicate json =
    match string_member json "build_id" with
    | Some existing when String.equal existing build_id -> false
    | _ -> true
  in
  List.filter predicate entries

let stats_size_bytes path =
  try
    let stat = Unix.stat path in
    Some (string_of_int stat.Unix.st_size)
  with Unix.Unix_error _ -> None

let trim_entries max_count entries =
  let total = List.length entries in
  if total <= max_count then ([], entries)
  else
    let rec drop n acc rest =
      if n = 0 then (List.rev acc, rest)
      else
        match rest with
        | [] -> (List.rev acc, [])
        | x :: xs -> drop (n - 1) (x :: acc) xs
    in
    drop (total - max_count) [] entries

let max_index_entries = 200
let max_history_entries = 20

let run_safely label f =
  try f () with
  | exn ->
      prerr_endline
        (Printf.sprintf "[audit] %s: %s" label (Printexc.to_string exn))

let ensure_audit_file path =
  if not (Sys.file_exists path) then (
    ensure_parent_directory path;
    let oc = open_out_gen [ Open_creat; Open_text ] 0o644 path in
    close_out oc)

let gzip_output_string gz str =
  let bytes = Bytes.unsafe_of_string str in
  Gzip.output gz bytes 0 (Bytes.length bytes)

let copy_file ~src ~dst =
  if Sys.file_exists src then (
    ensure_parent_directory dst;
    let buffer = Bytes.create 4096 in
    let ic = open_in_bin src in
    Fun.protect
      ~finally:(fun () -> close_in_noerr ic)
      (fun () ->
        let oc = open_out_bin dst in
        Fun.protect
          ~finally:(fun () -> close_out_noerr oc)
          (fun () ->
            let rec loop () =
              match input ic buffer 0 (Bytes.length buffer) with
              | 0 -> ()
              | read ->
                  output oc buffer 0 read;
                  loop ()
            in
            (try loop () with End_of_file -> ()))))

let update_index context outcome audit_path =
  match context.index_path with
  | None -> ([], None)
  | Some index_path ->
      ensure_parent_directory index_path;
      let current =
        if Sys.file_exists index_path then Json.from_file index_path
        else `Assoc []
      in
      let (existing_entries, existing_pruned, other_fields) =
        match current with
        | `Assoc entries ->
            let entries_json =
              find_assoc "entries" entries |> Option.value ~default:`Null
            in
            let pruned_json =
              find_assoc "pruned" entries |> Option.value ~default:`Null
            in
            let rest =
              List.filter
                (fun (k, _) -> not (String.equal k "entries" || String.equal k "pruned"))
                entries
            in
            (json_list entries_json, json_list pruned_json, rest)
        | _ -> ([], [], [])
      in
      let filtered =
        remove_entries_with_build_id context.build_id existing_entries
      in
      let entry_fields =
        let base_fields =
          [
            ("build_id", `String context.build_id);
            ("timestamp", `String context.timestamp_iso);
            ("profile", `String context.profile_string);
            ("audit_store", `String context.profile_string);
            ("target", `String context.target);
            ("audit_level", `String context.audit_level_string);
            ("path", `String audit_path);
            ("status", `String (outcome_to_string outcome));
            ("pass_rate", `Null);
          ]
        in
        let optional_fields =
          [
            (match context.commit_id with
            | Some commit when commit <> "" ->
                Some ("commit", `String commit)
            | _ -> None);
            (match context.run_dir with
            | Some dir -> Some ("run_dir", `String dir)
            | None -> None);
            (match stats_size_bytes audit_path with
            | Some size -> Some ("size_bytes", `String size)
            | None -> None);
          ]
          |> List.filter_map (fun x -> x)
        in
        base_fields @ optional_fields
      in
      let new_entry = `Assoc entry_fields in
      let combined_entries = filtered @ [ new_entry ] in
      let pruned_now, kept_entries =
        trim_entries max_index_entries combined_entries
      in
      let combined =
        `Assoc
          (("entries", `List kept_entries)
          :: ("pruned", `List (existing_pruned @ pruned_now))
          :: other_fields)
      in
      Json.to_file index_path combined;
      (kept_entries, Some new_entry)

let write_summary context entries =
  match context.summary_path with
  | None -> ()
  | Some summary_path ->
      ensure_parent_directory summary_path;
      let header =
        [
          "# 監査ログサマリー";
          "";
          "| Timestamp | Build ID | Target | Profile | Level | Status | パス |";
          "|-----------|----------|--------|---------|-------|--------|------|";
        ]
      in
      let rows =
        entries
        |> List.rev
        |> List.map (fun entry ->
               let field key default =
                 string_member entry key |> Option.value ~default
               in
               let timestamp = field "timestamp" "-" in
               let build_id = field "build_id" "-" in
               let target = field "target" "-" in
               let profile = field "profile" "-" in
               let level = field "audit_level" "-" in
               let status = field "status" "-" in
               let path =
                 field "path" "-"
                 |> fun v ->
                 if String.equal v "-" then v else Printf.sprintf "`%s`" v
               in
               Printf.sprintf "| %s | %s | %s | %s | %s | %s | %s |" timestamp
                 build_id target profile level status path)
      in
      let footer =
        [
          "";
          Printf.sprintf "最終更新: %s" context.timestamp_iso;
        ]
      in
      let oc = open_out summary_path in
      List.iter
        (fun line ->
          output_string oc line;
          output_char oc '\n')
        (header @ rows @ footer);
      close_out oc

let write_history context entries =
  match context.history_path with
  | None -> ()
  | Some history_path ->
      ensure_parent_directory history_path;
      let total = List.length entries in
      let relevant =
        let drop = max 0 (total - max_history_entries) in
        let rec drop_n n lst =
          if n <= 0 then lst
          else
            match lst with
            | [] -> []
            | _ :: xs -> drop_n (n - 1) xs
        in
        drop_n drop entries
      in
      if relevant = [] then (
        if Sys.file_exists history_path then Sys.remove history_path)
      else
        let gz = Gzip.open_out history_path in
        let write_log path =
          if Sys.file_exists path then
            let ic = open_in path in
            Fun.protect
              ~finally:(fun () -> close_in_noerr ic)
              (fun () ->
                (try
                   while true do
                     let line = input_line ic in
                     gzip_output_string gz line;
                     Gzip.output_char gz '\n'
                   done
                 with End_of_file -> ()))
        in
        Fun.protect
          ~finally:(fun () -> Gzip.close_out gz)
          (fun () ->
            List.iter
              (fun entry ->
                match string_member entry "path" with
                | Some log_path -> write_log log_path
                | None -> ())
              relevant)

let persist_failure_artifacts context ~entry ~audit_path =
  match context.failed_dir with
  | None -> ()
  | Some failed_dir ->
      ensure_directory failed_dir;
      if Sys.file_exists audit_path then
        let dest =
          Filename.concat failed_dir (Filename.basename audit_path)
        in
        copy_file ~src:audit_path ~dst:dest;
      let metadata_path = Filename.concat failed_dir "entry.json" in
      Json.to_file metadata_path entry

let append_events context ?(outcome = Success) events =
  if not context.enabled then ()
  else (
    option_iter ensure_directory context.store_root;
    option_iter ensure_directory context.run_dir;
    option_iter ensure_parent_directory context.audit_path;
    option_iter ensure_parent_directory context.index_path;
    option_iter ensure_parent_directory context.summary_path;
    option_iter ensure_parent_directory context.history_path;
    match context.audit_path with
    | None -> ()
    | Some path ->
        run_safely "audit log append" (fun () ->
            Audit_envelope.append_events path events);
        ensure_audit_file path;
        let (entries, current_entry_opt) =
          update_index context outcome path
        in
        run_safely "summary update" (fun () ->
            write_summary context entries);
        run_safely "history update" (fun () ->
            write_history context entries);
        (match (outcome, current_entry_opt) with
        | Failure, Some entry ->
            run_safely "failure artifact persist" (fun () ->
                persist_failure_artifacts context ~entry ~audit_path:path)
        | _ -> ()))
