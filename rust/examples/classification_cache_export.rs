#[cfg(all(windows, feature = "classification-worker"))]
use std::fs::{self, OpenOptions};
#[cfg(all(windows, feature = "classification-worker"))]
use std::io::{Read, Write};
#[cfg(all(windows, feature = "classification-worker"))]
use std::path::Path;

#[cfg(all(windows, feature = "classification-worker"))]
use rust_lib_cedarflake_ame::classification::{
    ClassificationEncoderFactory, ClassificationFeatureRepository, ClassificationRevision,
    SqliteClassificationFeatureCache,
};
#[cfg(all(windows, feature = "classification-worker"))]
use rust_lib_cedarflake_ame::classification_worker::LocalClassificationEncoderFactory;

#[cfg(all(windows, feature = "classification-worker"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 3 {
        return Err("Expected a description JSON file, retained feature-cache directory and new output file".into());
    }
    let mut input = Vec::new();
    fs::File::open(&arguments[0])?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    if input.len() > 8 * 1024 * 1024 {
        return Err("Description input exceeds the diagnostic byte bound".into());
    }
    let descriptions: Vec<String> = serde_json::from_slice(&input)?;
    if descriptions.len() > 4096
        || descriptions
            .iter()
            .any(|text| text.is_empty() || text.len() > 4096)
    {
        return Err("Description input exceeds the diagnostic item bounds".into());
    }
    let factory = LocalClassificationEncoderFactory {
        executable: "unused-cache-export-worker".into(),
        model_directory: "unused-cache-export-models".into(),
        runtime_directory: "unused-cache-export-runtime".into(),
        backend: Default::default(),
        ..Default::default()
    };
    let model = factory.compatibility();
    let mut cache = SqliteClassificationFeatureCache::open(Path::new(&arguments[1]))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&arguments[2])?;
    output.write_all(b"AMECF01\0")?;
    output.write_all(&(descriptions.len() as u32).to_le_bytes())?;
    output.write_all(&(model.dimensions() as u32).to_le_bytes())?;
    let mut found = 0;
    for window in descriptions.chunks(64) {
        let revisions: Vec<_> = window
            .iter()
            .map(|description| ClassificationRevision::description(description))
            .collect();
        for feature in cache.load_text_features(model, &revisions)? {
            match feature {
                Some(feature) => {
                    output.write_all(&[1])?;
                    for value in feature.embedding.values() {
                        output.write_all(&value.to_le_bytes())?;
                    }
                    found += 1;
                }
                None => output.write_all(&[0])?,
            }
        }
    }
    output.sync_all()?;
    println!(
        "{}",
        serde_json::json!({"requested": descriptions.len(), "found": found, "dimensions": model.dimensions(), "inference": false}),
    );
    Ok(())
}

#[cfg(not(all(windows, feature = "classification-worker")))]
fn main() {
    eprintln!("This probe requires Windows and the classification-worker feature");
    std::process::exit(1);
}
