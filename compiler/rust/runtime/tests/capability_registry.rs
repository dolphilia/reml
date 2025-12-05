use reml_runtime::capability::registry::CapabilityRegistry;
use static_assertions::assert_impl_all;

#[test]
fn capability_registry_traits() {
    assert_impl_all!(CapabilityRegistry: Send, Sync);
    assert!(std::ptr::eq(
        CapabilityRegistry::registry(),
        CapabilityRegistry::registry()
    ));
}
