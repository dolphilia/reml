(* OCaml 5 Basic Interpreter *)

type value =
  | VNumber of float
  | VString of string
  | VArray of value array

module Env = Map.Make(String)

type env = value Env.t

type bin_operator =
  | Add | Sub | Mul | Div
  | Eq | Ne | Lt | Le | Gt | Ge
  | And | Or

type unary_operator =
  | Neg | Not

type expr =
  | Number of float
  | String of string
  | Variable of string
  | ArrayAccess of { var: string; index: expr }
  | BinOp of { op: bin_operator; left: expr; right: expr }
  | UnaryOp of { op: unary_operator; operand: expr }

type statement =
  | Let of { var: string; expr: expr }
  | Print of expr list
  | If of { cond: expr; then_block: statement list; else_block: statement list }
  | For of { var: string; start: expr; end_: expr; step: expr; body: statement list }
  | While of { cond: expr; body: statement list }
  | Goto of int
  | Gosub of int
  | Return
  | Dim of { var: string; size: expr }
  | End

type program = (int * statement) list

type runtime_state = {
  env: env;
  call_stack: int list;
  output: string list;
}

type runtime_error =
  | UndefinedVariable of string
  | UndefinedLabel of int
  | TypeMismatch of { expected: string; got: string }
  | IndexOutOfBounds
  | DivisionByZero
  | StackUnderflow

(* Utility functions *)

let is_truthy = function
  | VNumber n -> n <> 0.0
  | VString s -> s <> ""
  | VArray a -> Array.length a > 0

let value_to_string = function
  | VNumber n -> string_of_float n
  | VString s -> s
  | VArray _ -> "[Array]"

let find_line program target =
  let rec find_idx idx = function
    | [] -> Error (UndefinedLabel target)
    | (line, _) :: _ when line = target -> Ok idx
    | _ :: rest -> find_idx (idx + 1) rest
  in
  find_idx 0 program

(* Expression evaluation *)

let rec eval_expr expr env =
  match expr with
  | Number n -> Ok (VNumber n)
  | String s -> Ok (VString s)
  | Variable name ->
      (match Env.find_opt name env with
       | Some v -> Ok v
       | None -> Error (UndefinedVariable name))

  | ArrayAccess { var; index } ->
      (match Env.find_opt var env with
       | None -> Error (UndefinedVariable var)
       | Some (VArray arr) ->
           (match eval_expr index env with
            | Error e -> Error e
            | Ok (VNumber idx) ->
                let i = int_of_float idx in
                if i >= 0 && i < Array.length arr then
                  Ok arr.(i)
                else
                  Error IndexOutOfBounds
            | Ok _ -> Error (TypeMismatch { expected = "Number"; got = "Other" }))
       | Some _ -> Error (TypeMismatch { expected = "Array"; got = "Other" }))

  | BinOp { op; left; right } ->
      (match eval_expr left env, eval_expr right env with
       | Ok l_val, Ok r_val -> eval_binop op l_val r_val
       | Error e, _ | _, Error e -> Error e)

  | UnaryOp { op; operand } ->
      (match eval_expr operand env with
       | Ok v -> eval_unaryop op v
       | Error e -> Error e)

and eval_binop op l r =
  match (op, l, r) with
  | (Add, VNumber l, VNumber r) -> Ok (VNumber (l +. r))
  | (Sub, VNumber l, VNumber r) -> Ok (VNumber (l -. r))
  | (Mul, VNumber l, VNumber r) -> Ok (VNumber (l *. r))
  | (Div, VNumber _, VNumber r) when r = 0.0 -> Error DivisionByZero
  | (Div, VNumber l, VNumber r) -> Ok (VNumber (l /. r))
  | (Eq, VNumber l, VNumber r) -> Ok (VNumber (if l = r then 1.0 else 0.0))
  | (Ne, VNumber l, VNumber r) -> Ok (VNumber (if l <> r then 1.0 else 0.0))
  | (Lt, VNumber l, VNumber r) -> Ok (VNumber (if l < r then 1.0 else 0.0))
  | (Le, VNumber l, VNumber r) -> Ok (VNumber (if l <= r then 1.0 else 0.0))
  | (Gt, VNumber l, VNumber r) -> Ok (VNumber (if l > r then 1.0 else 0.0))
  | (Ge, VNumber l, VNumber r) -> Ok (VNumber (if l >= r then 1.0 else 0.0))
  | (And, l, r) -> Ok (VNumber (if is_truthy l && is_truthy r then 1.0 else 0.0))
  | (Or, l, r) -> Ok (VNumber (if is_truthy l || is_truthy r then 1.0 else 0.0))
  | _ -> Error (TypeMismatch { expected = "Number"; got = "Other" })

and eval_unaryop op operand =
  match (op, operand) with
  | (Neg, VNumber n) -> Ok (VNumber (~-. n))
  | (Not, v) -> Ok (VNumber (if is_truthy v then 0.0 else 1.0))
  | _ -> Error (TypeMismatch { expected = "Number"; got = "Other" })

(* Statement execution *)

let rec execute_block stmts state =
  match stmts with
  | [] -> Ok state
  | stmt :: rest ->
      match execute_single_statement stmt state with
      | Ok new_state -> execute_block rest new_state
      | Error e -> Error e

and execute_single_statement stmt state =
  match stmt with
  | Let { var; expr } ->
      (match eval_expr expr state.env with
       | Ok value ->
           Ok { state with env = Env.add var value state.env }
       | Error e -> Error e)

  | Print exprs ->
      let rec eval_all = function
        | [] -> Ok []
        | e :: rest ->
            (match eval_expr e state.env, eval_all rest with
             | Ok v, Ok vs -> Ok (v :: vs)
             | Error e, _ | _, Error e -> Error e)
      in
      (match eval_all exprs with
       | Ok values ->
           let text = String.concat " " (List.map value_to_string values) in
           Ok { state with output = state.output @ [text] }
       | Error e -> Error e)

  | _ -> Ok state

let rec execute_for_loop var current end_ step body program pc state =
  if (step > 0.0 && current > end_) || (step < 0.0 && current < end_) then
    execute_program program (pc + 1) state
  else
    let new_env = Env.add var (VNumber current) state.env in
    match execute_block body { state with env = new_env } with
    | Ok new_state -> execute_for_loop var (current +. step) end_ step body program pc new_state
    | Error e -> Error e

and execute_while_loop cond body program pc state =
  match eval_expr cond state.env with
  | Error e -> Error e
  | Ok cond_val ->
      if is_truthy cond_val then
        match execute_block body state with
        | Ok new_state -> execute_while_loop cond body program pc new_state
        | Error e -> Error e
      else
        execute_program program (pc + 1) state

and execute_program program pc state =
  if pc >= List.length program then
    Ok state.output
  else
    let (_, stmt) = List.nth program pc in
    match stmt with
    | End -> Ok state.output

    | Let { var; expr } ->
        (match eval_expr expr state.env with
         | Ok value ->
             let new_env = Env.add var value state.env in
             execute_program program (pc + 1) { state with env = new_env }
         | Error e -> Error e)

    | Print exprs ->
        let rec eval_all = function
          | [] -> Ok []
          | e :: rest ->
              (match eval_expr e state.env, eval_all rest with
               | Ok v, Ok vs -> Ok (v :: vs)
               | Error e, _ | _, Error e -> Error e)
        in
        (match eval_all exprs with
         | Ok values ->
             let text = String.concat " " (List.map value_to_string values) in
             let new_output = state.output @ [text] in
             execute_program program (pc + 1) { state with output = new_output }
         | Error e -> Error e)

    | If { cond; then_block; else_block } ->
        (match eval_expr cond state.env with
         | Ok cond_val ->
             let branch = if is_truthy cond_val then then_block else else_block in
             (match execute_block branch state with
              | Ok new_state -> execute_program program (pc + 1) new_state
              | Error e -> Error e)
         | Error e -> Error e)

    | For { var; start; end_; step; body } ->
        (match eval_expr start state.env, eval_expr end_ state.env, eval_expr step state.env with
         | Ok (VNumber s), Ok (VNumber e), Ok (VNumber st) ->
             execute_for_loop var s e st body program pc state
         | Error e, _, _ | _, Error e, _ | _, _, Error e -> Error e
         | _ -> Error (TypeMismatch { expected = "Number"; got = "Other" }))

    | While { cond; body } ->
        execute_while_loop cond body program pc state

    | Goto target ->
        (match find_line program target with
         | Ok new_pc -> execute_program program new_pc state
         | Error e -> Error e)

    | Gosub target ->
        (match find_line program target with
         | Ok new_pc ->
             let new_call_stack = state.call_stack @ [pc + 1] in
             execute_program program new_pc { state with call_stack = new_call_stack }
         | Error e -> Error e)

    | Return ->
        (match List.rev state.call_stack with
         | [] -> Error StackUnderflow
         | return_pc :: rest ->
             let new_call_stack = List.rev rest in
             execute_program program return_pc { state with call_stack = new_call_stack })

    | Dim { var; size } ->
        (match eval_expr size state.env with
         | Ok (VNumber n) ->
             let array = Array.make (int_of_float n) (VNumber 0.0) in
             let new_env = Env.add var (VArray array) state.env in
             execute_program program (pc + 1) { state with env = new_env }
         | Error e -> Error e
         | Ok _ -> Error (TypeMismatch { expected = "Number"; got = "Other" }))

(* Main entry point *)

let run program =
  let initial_state = {
    env = Env.empty;
    call_stack = [];
    output = [];
  } in
  let sorted = List.sort (fun (a, _) (b, _) -> compare a b) program in
  execute_program sorted 0 initial_state
