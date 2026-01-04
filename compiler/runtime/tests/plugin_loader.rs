use reml_runtime::{
    capability::{registry::reset_for_tests, CapabilityProvider, CapabilityRegistry},
    config::manifest::{
        CapabilityId, Manifest, PackageName, ProjectKind, ProjectSection, RunCapabilityEntry,
        RunSection, RunTargetSection, SemanticVersion,
    },
    runtime::plugin::{
        take_plugin_audit_events, PluginBundleManifest, PluginLoader, PluginSignature,
        SignatureAlgorithm, VerificationPolicy,
    },
};

fn sample_manifest() -> Manifest {
    let mut manifest = Manifest::default();
    manifest.project = ProjectSection {
        name: PackageName("plugin.demo".to_string()),
        version: SemanticVersion("1.2.3".to_string()),
        kind: ProjectKind::Plugin,
        ..ProjectSection::default()
    };
    manifest.run = RunSection {
        target: RunTargetSection {
            capabilities: vec![RunCapabilityEntry {
                id: CapabilityId("plugin.demo.audit".to_string()),
                stage: Some("beta".to_string()),
                declared_effects: vec!["audit".to_string()],
                source_span: None,
                provider: None,
            }],
        },
    };
    manifest
}

#[test]
fn plugin_loader_registers_manifest_capabilities() {
    let _guard = reml_runtime::test_support::lock();
    reset_for_tests();
    let _ = take_plugin_audit_events();
    let loader = PluginLoader::new();
    let manifest = sample_manifest();
    let registration = loader
        .register_manifest(&manifest)
        .expect("plugin manifest registration should succeed");
    assert_eq!(registration.plugin_id, "plugin.demo");
    assert_eq!(registration.capabilities, vec!["plugin.demo.audit"]);

    let descriptor = CapabilityRegistry::registry()
        .describe("plugin.demo.audit")
        .expect("plugin capability should be registered");
    match &descriptor.metadata().provider {
        CapabilityProvider::Plugin { package, version } => {
            assert_eq!(package, "plugin.demo");
            assert_eq!(version.as_deref(), Some("1.2.3"));
        }
        other => panic!("unexpected provider: {other:?}"),
    }
}

#[test]
fn plugin_bundle_requires_signature_in_strict_mode() {
    let _guard = reml_runtime::test_support::lock();
    reset_for_tests();
    let _ = take_plugin_audit_events();
    let loader = PluginLoader::new();
    let bundle = PluginBundleManifest {
        bundle_id: "bundle.demo".to_string(),
        bundle_version: "0.1.0".to_string(),
        plugins: vec![sample_manifest()],
        signature: None,
        bundle_hash: Some("sha256:demo".to_string()),
        modules: Vec::new(),
        manifest_paths: Vec::new(),
    };
    let err = loader
        .register_bundle(bundle, VerificationPolicy::Strict)
        .expect_err("strict policy should reject missing signature");
    assert!(matches!(
        err,
        reml_runtime::runtime::plugin::PluginLoadError::SignatureMissing
    ));
}

#[test]
fn plugin_bundle_accepts_signature_in_strict_mode() {
    let _guard = reml_runtime::test_support::lock();
    reset_for_tests();
    let _ = take_plugin_audit_events();
    let loader = PluginLoader::new();
    let bundle = PluginBundleManifest {
        bundle_id: "bundle.demo".to_string(),
        bundle_version: "0.1.0".to_string(),
        plugins: vec![sample_manifest()],
        signature: Some(PluginSignature {
            algorithm: SignatureAlgorithm::Ed25519,
            certificate: Some("demo-cert".to_string()),
            issued_to: Some("plugin.demo".to_string()),
            valid_until: Some("2027-01-01T00:00:00Z".to_string()),
            bundle_hash: Some("sha256:demo".to_string()),
        }),
        bundle_hash: Some("sha256:demo".to_string()),
        modules: Vec::new(),
        manifest_paths: Vec::new(),
    };
    let registration = loader
        .register_bundle(bundle, VerificationPolicy::Strict)
        .expect("bundle registration should succeed");
    assert_eq!(registration.bundle_id, "bundle.demo");
    assert_eq!(registration.plugins.len(), 1);

    let events = take_plugin_audit_events();
    assert_eq!(events.len(), 2);
    let install = events
        .iter()
        .find(|event| {
            event
                .envelope
                .metadata
                .get("event.kind")
                .and_then(|value| value.as_str())
                == Some("plugin.install")
        })
        .expect("plugin.install event should be recorded");
    assert_eq!(
        install
            .envelope
            .metadata
            .get("plugin.bundle_id")
            .and_then(|value| value.as_str()),
        Some("bundle.demo")
    );
}
