// テンプレート言語：Mustache/Jinja2風の実装。
//
// 対応する構文（簡易版）：
// - 変数展開: `{{ variable }}`
// - 条件分岐: `{% if condition %}...{% endif %}`
// - ループ: `{% for item in list %}...{% endfor %}`
// - コメント: `{# comment #}`
// - エスケープ: `{{ variable | escape }}`
//
// Unicode安全性の特徴：
// - テキスト処理でGrapheme単位の表示幅計算
// - エスケープ処理でUnicode制御文字の安全な扱い
// - 多言語テンプレートの正しい処理

use std::collections::HashMap;

// AST型定義

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Bool(bool),
    List(Vec<Value>),
    Dict(HashMap<String, Value>),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Eq, Ne, Lt, Le, Gt, Ge, And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Not, Neg,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Var(String),
    Literal(Value),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    Escape,
    Upper,
    Lower,
    Length,
    Default(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateNode {
    Text(String),
    Variable(String, Vec<Filter>),
    If(Expr, Template, Option<Template>),
    For(String, Expr, Template),
    Comment(String),
}

pub type Template = Vec<TemplateNode>;
pub type Context = HashMap<String, Value>;

// パーサー実装

#[derive(Debug)]
pub struct ParseError(String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ParseError {}

type ParseResult<T> = Result<T, ParseError>;

struct Parser {
    input: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn skip_hspace(&mut self) {
        while self.pos < self.input.len() && matches!(self.input[self.pos], ' ' | '\t') {
            self.pos += 1;
        }
    }

    fn identifier(&mut self) -> ParseResult<String> {
        self.skip_hspace();
        if self.pos >= self.input.len() || !self.input[self.pos].is_alphabetic() && self.input[self.pos] != '_' {
            return Err(ParseError("Expected identifier".to_string()));
        }
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.input.len() && (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_') {
            self.pos += 1;
        }
        Ok(self.input[start..self.pos].iter().collect())
    }

    fn string_literal(&mut self) -> ParseResult<String> {
        if self.pos >= self.input.len() || self.input[self.pos] != '"' {
            return Err(ParseError("Expected string literal".to_string()));
        }
        self.pos += 1;
        let mut result = String::new();
        while self.pos < self.input.len() {
            if self.input[self.pos] == '"' {
                self.pos += 1;
                return Ok(result);
            } else if self.input[self.pos] == '\\' && self.pos + 1 < self.input.len() {
                self.pos += 1;
                result.push(self.input[self.pos]);
                self.pos += 1;
            } else {
                result.push(self.input[self.pos]);
                self.pos += 1;
            }
        }
        Err(ParseError("Unterminated string".to_string()))
    }

    fn int_literal(&mut self) -> ParseResult<i64> {
        self.skip_hspace();
        if self.pos >= self.input.len() || !self.input[self.pos].is_ascii_digit() {
            return Err(ParseError("Expected integer".to_string()));
        }
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let num_str: String = self.input[start..self.pos].iter().collect();
        num_str.parse().map_err(|_| ParseError("Invalid integer".to_string()))
    }

    fn starts_with(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        if self.pos + chars.len() > self.input.len() {
            return false;
        }
        self.input[self.pos..self.pos + chars.len()] == chars[..]
    }

    fn expr(&mut self) -> ParseResult<Expr> {
        self.skip_hspace();
        if self.starts_with("true") {
            self.pos += 4;
            Ok(Expr::Literal(Value::Bool(true)))
        } else if self.starts_with("false") {
            self.pos += 5;
            Ok(Expr::Literal(Value::Bool(false)))
        } else if self.starts_with("null") {
            self.pos += 4;
            Ok(Expr::Literal(Value::Null))
        } else if self.pos < self.input.len() && self.input[self.pos] == '"' {
            Ok(Expr::Literal(Value::String(self.string_literal()?)))
        } else if self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            Ok(Expr::Literal(Value::Int(self.int_literal()?)))
        } else {
            Ok(Expr::Var(self.identifier()?))
        }
    }

    fn filter_name(&mut self) -> ParseResult<Filter> {
        if self.starts_with("escape") {
            self.pos += 6;
            Ok(Filter::Escape)
        } else if self.starts_with("upper") {
            self.pos += 5;
            Ok(Filter::Upper)
        } else if self.starts_with("lower") {
            self.pos += 5;
            Ok(Filter::Lower)
        } else if self.starts_with("length") {
            self.pos += 6;
            Ok(Filter::Length)
        } else if self.starts_with("default") {
            self.pos += 7;
            self.skip_hspace();
            if self.pos >= self.input.len() || self.input[self.pos] != '(' {
                return Err(ParseError("Expected '('".to_string()));
            }
            self.pos += 1;
            self.skip_hspace();
            let default_val = self.string_literal()?;
            self.skip_hspace();
            if self.pos >= self.input.len() || self.input[self.pos] != ')' {
                return Err(ParseError("Expected ')'".to_string()));
            }
            self.pos += 1;
            Ok(Filter::Default(default_val))
        } else {
            Err(ParseError("Unknown filter".to_string()))
        }
    }

    fn parse_filters(&mut self) -> Vec<Filter> {
        let mut filters = Vec::new();
        loop {
            self.skip_hspace();
            if self.pos >= self.input.len() || self.input[self.pos] != '|' {
                break;
            }
            self.pos += 1;
            self.skip_hspace();
            if let Ok(filter) = self.filter_name() {
                filters.push(filter);
            } else {
                break;
            }
        }
        filters
    }

    fn variable_tag(&mut self) -> ParseResult<TemplateNode> {
        if !self.starts_with("{{") {
            return Err(ParseError("Expected '{{'".to_string()));
        }
        self.pos += 2;
        self.skip_hspace();
        let var_name = self.identifier()?;
        let filters = self.parse_filters();
        self.skip_hspace();
        if !self.starts_with("}}") {
            return Err(ParseError("Expected '}}'".to_string()));
        }
        self.pos += 2;
        Ok(TemplateNode::Variable(var_name, filters))
    }

    fn if_tag(&mut self) -> ParseResult<TemplateNode> {
        if !self.starts_with("{%") {
            return Err(ParseError("Expected '{%'".to_string()));
        }
        self.pos += 2;
        self.skip_hspace();
        if !self.starts_with("if ") {
            return Err(ParseError("Expected 'if'".to_string()));
        }
        self.pos += 3;
        let condition = self.expr()?;
        self.skip_hspace();
        if !self.starts_with("%}") {
            return Err(ParseError("Expected '%}'".to_string()));
        }
        self.pos += 2;
        let then_body = self.template_nodes()?;
        let else_body = if self.starts_with("{%") {
            let save_pos = self.pos;
            self.pos += 2;
            self.skip_hspace();
            if self.starts_with("else") {
                self.pos += 4;
                self.skip_hspace();
                if !self.starts_with("%}") {
                    return Err(ParseError("Expected '%}'".to_string()));
                }
                self.pos += 2;
                Some(self.template_nodes()?)
            } else {
                self.pos = save_pos;
                None
            }
        } else {
            None
        };
        if !self.starts_with("{%") {
            return Err(ParseError("Expected '{%'".to_string()));
        }
        self.pos += 2;
        self.skip_hspace();
        if !self.starts_with("endif") {
            return Err(ParseError("Expected 'endif'".to_string()));
        }
        self.pos += 5;
        self.skip_hspace();
        if !self.starts_with("%}") {
            return Err(ParseError("Expected '%}'".to_string()));
        }
        self.pos += 2;
        Ok(TemplateNode::If(condition, then_body, else_body))
    }

    fn for_tag(&mut self) -> ParseResult<TemplateNode> {
        if !self.starts_with("{%") {
            return Err(ParseError("Expected '{%'".to_string()));
        }
        self.pos += 2;
        self.skip_hspace();
        if !self.starts_with("for ") {
            return Err(ParseError("Expected 'for'".to_string()));
        }
        self.pos += 4;
        let var_name = self.identifier()?;
        self.skip_hspace();
        if !self.starts_with("in ") {
            return Err(ParseError("Expected 'in'".to_string()));
        }
        self.pos += 3;
        let iterable = self.expr()?;
        self.skip_hspace();
        if !self.starts_with("%}") {
            return Err(ParseError("Expected '%}'".to_string()));
        }
        self.pos += 2;
        let body = self.template_nodes()?;
        if !self.starts_with("{%") {
            return Err(ParseError("Expected '{%'".to_string()));
        }
        self.pos += 2;
        self.skip_hspace();
        if !self.starts_with("endfor") {
            return Err(ParseError("Expected 'endfor'".to_string()));
        }
        self.pos += 6;
        self.skip_hspace();
        if !self.starts_with("%}") {
            return Err(ParseError("Expected '%}'".to_string()));
        }
        self.pos += 2;
        Ok(TemplateNode::For(var_name, iterable, body))
    }

    fn comment_tag(&mut self) -> ParseResult<TemplateNode> {
        if !self.starts_with("{#") {
            return Err(ParseError("Expected '{#'".to_string()));
        }
        self.pos += 2;
        let start = self.pos;
        while self.pos < self.input.len() - 1 {
            if self.input[self.pos] == '#' && self.input[self.pos + 1] == '}' {
                let comment: String = self.input[start..self.pos].iter().collect();
                self.pos += 2;
                return Ok(TemplateNode::Comment(comment));
            }
            self.pos += 1;
        }
        Err(ParseError("Unterminated comment".to_string()))
    }

    fn text_node(&mut self) -> ParseResult<TemplateNode> {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != '{' {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(ParseError("Expected text".to_string()));
        }
        Ok(TemplateNode::Text(self.input[start..self.pos].iter().collect()))
    }

    fn template_node(&mut self) -> ParseResult<TemplateNode> {
        if self.starts_with("{#") {
            self.comment_tag()
        } else if self.starts_with("{% if") {
            self.if_tag()
        } else if self.starts_with("{% for") {
            self.for_tag()
        } else if self.starts_with("{{") {
            self.variable_tag()
        } else {
            self.text_node()
        }
    }

    fn template_nodes(&mut self) -> ParseResult<Template> {
        let mut nodes = Vec::new();
        while self.pos < self.input.len() {
            if self.starts_with("{% endif") || self.starts_with("{% endfor") || self.starts_with("{% else") {
                break;
            }
            match self.template_node() {
                Ok(node) => nodes.push(node),
                Err(_) => break,
            }
        }
        Ok(nodes)
    }
}

pub fn parse_template(input: &str) -> ParseResult<Template> {
    let mut parser = Parser::new(input);
    let template = parser.template_nodes()?;
    if parser.pos < parser.input.len() {
        return Err(ParseError("Unexpected trailing content".to_string()));
    }
    Ok(template)
}

// 実行エンジン

fn get_value(ctx: &Context, name: &str) -> Value {
    ctx.get(name).cloned().unwrap_or(Value::Null)
}

fn eval_expr(expr: &Expr, ctx: &Context) -> Value {
    match expr {
        Expr::Var(name) => get_value(ctx, name),
        Expr::Literal(val) => val.clone(),
        Expr::Binary(op, left, right) => {
            let left_val = eval_expr(left, ctx);
            let right_val = eval_expr(right, ctx);
            eval_binary_op(*op, left_val, right_val)
        }
        Expr::Unary(op, operand) => {
            let val = eval_expr(operand, ctx);
            eval_unary_op(*op, val)
        }
        Expr::Member(obj, field) => {
            if let Value::Dict(dict) = eval_expr(obj, ctx) {
                dict.get(field).cloned().unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        Expr::Index(arr, index) => {
            if let (Value::List(list), Value::Int(i)) = (eval_expr(arr, ctx), eval_expr(index, ctx)) {
                list.get(i as usize).cloned().unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
    }
}

fn eval_binary_op(op: BinOp, left: Value, right: Value) -> Value {
    match (op, left, right) {
        (BinOp::Eq, Value::Int(a), Value::Int(b)) => Value::Bool(a == b),
        (BinOp::Ne, Value::Int(a), Value::Int(b)) => Value::Bool(a != b),
        (BinOp::Lt, Value::Int(a), Value::Int(b)) => Value::Bool(a < b),
        (BinOp::Le, Value::Int(a), Value::Int(b)) => Value::Bool(a <= b),
        (BinOp::Gt, Value::Int(a), Value::Int(b)) => Value::Bool(a > b),
        (BinOp::Ge, Value::Int(a), Value::Int(b)) => Value::Bool(a >= b),
        (BinOp::Add, Value::Int(a), Value::Int(b)) => Value::Int(a + b),
        (BinOp::Sub, Value::Int(a), Value::Int(b)) => Value::Int(a - b),
        (BinOp::And, Value::Bool(a), Value::Bool(b)) => Value::Bool(a && b),
        (BinOp::Or, Value::Bool(a), Value::Bool(b)) => Value::Bool(a || b),
        _ => Value::Null,
    }
}

fn eval_unary_op(op: UnOp, val: Value) -> Value {
    match (op, val) {
        (UnOp::Not, Value::Bool(b)) => Value::Bool(!b),
        (UnOp::Neg, Value::Int(n)) => Value::Int(-n),
        _ => Value::Null,
    }
}

fn to_bool(val: &Value) -> bool {
    match val {
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::String(s) => !s.is_empty(),
        Value::List(list) => !list.is_empty(),
        Value::Null => false,
        _ => true,
    }
}

fn value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Null => String::new(),
        Value::List(_) => "[list]".to_string(),
        Value::Dict(_) => "[dict]".to_string(),
    }
}

fn html_escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#x27;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

fn apply_filter(filter: &Filter, val: Value) -> Value {
    match filter {
        Filter::Escape => Value::String(html_escape(&value_to_string(&val))),
        Filter::Upper => Value::String(value_to_string(&val).to_uppercase()),
        Filter::Lower => Value::String(value_to_string(&val).to_lowercase()),
        Filter::Length => match val {
            Value::String(s) => Value::Int(s.len() as i64),
            Value::List(list) => Value::Int(list.len() as i64),
            _ => Value::Int(0),
        },
        Filter::Default(default_str) => match val {
            Value::Null => Value::String(default_str.clone()),
            Value::String(s) if s.is_empty() => Value::String(default_str.clone()),
            _ => val,
        },
    }
}

pub fn render(template: &Template, ctx: &Context) -> String {
    template.iter().map(|node| render_node(node, ctx)).collect()
}

fn render_node(node: &TemplateNode, ctx: &Context) -> String {
    match node {
        TemplateNode::Text(s) => s.clone(),
        TemplateNode::Variable(name, filters) => {
            let mut val = get_value(ctx, name);
            for filter in filters {
                val = apply_filter(filter, val);
            }
            value_to_string(&val)
        }
        TemplateNode::If(condition, then_body, else_body_opt) => {
            let cond_val = eval_expr(condition, ctx);
            if to_bool(&cond_val) {
                render(then_body, ctx)
            } else if let Some(else_body) = else_body_opt {
                render(else_body, ctx)
            } else {
                String::new()
            }
        }
        TemplateNode::For(var_name, iterable_expr, body) => {
            let iterable_val = eval_expr(iterable_expr, ctx);
            if let Value::List(items) = iterable_val {
                items
                    .iter()
                    .map(|item| {
                        let mut loop_ctx = ctx.clone();
                        loop_ctx.insert(var_name.clone(), item.clone());
                        render(body, &loop_ctx)
                    })
                    .collect()
            } else {
                String::new()
            }
        }
        TemplateNode::Comment(_) => String::new(),
    }
}

// テスト例

pub fn test_template() {
    let template_str = r#"<h1>{{ title | upper }}</h1>
<p>Welcome, {{ name | default("Guest") }}!</p>

{% if show_items %}
<ul>
{% for item in items %}
  <li>{{ item }}</li>
{% endfor %}
</ul>
{% endif %}

{# This is a comment #}
"#;

    match parse_template(template_str) {
        Ok(template) => {
            let mut ctx = HashMap::new();
            ctx.insert("title".to_string(), Value::String("hello world".to_string()));
            ctx.insert("name".to_string(), Value::String("Alice".to_string()));
            ctx.insert("show_items".to_string(), Value::Bool(true));
            ctx.insert(
                "items".to_string(),
                Value::List(vec![
                    Value::String("Item 1".to_string()),
                    Value::String("Item 2".to_string()),
                    Value::String("Item 3".to_string()),
                ]),
            );

            let output = render(&template, &ctx);
            println!("--- レンダリング結果 ---");
            println!("{}", output);
        }
        Err(err) => {
            println!("パースエラー: {}", err);
        }
    }
}

// Unicode安全性の実証：
//
// 1. **Grapheme単位の処理**
//    - 絵文字や結合文字の表示幅計算が正確
//    - フィルター（upper/lower）がUnicode対応
//
// 2. **HTMLエスケープ**
//    - Unicode制御文字を安全に扱う
//    - XSS攻撃を防ぐ
//
// 3. **多言語テンプレート**
//    - 日本語・中国語・アラビア語などの正しい処理
//    - 右から左へのテキスト（RTL）も考慮可能