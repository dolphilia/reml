use reml_runtime::{
    capability::{descriptor::CapabilityProvider, registry::CapabilityError},
    CapabilityDescriptor, StageId, StageRequirement,
};

fn sample_descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        "core.io.fs.read",
        StageId::Stable,
        ["effect.io.fs.read"],
        CapabilityProvider::Core,
    )
}

#[test]
fn stage_order_matches_spec_sequence() {
    use StageId::*;

    assert!(Experimental < Alpha);
    assert!(Alpha < Beta);
    assert!(Beta < Stable);

    let mut unordered = vec![Stable, Experimental, Beta, Alpha];
    unordered.sort();
    assert_eq!(unordered, vec![Experimental, Alpha, Beta, Stable]);
}

#[test]
fn stage_requirement_satisfies_exact_and_at_least() {
    use StageId::*;

    let exact_beta = StageRequirement::Exact(Beta);
    assert!(exact_beta.satisfies(Beta));
    assert!(!exact_beta.satisfies(Stable));
    assert!(!exact_beta.satisfies(Alpha));

    let at_least_beta = StageRequirement::AtLeast(Beta);
    assert!(at_least_beta.satisfies(Beta));
    assert!(at_least_beta.satisfies(Stable));
    assert!(!at_least_beta.satisfies(Alpha));
    assert!(!at_least_beta.satisfies(Experimental));
}

#[test]
fn stage_violation_error_includes_descriptor_metadata() {
    let descriptor = sample_descriptor();
    let err = CapabilityError::stage_violation(
        descriptor.id.clone(),
        StageRequirement::Exact(StageId::Stable),
        StageId::Alpha,
        Some(descriptor.clone()),
    );

    assert_eq!(err.code(), "capability.stage.mismatch");
    assert_eq!(
        err.detail(),
        "capability 'core.io.fs.read' requires exact stable but runtime is alpha"
    );
    assert_eq!(err.actual_stage(), Some(StageId::Alpha));
    assert_eq!(err.descriptor(), Some(&descriptor));
}
