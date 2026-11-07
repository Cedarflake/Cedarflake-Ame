use std::time::Duration;

use crate::domain::ScanError;
use crate::domain::classification::maintenance::CategoryMaintenancePlan;
use crate::ports::classification_execution::ClassificationWorkExecutor;

use super::super::classification_dispatch::{
    ClassificationDispatchEvent, ClassificationDispatchStep,
};
use super::NativeClassificationRuntime;

pub(super) fn run(
    runtime: &mut NativeClassificationRuntime,
) -> Result<Option<CategoryMaintenancePlan>, ScanError> {
    let signals = runtime.control.signals();
    let executor = runtime.dispatcher.resources().1;
    executor.feature_resources()?;
    let scheduler = executor.scheduler();
    let mut search = super::interactive::SemanticSearchWorker::start(
        executor.search_executor(),
        runtime.control.search.clone(),
        signals.clone(),
    )?;
    let mut maintenance = super::discovery::CategoryDiscovery::new(super::production::vocabulary);
    let mut semantic = super::semantic::SemanticExecution::default();
    let mut index = super::indexing::SemanticIndexPass::default();
    let outcome = (|| loop {
        let observed = signals.notification()?;
        if signals.is_closed()? {
            return Ok(None);
        }
        if let Some(delay) = scheduler.background_delay()? {
            runtime.dispatcher.resources().1.retire_if_idle()?;
            signals.wait(observed, Some(delay))?;
            continue;
        }
        let indexing = {
            let (catalog, executor) = runtime.dispatcher.resources();
            index.step(
                catalog,
                executor,
                &mut semantic,
                &signals,
                &runtime.control.search,
            )?
        };
        match runtime.dispatcher.step_with_maintenance(&mut maintenance)? {
            ClassificationDispatchEvent::Step(ClassificationDispatchStep::Progress) => {}
            ClassificationDispatchEvent::Step(ClassificationDispatchStep::Waiting {
                wake_after,
            }) => {
                if !indexing {
                    signals.wait(observed, wake_after)?;
                }
            }
            ClassificationDispatchEvent::Step(ClassificationDispatchStep::Closed) => {
                return Ok(None);
            }
            ClassificationDispatchEvent::Maintenance(plan) => return Ok(Some(plan)),
        }
        if runtime.dispatcher.resources().1.take_yielded() {
            signals.wait(observed, Some(Duration::from_millis(25)))?;
        }
    })();
    drop(maintenance);
    let search_stopped = search.stop();
    let models_stopped = scheduler.retire();
    let verification_finished = semantic.finish(runtime.dispatcher.resources().0);
    search_stopped?;
    models_stopped?;
    verification_finished?;
    outcome
}
