use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign { name: String, expr: Expr },
    While { cond: Expr, body: Vec<Stmt> },
    Write { expr: Expr },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    Var(String),
    Binary { op: Op, lhs: Box<Expr>, rhs: Box<Expr> },
}

#[derive(Debug, Clone, Copy)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Default)]
pub struct Runtime {
    vars: HashMap<String, i64>,
    pub output: Vec<i64>,
}

#[derive(Debug)]
pub enum ExecError {
    UndefinedVar(String),
    DivisionByZero,
}

pub fn exec(program: &[Stmt]) -> Result<Runtime, ExecError> {
    let mut runtime = Runtime::default();
    for stmt in program {
        exec_stmt(stmt, &mut runtime)?;
    }
    Ok(runtime)
}

fn exec_stmt(stmt: &Stmt, runtime: &mut Runtime) -> Result<(), ExecError> {
    match stmt {
        Stmt::Assign { name, expr } => {
            let value = eval_expr(expr, &runtime.vars)?;
            runtime.vars.insert(name.clone(), value);
        }
        Stmt::While { cond, body } => {
            while eval_expr(cond, &runtime.vars)? != 0 {
                for inner in body {
                    exec_stmt(inner, runtime)?;
                }
            }
        }
        Stmt::Write { expr } => {
            let value = eval_expr(expr, &runtime.vars)?;
            runtime.output.push(value);
        }
    }
    Ok(())
}

fn eval_expr(expr: &Expr, vars: &HashMap<String, i64>) -> Result<i64, ExecError> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Var(name) => vars
            .get(name)
            .copied()
            .ok_or_else(|| ExecError::UndefinedVar(name.clone())),
        Expr::Binary { op, lhs, rhs } => {
            let l = eval_expr(lhs, vars)?;
            let r = eval_expr(rhs, vars)?;
            Ok(match op {
                Op::Add => l + r,
                Op::Sub => l - r,
                Op::Mul => l * r,
                Op::Div => {
                    if r == 0 {
                        return Err(ExecError::DivisionByZero);
                    }
                    l / r
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_simple_loop() {
        let program = vec![
            Stmt::Assign { name: "x".into(), expr: Expr::Number(3) },
            Stmt::While {
                cond: Expr::Var("x".into()),
                body: vec![
                    Stmt::Write { expr: Expr::Var("x".into()) },
                    Stmt::Assign {
                        name: "x".into(),
                        expr: Expr::Binary {
                            op: Op::Sub,
                            lhs: Box::new(Expr::Var("x".into())),
                            rhs: Box::new(Expr::Number(1)),
                        },
                    },
                ],
            },
        ];

        let runtime = exec(&program).unwrap();
        assert_eq!(runtime.output, vec![3, 2, 1]);
    }
}
