use std::{
    cell::Cell as EffectCell,
    error::Error,
    fmt,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError},
};

use once_cell::sync::OnceCell;

#[cfg(feature = "core_prelude")]
use crate::core_prelude::iter::{EffectLabels, EffectSet};
#[cfg(not(feature = "core_prelude"))]
use crate::prelude::iter::{EffectLabels, EffectSet};
use crate::{
    capability::registry::{CapabilityError, CapabilityRegistry},
    stage::{StageId, StageRequirement},
};

const CORE_COLLECTIONS_REF_CAPABILITY: &str = "core.collections.ref";
const REF_STAGE_REQUIREMENT: StageRequirement = StageRequirement::Exact(StageId::Stable);
const REF_REQUIRED_EFFECTS: [&str; 3] = ["mem", "mut", "rc"];
static REF_CAPABILITY_STAGE: OnceCell<Result<StageId, CapabilityError>> = OnceCell::new();

fn ensure_ref_capability_stage() -> Result<StageId, CapabilityError> {
    REF_CAPABILITY_STAGE
        .get_or_init(|| {
            let registry = CapabilityRegistry::registry();
            let required_effects = REF_REQUIRED_EFFECTS
                .iter()
                .map(|tag| tag.to_string())
                .collect::<Vec<_>>();
            registry.verify_capability_stage(
                CORE_COLLECTIONS_REF_CAPABILITY,
                REF_STAGE_REQUIREMENT,
                &required_effects,
            )
        })
        .clone()
}

fn ensure_ref_capability() -> Result<(), BorrowError> {
    ensure_ref_capability_stage()
        .map(|_| ())
        .map_err(Into::into)
}

struct RefInner<T> {
    value: RwLock<T>,
}

impl<T> RefInner<T> {
    fn new(value: T) -> Self {
        Self {
            value: RwLock::new(value),
        }
    }
}

/// 共有参照を表すハンドル。
pub struct Ref<T> {
    inner: Arc<RefInner<T>>,
}

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Ref<T> {
    /// 新しい `Ref` を生成する。
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RefInner::new(value)),
        }
    }

    /// 共有読み取り借用を取得する。
    pub fn borrow(&self) -> Result<RefGuard<'_, T>, BorrowError> {
        match self.inner.value.read() {
            Ok(guard) => Ok(RefGuard { guard }),
            Err(_) => Err(BorrowError::Poisoned("Ref::borrow")),
        }
    }

    /// 排他的借用を取得する。
    pub fn borrow_mut(&self) -> Result<RefMutGuard<'_, T>, BorrowError> {
        match self.inner.value.write() {
            Ok(guard) => Ok(RefMutGuard { guard }),
            Err(_) => Err(BorrowError::Poisoned("Ref::borrow_mut")),
        }
    }

    /// できる限り排他的借用を試みる。
    pub fn try_borrow_mut(&self) -> Result<Option<RefMutGuard<'_, T>>, BorrowError> {
        match self.inner.value.try_write() {
            Ok(guard) => Ok(Some(RefMutGuard { guard })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Poisoned(_)) => Err(BorrowError::Poisoned("Ref::try_borrow_mut")),
        }
    }

    /// 参照カウントを返す。
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// 参照を単独所有状態へ戻し中身を取り出す。
    pub fn into_inner(self) -> Result<T, BorrowError> {
        match Arc::try_unwrap(self.inner) {
            Ok(inner) => match inner.value.into_inner() {
                Ok(value) => Ok(value),
                Err(_) => Err(BorrowError::Poisoned("Ref::into_inner")),
            },
            Err(_) => Err(BorrowError::BorrowConflict("Ref::into_inner")),
        }
    }
}

impl<T> fmt::Debug for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ref")
            .field("strong_count", &self.strong_count())
            .finish()
    }
}

/// 読み取りガード。
pub struct RefGuard<'a, T> {
    guard: RwLockReadGuard<'a, T>,
}

impl<'a, T> Deref for RefGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

/// 排他的ガード。
pub struct RefMutGuard<'a, T> {
    guard: RwLockWriteGuard<'a, T>,
}

impl<'a, T> Deref for RefMutGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> DerefMut for RefMutGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

/// 借用失敗を示すエラー。
#[derive(Debug)]
pub enum BorrowError {
    Poisoned(&'static str),
    BorrowConflict(&'static str),
    CapabilityDenied(CapabilityError),
}

impl fmt::Display for BorrowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Poisoned(op) => write!(f, "{op} lock poisoned"),
            Self::BorrowConflict(op) => write!(f, "{op} borrow conflict"),
            Self::CapabilityDenied(err) => write!(f, "{err}"),
        }
    }
}

impl Error for BorrowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CapabilityDenied(err) => Some(err),
            _ => None,
        }
    }
}

impl From<CapabilityError> for BorrowError {
    fn from(value: CapabilityError) -> Self {
        BorrowError::CapabilityDenied(value)
    }
}

/// 効果計測付きの共有参照。
pub struct EffectfulRef<T> {
    handle: Ref<T>,
    effects: EffectCell<EffectSet>,
}

impl<T> EffectfulRef<T> {
    /// 新しいハンドルを構築する。
    pub fn new(value: T) -> Self {
        Self::try_new(value).expect("core.collections.ref capability verification failed")
    }

    /// Capability を検証したうえで新しいハンドルを構築する。
    pub fn try_new(value: T) -> Result<Self, BorrowError> {
        ensure_ref_capability()?;
        Ok(Self {
            handle: Ref::new(value),
            effects: EffectCell::new(EffectSet::PURE),
        })
    }

    fn with_effects(&self, update: impl FnOnce(&mut EffectSet)) {
        let mut effects = self.effects.get();
        update(&mut effects);
        self.effects.set(effects);
    }

    fn record_clone(&self) {
        self.with_effects(|effects| {
            effects.mark_mem();
            effects.mark_rc();
        });
    }

    fn record_rc(&self) {
        self.with_effects(|effects| {
            effects.mark_rc();
        });
    }

    fn record_mut(&self) {
        self.with_effects(|effects| {
            effects.mark_mut();
            effects.mark_rc();
        });
    }

    /// 読み取りガードを取得する。
    pub fn borrow(&self) -> Result<RefGuard<'_, T>, BorrowError> {
        let guard = self.handle.borrow()?;
        self.record_rc();
        Ok(guard)
    }

    /// 排他的ガードを取得する（effect を記録）。
    pub fn borrow_mut(&self) -> Result<RefMutGuard<'_, T>, BorrowError> {
        let guard = self.handle.borrow_mut()?;
        self.record_mut();
        Ok(guard)
    }

    /// 排他的借用を試みる。
    pub fn try_borrow_mut(&self) -> Result<Option<RefMutGuard<'_, T>>, BorrowError> {
        match self.handle.try_borrow_mut()? {
            Some(guard) => {
                self.record_mut();
                Ok(Some(guard))
            }
            None => Ok(None),
        }
    }

    /// 効果ラベルを返す。
    pub fn effect_labels(&self) -> EffectLabels {
        self.effects.get().to_labels()
    }

    /// 内部 `Ref` を取得する。
    pub fn into_parts(self) -> (Ref<T>, EffectSet) {
        let this = ManuallyDrop::new(self);
        let handle = unsafe { ptr::read(&this.handle) };
        let effects = unsafe { ptr::read(&this.effects) };
        (handle, effects.into_inner())
    }

    /// 内包値を単独所有で取得する。
    pub fn into_inner(self) -> Result<T, BorrowError> {
        let this = ManuallyDrop::new(self);
        let handle = unsafe { ptr::read(&this.handle) };
        handle.into_inner()
    }
}

impl<T> Clone for EffectfulRef<T> {
    fn clone(&self) -> Self {
        self.record_clone();
        Self {
            handle: self.handle.clone(),
            effects: EffectCell::new(self.effects.get()),
        }
    }
}

impl<T> Drop for EffectfulRef<T> {
    fn drop(&mut self) {
        self.with_effects(|effects| effects.release_rc());
    }
}
