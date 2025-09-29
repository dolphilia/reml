type op = Add | Sub | Mul | Div

type expr =
  | Number of int
  | Var of string
  | Binary of op * expr * expr

type stmt =
  | Assign of string * expr
  | While of expr * stmt list
  | Write of expr

exception Exec_error of string

type runtime = { vars : (string, int) Hashtbl.t; output : int Queue.t }

let create_runtime () = { vars = Hashtbl.create 16; output = Queue.create () }

let rec eval_expr rt = function
  | Number n -> n
  | Var name -> (try Hashtbl.find rt.vars name with Not_found -> raise (Exec_error ("未定義変数: " ^ name)))
  | Binary (op, lhs, rhs) ->
      let l = eval_expr rt lhs in
      let r = eval_expr rt rhs in
      match op with
      | Add -> l + r
      | Sub -> l - r
      | Mul -> l * r
      | Div -> if r = 0 then raise (Exec_error "0 で割れません") else l / r

let rec exec_stmt rt = function
  | Assign (name, expr) ->
      let value = eval_expr rt expr in
      Hashtbl.replace rt.vars name value
  | Write expr ->
      let value = eval_expr rt expr in
      Queue.add value rt.output
  | While (cond, body) ->
      while eval_expr rt cond <> 0 do
        List.iter (exec_stmt rt) body
      done

let exec program =
  let rt = create_runtime () in
  List.iter (exec_stmt rt) program;
  rt
