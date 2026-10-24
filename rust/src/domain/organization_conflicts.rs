#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationConflictAlbum {
    pub album_id: String,
    pub name: String,
    pub retired: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationConflictAlbums {
    pub organization_revision: u64,
    pub albums: Vec<OrganizationConflictAlbum>,
    pub next_album_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationConflictStatus {
    pub catalog_revision: u64,
    pub conflicting_groups: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationConflictAnnotations {
    pub catalog_revision: u64,
    pub location_ids: Vec<String>,
}
