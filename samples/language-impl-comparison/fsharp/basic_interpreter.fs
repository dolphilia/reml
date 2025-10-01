// F# Basic Interpreter

module BasicInterpreter

open System.Collections.Generic

type Value =
    | VNumber of float
    | VString of string
    | VArray of Value[]

type Env = Dictionary<string, Value>

type BinOperator =
    | Add | Sub | Mul | Div
    | Eq | Ne | Lt | Le | Gt | Ge
    | And | Or

type UnaryOperator =
    | Neg | Not

type Expr =
    | Number of float
    | String of string
    | Variable of string
    | ArrayAccess of var: string * index: Expr
    | BinOp of op: BinOperator * left: Expr * right: Expr
    | UnaryOp of op: UnaryOperator * operand: Expr

type Statement =
    | Let of var: string * expr: Expr
    | Print of Expr list
    | If of cond: Expr * thenBlock: Statement list * elseBlock: Statement list
    | For of var: string * start: Expr * end_: Expr * step: Expr * body: Statement list
    | While of cond: Expr * body: Statement list
    | Goto of int
    | Gosub of int
    | Return
    | Dim of var: string * size: Expr
    | End

type Program = (int * Statement) list

type RuntimeState = {
    Env: Env
    CallStack: int list
    Output: string list
}

type RuntimeError =
    | UndefinedVariable of string
    | UndefinedLabel of int
    | TypeMismatch of expected: string * got: string
    | IndexOutOfBounds
    | DivisionByZero
    | StackUnderflow

// Utility functions

let isTruthy = function
    | VNumber n -> n <> 0.0
    | VString s -> s <> ""
    | VArray a -> a.Length > 0

let valueToString = function
    | VNumber n -> string n
    | VString s -> s
    | VArray _ -> "[Array]"

let findLine (program: Program) target =
    program
    |> List.tryFindIndex (fun (line, _) -> line = target)
    |> Option.map Ok
    |> Option.defaultValue (Error (UndefinedLabel target))

// Expression evaluation

let rec evalExpr expr (env: Env) =
    match expr with
    | Number n -> Ok (VNumber n)
    | String s -> Ok (VString s)
    | Variable name ->
        match env.TryGetValue(name) with
        | true, v -> Ok v
        | false, _ -> Error (UndefinedVariable name)

    | ArrayAccess (var, index) ->
        match env.TryGetValue(var) with
        | false, _ -> Error (UndefinedVariable var)
        | true, VArray arr ->
            match evalExpr index env with
            | Error e -> Error e
            | Ok (VNumber idx) ->
                let i = int idx
                if i >= 0 && i < arr.Length then
                    Ok arr.[i]
                else
                    Error IndexOutOfBounds
            | Ok _ -> Error (TypeMismatch ("Number", "Other"))
        | true, _ -> Error (TypeMismatch ("Array", "Other"))

    | BinOp (op, left, right) ->
        match evalExpr left env, evalExpr right env with
        | Ok lVal, Ok rVal -> evalBinOp op lVal rVal
        | Error e, _ | _, Error e -> Error e

    | UnaryOp (op, operand) ->
        match evalExpr operand env with
        | Ok v -> evalUnaryOp op v
        | Error e -> Error e

and evalBinOp op l r =
    match (op, l, r) with
    | (Add, VNumber l, VNumber r) -> Ok (VNumber (l + r))
    | (Sub, VNumber l, VNumber r) -> Ok (VNumber (l - r))
    | (Mul, VNumber l, VNumber r) -> Ok (VNumber (l * r))
    | (Div, VNumber _, VNumber r) when r = 0.0 -> Error DivisionByZero
    | (Div, VNumber l, VNumber r) -> Ok (VNumber (l / r))
    | (Eq, VNumber l, VNumber r) -> Ok (VNumber (if l = r then 1.0 else 0.0))
    | (Ne, VNumber l, VNumber r) -> Ok (VNumber (if l <> r then 1.0 else 0.0))
    | (Lt, VNumber l, VNumber r) -> Ok (VNumber (if l < r then 1.0 else 0.0))
    | (Le, VNumber l, VNumber r) -> Ok (VNumber (if l <= r then 1.0 else 0.0))
    | (Gt, VNumber l, VNumber r) -> Ok (VNumber (if l > r then 1.0 else 0.0))
    | (Ge, VNumber l, VNumber r) -> Ok (VNumber (if l >= r then 1.0 else 0.0))
    | (And, l, r) -> Ok (VNumber (if isTruthy l && isTruthy r then 1.0 else 0.0))
    | (Or, l, r) -> Ok (VNumber (if isTruthy l || isTruthy r then 1.0 else 0.0))
    | _ -> Error (TypeMismatch ("Number", "Other"))

and evalUnaryOp op operand =
    match (op, operand) with
    | (Neg, VNumber n) -> Ok (VNumber (-n))
    | (Not, v) -> Ok (VNumber (if isTruthy v then 0.0 else 1.0))
    | _ -> Error (TypeMismatch ("Number", "Other"))

// Statement execution

let rec executeBlock stmts state =
    match stmts with
    | [] -> Ok state
    | stmt :: rest ->
        match executeSingleStatement stmt state with
        | Ok newState -> executeBlock rest newState
        | Error e -> Error e

and executeSingleStatement stmt state =
    match stmt with
    | Let (var, expr) ->
        match evalExpr expr state.Env with
        | Ok value ->
            state.Env.[var] <- value
            Ok state
        | Error e -> Error e

    | Print exprs ->
        let rec evalAll = function
            | [] -> Ok []
            | e :: rest ->
                match evalExpr e state.Env, evalAll rest with
                | Ok v, Ok vs -> Ok (v :: vs)
                | Error e, _ | _, Error e -> Error e

        match evalAll exprs with
        | Ok values ->
            let text = values |> List.map valueToString |> String.concat " "
            Ok { state with Output = state.Output @ [text] }
        | Error e -> Error e

    | _ -> Ok state

let rec executeForLoop var current end_ step body program pc state =
    if (step > 0.0 && current > end_) || (step < 0.0 && current < end_) then
        executeProgram program (pc + 1) state
    else
        state.Env.[var] <- VNumber current
        match executeBlock body state with
        | Ok newState -> executeForLoop var (current + step) end_ step body program pc newState
        | Error e -> Error e

and executeWhileLoop cond body program pc state =
    match evalExpr cond state.Env with
    | Error e -> Error e
    | Ok condVal ->
        if isTruthy condVal then
            match executeBlock body state with
            | Ok newState -> executeWhileLoop cond body program pc newState
            | Error e -> Error e
        else
            executeProgram program (pc + 1) state

and executeProgram (program: Program) pc state =
    if pc >= program.Length then
        Ok state.Output
    else
        let (_, stmt) = program.[pc]
        match stmt with
        | End -> Ok state.Output

        | Let (var, expr) ->
            match evalExpr expr state.Env with
            | Ok value ->
                state.Env.[var] <- value
                executeProgram program (pc + 1) state
            | Error e -> Error e

        | Print exprs ->
            let rec evalAll = function
                | [] -> Ok []
                | e :: rest ->
                    match evalExpr e state.Env, evalAll rest with
                    | Ok v, Ok vs -> Ok (v :: vs)
                    | Error e, _ | _, Error e -> Error e

            match evalAll exprs with
            | Ok values ->
                let text = values |> List.map valueToString |> String.concat " "
                let newOutput = state.Output @ [text]
                executeProgram program (pc + 1) { state with Output = newOutput }
            | Error e -> Error e

        | If (cond, thenBlock, elseBlock) ->
            match evalExpr cond state.Env with
            | Ok condVal ->
                let branch = if isTruthy condVal then thenBlock else elseBlock
                match executeBlock branch state with
                | Ok newState -> executeProgram program (pc + 1) newState
                | Error e -> Error e
            | Error e -> Error e

        | For (var, start, end_, step, body) ->
            match evalExpr start state.Env, evalExpr end_ state.Env, evalExpr step state.Env with
            | Ok (VNumber s), Ok (VNumber e), Ok (VNumber st) ->
                executeForLoop var s e st body program pc state
            | Error e, _, _ | _, Error e, _ | _, _, Error e -> Error e
            | _ -> Error (TypeMismatch ("Number", "Other"))

        | While (cond, body) ->
            executeWhileLoop cond body program pc state

        | Goto target ->
            match findLine program target with
            | Ok newPc -> executeProgram program newPc state
            | Error e -> Error e

        | Gosub target ->
            match findLine program target with
            | Ok newPc ->
                let newCallStack = state.CallStack @ [pc + 1]
                executeProgram program newPc { state with CallStack = newCallStack }
            | Error e -> Error e

        | Return ->
            match List.rev state.CallStack with
            | [] -> Error StackUnderflow
            | returnPc :: rest ->
                let newCallStack = List.rev rest
                executeProgram program returnPc { state with CallStack = newCallStack }

        | Dim (var, size) ->
            match evalExpr size state.Env with
            | Ok (VNumber n) ->
                let array = Array.create (int n) (VNumber 0.0)
                state.Env.[var] <- VArray array
                executeProgram program (pc + 1) state
            | Error e -> Error e
            | Ok _ -> Error (TypeMismatch ("Number", "Other"))

// Main entry point

let run (program: Program) =
    let initialState = {
        Env = Dictionary<string, Value>()
        CallStack = []
        Output = []
    }
    let sorted = program |> List.sortBy fst
    executeProgram sorted 0 initialState
