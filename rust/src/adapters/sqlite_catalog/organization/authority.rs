use crate::domain::ScanError;
use crate::domain::organization::{
    MAX_ORGANIZATION_SELECTION, OrganizationGroupKey, OrganizationPreparedSelection,
    stale_selection, validate_identifier, validate_selections,
};

pub(super) fn field(hasher: &mut blake3::Hasher, value: &[u8]) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value);
}

pub(super) fn validate_prepared(prepared: &OrganizationPreparedSelection) -> Result<(), ScanError> {
    validate_selections(&prepared.selections)?;
    if prepared.groups.is_empty() || prepared.groups.len() > MAX_ORGANIZATION_SELECTION {
        return Err(stale_selection());
    }
    for group in &prepared.groups {
        validate_selections(std::slice::from_ref(&group.representative))?;
        if !is_digest(&group.membership_authority) || group.location_count == 0 {
            return Err(stale_selection());
        }
        match &group.key {
            OrganizationGroupKey::Confirmed { digest, .. } if !is_digest(digest) => {
                return Err(stale_selection());
            }
            OrganizationGroupKey::Provisional { asset_id, .. } => validate_identifier(asset_id)?,
            _ => {}
        }
    }
    Ok(())
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn favorite_request(prepared: &OrganizationPreparedSelection, favorite: bool) -> String {
    let mut hasher = blake3::Hasher::new();
    field(&mut hasher, b"ame-organization-favorite-v1");
    field(&mut hasher, &[u8::from(favorite)]);
    prepared_fields(&mut hasher, prepared);
    hasher.finalize().to_hex().to_string()
}

pub(super) fn change_request(
    prepared: &OrganizationPreparedSelection,
    action: &crate::domain::organization::OrganizationAction,
) -> String {
    let mut hasher = blake3::Hasher::new();
    field(&mut hasher, b"ame-organization-change-v1");
    field(&mut hasher, action.request_name().as_bytes());
    field(&mut hasher, action.decision().1.unwrap_or("").as_bytes());
    prepared_fields(&mut hasher, prepared);
    hasher.finalize().to_hex().to_string()
}

fn prepared_fields(hasher: &mut blake3::Hasher, prepared: &OrganizationPreparedSelection) {
    field(hasher, &prepared.catalog_revision.to_le_bytes());
    field(hasher, &prepared.organization_revision.to_le_bytes());
    field(hasher, &(prepared.selections.len() as u64).to_le_bytes());
    for selection in &prepared.selections {
        field(hasher, selection.asset_id.as_bytes());
        field(hasher, selection.location_id.as_bytes());
        field(hasher, &selection.source_generation.to_le_bytes());
    }
    field(hasher, &(prepared.groups.len() as u64).to_le_bytes());
    for group in &prepared.groups {
        group_fields(hasher, group);
    }
}

pub(super) fn group_fields(
    hasher: &mut blake3::Hasher,
    group: &crate::domain::organization::OrganizationGroupTarget,
) {
    match &group.key {
        OrganizationGroupKey::Confirmed {
            digest,
            byte_length,
        } => {
            field(hasher, b"confirmed");
            field(hasher, digest.as_bytes());
            field(hasher, &byte_length.to_le_bytes());
        }
        OrganizationGroupKey::Provisional {
            asset_id,
            source_generation,
        } => {
            field(hasher, b"provisional");
            field(hasher, asset_id.as_bytes());
            field(hasher, &source_generation.to_le_bytes());
        }
    }
    field(hasher, group.representative.asset_id.as_bytes());
    field(hasher, group.representative.location_id.as_bytes());
    field(
        hasher,
        &group.representative.source_generation.to_le_bytes(),
    );
    field(hasher, &group.location_count.to_le_bytes());
    field(hasher, group.membership_authority.as_bytes());
}
