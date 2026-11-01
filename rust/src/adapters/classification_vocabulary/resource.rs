use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use serde::Deserialize;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{
    ConceptId, ConceptNamingScope, NamingContext, VocabularyConcept,
};

use super::{check_cancelled, invalid};

const MAX_BYTES: usize = 64 * 1024 * 1024;
const CONTEXT_NAMESPACE: &str = "ame.managed-naming-context";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Document {
    format_version: u32,
    pub source_namespace: String,
    pub concepts: Vec<Concept>,
    pub entries: Vec<String>,
    pub contexts: Vec<Context>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Definition {
    label: String,
    description: String,
    #[serde(default)]
    subject: Option<String>,
}

impl Definition {
    pub fn into_concept(
        self,
        id: ConceptId,
        scope: ConceptNamingScope,
    ) -> Result<VocabularyConcept, ScanError> {
        let mut concept = VocabularyConcept::new(id, self.label, self.description)
            .map_err(|_| invalid())?
            .with_naming_scope(scope);
        if let Some(subject) = self.subject {
            concept = concept.with_subject(subject).map_err(|_| invalid())?;
        }
        Ok(concept)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Concept {
    pub id: String,
    pub definition: Definition,
    pub related: Vec<String>,
    pub broader: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Context {
    id: String,
    definition: Definition,
    cues: Vec<String>,
    broader: Vec<String>,
}

pub(super) fn load(
    path: &Path,
    expected_digest: &[u8; 32],
    cancelled: &AtomicBool,
) -> Result<Document, ScanError> {
    check_cancelled(cancelled)?;
    let mut file = std::fs::File::open(path).map_err(|_| invalid())?;
    if file.metadata().map_err(|_| invalid())?.len() > MAX_BYTES as u64 {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_cancelled(cancelled)?;
        let count = file.read(&mut buffer).map_err(|_| invalid())?;
        if count == 0 {
            break;
        }
        if bytes.len().saturating_add(count) > MAX_BYTES {
            return Err(invalid());
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    check_cancelled(cancelled)?;
    if blake3::hash(&bytes).as_bytes() != expected_digest {
        return Err(invalid());
    }
    let document: Document = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    check_cancelled(cancelled)?;
    if document.format_version != 1
        || document.concepts.len() > 100_000
        || document.entries.len() > 4096
        || document.contexts.len() > 4096
        || document.source_namespace == CONTEXT_NAMESPACE
    {
        return Err(invalid());
    }
    validate_key(&document.source_namespace)?;
    Ok(document)
}

pub(super) fn identity(namespace: &str, key: &str) -> Result<ConceptId, ScanError> {
    validate_key(key)?;
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.vocabulary-identity.v1");
    hash.update(&(namespace.len() as u64).to_le_bytes());
    hash.update(namespace.as_bytes());
    hash.update(key.as_bytes());
    Ok(ConceptId::from_bytes(*hash.finalize().as_bytes()))
}

fn validate_key(value: &str) -> Result<(), ScanError> {
    if value.is_empty()
        || value.len() > 128
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn contexts(
    definitions: Vec<Context>,
    namespace: &str,
    concepts: &BTreeMap<ConceptId, VocabularyConcept>,
    cancelled: &AtomicBool,
) -> Result<Vec<NamingContext>, ScanError> {
    let mut contexts = Vec::with_capacity(definitions.len());
    let mut identities = BTreeSet::new();
    let mut total = 0;
    for definition in definitions {
        check_cancelled(cancelled)?;
        let id = identity(CONTEXT_NAMESPACE, &definition.id)?;
        if !identities.insert(id) {
            return Err(invalid());
        }
        let cues = super::references(namespace, definition.cues, &mut total)?;
        let broader = super::references(CONTEXT_NAMESPACE, definition.broader, &mut total)?;
        if total > 65_536 || cues.iter().any(|id| !concepts.contains_key(id)) {
            return Err(invalid());
        }
        contexts.push(NamingContext {
            concept: definition
                .definition
                .into_concept(id, ConceptNamingScope::Candidate)?,
            cues,
            broader,
        });
    }
    if contexts
        .iter()
        .flat_map(|context| &context.broader)
        .any(|id| !identities.contains(id))
    {
        return Err(invalid());
    }
    check_cancelled(cancelled)?;
    Ok(contexts)
}
