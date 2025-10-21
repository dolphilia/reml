open Audit_path_resolver

let rec ensure_directory path =
  if path = "" || path = "." then ()
  else if Sys.file_exists path then (
    if not (Sys.is_directory path) then
      invalid_arg (Printf.sprintf "\"%s\" はディレクトリではありません" path))
  else (
    let parent = Filename.dirname path in
    if parent <> path then ensure_directory parent;
    Unix.mkdir path 0o755)

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

let update_index context audit_path =
  match context.index_path with
  | None -> ()
  | Some index_path ->
      ensure_directory (Filename.dirname index_path);
      let current =
        if Sys.file_exists index_path then
          Yojson.Basic.from_file index_path
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
                (fun (k, _) -> k <> "entries" && k <> "pruned")
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
      let new_entries = filtered @ [ `Assoc entry_fields ] in
      let combined =
        `Assoc
          (("entries", `List new_entries)
          :: ("pruned", `List existing_pruned)
          :: other_fields)
      in
      Yojson.Basic.to_file index_path combined

let append_events context events =
  if not context.enabled then ()
  else
    match context.audit_path with
    | None -> ()
    | Some path ->
        Audit_envelope.append_events path events;
        update_index context path
