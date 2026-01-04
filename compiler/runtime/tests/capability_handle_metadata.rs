use std::convert::TryInto;

use reml_runtime::{
    capability::{
        ActorCapability, ActorCapabilityMetadata, CapabilityHandle, CapabilityHandleKind,
        CapabilityHandleTypeError, CollectionsCapability, CollectionsCapabilityMetadata,
        IoCapability, IoCapabilityMetadata, IoOperationKind, PluginCapability,
        PluginCapabilityMetadata, SecurityCapability, SecurityCapabilityMetadata,
    },
    CapabilityDescriptor, CapabilityProvider, StageId,
};

fn make_descriptor(id: &str, stage: StageId, effects: &[&str]) -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        id,
        stage,
        effects.iter().copied().collect::<Vec<_>>(),
        CapabilityProvider::Core,
    )
}

#[test]
fn handle_descriptor_preserves_effect_scope_and_provider() {
    let io_desc = make_descriptor(
        "io.fs.read",
        StageId::Beta,
        &["effect.io", "effect.io.blocking"],
    );
    let handle: CapabilityHandle =
        IoCapability::new(io_desc, IoCapabilityMetadata::default()).into();

    assert_eq!(handle.kind(), CapabilityHandleKind::Io);
    let desc_ref = handle.descriptor();
    assert_eq!(desc_ref.stage(), StageId::Beta);
    assert!(desc_ref.effect_scope().contains("effect.io"));
    assert!(matches!(
        desc_ref.metadata().provider,
        CapabilityProvider::Core
    ));

    let io_ref: &IoCapability = (&handle).try_into().unwrap();
    assert!(io_ref
        .metadata()
        .operations
        .contains(&IoOperationKind::Read));

    // 別種の Capability も生成しておき、descriptor API が共有できることを確認。
    let plugin_handle: CapabilityHandle = PluginCapability::new(
        make_descriptor(
            "plugin.core.audit",
            StageId::Experimental,
            &["effect.audit"],
        ),
        PluginCapabilityMetadata::default(),
    )
    .into();
    assert_eq!(plugin_handle.descriptor().stage(), StageId::Experimental);

    let collections_handle: CapabilityHandle = CollectionsCapability::new(
        make_descriptor(
            "core.collections.ref",
            StageId::Stable,
            &["mem", "mut", "rc"],
        ),
        CollectionsCapabilityMetadata {
            collector_effects: vec!["collector.effect.rc".into()],
            tracks_mutation: true,
            tracks_reference_count: true,
        },
    )
    .into();
    assert_eq!(collections_handle.kind(), CapabilityHandleKind::Collections);
    assert!(collections_handle
        .descriptor()
        .effect_scope()
        .contains("mut"));
}

#[test]
fn try_from_reports_handle_mismatch() {
    let security_handle: CapabilityHandle = SecurityCapability::new(
        make_descriptor("security.fs.policy", StageId::Stable, &["effect.security"]),
        SecurityCapabilityMetadata::default(),
    )
    .into();

    let result: Result<IoCapability, CapabilityHandleTypeError> =
        security_handle.clone().try_into();
    let err = result.expect_err("security handle should not convert to IO");
    assert_eq!(err.expected(), CapabilityHandleKind::Io);
    assert_eq!(err.actual(), CapabilityHandleKind::Security);

    let actor_handle: CapabilityHandle = ActorCapability::new(
        make_descriptor("actor.runtime", StageId::Alpha, &["effect.actor"]),
        ActorCapabilityMetadata::default(),
    )
    .into();
    let _: ActorCapability = actor_handle
        .clone()
        .try_into()
        .expect("actor handle should convert successfully");
}
