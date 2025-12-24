use crate::parse::{InputPosition, Span};

use super::LspServer;

#[derive(Debug, Clone)]
pub struct EmbeddedLspRoute {
    pub span: Span,
    pub dsl_id: String,
    pub server: LspServer,
}

#[derive(Debug, Default, Clone)]
pub struct EmbeddedLspRegistry {
    routes: Vec<EmbeddedLspRoute>,
}

impl EmbeddedLspRegistry {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn register_route(&mut self, span: Span, dsl_id: impl Into<String>, server: LspServer) {
        self.routes.push(EmbeddedLspRoute {
            span,
            dsl_id: dsl_id.into(),
            server,
        });
    }

    pub fn resolve_route(&self, position: InputPosition) -> Option<&EmbeddedLspRoute> {
        self.routes
            .iter()
            .find(|route| span_contains(&route.span, position))
    }
}

fn span_contains(span: &Span, position: InputPosition) -> bool {
    let start = span.start;
    let end = span.end;
    if position.line < start.line || position.line > end.line {
        return false;
    }
    if position.line == start.line && position.column < start.column {
        return false;
    }
    if position.line == end.line && position.column > end.column {
        return false;
    }
    true
}
