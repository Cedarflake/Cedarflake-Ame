use rust_lib_cedarflake_ame::classification::{
    ContextualVocabulary, CuratedClassificationVocabulary, LocalClassificationVocabulary,
    NamingContext, NamingContextCatalog, ScanError,
};

use super::*;

#[derive(Deserialize)]
pub(super) struct Context {
    id: String,
    label: String,
    description: String,
    subject: String,
    cues: Vec<String>,
    #[serde(default)]
    broader: Vec<String>,
}

pub(super) enum Vocabulary<'a> {
    Retrieval(ProbeVocabulary),
    Contextual(ContextualVocabulary<'a, ProbeVocabulary>),
    Resource(ContextualVocabulary<'a, LocalClassificationVocabulary>),
    Curated(CuratedClassificationVocabulary),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Resource {
    File { file: String, digest: [u8; 32] },
    Bundled { catalog: BundledCatalog },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BundledCatalog {
    Curated,
}

impl<'a> Vocabulary<'a> {
    pub(super) fn open_resource(
        resource: Resource,
        fixture: &Path,
        cancelled: &'a AtomicBool,
    ) -> Result<Self> {
        match resource {
            Resource::File { file, digest } => {
                let path = fixture
                    .parent()
                    .ok_or("Missing fixture directory")?
                    .join(file);
                let (source, contexts) =
                    LocalClassificationVocabulary::open(&path, &digest, cancelled)?;
                let catalog = NamingContextCatalog::new(contexts)?;
                Ok(Self::Resource(ContextualVocabulary::new(
                    source, catalog, cancelled,
                )))
            }
            Resource::Bundled {
                catalog: BundledCatalog::Curated,
            } => Ok(Self::Curated(CuratedClassificationVocabulary::bundled(
                cancelled,
            )?)),
        }
    }

    pub(super) fn new(
        source: ProbeVocabulary,
        definitions: Option<Vec<Context>>,
        cancelled: &'a AtomicBool,
    ) -> Result<Self> {
        let Some(definitions) = definitions else {
            return Ok(Self::Retrieval(source));
        };
        if definitions.len() > 4096 {
            return Err("Too many naming contexts".into());
        }
        let mut contexts = Vec::with_capacity(definitions.len());
        let mut edges = 0;
        for definition in definitions {
            edges += definition.cues.len() + definition.broader.len();
            if edges > 65_536
                || definition.id.len() > 128
                || definition
                    .cues
                    .iter()
                    .chain(&definition.broader)
                    .any(|id| id.len() > 128)
                || definition
                    .cues
                    .iter()
                    .any(|id| !source.concepts.contains_key(&concept_id(id)))
                || source.concepts.contains_key(&concept_id(&definition.id))
            {
                return Err("Invalid naming context identity or relation".into());
            }
            contexts.push(NamingContext {
                concept: VocabularyConcept::new(
                    concept_id(&definition.id),
                    definition.label,
                    definition.description,
                )?
                .with_subject(definition.subject)?,
                cues: definition
                    .cues
                    .iter()
                    .map(|value| concept_id(value))
                    .collect(),
                broader: definition
                    .broader
                    .iter()
                    .map(|value| concept_id(value))
                    .collect(),
            });
        }
        let catalog = NamingContextCatalog::new(contexts)?;
        Ok(Self::Contextual(ContextualVocabulary::new(
            source, catalog, cancelled,
        )))
    }
}

impl ClassificationVocabulary for Vocabulary<'_> {
    fn revision(&self) -> ClassificationRevision {
        match self {
            Self::Retrieval(source) => source.revision(),
            Self::Contextual(source) => source.revision(),
            Self::Resource(source) => source.revision(),
            Self::Curated(source) => source.revision(),
        }
    }

    fn read_concepts(
        &mut self,
        neighborhood: ConceptNeighborhood,
        after: Option<ConceptId>,
        limit: usize,
    ) -> std::result::Result<ConceptPage, ScanError> {
        match self {
            Self::Retrieval(source) => source.read_concepts(neighborhood, after, limit),
            Self::Contextual(source) => source.read_concepts(neighborhood, after, limit),
            Self::Resource(source) => source.read_concepts(neighborhood, after, limit),
            Self::Curated(source) => source.read_concepts(neighborhood, after, limit),
        }
    }
}
