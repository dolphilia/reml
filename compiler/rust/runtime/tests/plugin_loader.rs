use reml_runtime::{
    capability::{registry::reset_for_tests, CapabilityProvider, CapabilityRegistry},
    config::manifest::{
        Manifest, PackageName, ProjectKind, ProjectSection, RunCapabilityEntry, RunSection,
        RunTargetSection, SemanticVersion,
    },
    runtime::plugin::PluginLoader,
};

fn sample_manifest() -> Manifest {
    Manifest {
        project: ProjectSection {
            name: PackageName("plugin.demo".to_string()),
            version: SemanticVersion("1.2.3".to_string()),
            kind: ProjectKind::Plugin,
            ..ProjectSection::default()
        },
        run: RunSection {
            target: RunTargetSection {
                capabilities: vec![RunCapabilityEntry {
                    id: "plugin.demo.audit".into(),
                    stage: Some("beta".to_string()),
                    declared_effects: vec!["audit".to_string()],
                    source_span: None,
                    provider: None,
                }],
            },
        },
        ..Manifest::default()
    }
}

#[test]
fn plugin_loader_registers_manifest_capabilities() {
    reset_for_tests();
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
    match descriptor.metadata().provider {
        CapabilityProvider::Plugin { ref package, ref version } => {
            assert_eq!(package, "plugin.demo");
            assert_eq!(version.as_deref(), Some("1.2.3"));
        }
        other => panic!("unexpected provider: {other:?}"),
    }
}
