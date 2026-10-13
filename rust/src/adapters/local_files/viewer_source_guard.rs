use windows_sys::Win32::System::SystemServices::{IO_REPARSE_TAG_CLOUD, IO_REPARSE_TAG_CLOUD_MASK};

use super::*;

const MAX_RELATIVE_PATH_UNITS: usize = 32_767;
const MAX_RELATIVE_COMPONENTS: usize = 256;

pub(crate) fn open_viewer_source_guard(
    expected: &ExpectedFileState,
    source_root: &Path,
    relative_path: &Path,
    root_identity: &FileIdentityEvidence,
) -> Result<(String, PreviewPublicationGuard), ScanIssue> {
    let fail = |error: std::io::Error| ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: "viewer_source_guard_failed".to_owned(),
        message: error.to_string(),
    };
    if relative_path
        .as_os_str()
        .encode_wide()
        .take(MAX_RELATIVE_PATH_UNITS + 1)
        .count()
        > MAX_RELATIVE_PATH_UNITS
    {
        return Err(fail(std::io::Error::other(
            "The viewer relative path exceeds the Windows path limit",
        )));
    }
    let components = relative_path
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err(std::io::Error::other(
                "The viewer source is not a root-relative normal path",
            )),
        })
        .take(MAX_RELATIVE_COMPONENTS + 1)
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(fail)?;
    if components.len() > MAX_RELATIVE_COMPONENTS {
        return Err(fail(std::io::Error::other(
            "The viewer source exceeds the bounded namespace depth",
        )));
    }
    let Some((file_name, parents)) = components.split_last() else {
        return Err(fail(std::io::Error::other(
            "The viewer source has no file component",
        )));
    };
    let (root_proof, mut source_path, mut guards) =
        WindowsDirectoryRootProof::open_publication_namespace(source_root, Some(root_identity))
            .map_err(fail)?;
    let mut descendants = Vec::<File>::with_capacity(parents.len());
    for component in parents {
        let parent = descendants.last().unwrap_or(&root_proof.handle);
        let case_sensitive = directory_is_case_sensitive(parent).map_err(fail)?;
        let child = nt_create_root_relative_handle_with_case_semantics(
            parent,
            component,
            FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            Some(true),
            false,
            false,
            case_sensitive,
        )
        .map_err(fail)?;
        validate_viewer_directory(&child, root_proof.volume_serial).map_err(fail)?;
        validate_publication_namespace_child(parent, &child, component, case_sensitive)
            .map_err(fail)?;
        source_path.push(component);
        // The relative handle pins the name before the read-only namespace lease is opened.
        // Deny data-write and delete access during the bounded source read. Windows
        // sharing does not restrict attribute-only access; availability is checked separately.
        let protected = OpenOptions::new()
            .access_mode(FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&source_path)
            .map_err(fail)?;
        if raw_file_id_info(&protected)
            .map_err(fail)?
            .FileId
            .Identifier
            != raw_file_id_info(&child).map_err(fail)?.FileId.Identifier
        {
            return Err(fail(std::io::Error::other(
                "The viewer directory changed before protection",
            )));
        }
        validate_viewer_directory(&protected, root_proof.volume_serial).map_err(fail)?;
        validate_publication_namespace_child(parent, &protected, component, case_sensitive)
            .map_err(fail)?;
        descendants.push(protected);
    }
    let parent = descendants.last().unwrap_or(&root_proof.handle);
    let case_sensitive = directory_is_case_sensitive(parent).map_err(fail)?;
    source_path.push(file_name);
    let file = OpenOptions::new()
        .access_mode(FILE_READ_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&source_path)
        .map_err(fail)?;
    let info = file_attribute_tag_info_from_handle(&file).map_err(fail)?;
    if info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return Err(fail(std::io::Error::other(
            "The viewer source is a directory",
        )));
    }
    validate_source_file_info(&AttributeTagEvidence {
        attributes: info.FileAttributes,
        reparse_tag: info.ReparseTag,
    })
    .map_err(fail)?;
    validate_publication_namespace_child(parent, &file, file_name, case_sensitive).map_err(fail)?;
    validate_handle_within_root(&file, &root_proof).map_err(fail)?;
    validate_publication_namespace_handle_path(&file, Path::new(&expected.absolute_path))
        .map_err(fail)?;
    revalidate_open_preview_source(&file, expected)?;
    // Rebind durable root authority while every source-relative namespace component is held.
    let (current_root, _) =
        WindowsDirectoryRootProof::open_configured(source_root, true).map_err(fail)?;
    if current_root.identity != *root_identity || current_root.identity != root_proof.identity {
        return Err(fail(std::io::Error::other(
            "The viewer root changed before admission",
        )));
    }
    guards.extend(descendants);
    Ok((
        source_path.to_string_lossy().into_owned(),
        PreviewPublicationGuard {
            source_file: file,
            _root_proof: root_proof,
            _namespace_guards: guards,
        },
    ))
}

fn validate_viewer_directory(directory: &File, volume_serial: u64) -> std::io::Result<()> {
    let info = file_attribute_tag_info_from_handle(directory)?;
    classify_viewer_directory(info.FileAttributes, info.ReparseTag)?;
    if raw_file_id_info(directory)?.VolumeSerialNumber != volume_serial {
        return Err(std::io::Error::other(
            "The viewer directory crossed a volume boundary",
        ));
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum ViewerDirectoryKind {
    Ordinary,
    HydratedCloud,
}

fn classify_viewer_directory(attributes: u32, tag: u32) -> std::io::Result<ViewerDirectoryKind> {
    if attributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || metadata_placeholder_state_from_attributes(attributes)
            != MetadataInventoryPlaceholderState::Available
    {
        return Err(std::io::Error::other(
            "The viewer directory is not locally available",
        ));
    }
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        return if tag == 0 {
            Ok(ViewerDirectoryKind::Ordinary)
        } else {
            Err(std::io::Error::other(
                "The viewer directory has inconsistent reparse evidence",
            ))
        };
    }
    // WinNT.h defines only CLOUD and CLOUD_1..F through CLOUD_MASK. Their name-surrogate
    // bit is unset; other reparse families must not influence the held source namespace.
    if tag & !IO_REPARSE_TAG_CLOUD_MASK != IO_REPARSE_TAG_CLOUD {
        return Err(std::io::Error::other(
            "The viewer directory is an unsupported reparse point",
        ));
    }
    validate_source_file_info(&AttributeTagEvidence {
        attributes,
        reparse_tag: tag,
    })?;
    Ok(ViewerDirectoryKind::HydratedCloud)
}

#[cfg(test)]
mod tests;
