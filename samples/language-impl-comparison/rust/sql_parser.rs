// 簡易SQL Parser - Rust実装
// SELECT, WHERE, JOIN, ORDER BY対応
// nomパーサーコンビネーターライブラリを使用

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while, take_while1},
    character::complete::{char, digit1, multispace0, multispace1},
    combinator::{map, opt, recognize},
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, terminated, tuple},
};

// AST定義
#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    InnerJoin,
    LeftJoin,
    RightJoin,
    FullJoin,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or, Like,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Not,
    IsNull,
    IsNotNull,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    NullLit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Column(String),
    QualifiedColumn { table: String, column: String },
    BinaryOp { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    UnaryOp { op: UnOp, expr: Box<Expr> },
    FunctionCall { name: String, args: Vec<Expr> },
    Parenthesized(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    AllColumns,
    ColumnExpr { expr: Expr, alias: Option<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableRef {
    pub table: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Join {
    pub join_type: JoinType,
    pub table: TableRef,
    pub on_condition: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    pub columns: Vec<(Expr, OrderDirection)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub columns: Vec<Column>,
    pub from_table: TableRef,
    pub where_clause: Option<Expr>,
    pub joins: Vec<Join>,
    pub order_by: Option<OrderBy>,
}

// パーサー補助関数
fn ws<'a, F, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    delimited(multispace0, inner, multispace0)
}

fn keyword<'a>(kw: &'static str) -> impl FnMut(&'a str) -> IResult<&'a str, ()> {
    move |input: &'a str| {
        let (input, _) = ws(tag_no_case(kw))(input)?;
        // キーワード後に英数字が続かないことを確認
        match input.chars().next() {
            Some(c) if c.is_alphanumeric() || c == '_' => {
                Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
            }
            _ => Ok((input, ())),
        }
    }
}

fn identifier(input: &str) -> IResult<&str, String> {
    let reserved = vec![
        "select", "from", "where", "join", "inner", "left",
        "right", "full", "on", "and", "or", "not", "like",
        "order", "by", "asc", "desc", "null", "true", "false", "as"
    ];

    let (input, ident) = ws(recognize(pair(
        alt((
            take_while1(|c: char| c.is_alphabetic()),
            tag("_"),
        )),
        take_while(|c: char| c.is_alphanumeric() || c == '_'),
    )))(input)?;

    let ident_lower = ident.to_lowercase();
    if reserved.contains(&ident_lower.as_str()) {
        Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    } else {
        Ok((input, ident.to_string()))
    }
}

// リテラル
fn integer(input: &str) -> IResult<&str, Literal> {
    let (input, num) = ws(digit1)(input)?;
    Ok((input, Literal::IntLit(num.parse().unwrap_or(0))))
}

fn float_lit(input: &str) -> IResult<&str, Literal> {
    let (input, num) = ws(recognize(tuple((
        digit1,
        char('.'),
        digit1,
    ))))(input)?;
    Ok((input, Literal::FloatLit(num.parse().unwrap_or(0.0))))
}

fn string_lit(input: &str) -> IResult<&str, Literal> {
    let (input, s) = ws(delimited(
        char('\''),
        take_while(|c| c != '\''),
        char('\''),
    ))(input)?;
    Ok((input, Literal::StringLit(s.to_string())))
}

fn literal(input: &str) -> IResult<&str, Literal> {
    alt((
        map(keyword("null"), |_| Literal::NullLit),
        map(keyword("true"), |_| Literal::BoolLit(true)),
        map(keyword("false"), |_| Literal::BoolLit(false)),
        float_lit,
        integer,
        string_lit,
    ))(input)
}

// 式パーサー（演算子優先度を考慮）
fn expr(input: &str) -> IResult<&str, Expr> {
    or_expr(input)
}

fn or_expr(input: &str) -> IResult<&str, Expr> {
    let (input, first) = and_expr(input)?;
    let (input, rest) = many0(preceded(keyword("or"), and_expr))(input)?;
    Ok((input, rest.into_iter().fold(first, |acc, e| {
        Expr::BinaryOp {
            op: BinOp::Or,
            left: Box::new(acc),
            right: Box::new(e),
        }
    })))
}

fn and_expr(input: &str) -> IResult<&str, Expr> {
    let (input, first) = cmp_expr(input)?;
    let (input, rest) = many0(preceded(keyword("and"), cmp_expr))(input)?;
    Ok((input, rest.into_iter().fold(first, |acc, e| {
        Expr::BinaryOp {
            op: BinOp::And,
            left: Box::new(acc),
            right: Box::new(e),
        }
    })))
}

fn cmp_expr(input: &str) -> IResult<&str, Expr> {
    let (input, left) = add_expr(input)?;
    let (input, op_right) = opt(pair(
        alt((
            map(ws(tag("=")), |_| BinOp::Eq),
            map(ws(tag("<>")), |_| BinOp::Ne),
            map(ws(tag("!=")), |_| BinOp::Ne),
            map(ws(tag("<=")), |_| BinOp::Le),
            map(ws(tag(">=")), |_| BinOp::Ge),
            map(ws(tag("<")), |_| BinOp::Lt),
            map(ws(tag(">")), |_| BinOp::Gt),
            map(keyword("like"), |_| BinOp::Like),
        )),
        add_expr,
    ))(input)?;

    Ok((input, match op_right {
        None => left,
        Some((op, right)) => Expr::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }))
}

fn add_expr(input: &str) -> IResult<&str, Expr> {
    let (input, first) = mul_expr(input)?;
    let (input, rest) = many0(pair(
        alt((
            map(ws(tag("+")), |_| BinOp::Add),
            map(ws(tag("-")), |_| BinOp::Sub),
        )),
        mul_expr,
    ))(input)?;
    Ok((input, rest.into_iter().fold(first, |acc, (op, e)| {
        Expr::BinaryOp {
            op,
            left: Box::new(acc),
            right: Box::new(e),
        }
    })))
}

fn mul_expr(input: &str) -> IResult<&str, Expr> {
    let (input, first) = unary_expr(input)?;
    let (input, rest) = many0(pair(
        alt((
            map(ws(tag("*")), |_| BinOp::Mul),
            map(ws(tag("/")), |_| BinOp::Div),
            map(ws(tag("%")), |_| BinOp::Mod),
        )),
        unary_expr,
    ))(input)?;
    Ok((input, rest.into_iter().fold(first, |acc, (op, e)| {
        Expr::BinaryOp {
            op,
            left: Box::new(acc),
            right: Box::new(e),
        }
    })))
}

fn unary_expr(input: &str) -> IResult<&str, Expr> {
    alt((
        map(preceded(keyword("not"), unary_expr), |e| {
            Expr::UnaryOp {
                op: UnOp::Not,
                expr: Box::new(e),
            }
        }),
        postfix_expr,
    ))(input)
}

fn postfix_expr(input: &str) -> IResult<&str, Expr> {
    let (input, e) = primary_expr(input)?;
    let (input, is_null_op) = opt(tuple((
        keyword("is"),
        opt(keyword("not")),
        keyword("null"),
    )))(input)?;

    Ok((input, match is_null_op {
        None => e,
        Some((_, Some(_), _)) => Expr::UnaryOp {
            op: UnOp::IsNotNull,
            expr: Box::new(e),
        },
        Some(_) => Expr::UnaryOp {
            op: UnOp::IsNull,
            expr: Box::new(e),
        },
    }))
}

fn primary_expr(input: &str) -> IResult<&str, Expr> {
    alt((
        map(
            delimited(ws(tag("(")), expr, ws(tag(")"))),
            |e| Expr::Parenthesized(Box::new(e)),
        ),
        function_call,
        column_ref,
        map(literal, Expr::Literal),
    ))(input)
}

fn function_call(input: &str) -> IResult<&str, Expr> {
    let (input, name) = identifier(input)?;
    let (input, args) = delimited(
        ws(tag("(")),
        separated_list0(ws(tag(",")), expr),
        ws(tag(")")),
    )(input)?;
    Ok((input, Expr::FunctionCall { name, args }))
}

fn column_ref(input: &str) -> IResult<&str, Expr> {
    let (input, first) = identifier(input)?;
    let (input, second) = opt(preceded(ws(tag(".")), identifier))(input)?;
    Ok((input, match second {
        None => Expr::Column(first),
        Some(col) => Expr::QualifiedColumn { table: first, column: col },
    }))
}

// カラムリスト
fn column_list(input: &str) -> IResult<&str, Vec<Column>> {
    alt((
        map(ws(tag("*")), |_| vec![Column::AllColumns]),
        separated_list1(ws(tag(",")), column_expr),
    ))(input)
}

fn column_expr(input: &str) -> IResult<&str, Column> {
    let (input, e) = expr(input)?;
    let (input, alias) = opt(preceded(opt(keyword("as")), identifier))(input)?;
    Ok((input, Column::ColumnExpr { expr: e, alias }))
}

// テーブル参照
fn table_ref(input: &str) -> IResult<&str, TableRef> {
    let (input, table) = identifier(input)?;
    let (input, alias) = opt(preceded(opt(keyword("as")), identifier))(input)?;
    Ok((input, TableRef { table, alias }))
}

// JOIN句
fn join_clause(input: &str) -> IResult<&str, Join> {
    let (input, join_type) = alt((
        map(tuple((keyword("inner"), keyword("join"))), |_| JoinType::InnerJoin),
        map(tuple((keyword("left"), keyword("join"))), |_| JoinType::LeftJoin),
        map(tuple((keyword("right"), keyword("join"))), |_| JoinType::RightJoin),
        map(tuple((keyword("full"), keyword("join"))), |_| JoinType::FullJoin),
        map(keyword("join"), |_| JoinType::InnerJoin),
    ))(input)?;
    let (input, table) = table_ref(input)?;
    let (input, _) = keyword("on")(input)?;
    let (input, condition) = expr(input)?;
    Ok((input, Join { join_type, table, on_condition: condition }))
}

// ORDER BY句
fn order_by_clause(input: &str) -> IResult<&str, OrderBy> {
    let (input, _) = tuple((keyword("order"), keyword("by")))(input)?;
    let (input, columns) = separated_list1(ws(tag(",")), order_expr)(input)?;
    Ok((input, OrderBy { columns }))
}

fn order_expr(input: &str) -> IResult<&str, (Expr, OrderDirection)> {
    let (input, e) = expr(input)?;
    let (input, dir) = opt(alt((
        map(keyword("asc"), |_| OrderDirection::Asc),
        map(keyword("desc"), |_| OrderDirection::Desc),
    )))(input)?;
    Ok((input, (e, dir.unwrap_or(OrderDirection::Asc))))
}

// SELECT文
fn select_query(input: &str) -> IResult<&str, Query> {
    let (input, _) = keyword("select")(input)?;
    let (input, columns) = column_list(input)?;
    let (input, _) = keyword("from")(input)?;
    let (input, from_table) = table_ref(input)?;
    let (input, joins) = many0(join_clause)(input)?;
    let (input, where_clause) = opt(preceded(keyword("where"), expr))(input)?;
    let (input, order_by) = opt(order_by_clause)(input)?;

    Ok((input, Query {
        columns,
        from_table,
        where_clause,
        joins,
        order_by,
    }))
}

// パブリックAPI
pub fn parse(input: &str) -> Result<Query, String> {
    let input = input.trim();
    match terminated(select_query, pair(opt(ws(tag(";"))), multispace0))(input) {
        Ok(("", query)) => Ok(query),
        Ok((remaining, _)) => Err(format!("Unexpected input: {}", remaining)),
        Err(e) => Err(format!("Parse error: {:?}", e)),
    }
}

// レンダリング関数
pub fn render_query(q: &Query) -> String {
    let cols = q.columns.iter()
        .map(render_column)
        .collect::<Vec<_>>()
        .join(", ");

    let from = format!("FROM {}{}",
        q.from_table.table,
        q.from_table.alias.as_ref().map(|a| format!(" AS {}", a)).unwrap_or_default()
    );

    let joins = q.joins.iter()
        .map(|j| {
            let jt = match j.join_type {
                JoinType::InnerJoin => "INNER JOIN",
                JoinType::LeftJoin => "LEFT JOIN",
                JoinType::RightJoin => "RIGHT JOIN",
                JoinType::FullJoin => "FULL JOIN",
            };
            format!("{} {} ON {}", jt, j.table.table, render_expr(&j.on_condition))
        })
        .collect::<Vec<_>>()
        .join(" ");

    let where_clause = q.where_clause.as_ref()
        .map(|e| format!(" WHERE {}", render_expr(e)))
        .unwrap_or_default();

    let order_by = q.order_by.as_ref()
        .map(|ob| {
            let cols = ob.columns.iter()
                .map(|(e, dir)| format!("{} {}",
                    render_expr(e),
                    match dir {
                        OrderDirection::Asc => "ASC",
                        OrderDirection::Desc => "DESC",
                    }
                ))
                .collect::<Vec<_>>()
                .join(", ");
            format!(" ORDER BY {}", cols)
        })
        .unwrap_or_default();

    format!("SELECT {} {} {}{}{}", cols, from, joins, where_clause, order_by).trim().to_string()
}

fn render_column(col: &Column) -> String {
    match col {
        Column::AllColumns => "*".to_string(),
        Column::ColumnExpr { expr, alias } => {
            let e = render_expr(expr);
            match alias {
                Some(a) => format!("{} AS {}", e, a),
                None => e,
            }
        }
    }
}

fn render_expr(expr: &Expr) -> String {
    match expr {
        Expr::Literal(lit) => render_literal(lit),
        Expr::Column(name) => name.clone(),
        Expr::QualifiedColumn { table, column } => format!("{}.{}", table, column),
        Expr::BinaryOp { op, left, right } => {
            format!("({} {} {})", render_expr(left), render_binop(op), render_expr(right))
        }
        Expr::UnaryOp { op, expr } => match op {
            UnOp::Not => format!("NOT {}", render_expr(expr)),
            UnOp::IsNull => format!("{} IS NULL", render_expr(expr)),
            UnOp::IsNotNull => format!("{} IS NOT NULL", render_expr(expr)),
        },
        Expr::FunctionCall { name, args } => {
            format!("{}({})", name, args.iter().map(render_expr).collect::<Vec<_>>().join(", "))
        }
        Expr::Parenthesized(e) => format!("({})", render_expr(e)),
    }
}

fn render_literal(lit: &Literal) -> String {
    match lit {
        Literal::IntLit(n) => n.to_string(),
        Literal::FloatLit(f) => f.to_string(),
        Literal::StringLit(s) => format!("'{}'", s),
        Literal::BoolLit(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Literal::NullLit => "NULL".to_string(),
    }
}

fn render_binop(op: &BinOp) -> &str {
    match op {
        BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*",
        BinOp::Div => "/", BinOp::Mod => "%",
        BinOp::Eq => "=", BinOp::Ne => "<>", BinOp::Lt => "<",
        BinOp::Le => "<=", BinOp::Gt => ">", BinOp::Ge => ">=",
        BinOp::And => "AND", BinOp::Or => "OR", BinOp::Like => "LIKE",
    }
}

// テスト
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let sql = "SELECT * FROM users";
        let query = parse(sql).unwrap();
        assert_eq!(query.columns.len(), 1);
        assert_eq!(query.from_table.table, "users");
    }

    #[test]
    fn test_where_clause() {
        let sql = "SELECT name FROM users WHERE id = 1";
        let query = parse(sql).unwrap();
        assert!(query.where_clause.is_some());
    }
}

fn main() {
    println!("=== Rust SQL Parser テスト ===");
    let test_sql = "SELECT name, age FROM users WHERE age > 18 ORDER BY name ASC";
    match parse(test_sql) {
        Ok(q) => {
            println!("パース成功: {}", test_sql);
            println!("レンダリング: {}", render_query(&q));
        }
        Err(e) => println!("パースエラー: {}", e),
    }
}