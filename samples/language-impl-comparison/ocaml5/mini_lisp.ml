(* ミニ Lisp 評価機 (OCaml 5 実装)
   S式構文を持つ式を解析して評価する
   OCaml 5 の代数的効果を活用 *)

module MiniLisp = struct
  (* 式の抽象構文木 *)
  type expr =
    | Number of float
    | Symbol of string
    | List of expr list

  (* 評価値 *)
  type value =
    | VNumber of float
    | VLambda of {
        params : string list;
        body : expr;
        env : env;
      }
    | VBuiltin of (value list -> (value, string) result)

  and env = (string * value) list

  (* パースエラー *)
  type parse_error =
    | UnexpectedToken of string
    | UnmatchedParen
    | EmptyInput

  (* トークン化: S式の括弧をスペースで区切る *)
  let tokenize source =
    source
    |> String.split_on_char ' '
    |> List.concat_map (fun s ->
        s
        |> String.split_on_char '('
        |> List.concat_map (fun s2 -> String.split_on_char ')' s2)
      )
    |> List.filter (fun s -> s <> "")

  (* 式のパース *)
  let rec parse_expr tokens =
    match tokens with
    | [] -> Error EmptyInput
    | token :: rest -> parse_token token rest

  and parse_token token rest =
    if token = "(" then
      parse_list rest []
    else if token = ")" then
      Error UnmatchedParen
    else
      match Float.of_string_opt token with
      | Some num -> Ok (Number num, rest)
      | None -> Ok (Symbol token, rest)

  and parse_list tokens acc =
    match tokens with
    | [] -> Error UnmatchedParen
    | ")" :: rest -> Ok (List (List.rev acc), rest)
    | token :: rest ->
      (match parse_token token rest with
       | Ok (expr, next) -> parse_list next (expr :: acc)
       | Error err -> Error err)

  (* 式の評価 *)
  let rec eval_expr expr env =
    match expr with
    | Number n -> Ok (VNumber n)
    | Symbol name ->
      (match List.assoc_opt name env with
       | Some value -> Ok value
       | None -> Error ("未定義シンボル: " ^ name))
    | List items -> eval_list items env

  and eval_list items env =
    match items with
    | [] -> Error "空のリストは評価できません"
    | head :: rest ->
      (match eval_expr head env with
       | Error err -> Error err
       | Ok callee ->
         (match evaluate_args rest env with
          | Error err -> Error err
          | Ok args -> apply callee args))

  and evaluate_args exprs env =
    List.fold_left
      (fun acc_res expr ->
         match acc_res with
         | Error err -> Error err
         | Ok acc ->
           (match eval_expr expr env with
            | Ok value -> Ok (acc @ [value])
            | Error err -> Error err))
      (Ok [])
      exprs

  and apply callee args =
    match callee with
    | VBuiltin fn -> fn args
    | VLambda { params; body; env } -> apply_lambda params body env args
    | VNumber _ -> Error "数値を関数として適用できません"

  and apply_lambda params body lambda_env args =
    if List.length params <> List.length args then
      Error "引数の数が一致しません"
    else
      let new_env = List.combine params args @ lambda_env in
      eval_expr body new_env

  (* 組み込み数値演算 *)
  let builtin_numeric op =
    fun args ->
      match args with
      | [VNumber lhs; VNumber rhs] -> Ok (VNumber (op lhs rhs))
      | _ -> Error "数値演算は2引数の数値のみ対応します"

  (* デフォルト環境 *)
  let default_env () =
    [
      ("+", VBuiltin (builtin_numeric ( +. )));
      ("-", VBuiltin (builtin_numeric ( -. )));
      ("*", VBuiltin (builtin_numeric ( *. )));
      ("/", VBuiltin (builtin_numeric ( /. )));
    ]

  (* メイン評価関数 *)
  let eval source =
    let tokens = tokenize source in
    match parse_expr tokens with
    | Error EmptyInput -> Error "入力が空です"
    | Error UnmatchedParen -> Error "括弧が一致しません"
    | Error (UnexpectedToken token) -> Error ("予期しないトークン: " ^ token)
    | Ok (expr, rest) ->
      if rest = [] then
        eval_expr expr (default_env ())
      else
        Error "末尾に未消費トークンがあります"
end

(* 利用例 *)
(* MiniLisp.eval "(+ 40 2)" => Ok (VNumber 42.0) *)