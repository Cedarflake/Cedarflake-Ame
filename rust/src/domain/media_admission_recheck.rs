use super::FileIdentityEvidence;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaAdmissionRecheck {
    pub asset_id: String,
    pub location_id: String,
    pub file_identity: Option<FileIdentityEvidence>,
    pub requires_inspection: bool,
}
