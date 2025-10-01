module BasicInterpreter exposing (..)

-- Elm Basic Interpreter

import Dict exposing (Dict)
import Array exposing (Array)

type Value
    = VNumber Float
    | VString String
    | VArray (Array Value)

type alias Env = Dict String Value

type BinOperator
    = Add | Sub | Mul | Div
    | Eq | Ne | Lt | Le | Gt | Ge
    | And | Or

type UnaryOperator
    = Neg | Not

type Expr
    = Number Float
    | String String
    | Variable String
    | ArrayAccess { var : String, index : Expr }
    | BinOp { op : BinOperator, left : Expr, right : Expr }
    | UnaryOp { op : UnaryOperator, operand : Expr }

type Statement
    = Let { var : String, expr : Expr }
    | Print (List Expr)
    | If { cond : Expr, thenBlock : List Statement, elseBlock : List Statement }
    | For { var : String, start : Expr, end : Expr, step : Expr, body : List Statement }
    | While { cond : Expr, body : List Statement }
    | Goto Int
    | Gosub Int
    | Return
    | Dim { var : String, size : Expr }
    | End

type alias Program = List (Int, Statement)

type alias RuntimeState =
    { env : Env
    , callStack : List Int
    , output : List String
    }

type RuntimeError
    = UndefinedVariable String
    | UndefinedLabel Int
    | TypeMismatch { expected : String, got : String }
    | IndexOutOfBounds
    | DivisionByZero
    | StackUnderflow

-- Utility functions

isTruthy : Value -> Bool
isTruthy value =
    case value of
        VNumber n -> n /= 0.0
        VString s -> not (String.isEmpty s)
        VArray a -> Array.length a > 0

valueToString : Value -> String
valueToString value =
    case value of
        VNumber n -> String.fromFloat n
        VString s -> s
        VArray _ -> "[Array]"

findLine : Program -> Int -> Result RuntimeError Int
findLine program target =
    program
        |> List.indexedMap (\idx (line, _) -> if line == target then Just idx else Nothing)
        |> List.filterMap identity
        |> List.head
        |> Result.fromMaybe (UndefinedLabel target)

-- Expression evaluation

evalExpr : Expr -> Env -> Result RuntimeError Value
evalExpr expr env =
    case expr of
        Number n ->
            Ok (VNumber n)

        String s ->
            Ok (VString s)

        Variable name ->
            Dict.get name env
                |> Result.fromMaybe (UndefinedVariable name)

        ArrayAccess { var, index } ->
            case Dict.get var env of
                Nothing ->
                    Err (UndefinedVariable var)

                Just (VArray arr) ->
                    evalExpr index env
                        |> Result.andThen (\idxVal ->
                            case idxVal of
                                VNumber idx ->
                                    Array.get (floor idx) arr
                                        |> Result.fromMaybe IndexOutOfBounds

                                _ ->
                                    Err (TypeMismatch { expected = "Number", got = "Other" })
                        )

                Just _ ->
                    Err (TypeMismatch { expected = "Array", got = "Other" })

        BinOp { op, left, right } ->
            Result.map2 (evalBinOp op) (evalExpr left env) (evalExpr right env)
                |> Result.andThen identity

        UnaryOp { op, operand } ->
            evalExpr operand env
                |> Result.andThen (evalUnaryOp op)

evalBinOp : BinOperator -> Value -> Value -> Result RuntimeError Value
evalBinOp op l r =
    case (op, l, r) of
        (Add, VNumber lv, VNumber rv) ->
            Ok (VNumber (lv + rv))

        (Sub, VNumber lv, VNumber rv) ->
            Ok (VNumber (lv - rv))

        (Mul, VNumber lv, VNumber rv) ->
            Ok (VNumber (lv * rv))

        (Div, VNumber _, VNumber rv) ->
            if rv == 0.0 then
                Err DivisionByZero
            else
                Ok (VNumber (l / rv))

        (Eq, VNumber lv, VNumber rv) ->
            Ok (VNumber (if lv == rv then 1.0 else 0.0))

        (Ne, VNumber lv, VNumber rv) ->
            Ok (VNumber (if lv /= rv then 1.0 else 0.0))

        (Lt, VNumber lv, VNumber rv) ->
            Ok (VNumber (if lv < rv then 1.0 else 0.0))

        (Le, VNumber lv, VNumber rv) ->
            Ok (VNumber (if lv <= rv then 1.0 else 0.0))

        (Gt, VNumber lv, VNumber rv) ->
            Ok (VNumber (if lv > rv then 1.0 else 0.0))

        (Ge, VNumber lv, VNumber rv) ->
            Ok (VNumber (if lv >= rv then 1.0 else 0.0))

        (And, lv, rv) ->
            Ok (VNumber (if isTruthy lv && isTruthy rv then 1.0 else 0.0))

        (Or, lv, rv) ->
            Ok (VNumber (if isTruthy lv || isTruthy rv then 1.0 else 0.0))

        _ ->
            Err (TypeMismatch { expected = "Number", got = "Other" })

evalUnaryOp : UnaryOperator -> Value -> Result RuntimeError Value
evalUnaryOp op operand =
    case (op, operand) of
        (Neg, VNumber n) ->
            Ok (VNumber (-n))

        (Not, v) ->
            Ok (VNumber (if isTruthy v then 0.0 else 1.0))

        _ ->
            Err (TypeMismatch { expected = "Number", got = "Other" })

-- Statement execution

executeBlock : List Statement -> RuntimeState -> Result RuntimeError RuntimeState
executeBlock stmts state =
    case stmts of
        [] ->
            Ok state

        stmt :: rest ->
            executeSingleStatement stmt state
                |> Result.andThen (executeBlock rest)

executeSingleStatement : Statement -> RuntimeState -> Result RuntimeError RuntimeState
executeSingleStatement stmt state =
    case stmt of
        Let { var, expr } ->
            evalExpr expr state.env
                |> Result.map (\value -> { state | env = Dict.insert var value state.env })

        Print exprs ->
            evalAll exprs state.env
                |> Result.map (\values ->
                    let
                        text = values |> List.map valueToString |> String.join " "
                    in
                    { state | output = state.output ++ [text] }
                )

        _ ->
            Ok state

evalAll : List Expr -> Env -> Result RuntimeError (List Value)
evalAll exprs env =
    case exprs of
        [] ->
            Ok []

        e :: rest ->
            Result.map2 (::) (evalExpr e env) (evalAll rest env)

executeForLoop : String -> Float -> Float -> Float -> List Statement -> Program -> Int -> RuntimeState -> Result RuntimeError (List String)
executeForLoop var current end step body program pc state =
    if (step > 0.0 && current > end) || (step < 0.0 && current < end) then
        executeProgram program (pc + 1) state
    else
        let
            newEnv = Dict.insert var (VNumber current) state.env
        in
        executeBlock body { state | env = newEnv }
            |> Result.andThen (executeForLoop var (current + step) end step body program pc)

executeWhileLoop : Expr -> List Statement -> Program -> Int -> RuntimeState -> Result RuntimeError (List String)
executeWhileLoop cond body program pc state =
    evalExpr cond state.env
        |> Result.andThen (\condVal ->
            if isTruthy condVal then
                executeBlock body state
                    |> Result.andThen (executeWhileLoop cond body program pc)
            else
                executeProgram program (pc + 1) state
        )

executeProgram : Program -> Int -> RuntimeState -> Result RuntimeError (List String)
executeProgram program pc state =
    case List.drop pc program |> List.head of
        Nothing ->
            Ok state.output

        Just (_, stmt) ->
            case stmt of
                End ->
                    Ok state.output

                Let { var, expr } ->
                    evalExpr expr state.env
                        |> Result.andThen (\value ->
                            let
                                newEnv = Dict.insert var value state.env
                            in
                            executeProgram program (pc + 1) { state | env = newEnv }
                        )

                Print exprs ->
                    evalAll exprs state.env
                        |> Result.andThen (\values ->
                            let
                                text = values |> List.map valueToString |> String.join " "
                                newOutput = state.output ++ [text]
                            in
                            executeProgram program (pc + 1) { state | output = newOutput }
                        )

                If { cond, thenBlock, elseBlock } ->
                    evalExpr cond state.env
                        |> Result.andThen (\condVal ->
                            let
                                branch = if isTruthy condVal then thenBlock else elseBlock
                            in
                            executeBlock branch state
                                |> Result.andThen (\newState -> executeProgram program (pc + 1) newState)
                        )

                For { var, start, end, step, body } ->
                    Result.map3 (\s e st -> (s, e, st))
                        (evalExpr start state.env)
                        (evalExpr end state.env)
                        (evalExpr step state.env)
                        |> Result.andThen (\(s, e, st) ->
                            case (s, e, st) of
                                (VNumber sv, VNumber ev, VNumber stv) ->
                                    executeForLoop var sv ev stv body program pc state

                                _ ->
                                    Err (TypeMismatch { expected = "Number", got = "Other" })
                        )

                While { cond, body } ->
                    executeWhileLoop cond body program pc state

                Goto target ->
                    findLine program target
                        |> Result.andThen (\newPc -> executeProgram program newPc state)

                Gosub target ->
                    findLine program target
                        |> Result.andThen (\newPc ->
                            let
                                newCallStack = state.callStack ++ [pc + 1]
                            in
                            executeProgram program newPc { state | callStack = newCallStack }
                        )

                Return ->
                    case List.reverse state.callStack of
                        [] ->
                            Err StackUnderflow

                        returnPc :: rest ->
                            let
                                newCallStack = List.reverse rest
                            in
                            executeProgram program returnPc { state | callStack = newCallStack }

                Dim { var, size } ->
                    evalExpr size state.env
                        |> Result.andThen (\sizeVal ->
                            case sizeVal of
                                VNumber n ->
                                    let
                                        array = Array.repeat (floor n) (VNumber 0.0)
                                        newEnv = Dict.insert var (VArray array) state.env
                                    in
                                    executeProgram program (pc + 1) { state | env = newEnv }

                                _ ->
                                    Err (TypeMismatch { expected = "Number", got = "Other" })
                        )

-- Main entry point

run : Program -> Result RuntimeError (List String)
run program =
    let
        initialState =
            { env = Dict.empty
            , callStack = []
            , output = []
            }

        sorted = List.sortBy Tuple.first program
    in
    executeProgram sorted 0 initialState
