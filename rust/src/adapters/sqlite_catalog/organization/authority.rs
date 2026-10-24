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
    field(&mut hasher, &prepared.catalog_revision.to_le_bytes());
    field(&mut hasher, &prepared.organization_revision.to_le_bytes());
    field(
        &mut hasher,
        &(prepared.selections.len() as u64).to_le_bytes(),
    );
    for selection in &prepared.selections {
        field(&mut hasher, selection.asset_id.as_bytes());
        field(&mut hasher, selection.location_id.as_bytes());
        field(&mut hasher, &selection.source_generation.to_le_bytes());
    }
    field(&mut hasher, &(prepared.groups.len() as u64).to_le_bytes());
    for group in &prepared.groups {
        match &group.key {
            OrganizationGroupKey::Confirmed {
                digest,
                byte_length,
            } => {
                field(&mut hasher, b"confirmed");
                field(&mut hasher, digest.as_bytes());
                field(&mut hasher, &byte_length.to_le_bytes());
            }
            OrganizationGroupKey::Provisional {
                asset_id,
                source_generation,
            } => {
                field(&mut hasher, b"provisional");
                field(&mut hasher, asset_id.as_bytes());
                field(&mut hasher, &source_generation.to_le_bytes());
            }
        }
        field(&mut hasher, group.representative.asset_id.as_bytes());
        field(&mut hasher, group.representative.location_id.as_bytes());
        field(
            &mut hasher,
            &group.representative.source_generation.to_le_bytes(),
        );
        field(&mut hasher, &group.location_count.to_le_bytes());
        field(&mut hasher, group.membership_authority.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}
