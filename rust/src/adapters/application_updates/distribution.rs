use crate::adapters::windows_package_identity::{STORE_PACKAGE_FAMILY, current_package_family};
use crate::domain::application_update::{ApplicationUpdateError, ApplicationUpdateFailureCode};
use crate::ports::application_updates::ApplicationUpdateSource;

use super::{WindowsApplicationUpdates, store::StoreApplicationUpdates};

pub(crate) fn open(
    version: String,
    build: u64,
) -> Result<Box<dyn ApplicationUpdateSource>, ApplicationUpdateError> {
    let family = current_package_family().map_err(identity_error)?;
    select(family.as_deref(), || {
        Ok(Box::new(WindowsApplicationUpdates::new(version, build)?))
    })
}

fn select(
    family: Option<&str>,
    direct: impl FnOnce() -> Result<Box<dyn ApplicationUpdateSource>, ApplicationUpdateError>,
) -> Result<Box<dyn ApplicationUpdateSource>, ApplicationUpdateError> {
    match family {
        None => direct(),
        Some(STORE_PACKAGE_FAMILY) => Ok(Box::new(StoreApplicationUpdates)),
        Some(_) => Err(identity_error("Unrecognized application package family")),
    }
}

pub(crate) fn startup() -> Result<(), ApplicationUpdateError> {
    if current_package_family().map_err(identity_error)?.is_none() {
        super::portable::startup();
    }
    Ok(())
}

fn identity_error(error: impl std::fmt::Display) -> ApplicationUpdateError {
    ApplicationUpdateError::new(
        ApplicationUpdateFailureCode::Unavailable,
        format!("Package identity: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::application_update::{ApplicationUpdateChannel, ApplicationUpdateDelivery};

    #[test]
    fn packaged_execution_never_constructs_the_direct_updater() {
        let store = select(Some(STORE_PACKAGE_FAMILY), || {
            panic!("Direct updater constructed")
        })
        .unwrap();
        assert_eq!(store.delivery(), ApplicationUpdateDelivery::MicrosoftStore);
        assert!(store.take_result().unwrap().is_none());
        assert!(store.check(ApplicationUpdateChannel::Preview).is_err());
        assert!(
            select(Some("Other.Application_unknown"), || panic!(
                "Direct updater constructed"
            ))
            .is_err()
        );
    }

    #[test]
    fn unpackaged_execution_retains_the_direct_factory_result() {
        let result = select(None, || Err(identity_error("direct factory reached")));
        assert_eq!(
            result.err().unwrap().diagnostic,
            "Package identity: direct factory reached"
        );
    }
}
