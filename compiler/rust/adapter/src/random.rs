use std::io;

use serde_json::{Map, Value};
use getrandom::getrandom;

use crate::capability::AdapterCapability;

const RANDOM_EFFECT_SCOPE: &[&str] = &["effect {io.random}", "effect {security}"];

/// 乱数 API の Capability。
pub const RANDOM_CAPABILITY: AdapterCapability = AdapterCapability::new(
    "adapter.random",
    "beta",
    RANDOM_EFFECT_SCOPE,
    "adapter.random",
);

/// OS の乱数源でバッファを埋める。
pub fn fill_random(buffer: &mut [u8]) -> io::Result<()> {
    getrandom(buffer).map_err(|err| {
        io::Error::new(io::ErrorKind::Other, err.to_string())
    })
}

/// 監査メタデータ。
pub fn audit_metadata(operation: &str, status: &str) -> Map<String, Value> {
    RANDOM_CAPABILITY.audit_metadata(operation, status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_exports_random_keys() {
        let metadata = audit_metadata("fill", "success");
        assert_eq!(metadata["capability.id"], "adapter.random");
        assert!(metadata["capability.effect_scope"].is_array());
    }

    #[test]
    fn fill_random_mutates_buffer() {
        let mut buf = [0u8; 16];
        fill_random(&mut buf).expect("random fill");
        assert!(buf.iter().any(|&b| b != 0));
    }
}
