use reml_runtime::{
    stage::{StageId, StageRequirement},
    CapabilityRegistry,
};
use serde::Deserialize;

const CORE_IO_CAPABILITY_MATRIX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../..",
    "/tests/capabilities/core_io_registry.json"
));

#[derive(Debug, Deserialize)]
struct CapabilityMatrix {
    cases: Vec<CapabilityCase>,
}

#[derive(Debug, Deserialize)]
struct CapabilityCase {
    id: String,
    requirement: RequirementSpec,
    #[serde(default = "Expectation::pass")]
    expect: Expectation,
}

#[derive(Debug, Deserialize)]
struct RequirementSpec {
    #[serde(rename = "type")]
    kind: RequirementKind,
    stage: StageName,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequirementKind {
    Exact,
    AtLeast,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StageName {
    Experimental,
    Alpha,
    Beta,
    Stable,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Expectation {
    Pass,
    Fail,
}

impl Expectation {
    const fn pass() -> Self {
        Expectation::Pass
    }
}

impl RequirementSpec {
    fn to_requirement(&self) -> StageRequirement {
        match self.kind {
            RequirementKind::Exact => StageRequirement::Exact(self.stage.to_stage_id()),
            RequirementKind::AtLeast => StageRequirement::AtLeast(self.stage.to_stage_id()),
        }
    }
}

impl StageName {
    fn to_stage_id(&self) -> StageId {
        match self {
            StageName::Experimental => StageId::Experimental,
            StageName::Alpha => StageId::Alpha,
            StageName::Beta => StageId::Beta,
            StageName::Stable => StageId::Stable,
        }
    }
}

#[test]
fn core_io_capability_matrix_is_honored_by_registry() {
    let matrix: CapabilityMatrix =
        serde_json::from_str(CORE_IO_CAPABILITY_MATRIX).expect("valid capability matrix JSON");
    let registry = CapabilityRegistry::registry();
    let empty_effects: [String; 0] = [];

    for case in matrix.cases {
        let requirement = case.requirement.to_requirement();
        match case.expect {
            Expectation::Pass => {
                let stage = registry
                    .verify_capability_stage(&case.id, requirement, &empty_effects)
                    .unwrap_or_else(|err| {
                        panic!("expected capability `{}` to pass but got {err}", case.id)
                    });
                assert!(
                    requirement.matches(stage),
                    "registry should return a stage that satisfies the requirement for `{}`",
                    case.id
                );
            }
            Expectation::Fail => {
                let error = registry
                    .verify_capability_stage(&case.id, requirement, &empty_effects)
                    .expect_err(&format!(
                        "capability `{}` should not satisfy {:?}",
                        case.id, requirement
                    ));
                assert_eq!(
                    error.code(),
                    "capability.stage.mismatch",
                    "unexpected error code for `{}`",
                    case.id
                );
                assert_eq!(
                    error.actual_stage(),
                    Some(StageId::Stable),
                    "actual stage metadata should be attached for `{}`",
                    case.id
                );
            }
        }
    }
}
