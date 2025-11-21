use std::collections::VecDeque;

mod filter;
mod map;

use super::{
    BufferStrategy, EffectSet, Iter, IterDriver, IterError, IterSource, IterStep,
    IteratorStageProfile,
};

pub(super) struct AdapterPlan<T> {
    label: &'static str,
    stage: IteratorStageProfile,
    effects: EffectSet,
    driver: IterDriver<T>,
}

impl<T> AdapterPlan<T> {
    pub(crate) fn new(
        label: &'static str,
        stage: IteratorStageProfile,
        effects: EffectSet,
        driver: IterDriver<T>,
    ) -> Self {
        Self {
            label,
            stage,
            effects,
            driver,
        }
    }

    pub(crate) fn build(self) -> Iter<T> {
        let AdapterPlan {
            label,
            stage,
            effects,
            driver,
        } = self;
        let source = IterSource::Adapter {
            label,
            stage: stage.clone(),
            _marker: std::marker::PhantomData,
        };
        Iter::with_source(source, stage, effects, driver)
    }
}

impl<T> Iter<T> {
    pub fn filter_map<U, F>(self, mut f: F) -> Iter<U>
    where
        F: FnMut(T) -> Option<U> + Send + 'static,
        T: Send + 'static,
        U: Send + 'static,
    {
        let (stage_profile, effects) = self.metadata_for_adapter();
        let effects = effects.with_mut().with_predicate_calls(1);
        let source = self;
        let driver = IterDriver::stepper(move |_effects| loop {
            match source.next_step() {
                IterStep::Ready(value) => {
                    if let Some(mapped) = f(value) {
                        return IterStep::Ready(mapped);
                    }
                }
                IterStep::Pending => return IterStep::Pending,
                IterStep::Finished => return IterStep::Finished,
                IterStep::Error(err) => return IterStep::Error(err),
            }
        });
        AdapterPlan::new("Iter::filter_map", stage_profile, effects, driver).build()
    }

    pub fn flat_map<U, F>(self, mut f: F) -> Iter<U>
    where
        F: FnMut(T) -> Iter<U> + Send + 'static,
        T: Send + 'static,
        U: Send + 'static,
    {
        let (stage_profile, effects) = self.metadata_for_adapter();
        let effects = effects.with_mem();
        let source = self;
        let mut current: Option<Iter<U>> = None;
        let driver = IterDriver::stepper(move |_effects| loop {
            if let Some(inner) = current.as_mut() {
                match inner.next_step() {
                    IterStep::Ready(value) => return IterStep::Ready(value),
                    IterStep::Pending => return IterStep::Pending,
                    IterStep::Finished => {
                        current = None;
                        continue;
                    }
                    IterStep::Error(err) => {
                        current = None;
                        return IterStep::Error(err);
                    }
                }
            }

            match source.next_step() {
                IterStep::Ready(value) => {
                    current = Some(f(value));
                }
                IterStep::Pending => return IterStep::Pending,
                IterStep::Finished => return IterStep::Finished,
                IterStep::Error(err) => return IterStep::Error(err),
            }
        });
        AdapterPlan::new("Iter::flat_map", stage_profile, effects, driver).build()
    }

    pub fn scan<S, U, F>(self, state: S, mut f: F) -> Iter<U>
    where
        F: FnMut(&mut S, T) -> Option<U> + Send + 'static,
        S: Send + 'static,
        T: Send + 'static,
        U: Send + 'static,
    {
        let (stage_profile, effects) = self.metadata_for_adapter();
        let source = self;
        let mut state = state;
        let driver = IterDriver::stepper(move |_effects| loop {
            match source.next_step() {
                IterStep::Ready(value) => {
                    if let Some(mapped) = f(&mut state, value) {
                        return IterStep::Ready(mapped);
                    }
                }
                IterStep::Pending => return IterStep::Pending,
                IterStep::Finished => return IterStep::Finished,
                IterStep::Error(err) => return IterStep::Error(err),
            }
        });
        AdapterPlan::new("Iter::scan", stage_profile, effects, driver).build()
    }

    pub fn take(self, count: usize) -> Iter<T>
    where
        T: Send + 'static,
    {
        let (stage_profile, effects) = self.metadata_for_adapter();
        let source = self;
        let mut remaining = count;
        let driver = IterDriver::stepper(move |_effects| {
            if remaining == 0 {
                return IterStep::Finished;
            }
            match source.next_step() {
                IterStep::Ready(value) => {
                    remaining = remaining.saturating_sub(1);
                    IterStep::Ready(value)
                }
                IterStep::Pending => IterStep::Pending,
                IterStep::Finished => IterStep::Finished,
                IterStep::Error(err) => IterStep::Error(err),
            }
        });
        AdapterPlan::new("Iter::take", stage_profile, effects, driver).build()
    }

    pub fn drop(self, count: usize) -> Iter<T>
    where
        T: Send + 'static,
    {
        let (stage_profile, effects) = self.metadata_for_adapter();
        let source = self;
        let mut to_skip = count;
        let driver = IterDriver::stepper(move |_effects| loop {
            if to_skip > 0 {
                match source.next_step() {
                    IterStep::Ready(_) => {
                        to_skip = to_skip.saturating_sub(1);
                        continue;
                    }
                    IterStep::Pending => return IterStep::Pending,
                    IterStep::Finished => return IterStep::Finished,
                    IterStep::Error(err) => return IterStep::Error(err),
                }
            } else {
                return source.next_step();
            }
        });
        AdapterPlan::new("Iter::drop", stage_profile, effects, driver).build()
    }

    pub fn enumerate(self) -> Iter<(usize, T)>
    where
        T: Send + 'static,
    {
        let (stage_profile, effects) = self.metadata_for_adapter();
        let source = self;
        let mut index: usize = 0;
        let driver = IterDriver::stepper(move |_effects| match source.next_step() {
            IterStep::Ready(value) => {
                let current = index;
                index = index.wrapping_add(1);
                IterStep::Ready((current, value))
            }
            IterStep::Pending => IterStep::Pending,
            IterStep::Finished => IterStep::Finished,
            IterStep::Error(err) => IterStep::Error(err),
        });
        AdapterPlan::new("Iter::enumerate", stage_profile, effects, driver).build()
    }

    pub fn zip<U>(self, other: Iter<U>) -> Iter<(T, U)>
    where
        T: Send + 'static,
        U: Send + 'static,
    {
        let (stage_profile, left_effects) = self.metadata_for_adapter();
        let (_, right_effects) = other.metadata_for_adapter();
        let combined_effects = left_effects.union(right_effects).with_mut();

        let left_iter = self;
        let right_iter = other;
        let mut left_cache: Option<T> = None;
        let mut right_cache: Option<U> = None;
        let driver = IterDriver::stepper(move |_effects| loop {
            if left_cache.is_none() {
                match left_iter.next_step() {
                    IterStep::Ready(value) => {
                        left_cache = Some(value);
                    }
                    IterStep::Pending => return IterStep::Pending,
                    IterStep::Finished => return IterStep::Finished,
                    IterStep::Error(err) => return IterStep::Error(err),
                }
            }

            if right_cache.is_none() {
                match right_iter.next_step() {
                    IterStep::Ready(value) => {
                        right_cache = Some(value);
                    }
                    IterStep::Pending => return IterStep::Pending,
                    IterStep::Finished => return IterStep::Finished,
                    IterStep::Error(err) => return IterStep::Error(err),
                }
            }

            if let (Some(left), Some(right)) = (left_cache.take(), right_cache.take()) {
                return IterStep::Ready((left, right));
            }
        });

        AdapterPlan::new("Iter::zip", stage_profile, combined_effects, driver).build()
    }

    pub fn buffered(self, capacity: usize, strategy: BufferStrategy) -> Iter<T>
    where
        T: Send + 'static,
    {
        let requested_capacity = capacity.max(1);
        let (stage_profile, base_effects) = self.metadata_for_adapter();
        let effects = base_effects.with_mem().with_mem_bytes(requested_capacity);
        let source = self;
        let mut buffer = VecDeque::with_capacity(requested_capacity);
        let mut current_capacity = requested_capacity;
        let driver = IterDriver::stepper(move |_effects| loop {
            if let Some(value) = buffer.pop_front() {
                return IterStep::Ready(value);
            }

            match source.next_step() {
                IterStep::Ready(value) => {
                    if buffer.len() >= current_capacity {
                        match strategy {
                            BufferStrategy::DropOldest => {
                                buffer.pop_front();
                                if buffer.len() >= current_capacity {
                                    return IterStep::Error(IterError::buffer_overflow(
                                        current_capacity,
                                        strategy,
                                    ));
                                }
                                buffer.push_back(value);
                            }
                            BufferStrategy::Grow => {
                                current_capacity =
                                    current_capacity.saturating_mul(2).max(current_capacity + 1);
                                buffer.push_back(value);
                            }
                        }
                    } else {
                        buffer.push_back(value);
                    }
                    continue;
                }
                IterStep::Pending => return IterStep::Pending,
                IterStep::Finished => return IterStep::Finished,
                IterStep::Error(err) => return IterStep::Error(err),
            }
        });

        AdapterPlan::new("Iter::buffered", stage_profile, effects, driver).build()
    }
}
