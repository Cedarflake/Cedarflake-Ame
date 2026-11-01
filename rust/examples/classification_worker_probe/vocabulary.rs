use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use rust_lib_cedarflake_ame::classification::{
    ClassificationVocabulary, ConceptNeighborhood, LocalClassificationVocabulary,
    NamingContextCatalog,
};
use serde_json::json;

pub fn run(path: &OsStr) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Vocabulary resource exceeds the input bound".into());
    }
    let length = bytes.len();
    let digest = *blake3::hash(&bytes).as_bytes();
    drop(bytes);
    let started = Instant::now();
    let (mut vocabulary, contexts) =
        LocalClassificationVocabulary::open(path, &digest, &AtomicBool::new(false))?;
    let context_count = contexts.len();
    let cue_links: usize = contexts.iter().map(|context| context.cues.len()).sum();
    let catalog = NamingContextCatalog::new(contexts)?;
    let mut after = None;
    let mut entries = 0;
    loop {
        let page = vocabulary.read_concepts(ConceptNeighborhood::EntryPoints, after, 64)?;
        entries += page.concepts.len();
        after = page.concepts.last().map(|concept| concept.id());
        if !page.has_more {
            break;
        }
    }
    println!(
        "{}",
        json!({
            "bytes": length, "digest": digest, "seconds": started.elapsed().as_secs_f64(),
            "entry_points": entries, "contexts": context_count, "cue_links": cue_links,
            "vocabulary_revision": vocabulary.revision().bytes(),
            "context_revision": catalog.revision().bytes(), "source_reads": 0,
        })
    );
    Ok(())
}
