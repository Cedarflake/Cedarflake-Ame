use super::{FileIdentityEvidence, PublicationGuardedFileDiscovery, ScanIssue};

impl PublicationGuardedFileDiscovery {
    /// Compare one directory through the pinned parent without enumerating entries.
    pub(crate) fn contains_directory_identity(
        &self,
        relative_path: &str,
        expected: &FileIdentityEvidence,
    ) -> Result<bool, ScanIssue> {
        #[cfg(windows)]
        {
            use super::{
                FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_READ_ATTRIBUTES,
                MetadataInventoryPlaceholderState, file_attribute_tag_info_from_handle,
                file_identity_from_handle, metadata_placeholder_state_from_attributes,
                open_root_relative_handle, validated_relative_path,
            };

            let relative = validated_relative_path(relative_path)?;
            let unreadable = |error: std::io::Error| ScanIssue {
                code: "directory_identity_unavailable".to_owned(),
                message: error.to_string(),
                path: Some(relative_path.to_owned()),
            };
            // Attribute-only directory opens use ADR 0024's proven native option set.
            let directory = open_root_relative_handle(
                &self.discovery.root_proof.handle,
                relative,
                FILE_READ_ATTRIBUTES,
                Some(true),
            )
            .map_err(unreadable)?;
            let attributes = file_attribute_tag_info_from_handle(&directory)
                .map_err(unreadable)?
                .FileAttributes;
            if attributes & FILE_ATTRIBUTE_DIRECTORY == 0
                || attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
                || metadata_placeholder_state_from_attributes(attributes)
                    != MetadataInventoryPlaceholderState::Available
            {
                return Ok(false);
            }
            let observed = file_identity_from_handle(&directory).map_err(unreadable)?;
            Ok(observed.as_ref() == Some(expected))
        }
        #[cfg(not(windows))]
        {
            let _ = expected;
            Err(ScanIssue {
                code: "directory_identity_unsupported".to_owned(),
                message: "Directory coverage requires a pinned Windows namespace".to_owned(),
                path: Some(relative_path.to_owned()),
            })
        }
    }
}
