use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Number(f64),
    Symbol(String),
    List(Vec<Expr>),
}

#[derive(Clone)]
enum Value {
    Number(f64),
    Lambda { params: Vec<String>, body: Expr, env: Env },
    Builtin(fn(&[Value]) -> Result<Value, String>),
}

type Env = HashMap<String, Value>;

pub fn eval(source: &str) -> Result<Value, String> {
    let tokens = tokenize(source);
    let (expr, rest) = parse_expr(&tokens, 0)?;
    if rest != tokens.len() {
        return Err("未消費トークンがあります".into());
    }
    let mut env = default_env();
    eval_expr(&expr, &mut env)
}

fn tokenize(source: &str) -> Vec<String> {
    source
        .replace('(', " ( ")
        .replace(')', " ) ")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

fn parse_expr(tokens: &[String], index: usize) -> Result<(Expr, usize), String> {
    if index >= tokens.len() {
        return Err("入力が空です".into());
    }
    let token = &tokens[index];
    match token.as_str() {
        "(" => parse_list(tokens, index + 1),
        ")" => Err("対応しない閉じ括弧です".into()),
        t => Ok((parse_atom(t)?, index + 1)),
    }
}

fn parse_list(tokens: &[String], mut index: usize) -> Result<(Expr, usize), String> {
    let mut items = Vec::new();
    while index < tokens.len() {
        if tokens[index] == ")" {
            return Ok((Expr::List(items), index + 1));
        }
        let (expr, next) = parse_expr(tokens, index)?;
        items.push(expr);
        index = next;
    }
    Err("リストが閉じられていません".into())
}

fn parse_atom(token: &str) -> Result<Expr, String> {
    if let Ok(number) = token.parse::<f64>() {
        Ok(Expr::Number(number))
    } else {
        Ok(Expr::Symbol(token.to_string()))
    }
}

fn eval_expr(expr: &Expr, env: &mut Env) -> Result<Value, String> {
    match expr {
        Expr::Number(n) => Ok(Value::Number(*n)),
        Expr::Symbol(name) => env.get(name).cloned().ok_or_else(|| format!("未定義シンボル: {name}")),
        Expr::List(items) => {
            if items.is_empty() {
                return Err("空の式は評価できません".into());
            }
            let callee = eval_expr(&items[0], env)?;
            let mut args = Vec::new();
            for arg_expr in &items[1..] {
                args.push(eval_expr(arg_expr, env)?);
            }
            apply(callee, &args)
        }
    }
}

fn apply(callee: Value, args: &[Value]) -> Result<Value, String> {
    match callee {
        Value::Builtin(fun) => fun(args),
        Value::Lambda { params, body, mut env } => {
            if params.len() != args.len() {
                return Err("引数の数が一致しません".into());
            }
            for (param, arg) in params.into_iter().zip(args.iter().cloned()) {
                env.insert(param, arg);
            }
            eval_expr(&body, &mut env)
        }
        Value::Number(_) => Err("数値を関数適用できません".into()),
    }
}

fn default_env() -> Env {
    let mut env = HashMap::new();
    env.insert("+".into(), Value::Builtin(|args| numeric_op(args, |a, b| Ok(a + b))));
    env.insert("-".into(), Value::Builtin(|args| numeric_op(args, |a, b| Ok(a - b))));
    env.insert("*".into(), Value::Builtin(|args| numeric_op(args, |a, b| Ok(a * b))));
    env.insert("/".into(), Value::Builtin(|args| numeric_op(args, |a, b| {
        if b == 0.0 {
            Err("0 で割れません".into())
        } else {
            Ok(a / b)
        }
    })));
    env
}

fn numeric_op<F>(args: &[Value], op: F) -> Result<Value, String>
where
    F: Fn(f64, f64) -> Result<f64, String>,
{
    if args.len() != 2 {
        return Err("2 引数で呼び出してください".into());
    }
    match (&args[0], &args[1]) {
        (Value::Number(lhs), Value::Number(rhs)) => Ok(Value::Number(op(*lhs, *rhs)?)),
        _ => Err("数値以外を演算できません".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_add() {
        let result = eval("(+ 40 2)").unwrap();
        match result {
            Value::Number(n) => assert_eq!(n, 42.0),
            _ => panic!("数値が返る想定"),
        }
    }
}
