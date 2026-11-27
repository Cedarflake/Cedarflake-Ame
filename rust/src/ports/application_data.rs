use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct ApplicationDataDirectories {
    pub data: PathBuf,
    pub config: PathBuf,
    pub cache: PathBuf,
    pub preferences: PathBuf,
}
