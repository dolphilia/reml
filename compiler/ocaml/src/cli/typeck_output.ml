(* Typeck_output — 型推論成果物の JSON 出力
 *
 * W3 デュアルライト検証で利用する typed-ast / constraints / typeck-debug
 * ファイルを生成するためのユーティリティ。
 *)

open Typed_ast
module Json = Yojson.Basic

let ensure_parent_directory path =
  let dir = Filename.dirname path in
  if dir <> "" && dir <> "." then File_util.ensure_directory dir

let span_to_json (span : Ast.span) =
  `Assoc [ ("start", `Int span.start); ("end", `Int span.end_) ]

let stage_id_to_string = Effect_profile.stage_id_to_string

let runtime_stage_to_json (stage : Type_inference_effect.runtime_stage) =
  let capability_fields =
    List.map
      (fun (name, stage_id) ->
        `Assoc
          [
            ("capability", `String name);
            ("stage", `String (stage_id_to_string stage_id));
          ])
      stage.capability_stages
  in
  `Assoc
    [
      ("default_stage", `String (stage_id_to_string stage.default_stage));
      ("capability_stages", `List capability_fields);
    ]

let string_of_type_row_mode = function
  | Type_inference.Type_row_integrated -> "integrated"
  | Type_inference.Type_row_dual_write -> "dual-write"
  | Type_inference.Type_row_metadata_only -> "metadata-only"

let function_summaries (tcu : typed_compilation_unit) =
  let summary_of_decl tdecl =
    match tdecl.tdecl_kind with
    | TFnDecl fn ->
        `Assoc
          [
            ("name", `String fn.tfn_name.name);
            ("param_count", `Int (List.length fn.tfn_params));
            ("return_type", `String (Types.string_of_ty fn.tfn_ret_type));
            ("effect_row", `String (Types.string_of_effect_row fn.tfn_effect_row));
            ("span", span_to_json tdecl.tdecl_span);
            ("dict_refs", `Int (List.length tdecl.tdecl_dict_refs));
          ]
        |> Option.some
    | _ -> None
  in
  List.filter_map summary_of_decl tcu.tcu_items

let typed_ast_json ~input_file tcu =
  let rendered = Typed_ast.string_of_typed_compilation_unit tcu in
  `Assoc
    [
      ("input", `String input_file);
      ("function_summaries", `List (function_summaries tcu));
      ("rendered", `String rendered);
    ]

let constraints_json ~stats tcu =
  let functions = function_summaries tcu in
  let stats_assoc =
    [
      ("unify_calls", `Int stats.Stats.unify_calls);
      ("ast_nodes", `Int stats.Stats.ast_node_count);
      ("token_count", `Int stats.Stats.token_count);
    ]
  in
  `Assoc
    [
      ("functions", `List functions);
      ("stats", `Assoc stats_assoc);
    ]

let typeck_debug_json ~runtime_stage ~type_config ~stats =
  let stage_json = runtime_stage_to_json runtime_stage in
  let stats_json =
    [
      ("unify_calls", `Int stats.Stats.unify_calls);
      ("ast_nodes", `Int stats.Stats.ast_node_count);
      ("token_count", `Int stats.Stats.token_count);
    ]
  in
  `Assoc
    [
      ("effect_context", stage_json);
      ( "type_row_mode",
        `String (string_of_type_row_mode type_config.Type_inference.type_row_mode)
      );
      ("stats", `Assoc stats_json);
    ]

let write_json ~path json =
  ensure_parent_directory path;
  let channel = open_out path in
  Yojson.Basic.pretty_to_channel channel json;
  output_char channel '\n';
  close_out channel

let emit ~input ~typed_ast ~type_config ~runtime_stage ~stats
    ~typed_ast_path ~constraints_path ~debug_path =
  (match typed_ast_path with
  | Some path ->
      let json = typed_ast_json ~input_file:input typed_ast in
      write_json ~path json
  | None -> ());
  (match constraints_path with
  | Some path ->
      let json = constraints_json ~stats typed_ast in
      write_json ~path json
  | None -> ());
  (match debug_path with
  | Some path ->
      let json =
        typeck_debug_json ~runtime_stage ~type_config ~stats
      in
      write_json ~path json
  | None -> ())
