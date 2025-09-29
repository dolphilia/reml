type expr =
  | Number of float
  | Symbol of string
  | List of expr list

type value =
  | VNumber of float
  | VLambda of { params : string list; body : expr; env : env }
  | VBuiltin of (value list -> value)

and env = (string, value) Hashtbl.t

exception Error of string

let tokenize source =
  let buffer = Buffer.create (String.length source) in
  String.iter
    (fun c ->
      match c with
      | '(' | ')' -> Buffer.add_char buffer ' '; Buffer.add_char buffer c; Buffer.add_char buffer ' '
      | _ -> Buffer.add_char buffer c)
    source;
  buffer |> Buffer.contents |> String.split_on_char ' ' |> List.filter (fun token -> token <> "")

let rec parse tokens index =
  if index >= Array.length tokens then raise (Error "入力が空です")
  else
    match tokens.(index) with
    | "(" -> parse_list tokens (index + 1)
    | ")" -> raise (Error "対応しない ")")
    | token ->
        (match float_of_string_opt token with
        | Some n -> Number n, index + 1
        | None -> Symbol token, index + 1)

and parse_list tokens index =
  let rec aux acc i =
    if i >= Array.length tokens then raise (Error "リストが閉じていません")
    else if tokens.(i) = ")" then List (List.rev acc), i + 1
    else
      let expr, next = parse tokens i in
      aux (expr :: acc) next
  in
  aux [] index

let rec eval env = function
  | Number n -> VNumber n
  | Symbol name -> (try Hashtbl.find env name with Not_found -> raise (Error ("未定義: " ^ name)))
  | List [] -> raise (Error "空の式")
  | List (head :: tail) ->
      let callee = eval env head in
      let args = List.map (eval env) tail in
      apply callee args

and apply callee args =
  match callee with
  | VBuiltin fn -> fn args
  | VLambda { params; body; env } ->
      if List.length params <> List.length args then raise (Error "引数の数が一致しません")
      else
        let local = Hashtbl.copy env in
        List.iter2 (Hashtbl.replace local) params args;
        eval local body
  | VNumber _ -> raise (Error "数値は適用できません")

let numeric op =
  VBuiltin (function
    | [ VNumber lhs; VNumber rhs ] -> VNumber (op lhs rhs)
    | _ -> raise (Error "数値以外を演算できません"))

let default_env () =
  let env = Hashtbl.create 16 in
  Hashtbl.add env "+" (numeric ( +. ));
  Hashtbl.add env "-" (numeric ( -. ));
  Hashtbl.add env "*" (numeric ( *. ));
  Hashtbl.add env "/" (numeric ( /. ));
  env

let eval_source source =
  let tokens = tokenize source |> Array.of_list in
  let expr, index = parse tokens 0 in
  if index <> Array.length tokens then raise (Error "未消費トークンがあります")
  else
    let env = default_env () in
    eval env expr
