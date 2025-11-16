use crate::{
    ffi_lowering::FfiCallSignature,
    target_machine::{TargetMachine, Triple},
};

#[derive(Clone, Debug)]
pub struct BridgeStubMetadata {
    stub_index: usize,
    extern_name: String,
    stub_symbol: String,
    thunk_symbol: String,
    target: String,
    platform: String,
    callconv: String,
    abi: String,
    ownership: String,
    return_wrap: String,
    return_release_handler: String,
    return_rc_adjustment: String,
    extras: Vec<(String, String)>,
}

impl BridgeStubMetadata {
    fn from_call(index: usize, call: &FfiCallSignature, target: &str, platform: &str) -> Self {
        let sanitized = Self::sanitize_symbol(&call.name);
        let stub_symbol = format!("reml_bridge_stub_{}_{}", sanitized, index + 1);
        let thunk_symbol = format!("reml_bridge_thunk_{}_{}", sanitized, index + 1);
        let callconv = call.calling_conv.clone();
        let abi = Self::infer_abi(&callconv);
        Self {
            stub_index: index,
            extern_name: call.name.clone(),
            stub_symbol,
            thunk_symbol,
            target: target.to_string(),
            platform: platform.to_string(),
            callconv,
            abi,
            ownership: "borrowed".into(),
            return_wrap: "wrap_foreign_ptr".into(),
            return_release_handler: "none".into(),
            return_rc_adjustment: "none".into(),
            extras: Vec::new(),
        }
    }

    pub fn field_pairs(&self) -> Vec<(String, String)> {
        let mut fields = Vec::new();
        fields.push((
            "bridge.stub_index".into(),
            (self.stub_index + 1).to_string(),
        ));
        fields.push(("bridge.extern_name".into(), self.extern_name.clone()));
        fields.push(("bridge.stub_symbol".into(), self.stub_symbol.clone()));
        fields.push(("bridge.thunk_symbol".into(), self.thunk_symbol.clone()));
        fields.push(("bridge.target".into(), self.target.clone()));
        fields.push(("bridge.platform".into(), self.platform.clone()));
        fields.push(("bridge.callconv".into(), self.callconv.clone()));
        fields.push(("bridge.abi".into(), self.abi.clone()));
        fields.push(("bridge.ownership".into(), self.ownership.clone()));
        fields.push(("bridge.return.ownership".into(), self.ownership.clone()));
        fields.push(("bridge.return.wrap".into(), self.return_wrap.clone()));
        fields.push((
            "bridge.return.release_handler".into(),
            self.return_release_handler.clone(),
        ));
        fields.push((
            "bridge.return.rc_adjustment".into(),
            self.return_rc_adjustment.clone(),
        ));
        for (key, value) in &self.extras {
            fields.push((key.clone(), value.clone()));
        }
        fields
            .into_iter()
            .map(|(k, v)| (k, Self::sanitize_value(&v)))
            .collect()
    }

    fn snapshot_entries(&self) -> Vec<String> {
        let mut entries = Vec::new();
        let prefix = format!("reml.bridge.stubs[{}]", self.stub_index + 1);
        for (key, value) in self.field_pairs() {
            entries.push(format!("{}.{}={}", prefix, key, value));
        }
        entries
    }

    fn sanitize_symbol(value: &str) -> String {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            "stub".into()
        } else {
            trimmed
                .chars()
                .map(|ch| {
                    if ch.is_alphanumeric() || ch == '_' {
                        ch
                    } else {
                        '_'
                    }
                })
                .collect()
        }
    }

    fn infer_abi(callconv: &str) -> String {
        match callconv.to_lowercase().as_str() {
            "win64" | "msvc" => "msvc".into(),
            "aarch64_aapcscc" | "aapcs64" | "arm_aapcscc" | "darwin" => "darwin_aapcs64".into(),
            "ccc" | "system_v" | "systemv" => "system_v".into(),
            other => other.into(),
        }
    }

    fn sanitize_value(value: &str) -> String {
        value.trim().replace('\n', " ").replace('\r', " ")
    }
}

#[derive(Clone, Debug)]
pub struct BridgeMetadataContext {
    target: String,
    platform: String,
    stubs: Vec<BridgeStubMetadata>,
    next_index: usize,
}

impl BridgeMetadataContext {
    pub const VERSION: u32 = 1;

    pub fn new(target_machine: &TargetMachine) -> Self {
        let triple = target_machine.triple;
        Self {
            target: triple.to_string(),
            platform: platform_label(triple).to_string(),
            stubs: Vec::new(),
            next_index: 0,
        }
    }

    pub fn record_stub(&mut self, call: &FfiCallSignature) {
        let stub =
            BridgeStubMetadata::from_call(self.next_index, call, &self.target, &self.platform);
        self.next_index += 1;
        self.stubs.push(stub);
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn platform(&self) -> &str {
        &self.platform
    }

    pub fn has_stubs(&self) -> bool {
        !self.stubs.is_empty()
    }

    pub fn stub_count(&self) -> usize {
        self.stubs.len()
    }

    pub fn snapshot_entries(&self) -> Vec<String> {
        if self.stubs.is_empty() {
            return Vec::new();
        }
        let mut entries = Vec::new();
        entries.push(format!("reml.bridge.version={}", Self::VERSION));
        for stub in &self.stubs {
            entries.extend(stub.snapshot_entries());
        }
        entries
    }

    pub fn audit_entries(&self) -> Vec<(String, String)> {
        if self.stubs.is_empty() {
            return Vec::new();
        }
        let mut entries = Vec::new();
        entries.push(("audit.bridge.version".into(), Self::VERSION.to_string()));
        for stub_line in self.snapshot_entries().into_iter().skip(1) {
            entries.push(("audit.bridge.stub".into(), stub_line));
        }
        entries
    }
}

fn platform_label(triple: Triple) -> &'static str {
    match triple {
        Triple::LinuxGNU => "linux-x86_64",
        Triple::AppleDarwin => "macos-arm64",
        Triple::WindowsGNU | Triple::WindowsMSVC => "windows-msvc-x64",
    }
}
