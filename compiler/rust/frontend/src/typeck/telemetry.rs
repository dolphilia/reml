use crate::diagnostic::formatter;
use crate::effects::diagnostics::CapabilityMismatch;
use crate::span::Span;
use serde::Serialize;
use std::collections::HashMap;

use super::constraint::Constraint;
use super::driver::{
    TypecheckReport, TypecheckViolation, TypecheckViolationKind, TypedFunctionSummary,
};
use super::types::Type;

const TELEMETRY_SCHEMA_VERSION: &str = "3.0.0-alpha";

/// TypeChecker が生成した制約情報と診断を可視化するためのテレメトリ。
#[derive(Debug, Clone, Serialize)]
pub struct TraitResolutionTelemetry {
    pub schema_version: &'static str,
    pub generated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    pub functions: Vec<TypedFunctionSummary>,
    pub metrics: super::metrics::TypecheckMetrics,
    pub graph: ConstraintGraphSummary,
    pub resolutions: Vec<TraitResolutionRecord>,
}

impl TraitResolutionTelemetry {
    pub fn from_report(
        report: &TypecheckReport,
        input: Option<&str>,
        export_dot: Option<String>,
    ) -> Self {
        let generated_at = formatter::current_timestamp();
        let graph = ConstraintGraphSummary::from_constraints(&report.constraints, export_dot);
        let resolutions =
            TraitResolutionRecord::from_violations(&report.violations, graph.export_dot.clone());
        Self {
            schema_version: TELEMETRY_SCHEMA_VERSION,
            generated_at,
            input: input.map(|value| value.to_string()),
            functions: report.functions.clone(),
            metrics: report.metrics.clone(),
            graph,
            resolutions,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ConstraintGraphSummary {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<ConstraintGraphNode>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<ConstraintGraphEdge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_dot: Option<String>,
    pub node_count: usize,
    pub edge_count: usize,
}

impl ConstraintGraphSummary {
    fn from_constraints(constraints: &[Constraint], export_dot: Option<String>) -> Self {
        let mut builder = ConstraintGraphBuilder::default();
        for constraint in constraints {
            builder.register(constraint);
        }
        Self {
            node_count: builder.nodes.len(),
            edge_count: builder.edges.len(),
            nodes: builder.nodes,
            edges: builder.edges,
            export_dot,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConstraintGraphNode {
    pub id: usize,
    pub label: String,
    pub kind: ConstraintGraphNodeKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintGraphNodeKind {
    Type,
    Capability,
    Implementation,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConstraintGraphEdge {
    pub from: usize,
    pub to: usize,
    pub kind: ConstraintGraphEdgeKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintGraphEdgeKind {
    Equal,
    Capability,
    Implementation,
}

#[derive(Default)]
struct ConstraintGraphBuilder {
    nodes: Vec<ConstraintGraphNode>,
    edges: Vec<ConstraintGraphEdge>,
    index: HashMap<String, usize>,
}

impl ConstraintGraphBuilder {
    fn register(&mut self, constraint: &Constraint) {
        match constraint {
            Constraint::Equal { left, right } => {
                let left_id = self.ensure_type_node(left);
                let right_id = self.ensure_type_node(right);
                if left_id != right_id {
                    self.edges.push(ConstraintGraphEdge {
                        from: left_id,
                        to: right_id,
                        kind: ConstraintGraphEdgeKind::Equal,
                    });
                }
            }
            Constraint::HasCapability { ty, capability, .. } => {
                let ty_id = self.ensure_type_node(ty);
                let cap_id = self.ensure_node(
                    format!("cap:{capability}"),
                    format!("cap:{capability}"),
                    ConstraintGraphNodeKind::Capability,
                );
                self.edges.push(ConstraintGraphEdge {
                    from: ty_id,
                    to: cap_id,
                    kind: ConstraintGraphEdgeKind::Capability,
                });
            }
            Constraint::ImplBound { ty, implementation } => {
                let ty_id = self.ensure_type_node(ty);
                let impl_id = self.ensure_node(
                    format!("impl:{implementation}"),
                    format!("impl:{implementation}"),
                    ConstraintGraphNodeKind::Implementation,
                );
                self.edges.push(ConstraintGraphEdge {
                    from: ty_id,
                    to: impl_id,
                    kind: ConstraintGraphEdgeKind::Implementation,
                });
            }
        }
    }

    fn ensure_type_node(&mut self, ty: &Type) -> usize {
        self.ensure_node(
            format!("type:{}", ty.label()),
            ty.label(),
            ConstraintGraphNodeKind::Type,
        )
    }

    fn ensure_node(&mut self, key: String, label: String, kind: ConstraintGraphNodeKind) -> usize {
        if let Some(id) = self.index.get(&key) {
            return *id;
        }
        let id = self.nodes.len();
        self.nodes.push(ConstraintGraphNode { id, label, kind });
        self.index.insert(key, id);
        id
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitResolutionRecord {
    pub trait_name: String,
    pub type_args: Vec<String>,
    pub constraint: String,
    pub resolution_state: ResolutionState,
    pub dictionary: ResolutionDictionary,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<ResolutionDictionary>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub generalized_typevars: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SpanTelemetry>,
    pub graph: GraphPointer,
}

impl TraitResolutionRecord {
    fn from_violations(violations: &[TypecheckViolation], export_dot: Option<String>) -> Vec<Self> {
        violations
            .iter()
            .filter_map(|violation| Self::from_violation(violation, export_dot.clone()))
            .collect()
    }

    fn from_violation(violation: &TypecheckViolation, export_dot: Option<String>) -> Option<Self> {
        match violation.kind {
            TypecheckViolationKind::StageMismatch => {
                Self::from_stage_mismatch(violation, export_dot)
            }
            TypecheckViolationKind::IteratorStageMismatch => {
                Self::from_iterator_mismatch(violation, export_dot)
            }
            _ => None,
        }
    }

    fn from_stage_mismatch(
        violation: &TypecheckViolation,
        export_dot: Option<String>,
    ) -> Option<Self> {
        let mismatch = violation.capability_mismatch.as_ref()?;
        let trait_name = violation
            .capability
            .clone()
            .unwrap_or_else(|| mismatch.capability().to_string());
        let constraint = format!(
            "{}<expected={}, actual={}>",
            trait_name,
            mismatch.required_label(),
            mismatch.actual_label()
        );
        let dictionary = ResolutionDictionary::for_capability(mismatch);
        Some(Self {
            trait_name,
            type_args: vec![],
            constraint,
            resolution_state: ResolutionState::StageMismatch,
            dictionary,
            candidates: Vec::new(),
            pending: Vec::new(),
            generalized_typevars: Vec::new(),
            span: violation.span.map(SpanTelemetry::from),
            graph: GraphPointer {
                export_dot: export_dot.clone(),
            },
        })
    }

    fn from_iterator_mismatch(
        violation: &TypecheckViolation,
        export_dot: Option<String>,
    ) -> Option<Self> {
        let snapshot = violation.iterator_stage.as_ref()?;
        let trait_name = format!("iterator::{}", snapshot.kind);
        let constraint = format!(
            "{}<source={}, required={}, actual={}>",
            trait_name,
            snapshot.source,
            snapshot.required.label(),
            snapshot.actual.label()
        );
        Some(Self {
            trait_name,
            type_args: vec![snapshot.source.clone()],
            constraint,
            resolution_state: ResolutionState::StageMismatch,
            dictionary: ResolutionDictionary::iterator(snapshot),
            candidates: Vec::new(),
            pending: Vec::new(),
            generalized_typevars: Vec::new(),
            span: violation.span.map(SpanTelemetry::from),
            graph: GraphPointer {
                export_dot: export_dot.clone(),
            },
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphPointer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_dot: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionState {
    Resolved,
    StageMismatch,
    Unresolved,
    Ambiguous,
    UnresolvedTypevar,
    Cyclic,
    Pending,
}

impl Default for ResolutionState {
    fn default() -> Self {
        ResolutionState::Pending
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolutionDictionary {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trait_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub type_args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_index: Option<usize>,
}

impl ResolutionDictionary {
    #[allow(dead_code)]
    fn none() -> Self {
        Self {
            kind: "none".to_string(),
            identifier: None,
            trait_name: None,
            type_args: Vec::new(),
            repr: None,
            parameter_index: None,
        }
    }

    fn for_capability(mismatch: &CapabilityMismatch) -> Self {
        Self {
            kind: "capability".to_string(),
            identifier: Some(mismatch.capability().to_string()),
            trait_name: Some(mismatch.capability().to_string()),
            type_args: Vec::new(),
            repr: Some(format!(
                "required={}, actual={}",
                mismatch.required_label(),
                mismatch.actual_label()
            )),
            parameter_index: None,
        }
    }

    fn iterator(snapshot: &super::driver::IteratorStageViolationInfo) -> Self {
        Self {
            kind: "iterator".to_string(),
            identifier: snapshot.capability.clone().map(|value| value.to_string()),
            trait_name: Some(snapshot.kind.to_string()),
            type_args: vec![snapshot.source.clone()],
            repr: Some(format!(
                "required={}, actual={}",
                snapshot.required.label(),
                snapshot.actual.label()
            )),
            parameter_index: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SpanTelemetry {
    pub start: u32,
    pub end: u32,
}

impl From<Span> for SpanTelemetry {
    fn from(span: Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}
