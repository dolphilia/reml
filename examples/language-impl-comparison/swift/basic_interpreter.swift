import Foundation

// MARK: - Value型
enum Value: Equatable {
    case number(Double)
    case string(String)
    case array([Value])
}

// MARK: - 演算子
enum BinOperator {
    case add, sub, mul, div
    case eq, ne, lt, le, gt, ge
    case and, or
}

enum UnaryOperator {
    case neg, not
}

// MARK: - 式
indirect enum Expr {
    case number(Double)
    case string(String)
    case variable(String)
    case arrayAccess(var: String, index: Expr)
    case binOp(op: BinOperator, left: Expr, right: Expr)
    case unaryOp(op: UnaryOperator, operand: Expr)
}

// MARK: - 文
indirect enum Statement {
    case let_(var: String, expr: Expr)
    case print([Expr])
    case if_(cond: Expr, thenBlock: [Statement], elseBlock: [Statement])
    case for_(var: String, start: Expr, end: Expr, step: Expr, body: [Statement])
    case while_(cond: Expr, body: [Statement])
    case goto(Int)
    case gosub(Int)
    case return_
    case dim(var: String, size: Expr)
    case end
}

// MARK: - プログラム型
typealias Program = [(line: Int, stmt: Statement)]

// MARK: - ランタイム状態
struct RuntimeState {
    var env: [String: Value]
    var callStack: [Int]
    var output: [String]
}

// MARK: - ランタイムエラー
enum RuntimeError: Error, CustomStringConvertible {
    case undefinedVariable(String)
    case undefinedLabel(Int)
    case typeMismatch(expected: String, got: String)
    case indexOutOfBounds
    case divisionByZero
    case stackUnderflow

    var description: String {
        switch self {
        case .undefinedVariable(let name):
            return "未定義変数: \(name)"
        case .undefinedLabel(let line):
            return "未定義ラベル: \(line)"
        case .typeMismatch(let expected, let got):
            return "型不一致: 期待 \(expected), 実際 \(got)"
        case .indexOutOfBounds:
            return "インデックス範囲外"
        case .divisionByZero:
            return "0で割ることはできません"
        case .stackUnderflow:
            return "スタックアンダーフロー"
        }
    }
}

// MARK: - インタープリタ実行
func run(program: Program) -> Result<[String], RuntimeError> {
    let state = RuntimeState(env: [:], callStack: [], output: [])
    let sorted = program.sorted { $0.line < $1.line }
    return executeProgram(sorted, pc: 0, state: state)
}

func executeProgram(_ program: Program, pc: Int, state: RuntimeState) -> Result<[String], RuntimeError> {
    guard pc < program.count else {
        return .success(state.output)
    }

    var newState = state
    let stmt = program[pc].stmt

    switch stmt {
    case .end:
        return .success(state.output)

    case .let_(let varName, let expr):
        switch evalExpr(expr, env: state.env) {
        case .success(let value):
            newState.env[varName] = value
            return executeProgram(program, pc: pc + 1, state: newState)
        case .failure(let error):
            return .failure(error)
        }

    case .print(let exprs):
        var parts: [String] = []
        for expr in exprs {
            switch evalExpr(expr, env: state.env) {
            case .success(let value):
                parts.append(valueToString(value))
            case .failure(let error):
                return .failure(error)
            }
        }
        newState.output.append(parts.joined(separator: " "))
        return executeProgram(program, pc: pc + 1, state: newState)

    case .if_(let cond, let thenBlock, let elseBlock):
        switch evalExpr(cond, env: state.env) {
        case .success(let condVal):
            let branch = isTruthy(condVal) ? thenBlock : elseBlock
            switch executeBlock(branch, state: state) {
            case .success(let blockState):
                return executeProgram(program, pc: pc + 1, state: blockState)
            case .failure(let error):
                return .failure(error)
            }
        case .failure(let error):
            return .failure(error)
        }

    case .for_(let varName, let start, let end, let step, let body):
        switch (evalExpr(start, env: state.env), evalExpr(end, env: state.env), evalExpr(step, env: state.env)) {
        case (.success(.number(let s)), .success(.number(let e)), .success(.number(let st))):
            return executeForLoop(var: varName, current: s, end: e, step: st, body: body, program: program, pc: pc, state: state)
        case (.failure(let error), _, _), (_, .failure(let error), _), (_, _, .failure(let error)):
            return .failure(error)
        default:
            return .failure(.typeMismatch(expected: "Number", got: "Other"))
        }

    case .while_(let cond, let body):
        return executeWhileLoop(cond: cond, body: body, program: program, pc: pc, state: state)

    case .goto(let target):
        switch findLine(program, target: target) {
        case .success(let newPc):
            return executeProgram(program, pc: newPc, state: state)
        case .failure(let error):
            return .failure(error)
        }

    case .gosub(let target):
        switch findLine(program, target: target) {
        case .success(let newPc):
            newState.callStack.append(pc + 1)
            return executeProgram(program, pc: newPc, state: newState)
        case .failure(let error):
            return .failure(error)
        }

    case .return_:
        guard let returnPc = state.callStack.last else {
            return .failure(.stackUnderflow)
        }
        newState.callStack.removeLast()
        return executeProgram(program, pc: returnPc, state: newState)

    case .dim(let varName, let sizeExpr):
        switch evalExpr(sizeExpr, env: state.env) {
        case .success(.number(let size)):
            let array = Array(repeating: Value.number(0.0), count: Int(size))
            newState.env[varName] = .array(array)
            return executeProgram(program, pc: pc + 1, state: newState)
        case .success(_):
            return .failure(.typeMismatch(expected: "Number", got: "Other"))
        case .failure(let error):
            return .failure(error)
        }
    }
}

func executeBlock(_ block: [Statement], state: RuntimeState) -> Result<RuntimeState, RuntimeError> {
    var currentState = state
    for stmt in block {
        switch executeSingleStatement(stmt, state: currentState) {
        case .success(let newState):
            currentState = newState
        case .failure(let error):
            return .failure(error)
        }
    }
    return .success(currentState)
}

func executeSingleStatement(_ stmt: Statement, state: RuntimeState) -> Result<RuntimeState, RuntimeError> {
    var newState = state
    switch stmt {
    case .let_(let varName, let expr):
        switch evalExpr(expr, env: state.env) {
        case .success(let value):
            newState.env[varName] = value
            return .success(newState)
        case .failure(let error):
            return .failure(error)
        }

    case .print(let exprs):
        var parts: [String] = []
        for expr in exprs {
            switch evalExpr(expr, env: state.env) {
            case .success(let value):
                parts.append(valueToString(value))
            case .failure(let error):
                return .failure(error)
            }
        }
        newState.output.append(parts.joined(separator: " "))
        return .success(newState)

    default:
        return .success(state)
    }
}

func executeForLoop(var varName: String, current: Double, end: Double, step: Double, body: [Statement], program: Program, pc: Int, state: RuntimeState) -> Result<[String], RuntimeError> {
    if (step > 0.0 && current > end) || (step < 0.0 && current < end) {
        return executeProgram(program, pc: pc + 1, state: state)
    }

    var newState = state
    newState.env[varName] = .number(current)

    switch executeBlock(body, state: newState) {
    case .success(let blockState):
        return executeForLoop(var: varName, current: current + step, end: end, step: step, body: body, program: program, pc: pc, state: blockState)
    case .failure(let error):
        return .failure(error)
    }
}

func executeWhileLoop(cond: Expr, body: [Statement], program: Program, pc: Int, state: RuntimeState) -> Result<[String], RuntimeError> {
    switch evalExpr(cond, env: state.env) {
    case .success(let condVal):
        if isTruthy(condVal) {
            switch executeBlock(body, state: state) {
            case .success(let newState):
                return executeWhileLoop(cond: cond, body: body, program: program, pc: pc, state: newState)
            case .failure(let error):
                return .failure(error)
            }
        } else {
            return executeProgram(program, pc: pc + 1, state: state)
        }
    case .failure(let error):
        return .failure(error)
    }
}

func evalExpr(_ expr: Expr, env: [String: Value]) -> Result<Value, RuntimeError> {
    switch expr {
    case .number(let n):
        return .success(.number(n))

    case .string(let s):
        return .success(.string(s))

    case .variable(let name):
        guard let value = env[name] else {
            return .failure(.undefinedVariable(name))
        }
        return .success(value)

    case .arrayAccess(let varName, let indexExpr):
        guard let arrayVal = env[varName] else {
            return .failure(.undefinedVariable(varName))
        }
        guard case .array(let arr) = arrayVal else {
            return .failure(.typeMismatch(expected: "Array", got: "Other"))
        }
        switch evalExpr(indexExpr, env: env) {
        case .success(.number(let idx)):
            let i = Int(idx)
            guard i >= 0 && i < arr.count else {
                return .failure(.indexOutOfBounds)
            }
            return .success(arr[i])
        case .success(_):
            return .failure(.typeMismatch(expected: "Number", got: "Other"))
        case .failure(let error):
            return .failure(error)
        }

    case .binOp(let op, let left, let right):
        switch (evalExpr(left, env: env), evalExpr(right, env: env)) {
        case (.success(let l), .success(let r)):
            return evalBinOp(op, left: l, right: r)
        case (.failure(let error), _), (_, .failure(let error)):
            return .failure(error)
        }

    case .unaryOp(let op, let operand):
        switch evalExpr(operand, env: env) {
        case .success(let val):
            return evalUnaryOp(op, operand: val)
        case .failure(let error):
            return .failure(error)
        }
    }
}

func evalBinOp(_ op: BinOperator, left: Value, right: Value) -> Result<Value, RuntimeError> {
    switch (op, left, right) {
    case (.add, .number(let l), .number(let r)):
        return .success(.number(l + r))
    case (.sub, .number(let l), .number(let r)):
        return .success(.number(l - r))
    case (.mul, .number(let l), .number(let r)):
        return .success(.number(l * r))
    case (.div, .number(let l), .number(let r)):
        guard r != 0.0 else { return .failure(.divisionByZero) }
        return .success(.number(l / r))
    case (.eq, .number(let l), .number(let r)):
        return .success(.number(l == r ? 1.0 : 0.0))
    case (.ne, .number(let l), .number(let r)):
        return .success(.number(l != r ? 1.0 : 0.0))
    case (.lt, .number(let l), .number(let r)):
        return .success(.number(l < r ? 1.0 : 0.0))
    case (.le, .number(let l), .number(let r)):
        return .success(.number(l <= r ? 1.0 : 0.0))
    case (.gt, .number(let l), .number(let r)):
        return .success(.number(l > r ? 1.0 : 0.0))
    case (.ge, .number(let l), .number(let r)):
        return .success(.number(l >= r ? 1.0 : 0.0))
    case (.and, let l, let r):
        return .success(.number(isTruthy(l) && isTruthy(r) ? 1.0 : 0.0))
    case (.or, let l, let r):
        return .success(.number(isTruthy(l) || isTruthy(r) ? 1.0 : 0.0))
    default:
        return .failure(.typeMismatch(expected: "Number", got: "Other"))
    }
}

func evalUnaryOp(_ op: UnaryOperator, operand: Value) -> Result<Value, RuntimeError> {
    switch (op, operand) {
    case (.neg, .number(let n)):
        return .success(.number(-n))
    case (.not, let v):
        return .success(.number(isTruthy(v) ? 0.0 : 1.0))
    default:
        return .failure(.typeMismatch(expected: "Number", got: "Other"))
    }
}

func isTruthy(_ value: Value) -> Bool {
    switch value {
    case .number(let n):
        return n != 0.0
    case .string(let s):
        return !s.isEmpty
    case .array(let a):
        return !a.isEmpty
    }
}

func valueToString(_ value: Value) -> String {
    switch value {
    case .number(let n):
        return String(n)
    case .string(let s):
        return s
    case .array(_):
        return "[Array]"
    }
}

func findLine(_ program: Program, target: Int) -> Result<Int, RuntimeError> {
    if let index = program.firstIndex(where: { $0.line == target }) {
        return .success(index)
    }
    return .failure(.undefinedLabel(target))
}

// MARK: - テスト実行例
let testProgram: Program = [
    (10, .let_(var: "x", expr: .number(0.0))),
    (20, .let_(var: "x", expr: .binOp(op: .add, left: .variable("x"), right: .number(1.0)))),
    (30, .print([.variable("x")])),
    (40, .if_(cond: .binOp(op: .lt, left: .variable("x"), right: .number(5.0)),
              thenBlock: [.goto(20)],
              elseBlock: [])),
    (50, .end)
]

switch run(program: testProgram) {
case .success(let output):
    output.forEach { print($0) }
case .failure(let error):
    print("エラー: \(error)")
}
