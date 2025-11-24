use crate::capability_metadata::CapabilityDescriptor;

/// GC capability を表す型情報。
#[derive(Debug, Clone)]
pub struct GcCapability {
    descriptor: CapabilityDescriptor,
}

impl GcCapability {
    pub fn new(descriptor: CapabilityDescriptor) -> Self {
        Self { descriptor }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn descriptor_mut(&mut self) -> &mut CapabilityDescriptor {
        &mut self.descriptor
    }

    /// 将来の GC API 拡張用位置ヘルパ。
    pub fn collect(&self) {
        // stub: 実装では `gc.collect` 相当の呼び出しと監査ログを挿入
    }
}

/// I/O capability のプレースホルダー。
#[derive(Debug, Clone)]
pub struct IoCapability {
    descriptor: CapabilityDescriptor,
}

impl IoCapability {
    pub fn new(descriptor: CapabilityDescriptor) -> Self {
        Self { descriptor }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn descriptor_mut(&mut self) -> &mut CapabilityDescriptor {
        &mut self.descriptor
    }
}

/// 非同期ランタイム capability。
#[derive(Debug, Clone)]
pub struct AsyncRuntimeCapability {
    descriptor: CapabilityDescriptor,
}

impl AsyncRuntimeCapability {
    pub fn new(descriptor: CapabilityDescriptor) -> Self {
        Self { descriptor }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn descriptor_mut(&mut self) -> &mut CapabilityDescriptor {
        &mut self.descriptor
    }
}

/// 監査 capability。
#[derive(Debug, Clone)]
pub struct AuditCapability {
    descriptor: CapabilityDescriptor,
}

impl AuditCapability {
    pub fn new(descriptor: CapabilityDescriptor) -> Self {
        Self { descriptor }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn descriptor_mut(&mut self) -> &mut CapabilityDescriptor {
        &mut self.descriptor
    }
}

/// メトリクス capability。
#[derive(Debug, Clone)]
pub struct MetricsCapability {
    descriptor: CapabilityDescriptor,
}

impl MetricsCapability {
    pub fn new(descriptor: CapabilityDescriptor) -> Self {
        Self { descriptor }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn descriptor_mut(&mut self) -> &mut CapabilityDescriptor {
        &mut self.descriptor
    }
}

/// プラグイン capability。
#[derive(Debug, Clone)]
pub struct PluginCapability {
    descriptor: CapabilityDescriptor,
}

impl PluginCapability {
    pub fn new(descriptor: CapabilityDescriptor) -> Self {
        Self { descriptor }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn descriptor_mut(&mut self) -> &mut CapabilityDescriptor {
        &mut self.descriptor
    }
}

/// セキュリティ capability（監査・ポリシーの ABI を保証）。
#[derive(Debug, Clone)]
pub struct SecurityCapability {
    pub descriptor: CapabilityDescriptor,
}

impl SecurityCapability {
    pub fn new(descriptor: CapabilityDescriptor) -> Self {
        Self { descriptor }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn descriptor_mut(&mut self) -> &mut CapabilityDescriptor {
        &mut self.descriptor
    }
}

/// 可変参照用の capability。
#[derive(Debug, Clone)]
pub struct RefCapability {
    descriptor: CapabilityDescriptor,
}

impl RefCapability {
    pub fn new(descriptor: CapabilityDescriptor) -> Self {
        Self { descriptor }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn descriptor_mut(&mut self) -> &mut CapabilityDescriptor {
        &mut self.descriptor
    }
}

/// 型付き CapabilityHandle。
#[derive(Debug, Clone)]
pub enum CapabilityHandle {
    Gc(GcCapability),
    Io(IoCapability),
    Async(AsyncRuntimeCapability),
    Audit(AuditCapability),
    Metrics(MetricsCapability),
    Plugin(PluginCapability),
    Security(SecurityCapability),
    Ref(RefCapability),
}

impl CapabilityHandle {
    /// helper constructors
    pub fn gc(descriptor: CapabilityDescriptor) -> Self {
        CapabilityHandle::Gc(GcCapability::new(descriptor))
    }

    pub fn io(descriptor: CapabilityDescriptor) -> Self {
        CapabilityHandle::Io(IoCapability::new(descriptor))
    }

    pub fn async_runtime(descriptor: CapabilityDescriptor) -> Self {
        CapabilityHandle::Async(AsyncRuntimeCapability::new(descriptor))
    }

    pub fn audit(descriptor: CapabilityDescriptor) -> Self {
        CapabilityHandle::Audit(AuditCapability::new(descriptor))
    }

    pub fn metrics(descriptor: CapabilityDescriptor) -> Self {
        CapabilityHandle::Metrics(MetricsCapability::new(descriptor))
    }

    pub fn plugin(descriptor: CapabilityDescriptor) -> Self {
        CapabilityHandle::Plugin(PluginCapability::new(descriptor))
    }

    pub fn reference(descriptor: CapabilityDescriptor) -> Self {
        CapabilityHandle::Ref(RefCapability::new(descriptor))
    }

    pub fn security(descriptor: CapabilityDescriptor) -> Self {
        CapabilityHandle::Security(SecurityCapability::new(descriptor))
    }

    /// 登録済み Capability の descriptor へアクセス。
    pub fn descriptor(&self) -> &CapabilityDescriptor {
        match self {
            CapabilityHandle::Gc(cap) => cap.descriptor(),
            CapabilityHandle::Io(cap) => cap.descriptor(),
            CapabilityHandle::Async(cap) => cap.descriptor(),
            CapabilityHandle::Audit(cap) => cap.descriptor(),
            CapabilityHandle::Metrics(cap) => cap.descriptor(),
            CapabilityHandle::Plugin(cap) => cap.descriptor(),
            CapabilityHandle::Security(cap) => cap.descriptor(),
            CapabilityHandle::Ref(cap) => cap.descriptor(),
        }
    }

    /// Descriptor への可変参照。
    pub fn descriptor_mut(&mut self) -> &mut CapabilityDescriptor {
        match self {
            CapabilityHandle::Gc(cap) => cap.descriptor_mut(),
            CapabilityHandle::Io(cap) => cap.descriptor_mut(),
            CapabilityHandle::Async(cap) => cap.descriptor_mut(),
            CapabilityHandle::Audit(cap) => cap.descriptor_mut(),
            CapabilityHandle::Metrics(cap) => cap.descriptor_mut(),
            CapabilityHandle::Plugin(cap) => cap.descriptor_mut(),
            CapabilityHandle::Security(cap) => cap.descriptor_mut(),
            CapabilityHandle::Ref(cap) => cap.descriptor_mut(),
        }
    }

    /// GcCapability かどうかチェック。
    pub fn as_gc(&self) -> Option<&GcCapability> {
        match self {
            CapabilityHandle::Gc(cap) => Some(cap),
            _ => None,
        }
    }

    /// SecurityCapability かどうかチェック。
    pub fn as_security(&self) -> Option<&SecurityCapability> {
        match self {
            CapabilityHandle::Security(cap) => Some(cap),
            _ => None,
        }
    }

    /// IoCapability かどうかチェック。
    pub fn as_io(&self) -> Option<&IoCapability> {
        match self {
            CapabilityHandle::Io(cap) => Some(cap),
            _ => None,
        }
    }

    /// AsyncRuntimeCapability かどうかチェック。
    pub fn as_async(&self) -> Option<&AsyncRuntimeCapability> {
        match self {
            CapabilityHandle::Async(cap) => Some(cap),
            _ => None,
        }
    }

    /// RefCapability かどうかチェック。
    pub fn as_reference(&self) -> Option<&RefCapability> {
        match self {
            CapabilityHandle::Ref(cap) => Some(cap),
            _ => None,
        }
    }
}
