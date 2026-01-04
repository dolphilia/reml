#![cfg(feature = "core_prelude")]

use std::sync::Once;

use crate::{
    capability_handle::CapabilityHandle,
    capability_metadata::{CapabilityDescriptor, CapabilityProvider, StageId},
    registry::CapabilityRegistry,
};

const TABLE_CSV_CAPABILITY_ID: &str = "core.collections.table.csv_load";

pub fn register_table_csv_capability() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        let descriptor = CapabilityDescriptor::new(
            TABLE_CSV_CAPABILITY_ID,
            StageId::Stable,
            vec!["io".into(), "mut".into(), "mem".into()],
            CapabilityProvider::RuntimeComponent {
                name: TABLE_CSV_CAPABILITY_ID.into(),
            },
        );
        let handle = CapabilityHandle::io(descriptor);
        let _ = CapabilityRegistry::registry().register(handle);
    });
}
