use crate::domain::ScanError;
use crate::domain::organization_archive::OrganizationArchiveEvent;
use crate::frb_generated::StreamSink;

pub fn export_organization_archive(
    destination: String,
    sink: StreamSink<OrganizationArchiveEvent>,
) -> Result<(), ScanError> {
    let result = crate::application::organization_archive::export(&destination, &mut |progress| {
        sink.add(OrganizationArchiveEvent::Progress { progress })
            .is_ok()
    });
    let event = match result {
        Ok(receipt) => OrganizationArchiveEvent::Completed { receipt },
        Err(error) if error.code == "organization_archive_cancelled" => {
            OrganizationArchiveEvent::Cancelled
        }
        Err(error) => OrganizationArchiveEvent::Failed {
            code: error.code,
            message: error.message,
        },
    };
    let _ = sink.add(event);
    Ok(())
}
