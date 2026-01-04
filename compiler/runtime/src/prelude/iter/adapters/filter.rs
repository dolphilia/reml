use super::super::{Iter, IterDriver, IterStep, StageRequirement};
use super::AdapterPlan;

impl<T> Iter<T> {
    pub fn filter<F>(self, mut predicate: F) -> Iter<T>
    where
        F: FnMut(&T) -> bool + Send + 'static,
        T: Send + 'static,
    {
        let (stage_profile, mut effects) = self.metadata_for_adapter();
        let stage_profile = stage_profile
            .with_requirement(StageRequirement::Exact("stable"))
            .with_actual("stable");
        effects.mark_mut();
        let source = self;
        let driver = IterDriver::stepper(move |effects_state| loop {
            match source.next_step() {
                IterStep::Ready(value) => {
                    effects_state.mark_mut();
                    effects_state.record_predicate_call();
                    if predicate(&value) {
                        return IterStep::Ready(value);
                    }
                }
                IterStep::Pending => {
                    effects_state.mark_pending();
                    return IterStep::Pending;
                }
                IterStep::Finished => return IterStep::Finished,
                IterStep::Error(err) => return IterStep::Error(err),
            }
        });
        AdapterPlan::new("Iter::filter", stage_profile, effects, driver).build()
    }
}
