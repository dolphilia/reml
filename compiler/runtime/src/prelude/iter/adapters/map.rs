use super::super::{Iter, IterDriver, IterStep, StageRequirement};
use super::AdapterPlan;

impl<T> Iter<T> {
    pub fn map<U, F>(self, mut transform: F) -> Iter<U>
    where
        F: FnMut(T) -> U + Send + 'static,
        T: Send + 'static,
        U: Send + 'static,
    {
        let (stage_profile, effects) = self.metadata_for_adapter();
        let stage_profile = stage_profile
            .with_requirement(StageRequirement::Exact("stable"))
            .with_actual("stable");
        let source = self;
        let driver = IterDriver::stepper(move |effects_state| match source.next_step() {
            IterStep::Ready(value) => IterStep::Ready(transform(value)),
            IterStep::Pending => {
                effects_state.mark_pending();
                IterStep::Pending
            }
            IterStep::Finished => IterStep::Finished,
            IterStep::Error(err) => IterStep::Error(err),
        });
        AdapterPlan::new("Iter::map", stage_profile, effects, driver).build()
    }
}
