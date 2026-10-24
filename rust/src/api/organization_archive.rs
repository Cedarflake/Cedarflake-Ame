use crate::domain::ScanError;
use crate::domain::organization_archive::{
    OrganizationArchiveEvent, OrganizationArchiveRestoreRequest,
};
use crate::frb_generated::StreamSink;

pub fn export_organization_archive(
    destination: String,
    sink: StreamSink<OrganizationArchiveEvent>,
) -> Result<(), ScanError> {
    let result = crate::application::organization_archive::export(&destination, &mut |progress| {
        sink.add(OrganizationArchiveEvent::Progress { progress })
            .is_ok()
    });
    send_result(
        result.map(|receipt| OrganizationArchiveEvent::Completed { receipt }),
        &sink,
    );
    Ok(())
}

pub fn inspect_organization_archive(
    archive: String,
    sink: StreamSink<OrganizationArchiveEvent>,
) -> Result<(), ScanError> {
    let result = crate::application::organization_archive::inspect(&archive, &mut |progress| {
        sink.add(OrganizationArchiveEvent::Progress { progress })
            .is_ok()
    });
    send_result(
        result.map(|inspection| OrganizationArchiveEvent::Inspected { inspection }),
        &sink,
    );
    Ok(())
}

pub fn restore_organization_archive(
    archive: String,
    request: OrganizationArchiveRestoreRequest,
    sink: StreamSink<OrganizationArchiveEvent>,
) -> Result<(), ScanError> {
    let result =
        crate::application::organization_archive::restore(&archive, &request, &mut |progress| {
            sink.add(OrganizationArchiveEvent::Progress { progress })
                .is_ok()
        });
    send_result(
        result.map(|receipt| OrganizationArchiveEvent::Restored { receipt }),
        &sink,
    );
    Ok(())
}

fn send_result(
    result: Result<OrganizationArchiveEvent, ScanError>,
    sink: &StreamSink<OrganizationArchiveEvent>,
) {
    let event = match result {
        Ok(event) => event,
        Err(error) if error.code == "organization_archive_cancelled" => {
            OrganizationArchiveEvent::Cancelled
        }
        Err(error) => OrganizationArchiveEvent::Failed {
            code: error.code,
            message: error.message,
        },
    };
    let _ = sink.add(event);
}
