#![cfg(feature = "core_prelude")]

use std::sync::Once;

use crate::{
    capability_handle::CapabilityHandle,
    capability_metadata::{CapabilityDescriptor, CapabilityProvider, StageId},
    collections::mutable::{BorrowError, EffectfulRef, Ref, RefGuard, RefMutGuard},
    core_prelude::iter::{EffectLabels, EffectSet},
    registry::CapabilityRegistry,
};

const REF_CAPABILITY_ID: &str = "core.collections.ref";

/// FFI 向けの `Ref` ハンドル。
pub struct RefHandle<T> {
    inner: EffectfulRef<T>,
}

impl<T> RefHandle<T> {
    /// 内部可変性付きの `Ref` を構築する。
    pub fn new(value: T) -> Self {
        Self::try_new(value).expect("core.collections.ref capability verification failed")
    }

    /// Capability 検証込みで `Ref` を構築する。
    pub fn try_new(value: T) -> Result<Self, BorrowError> {
        register_ref_capability();
        EffectfulRef::try_new(value).map(|inner| Self { inner })
    }

    /// 共有借用を取得する。
    pub fn borrow(&self) -> Result<RefGuard<'_, T>, BorrowError> {
        self.inner.borrow()
    }

    /// 排他的借用を取得する。
    pub fn borrow_mut(&self) -> Result<RefMutGuard<'_, T>, BorrowError> {
        self.inner.borrow_mut()
    }

    /// 排他的借用を試行する。
    pub fn try_borrow_mut(&self) -> Result<Option<RefMutGuard<'_, T>>, BorrowError> {
        self.inner.try_borrow_mut()
    }

    /// 効果ラベルを取得する。
    pub fn effect_labels(&self) -> EffectLabels {
        self.inner.effect_labels()
    }

    /// 内包 `Ref` と `EffectSet` を取り出す。
    pub fn into_parts(self) -> (Ref<T>, EffectSet) {
        self.inner.into_parts()
    }

    /// 内包値を単独所有で取得する。
    pub fn into_inner(self) -> Result<T, BorrowError> {
        self.inner.into_inner()
    }
}

impl<T> Clone for RefHandle<T> {
    fn clone(&self) -> Self {
        register_ref_capability();
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// `core.collections.ref` Capability を登録しておく。
pub fn register_ref_capability() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        let descriptor = CapabilityDescriptor::new(
            REF_CAPABILITY_ID,
            StageId::Stable,
            vec!["mut".into(), "rc".into(), "mem".into()],
            CapabilityProvider::RuntimeComponent {
                name: REF_CAPABILITY_ID.into(),
            },
        );
        let handle = CapabilityHandle::reference(descriptor);
        let _ = CapabilityRegistry::registry().register(handle);
    });
}
