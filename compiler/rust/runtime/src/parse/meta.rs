use std::collections::HashMap;

use super::{ParserId, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParserMetaKind {
    Rule,
    Keyword,
    Symbol,
    Token,
}

#[derive(Clone, Debug)]
pub struct ParserMeta {
    pub id: ParserId,
    pub kind: ParserMetaKind,
    pub name: String,
    pub doc: Option<String>,
    pub children: Vec<ParserId>,
    pub token_kind: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ObservedToken {
    pub kind: String,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct ParseMetaRegistry {
    entries: HashMap<ParserId, ParserMeta>,
}

impl ParseMetaRegistry {
    pub fn register(
        &mut self,
        id: ParserId,
        kind: ParserMetaKind,
        name: impl Into<String>,
        token_kind: Option<String>,
    ) {
        let name = name.into();
        let token_kind_value = token_kind.clone();
        let entry = self.entries.entry(id).or_insert_with(|| ParserMeta {
            id,
            kind,
            name,
            doc: None,
            children: Vec::new(),
            token_kind: token_kind_value,
        });
        if entry.token_kind.is_none() {
            entry.token_kind = token_kind;
        }
    }

    pub fn update_doc(&mut self, id: ParserId, doc: String) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.doc = Some(doc);
        }
    }

    pub fn add_child(&mut self, parent: ParserId, child: ParserId) {
        if let Some(entry) = self.entries.get_mut(&parent) {
            if !entry.children.contains(&child) {
                entry.children.push(child);
            }
        }
    }

    pub fn get(&self, id: ParserId) -> Option<&ParserMeta> {
        self.entries.get(&id)
    }

    pub fn values(&self) -> impl Iterator<Item = &ParserMeta> {
        self.entries.values()
    }
}

pub fn normalize_doc(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}
