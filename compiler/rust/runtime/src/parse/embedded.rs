use crate::lsp::LspServer;
use std::fmt;
use std::sync::Arc;

use super::{CstNode, Input, InputPosition, ParseError, Parser, Span};

pub type ContextBridgeHandler = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone, Debug)]
pub struct EmbeddedDslSpec<T> {
    pub dsl_id: String,
    pub boundary: EmbeddedBoundary,
    pub parser: Parser<T>,
    pub lsp: Option<LspServer>,
    pub mode: EmbeddedMode,
    pub context: ContextBridge,
}

#[derive(Clone, Debug)]
pub struct EmbeddedBoundary {
    pub start: String,
    pub end: String,
}

impl EmbeddedBoundary {
    pub fn new(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
        }
    }

    pub fn match_start(&self, input: &Input) -> Option<Input> {
        if input.remaining().starts_with(self.start.as_str()) {
            Some(input.advance(self.start.len()))
        } else {
            None
        }
    }

    pub fn match_end(&self, input: &Input) -> Option<Input> {
        if input.remaining().starts_with(self.end.as_str()) {
            Some(input.advance(self.end.len()))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub enum EmbeddedMode {
    ParallelSafe,
    SequentialOnly,
    Exclusive,
}

#[derive(Clone)]
pub enum ContextBridge {
    Inherit(Vec<String>),
    Custom(ContextBridgeHandler),
}

impl fmt::Debug for ContextBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inherit(keys) => f.debug_tuple("Inherit").field(keys).finish(),
            Self::Custom(_) => f.write_str("Custom(<handler>)"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EmbeddedNode<T> {
    pub dsl_id: String,
    pub span: Span,
    pub ast: T,
    pub cst: Option<CstNode>,
    pub diagnostics: Vec<ParseError>,
}

pub(crate) fn shift_position(base: InputPosition, relative: InputPosition) -> InputPosition {
    let line_offset = relative.line.saturating_sub(1);
    let line = base.line + line_offset;
    let column = if relative.line <= 1 {
        base.column + relative.column.saturating_sub(1)
    } else {
        relative.column
    };
    InputPosition {
        byte: base.byte + relative.byte,
        line,
        column,
    }
}
