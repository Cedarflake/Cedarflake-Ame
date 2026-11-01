use serde::Deserialize;

use super::*;

#[derive(Deserialize)]
struct Distribution {
    #[serde(rename = "bundleDirectory")]
    directory: String,
    files: Vec<Resource>,
}

#[derive(Deserialize)]
struct Resource {
    name: String,
    blake3: Option<String>,
}

impl LocalClassificationVocabulary {
    pub fn open_bundled(
        application_directory: &Path,
        cancelled: &AtomicBool,
    ) -> Result<(Self, Vec<NamingContext>), ScanError> {
        let distribution: Distribution = serde_json::from_str(include_str!(
            "../../../../resources/classification/vocabulary-distribution.json"
        ))
        .map_err(|_| invalid())?;
        let resource = distribution
            .files
            .into_iter()
            .find(|resource| resource.name == "vocabulary.json")
            .ok_or_else(invalid)?;
        let digest =
            blake3::Hash::from_hex(resource.blake3.ok_or_else(invalid)?).map_err(|_| invalid())?;
        Self::open(
            &application_directory
                .join(distribution.directory)
                .join(resource.name),
            digest.as_bytes(),
            cancelled,
        )
    }
}
