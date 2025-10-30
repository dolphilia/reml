(** Core.Parse.Lex 橋渡しユーティリティ (LEXER-002 Step3)

    `RunConfig.extensions["config"].trivia` と `extensions["lex"]` を読み取り、
    仕様で定義される `ConfigTriviaProfile` を再構成しつつ、`lexeme` /
    `symbol` などのユーティリティを OCaml 実装で提供する。
*)

module Extensions = Parser_run_config.Extensions
module Namespace = Parser_run_config.Extensions.Namespace
module Lex = Parser_run_config.Lex

let bool_of_value = function Extensions.Bool value -> Some value | _ -> None

let string_of_value = function Extensions.String value -> Some value | _ -> None

let string_list_of_value value =
  match value with
  | Extensions.List values ->
      let rec collect acc = function
        | [] -> Some (List.rev acc)
        | item :: rest -> (
            match string_of_value item with
            | Some text -> collect (text :: acc) rest
            | None -> None)
      in
      collect [] values
  | Extensions.String value -> Some [ value ]
  | _ -> None

let comment_pair_of_value value =
  match value with
  | Extensions.List
      (Extensions.String start :: Extensions.String stop :: rest) ->
      let nested =
        match rest with
        | [] -> Some true
        | [ Extensions.Bool flag ] -> Some flag
        | _ -> None
      in
      Option.map
        (fun nested ->
          { Parser_run_config.Lex.Trivia_profile.start; stop; nested })
        nested
  | _ -> None

let comment_block_of_value value =
  match value with
  | Extensions.List pairs ->
      let rec collect acc = function
        | [] -> Some (List.rev acc)
        | item :: rest -> (
            match comment_pair_of_value item with
            | Some pair -> collect (pair :: acc) rest
            | None -> None)
      in
      collect [] pairs
  | _ -> None

module Trivia_profile = struct
  include Lex.Trivia_profile

  let override_lines namespace profile =
    match Namespace.find "line" namespace with
    | Some value -> (
        match string_list_of_value value with
        | Some lines when List.length lines > 0 -> { profile with line = lines }
        | _ -> profile)
    | None -> profile

  let override_block namespace profile =
    match Namespace.find "block" namespace with
    | Some value -> (
        match comment_block_of_value value with
        | Some block -> { profile with block }
        | None -> profile)
    | None -> profile

  let override_bool namespace key update profile =
    match Namespace.find key namespace with
    | Some value -> (
        match bool_of_value value with
        | Some flag -> update profile flag
        | None -> profile)
    | None -> profile

  let override_doc namespace profile =
    match Namespace.find "doc_comment" namespace with
    | Some value -> (
        match string_of_value value with
        | Some doc -> { profile with doc_comment = Some doc }
        | None -> (
            match value with
            | Extensions.Bool false -> { profile with doc_comment = None }
            | _ -> profile))
    | None -> profile

  let apply_overrides profile namespace =
    profile
    |> override_lines namespace
    |> override_block namespace
    |> override_bool namespace "shebang" (fun profile shebang ->
           { profile with shebang })
    |> override_bool namespace "hash_inline" (fun profile hash_inline ->
           { profile with hash_inline })
    |> override_doc namespace

  let base_profile = function
    | Lex.Strict_json -> strict_json
    | Lex.Json_relaxed -> json_relaxed
    | Lex.Toml_relaxed -> toml_relaxed
    | Lex.Custom _ -> strict_json

  let of_profile profile ~namespace =
    match namespace with
    | None -> base_profile profile
    | Some ns -> apply_overrides (base_profile profile) ns
end

module Pack = struct
  type source =
    | From_config_namespace
    | From_lex_namespace
    | Default

  type t = {
    profile : Lex.profile;
    trivia : Trivia_profile.t;
    namespace : Namespace.t option;
    space_id : int option;
    source : source;
  }
end

module Bridge = struct
  let derive run_config =
    let lex_namespace = Lex.of_run_config run_config in
    let config_namespace = Parser_run_config.Config.find run_config in
    let from_config =
      Option.bind config_namespace Parser_run_config.Config.trivia_profile
    in
    let profile, source =
      match from_config with
      | Some profile -> (profile, Pack.From_config_namespace)
      | None -> (
          match lex_namespace.namespace with
          | Some _ -> (lex_namespace.profile, Pack.From_lex_namespace)
          | None -> (lex_namespace.profile, Pack.Default))
    in
    let trivia =
      Trivia_profile.of_profile profile ~namespace:lex_namespace.namespace
    in
    let synced_config =
      Parser_run_config.Config.with_trivia_profile
        (Lex.set_profile run_config profile)
        profile
    in
    let pack =
      {
        Pack.profile = profile;
        trivia;
        namespace = lex_namespace.namespace;
        space_id = lex_namespace.space_id;
        source;
      }
    in
    (pack, synced_config)

  let with_space_id pack run_config space_id =
    let updated_config = Lex.set_space_id run_config (Some space_id) in
    let namespace =
      let base = Option.value pack.Pack.namespace ~default:Namespace.empty in
      Some (Namespace.add "space_id" (Extensions.Parser_id space_id) base)
    in
    ( { pack with space_id = Some space_id; namespace }, updated_config )
end

module Record = Core_parse_lex_record

module Api = struct
  type 'a reader = Lexing.lexbuf -> 'a

  let ensure_profile pack = Lexer.set_trivia_profile pack.Pack.trivia

  let config_trivia pack lexbuf =
    ensure_profile pack;
    ignore lexbuf

  let leading pack reader lexbuf =
    let () = config_trivia pack lexbuf in
    reader lexbuf

  let lexeme pack reader lexbuf =
    let result = reader lexbuf in
    let () = config_trivia pack lexbuf in
    result

  let trim pack reader lexbuf =
    let () = config_trivia pack lexbuf in
    let result = reader lexbuf in
    let () = config_trivia pack lexbuf in
    result

  let unexpected_symbol span ~expected ~actual =
    let message =
      Printf.sprintf "`%s` が必要ですが `%s` が見つかりました" expected actual
    in
    raise (Lexer.Lexer_error (message, span))

  let span_of_positions (start_pos : Lexing.position) (end_pos : Lexing.position) =
    {
      Ast.start = start_pos.Lexing.pos_cnum;
      end_ = end_pos.Lexing.pos_cnum;
    }

  let symbol pack expected lexbuf =
    let reader lexbuf =
      let token, start_pos, end_pos = Lexer.read_token lexbuf in
      let actual = Token.to_string token in
      if String.equal actual expected then ()
      else
        let span = span_of_positions start_pos end_pos in
        unexpected_symbol span ~expected ~actual
    in
    lexeme pack reader lexbuf

  let token pack reader lexbuf =
    let value = reader lexbuf in
    let start_pos = Lexing.lexeme_start_p lexbuf in
    let end_pos = Lexing.lexeme_end_p lexbuf in
    let span = span_of_positions start_pos end_pos in
    let () = config_trivia pack lexbuf in
    (value, span)
end
