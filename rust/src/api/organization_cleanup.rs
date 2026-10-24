use crate::domain::ScanError;
use crate::domain::organization_cleanup::{OrganizationCleanupEvent, OrganizationCleanupRequest};
use crate::frb_generated::StreamSink;

#[flutter_rust_bridge::frb(opaque)]
pub struct OrganizationCleanupTask {
    control: crate::application::organization_cleanup::CleanupTaskControl,
}

#[flutter_rust_bridge::frb(sync)]
pub fn create_organization_cleanup_task() -> OrganizationCleanupTask {
    OrganizationCleanupTask {
        control: Default::default(),
    }
}

impl OrganizationCleanupTask {
    #[flutter_rust_bridge::frb(sync)]
    pub fn cancel(&self) {
        self.control.cancel();
    }
}

pub fn inspect_organization_cleanup(
    cutoff_unix_ms: Option<i64>,
    task: &OrganizationCleanupTask,
    sink: StreamSink<OrganizationCleanupEvent>,
) -> Result<(), ScanError> {
    let result = crate::application::organization_cleanup::inspect(
        cutoff_unix_ms,
        &task.control,
        &mut |progress| {
            sink.add(OrganizationCleanupEvent::Progress { progress })
                .is_ok()
        },
    );
    send_result(
        result.map(|inspection| OrganizationCleanupEvent::Inspected { inspection }),
        &sink,
    );
    Ok(())
}

pub fn apply_organization_cleanup(
    request: OrganizationCleanupRequest,
    task: &OrganizationCleanupTask,
    sink: StreamSink<OrganizationCleanupEvent>,
) -> Result<(), ScanError> {
    let result =
        crate::application::organization_cleanup::apply(&request, &task.control, &mut |progress| {
            sink.add(OrganizationCleanupEvent::Progress { progress })
                .is_ok()
        });
    send_result(
        result.map(|receipt| OrganizationCleanupEvent::Completed { receipt }),
        &sink,
    );
    Ok(())
}

fn send_result(
    result: Result<OrganizationCleanupEvent, ScanError>,
    sink: &StreamSink<OrganizationCleanupEvent>,
) {
    let event = match result {
        Ok(event) => event,
        Err(error) if error.code == "organization_cleanup_cancelled" => {
            OrganizationCleanupEvent::Cancelled
        }
        Err(error) => OrganizationCleanupEvent::Failed {
            code: error.code,
            message: error.message,
        },
    };
    let _ = sink.add(event);
}
