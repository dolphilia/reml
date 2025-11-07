use indexmap::IndexMap;
use serde::Serialize;

/// 型推論で収集する基本メトリクス。
#[derive(Debug, Clone, Serialize)]
pub struct TypecheckMetrics {
    pub typed_functions: usize,
    pub typed_exprs: usize,
    pub constraints_total: usize,
    pub constraint_breakdown: IndexMap<String, usize>,
    pub unresolved_identifiers: usize,
    pub call_sites: usize,
    pub binary_expressions: usize,
}

impl Default for TypecheckMetrics {
    fn default() -> Self {
        Self {
            typed_functions: 0,
            typed_exprs: 0,
            constraints_total: 0,
            constraint_breakdown: IndexMap::new(),
            unresolved_identifiers: 0,
            call_sites: 0,
            binary_expressions: 0,
        }
    }
}

impl TypecheckMetrics {
    pub fn record_function(&mut self) {
        self.typed_functions += 1;
    }

    pub fn record_expr(&mut self) {
        self.typed_exprs += 1;
    }

    pub fn record_constraint(&mut self, key: impl Into<String>) {
        let key = key.into();
        self.constraints_total += 1;
        *self.constraint_breakdown.entry(key).or_insert(0) += 1;
    }

    pub fn record_unresolved_identifier(&mut self) {
        self.unresolved_identifiers += 1;
    }

    pub fn record_call_site(&mut self) {
        self.call_sites += 1;
    }

    pub fn record_binary_expr(&mut self) {
        self.binary_expressions += 1;
    }
}
