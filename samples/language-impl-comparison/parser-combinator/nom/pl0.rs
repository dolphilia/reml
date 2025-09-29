use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{char, multispace0};
use nom::combinator::{map, map_res};
use nom::multi::separated_list1;
use nom::sequence::{delimited, pair, preceded};
use nom::IResult;

#[derive(Debug, PartialEq, Clone)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Number(i64),
    Var(String),
    Binary { op: Op, lhs: Box<Expr>, rhs: Box<Expr> },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    Assign { name: String, expr: Expr },
    While { cond: Expr, body: Vec<Stmt> },
    Write { expr: Expr },
}

pub fn parse_program(input: &str) -> Result<Vec<Stmt>, String> {
    let parser = preceded(multispace0, block);
    match parser(input) {
        Ok((rest, stmts)) if rest.trim().is_empty() => Ok(stmts),
        Ok((_rest, _)) => Err("未消費文字が残っています".into()),
        Err(err) => Err(format!("解析に失敗しました: {err}")),
    }
}

fn block(input: &str) -> IResult<&str, Vec<Stmt>> {
    delimited(
        preceded(multispace0, tag_token("begin")),
        separated_list1(preceded(multispace0, char(';')), preceded(multispace0, stmt)),
        preceded(multispace0, tag_token("end")),
    )(input)
}

fn stmt(input: &str) -> IResult<&str, Stmt> {
    alt((while_stmt, write_stmt, assign_stmt))(input)
}

fn assign_stmt(input: &str) -> IResult<&str, Stmt> {
    map(
        pair(
            identifier,
            preceded(preceded(multispace0, tag_token(":=")), expr),
        ),
        |(name, expr)| Stmt::Assign { name, expr },
    )(input)
}

fn write_stmt(input: &str) -> IResult<&str, Stmt> {
    map(preceded(tag_token("write"), expr), |expr| Stmt::Write { expr })(input)
}

fn while_stmt(input: &str) -> IResult<&str, Stmt> {
    map(
        pair(
            preceded(tag_token("while"), expr),
            preceded(tag_token("do"), block),
        ),
        |(cond, body)| Stmt::While { cond, body },
    )(input)
}

fn expr(input: &str) -> IResult<&str, Expr> {
    parse_add_sub(input)
}

fn parse_add_sub(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut node) = parse_mul_div(input)?;
    loop {
        let parsed = preceded(multispace0, alt((char('+'), char('-'))))(input);
        match parsed {
            Ok((next, op_char)) => {
                let (after_rhs, rhs) = parse_mul_div(next)?;
                let op = if op_char == '+' { Op::Add } else { Op::Sub };
                node = Expr::Binary { op, lhs: Box::new(node), rhs: Box::new(rhs) };
                input = after_rhs;
            }
            Err(_) => break,
        }
    }
    Ok((input, node))
}

fn parse_mul_div(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut node) = factor(input)?;
    loop {
        let parsed = preceded(multispace0, alt((char('*'), char('/'))))(input);
        match parsed {
            Ok((next, op_char)) => {
                let (after_rhs, rhs) = factor(next)?;
                let op = if op_char == '*' { Op::Mul } else { Op::Div };
                node = Expr::Binary { op, lhs: Box::new(node), rhs: Box::new(rhs) };
                input = after_rhs;
            }
            Err(_) => break,
        }
    }
    Ok((input, node))
}

fn factor(input: &str) -> IResult<&str, Expr> {
    preceded(
        multispace0,
            alt((
                map(integer, Expr::Number),
                map(identifier, Expr::Var),
                delimited(
                    preceded(multispace0, char('(')),
                    expr,
                    preceded(multispace0, char(')')),
                ),
            )),
    )(input)
}

fn integer(input: &str) -> IResult<&str, i64> {
    map_res(preceded(multispace0, take_while1(|c: char| c.is_ascii_digit())), |digits: &str| {
        digits.parse::<i64>()
    })(input)
}

fn identifier(input: &str) -> IResult<&str, String> {
    map(preceded(multispace0, take_while1(is_ident_char)), |s: &str| s.into())(input)
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn tag_token<'a>(token: &'static str) -> impl Fn(&'a str) -> IResult<&'a str, &'a str> {
    move |input: &str| preceded(multispace0, tag(token))(input)
}
