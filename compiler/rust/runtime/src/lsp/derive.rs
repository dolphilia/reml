use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::parse::{ObservedToken, ParseMetaRegistry, Parser, ParserMetaKind, Span};
use crate::run_config::RunConfig;

use super::{position, LspCapabilities, LspServer, Range};

#[derive(Debug, Clone, Default, Serialize)]
pub struct DeriveModel {
    pub completions: Vec<CompletionItem>,
    pub outline: Vec<OutlineNode>,
    pub semantic_tokens: Vec<SemanticToken>,
    pub hovers: Vec<HoverEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutlineNode {
    pub name: String,
    pub kind: String,
    pub children: Vec<OutlineNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SemanticToken {
    pub kind: String,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize)]
pub struct HoverEntry {
    pub name: String,
    pub doc: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LspDeriveEnvelope {
    pub format: String,
    pub version: i64,
    pub source: String,
    pub capabilities: LspCapabilities,
    pub completions: Vec<CompletionItem>,
    pub outline: Vec<OutlineNode>,
    pub semantic_tokens: Vec<SemanticToken>,
    pub hovers: Vec<HoverEntry>,
}

impl LspDeriveEnvelope {
    pub fn from_model(source: impl Into<String>, model: &DeriveModel) -> Self {
        let capabilities = Derive::standard_capabilities(model);
        Self {
            format: "lsp-derive".to_string(),
            version: 1,
            source: source.into(),
            capabilities,
            completions: model.completions.clone(),
            outline: model.outline.clone(),
            semantic_tokens: model.semantic_tokens.clone(),
            hovers: model.hovers.clone(),
        }
    }
}

pub struct Derive;

impl Derive {
    pub fn collect<T>(parser: Parser<T>) -> DeriveModel
    where
        T: Clone + Send + Sync + 'static,
    {
        Self::collect_with_source(&parser, "", &RunConfig::default())
    }

    pub fn collect_with_source<T>(parser: &Parser<T>, source: &str, cfg: &RunConfig) -> DeriveModel
    where
        T: Clone + Send + Sync + 'static,
    {
        let mut state = crate::parse::ParseState::new(source, cfg.clone());
        let _ = parser.parse(&mut state);
        build_model(state.meta_registry(), state.observed_tokens())
    }

    pub fn standard_capabilities(model: &DeriveModel) -> LspCapabilities {
        LspCapabilities {
            completion: !model.completions.is_empty(),
            outline: !model.outline.is_empty(),
            semantic_tokens: !model.semantic_tokens.is_empty(),
            hover: !model.hovers.is_empty(),
        }
    }

    pub fn apply_standard_capabilities(model: &DeriveModel, server: LspServer) -> LspServer {
        server.with_capabilities(Self::standard_capabilities(model))
    }
}

fn build_model(registry: &ParseMetaRegistry, tokens: &[ObservedToken]) -> DeriveModel {
    let completions = collect_completions(registry);
    let outline = collect_outline(registry);
    let hovers = collect_hovers(registry);
    let semantic_tokens = tokens
        .iter()
        .map(|token| SemanticToken {
            kind: token.kind.clone(),
            range: range_from_span(&token.span),
        })
        .collect();
    DeriveModel {
        completions,
        outline,
        semantic_tokens,
        hovers,
    }
}

fn collect_completions(registry: &ParseMetaRegistry) -> Vec<CompletionItem> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for meta in registry.values() {
        let kind = match meta.kind {
            ParserMetaKind::Keyword => "keyword",
            ParserMetaKind::Symbol => "symbol",
            _ => continue,
        };
        let key = (meta.name.clone(), kind.to_string());
        if seen.insert(key.clone()) {
            items.push(CompletionItem {
                label: key.0,
                kind: key.1,
            });
        }
    }
    items.sort_by(|a, b| a.label.cmp(&b.label).then(a.kind.cmp(&b.kind)));
    items
}

fn collect_outline(registry: &ParseMetaRegistry) -> Vec<OutlineNode> {
    let rules: HashMap<_, _> = registry
        .values()
        .filter(|meta| matches!(meta.kind, ParserMetaKind::Rule))
        .map(|meta| (meta.id, meta))
        .collect();
    if rules.is_empty() {
        return Vec::new();
    }
    let mut child_ids = HashSet::new();
    for meta in rules.values() {
        for child in &meta.children {
            if rules.contains_key(child) {
                child_ids.insert(*child);
            }
        }
    }
    let mut roots: Vec<_> = rules
        .keys()
        .filter(|id| !child_ids.contains(id))
        .copied()
        .collect();
    roots.sort_by(|a, b| {
        let left = rules.get(a).map(|meta| meta.name.as_str()).unwrap_or("");
        let right = rules.get(b).map(|meta| meta.name.as_str()).unwrap_or("");
        left.cmp(right)
    });
    let mut visiting = HashSet::new();
    roots
        .into_iter()
        .filter_map(|id| build_outline_node(id, &rules, &mut visiting))
        .collect()
}

fn build_outline_node(
    id: crate::parse::ParserId,
    rules: &HashMap<crate::parse::ParserId, &crate::parse::ParserMeta>,
    visiting: &mut HashSet<crate::parse::ParserId>,
) -> Option<OutlineNode> {
    let meta = rules.get(&id)?;
    if visiting.contains(&id) {
        return Some(OutlineNode {
            name: meta.name.clone(),
            kind: "rule".to_string(),
            children: Vec::new(),
        });
    }
    visiting.insert(id);
    let mut children: Vec<_> = meta
        .children
        .iter()
        .filter_map(|child| build_outline_node(*child, rules, visiting))
        .collect();
    visiting.remove(&id);
    children.sort_by(|a, b| a.name.cmp(&b.name));
    Some(OutlineNode {
        name: meta.name.clone(),
        kind: "rule".to_string(),
        children,
    })
}

fn collect_hovers(registry: &ParseMetaRegistry) -> Vec<HoverEntry> {
    let mut entries = Vec::new();
    for meta in registry.values() {
        if !matches!(meta.kind, ParserMetaKind::Rule | ParserMetaKind::Token) {
            continue;
        }
        let doc = match meta.doc.as_ref() {
            Some(doc) if !doc.trim().is_empty() => doc.clone(),
            _ => continue,
        };
        entries.push(HoverEntry {
            name: meta.name.clone(),
            doc,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn range_from_span(span: &Span) -> Range {
    let start_line = span.start.line.saturating_sub(1) as i64;
    let start_col = span.start.column.saturating_sub(1) as i64;
    let end_line = span.end.line.saturating_sub(1) as i64;
    let end_col = span.end.column.saturating_sub(1) as i64;
    Range {
        start: position(start_line, start_col),
        end: position(end_line, end_col),
    }
}
