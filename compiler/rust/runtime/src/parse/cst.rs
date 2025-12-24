use crate::text::String as TextString;

use super::{InputPosition, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TriviaKind {
    Whitespace,
    Comment,
    Layout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub text: TextString,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TextString,
    pub text: TextString,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CstChild {
    Node(Box<CstNode>),
    Token(Token),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CstNode {
    pub kind: TextString,
    pub children: Vec<CstChild>,
    pub trivia_leading: Vec<Trivia>,
    pub trivia_trailing: Vec<Trivia>,
    pub span: Span,
}

impl CstNode {
    pub fn empty() -> Self {
        let pos = InputPosition {
            byte: 0,
            line: 1,
            column: 1,
        };
        Self {
            kind: TextString::from("empty"),
            children: Vec::new(),
            trivia_leading: Vec::new(),
            trivia_trailing: Vec::new(),
            span: Span::new(pos, pos),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CstOutput<T> {
    pub ast: T,
    pub cst: CstNode,
}

#[derive(Debug)]
pub struct CstBuilder {
    nodes: Vec<CstNode>,
    pending_trivia: Vec<Trivia>,
    start: InputPosition,
    end: Option<InputPosition>,
}

impl CstBuilder {
    pub fn new(start: InputPosition) -> Self {
        Self {
            nodes: Vec::new(),
            pending_trivia: Vec::new(),
            start,
            end: None,
        }
    }

    pub fn push_token(&mut self, token: Token) {
        let leading = std::mem::take(&mut self.pending_trivia);
        let span = token.span.clone();
        let kind = token.kind.clone();
        self.end = Some(span.end);
        self.nodes.push(CstNode {
            kind,
            children: vec![CstChild::Token(token)],
            trivia_leading: leading,
            trivia_trailing: Vec::new(),
            span,
        });
    }

    pub fn push_trivia(&mut self, trivia: Trivia, trailing: bool) {
        if trailing {
            if let Some(last) = self.nodes.last_mut() {
                last.trivia_trailing.push(trivia.clone());
            }
        }
        self.pending_trivia.push(trivia);
    }

    pub fn finish(mut self, fallback_end: InputPosition) -> CstNode {
        let mut leading = Vec::new();
        if !self.pending_trivia.is_empty() {
            if let Some(last) = self.nodes.last_mut() {
                last.trivia_trailing.extend(self.pending_trivia.drain(..));
            } else {
                leading = std::mem::take(&mut self.pending_trivia);
            }
        }
        let end = self.end.unwrap_or(fallback_end);
        CstNode {
            kind: TextString::from("root"),
            children: self
                .nodes
                .into_iter()
                .map(|node| CstChild::Node(Box::new(node)))
                .collect(),
            trivia_leading: leading,
            trivia_trailing: Vec::new(),
            span: Span::new(self.start, end),
        }
    }
}
