use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Number(f64),
    String(String),
    Array(Vec<Value>),
}

type Env = HashMap<String, Value>;

#[derive(Debug, Clone)]
enum Statement {
    Let { var: String, expr: Expr },
    Print(Vec<Expr>),
    If { cond: Expr, then_block: Vec<Statement>, else_block: Vec<Statement> },
    For { var: String, start: Expr, end: Expr, step: Expr, body: Vec<Statement> },
    While { cond: Expr, body: Vec<Statement> },
    Goto(i32),
    Gosub(i32),
    Return,
    Dim { var: String, size: Expr },
    End,
}

#[derive(Debug, Clone)]
enum Expr {
    Number(f64),
    String(String),
    Variable(String),
    ArrayAccess { var: String, index: Box<Expr> },
    BinOp { op: BinOperator, left: Box<Expr>, right: Box<Expr> },
    UnaryOp { op: UnaryOperator, operand: Box<Expr> },
}

#[derive(Debug, Clone, Copy)]
enum BinOperator {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
}

#[derive(Debug, Clone, Copy)]
enum UnaryOperator {
    Neg, Not,
}

type Program = Vec<(i32, Statement)>;

struct RuntimeState {
    env: Env,
    call_stack: Vec<i32>,
    output: Vec<String>,
}

#[derive(Debug)]
enum RuntimeError {
    UndefinedVariable(String),
    UndefinedLabel(i32),
    TypeMismatch { expected: String, got: String },
    IndexOutOfBounds,
    DivisionByZero,
    StackUnderflow,
}

pub fn run(program: Program) -> Result<Vec<String>, RuntimeError> {
    let state = RuntimeState {
        env: HashMap::new(),
        call_stack: Vec::new(),
        output: Vec::new(),
    };

    let mut sorted = program;
    sorted.sort_by_key(|(line, _)| *line);

    execute_program(&sorted, 0, state)
}

fn execute_program(
    program: &[(i32, Statement)],
    pc: usize,
    mut state: RuntimeState,
) -> Result<Vec<String>, RuntimeError> {
    if pc >= program.len() {
        return Ok(state.output);
    }

    let (_line, stmt) = &program[pc];

    match stmt {
        Statement::End => Ok(state.output),

        Statement::Let { var, expr } => {
            let value = eval_expr(expr, &state.env)?;
            state.env.insert(var.clone(), value);
            execute_program(program, pc + 1, state)
        }

        Statement::Print(exprs) => {
            let values: Result<Vec<_>, _> = exprs.iter()
                .map(|e| eval_expr(e, &state.env))
                .collect();
            let text = values?
                .iter()
                .map(value_to_string)
                .collect::<Vec<_>>()
                .join(" ");
            state.output.push(text);
            execute_program(program, pc + 1, state)
        }

        Statement::If { cond, then_block, else_block } => {
            let cond_val = eval_expr(cond, &state.env)?;
            let branch = if is_truthy(&cond_val) { then_block } else { else_block };
            state = execute_block(branch, state)?;
            execute_program(program, pc + 1, state)
        }

        Statement::For { var, start, end, step, body } => {
            let start_val = eval_expr(start, &state.env)?;
            let end_val = eval_expr(end, &state.env)?;
            let step_val = eval_expr(step, &state.env)?;
            execute_for_loop(var, start_val, end_val, step_val, body, program, pc, state)
        }

        Statement::While { cond, body } => {
            execute_while_loop(cond, body, program, pc, state)
        }

        Statement::Goto(target) => {
            let new_pc = find_line(program, *target)?;
            execute_program(program, new_pc, state)
        }

        Statement::Gosub(target) => {
            let new_pc = find_line(program, *target)?;
            state.call_stack.push((pc + 1) as i32);
            execute_program(program, new_pc, state)
        }

        Statement::Return => {
            let return_pc = state.call_stack.pop()
                .ok_or(RuntimeError::StackUnderflow)?;
            execute_program(program, return_pc as usize, state)
        }

        Statement::Dim { var, size } => {
            let size_val = eval_expr(size, &state.env)?;
            match size_val {
                Value::Number(n) => {
                    let array = vec![Value::Number(0.0); n as usize];
                    state.env.insert(var.clone(), Value::Array(array));
                    execute_program(program, pc + 1, state)
                }
                _ => Err(RuntimeError::TypeMismatch {
                    expected: "Number".into(),
                    got: "Other".into(),
                }),
            }
        }
    }
}

fn execute_block(
    block: &[Statement],
    mut state: RuntimeState,
) -> Result<RuntimeState, RuntimeError> {
    for stmt in block {
        state = execute_single_statement(stmt, state)?;
    }
    Ok(state)
}

fn execute_single_statement(
    stmt: &Statement,
    mut state: RuntimeState,
) -> Result<RuntimeState, RuntimeError> {
    match stmt {
        Statement::Let { var, expr } => {
            let value = eval_expr(expr, &state.env)?;
            state.env.insert(var.clone(), value);
            Ok(state)
        }

        Statement::Print(exprs) => {
            let values: Result<Vec<_>, _> = exprs.iter()
                .map(|e| eval_expr(e, &state.env))
                .collect();
            let text = values?
                .iter()
                .map(value_to_string)
                .collect::<Vec<_>>()
                .join(" ");
            state.output.push(text);
            Ok(state)
        }

        _ => Ok(state),
    }
}

fn execute_for_loop(
    var: &str,
    start: Value,
    end: Value,
    step: Value,
    body: &[Statement],
    program: &[(i32, Statement)],
    pc: usize,
    state: RuntimeState,
) -> Result<Vec<String>, RuntimeError> {
    match (start, end, step) {
        (Value::Number(s), Value::Number(e), Value::Number(st)) => {
            for_loop_helper(var, s, e, st, body, program, pc, state)
        }
        _ => Err(RuntimeError::TypeMismatch {
            expected: "Number".into(),
            got: "Other".into(),
        }),
    }
}

fn for_loop_helper(
    var: &str,
    current: f64,
    end: f64,
    step: f64,
    body: &[Statement],
    program: &[(i32, Statement)],
    pc: usize,
    mut state: RuntimeState,
) -> Result<Vec<String>, RuntimeError> {
    if (step > 0.0 && current > end) || (step < 0.0 && current < end) {
        execute_program(program, pc + 1, state)
    } else {
        state.env.insert(var.to_string(), Value::Number(current));
        state = execute_block(body, state)?;
        for_loop_helper(var, current + step, end, step, body, program, pc, state)
    }
}

fn execute_while_loop(
    cond: &Expr,
    body: &[Statement],
    program: &[(i32, Statement)],
    pc: usize,
    state: RuntimeState,
) -> Result<Vec<String>, RuntimeError> {
    let cond_val = eval_expr(cond, &state.env)?;
    if is_truthy(&cond_val) {
        let new_state = execute_block(body, state)?;
        execute_while_loop(cond, body, program, pc, new_state)
    } else {
        execute_program(program, pc + 1, state)
    }
}

fn eval_expr(expr: &Expr, env: &Env) -> Result<Value, RuntimeError> {
    match expr {
        Expr::Number(n) => Ok(Value::Number(*n)),
        Expr::String(s) => Ok(Value::String(s.clone())),
        Expr::Variable(name) => {
            env.get(name)
                .cloned()
                .ok_or_else(|| RuntimeError::UndefinedVariable(name.clone()))
        }

        Expr::ArrayAccess { var, index } => {
            match env.get(var) {
                Some(Value::Array(arr)) => {
                    let idx_val = eval_expr(index, env)?;
                    match idx_val {
                        Value::Number(idx) => {
                            arr.get(idx as usize)
                                .cloned()
                                .ok_or(RuntimeError::IndexOutOfBounds)
                        }
                        _ => Err(RuntimeError::TypeMismatch {
                            expected: "Number".into(),
                            got: "Other".into(),
                        }),
                    }
                }
                Some(_) => Err(RuntimeError::TypeMismatch {
                    expected: "Array".into(),
                    got: "Other".into(),
                }),
                None => Err(RuntimeError::UndefinedVariable(var.clone())),
            }
        }

        Expr::BinOp { op, left, right } => {
            let l = eval_expr(left, env)?;
            let r = eval_expr(right, env)?;
            eval_binop(*op, l, r)
        }

        Expr::UnaryOp { op, operand } => {
            let val = eval_expr(operand, env)?;
            eval_unaryop(*op, val)
        }
    }
}

fn eval_binop(op: BinOperator, left: Value, right: Value) -> Result<Value, RuntimeError> {
    match (op, left, right) {
        (BinOperator::Add, Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
        (BinOperator::Sub, Value::Number(l), Value::Number(r)) => Ok(Value::Number(l - r)),
        (BinOperator::Mul, Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
        (BinOperator::Div, Value::Number(l), Value::Number(r)) => {
            if r == 0.0 {
                Err(RuntimeError::DivisionByZero)
            } else {
                Ok(Value::Number(l / r))
            }
        }
        (BinOperator::Eq, Value::Number(l), Value::Number(r)) => {
            Ok(Value::Number(if l == r { 1.0 } else { 0.0 }))
        }
        (BinOperator::Ne, Value::Number(l), Value::Number(r)) => {
            Ok(Value::Number(if l != r { 1.0 } else { 0.0 }))
        }
        (BinOperator::Lt, Value::Number(l), Value::Number(r)) => {
            Ok(Value::Number(if l < r { 1.0 } else { 0.0 }))
        }
        (BinOperator::Le, Value::Number(l), Value::Number(r)) => {
            Ok(Value::Number(if l <= r { 1.0 } else { 0.0 }))
        }
        (BinOperator::Gt, Value::Number(l), Value::Number(r)) => {
            Ok(Value::Number(if l > r { 1.0 } else { 0.0 }))
        }
        (BinOperator::Ge, Value::Number(l), Value::Number(r)) => {
            Ok(Value::Number(if l >= r { 1.0 } else { 0.0 }))
        }
        (BinOperator::And, l, r) => {
            Ok(Value::Number(if is_truthy(&l) && is_truthy(&r) { 1.0 } else { 0.0 }))
        }
        (BinOperator::Or, l, r) => {
            Ok(Value::Number(if is_truthy(&l) || is_truthy(&r) { 1.0 } else { 0.0 }))
        }
        _ => Err(RuntimeError::TypeMismatch {
            expected: "Number".into(),
            got: "Other".into(),
        }),
    }
}

fn eval_unaryop(op: UnaryOperator, operand: Value) -> Result<Value, RuntimeError> {
    match (op, operand) {
        (UnaryOperator::Neg, Value::Number(n)) => Ok(Value::Number(-n)),
        (UnaryOperator::Not, v) => {
            Ok(Value::Number(if is_truthy(&v) { 0.0 } else { 1.0 }))
        }
        _ => Err(RuntimeError::TypeMismatch {
            expected: "Number".into(),
            got: "Other".into(),
        }),
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Number(n) => *n != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Number(n) => format!("{}", n),
        Value::String(s) => s.clone(),
        Value::Array(_) => "[Array]".to_string(),
    }
}

fn find_line(program: &[(i32, Statement)], target: i32) -> Result<usize, RuntimeError> {
    program
        .iter()
        .position(|(line, _)| *line == target)
        .ok_or(RuntimeError::UndefinedLabel(target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_loop() {
        let program = vec![
            (10, Statement::Let { var: "x".into(), expr: Expr::Number(0.0) }),
            (20, Statement::Let {
                var: "x".into(),
                expr: Expr::BinOp {
                    op: BinOperator::Add,
                    left: Box::new(Expr::Variable("x".into())),
                    right: Box::new(Expr::Number(1.0)),
                },
            }),
            (30, Statement::Print(vec![Expr::Variable("x".into())])),
            (40, Statement::If {
                cond: Expr::BinOp {
                    op: BinOperator::Lt,
                    left: Box::new(Expr::Variable("x".into())),
                    right: Box::new(Expr::Number(3.0)),
                },
                then_block: vec![Statement::Goto(20)],
                else_block: vec![],
            }),
            (50, Statement::End),
        ];

        let result = run(program).unwrap();
        assert_eq!(result, vec!["1", "2", "3"]);
    }
}
