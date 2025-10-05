use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{char, multispace0};
use nom::combinator::map;
use nom::multi::many0;
use nom::number::complete::double;
use nom::sequence::{delimited, preceded};
use nom::IResult;

#[derive(Debug, PartialEq)]
pub enum Expr {
    Number(f64),
    Symbol(String),
    List(Vec<Expr>),
}

pub fn parse_expr(input: &str) -> Result<Expr, String> {
    let parser = preceded(multispace0, expr);
    match parser(input) {
        Ok((rest, value)) if rest.trim().is_empty() => Ok(value),
        Ok((_rest, _)) => Err("未消費トークンがあります".into()),
        Err(err) => Err(format!("解析に失敗しました: {err}")),
    }
}

fn expr(input: &str) -> IResult<&str, Expr> {
    alt((list, number, symbol))(input)
}

fn list(input: &str) -> IResult<&str, Expr> {
    map(
        delimited(
            char('('),
            many0(preceded(multispace0, expr)),
            preceded(multispace0, char(')')),
        ),
        Expr::List,
    )(input)
}

fn number(input: &str) -> IResult<&str, Expr> {
    map(double, Expr::Number)(input)
}

fn symbol(input: &str) -> IResult<&str, Expr> {
    map(take_while1(is_symbol_char), |s: &str| Expr::Symbol(s.into()))(input)
}

fn is_symbol_char(c: char) -> bool {
    !c.is_whitespace() && c != '(' && c != ')'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_application() {
        let expr = parse_expr("(+ 1 2)").unwrap();
        assert!(matches!(expr, Expr::List(_)));
    }
}
