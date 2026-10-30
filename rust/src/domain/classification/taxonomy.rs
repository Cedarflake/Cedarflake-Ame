use std::collections::BTreeSet;

use super::ClassificationError;

pub(super) const MAX_CATEGORIES: usize = 256;
const MAX_DESCRIPTIONS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassificationRevision([u8; 32]);

impl ClassificationRevision {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub(crate) fn derive<'a>(context: &str, fields: impl IntoIterator<Item = &'a [u8]>) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(context);
        for field in fields {
            hasher.update(&(field.len() as u64).to_le_bytes());
            hasher.update(field);
        }
        Self(*hasher.finalize().as_bytes())
    }

    pub fn description(text: &str) -> Self {
        Self::derive("ame.classification.description.v1", [text.as_bytes()])
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CategoryId(String);

impl CategoryId {
    pub fn new(value: String) -> Result<Self, ClassificationError> {
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(ClassificationError::InvalidCategoryId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryDefinition {
    id: CategoryId,
    label: String,
    descriptions: Vec<String>,
}

impl CategoryDefinition {
    pub fn new(
        id: CategoryId,
        label: String,
        descriptions: Vec<String>,
    ) -> Result<Self, ClassificationError> {
        let label = validated_label(label)?;
        if descriptions.is_empty() || descriptions.len() > MAX_DESCRIPTIONS {
            return Err(ClassificationError::InvalidDescriptions);
        }
        let mut descriptions: Vec<String> = descriptions
            .into_iter()
            .map(|text| text.trim().to_owned())
            .collect();
        if descriptions.iter().any(|text| {
            text.is_empty() || text.chars().count() > 1024 || text.chars().any(char::is_control)
        }) {
            return Err(ClassificationError::InvalidDescriptions);
        }
        descriptions.sort();
        if descriptions.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ClassificationError::InvalidDescriptions);
        }
        Ok(Self {
            id,
            label,
            descriptions,
        })
    }

    pub fn id(&self) -> &CategoryId {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn descriptions(&self) -> &[String] {
        &self.descriptions
    }

    pub fn renamed(&self, label: String) -> Result<Self, ClassificationError> {
        Ok(Self {
            label: validated_label(label)?,
            ..self.clone()
        })
    }

    pub fn semantic_revision(&self) -> ClassificationRevision {
        ClassificationRevision::derive(
            "ame.classification.category.v1",
            std::iter::once(self.id.as_str().as_bytes())
                .chain(self.descriptions.iter().map(|text| text.as_bytes())),
        )
    }
}

fn validated_label(label: String) -> Result<String, ClassificationError> {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > 80 || label.chars().any(char::is_control) {
        return Err(ClassificationError::InvalidLabel);
    }
    Ok(label.to_owned())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Taxonomy {
    categories: Vec<CategoryDefinition>,
    revision: ClassificationRevision,
}

impl Taxonomy {
    pub fn new(mut categories: Vec<CategoryDefinition>) -> Result<Self, ClassificationError> {
        if categories.is_empty() {
            return Err(ClassificationError::NoCategories);
        }
        if categories.len() > MAX_CATEGORIES {
            return Err(ClassificationError::TooManyCategories);
        }
        categories.sort_by(|left, right| left.id.cmp(&right.id));
        if categories.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(ClassificationError::DuplicateCategory);
        }
        let revisions: Vec<_> = categories
            .iter()
            .map(CategoryDefinition::semantic_revision)
            .collect();
        let revision = ClassificationRevision::derive(
            "ame.classification.taxonomy.max-cosine.v1",
            revisions.iter().map(|revision| revision.bytes().as_slice()),
        );
        Ok(Self {
            categories,
            revision,
        })
    }

    pub fn categories(&self) -> &[CategoryDefinition] {
        &self.categories
    }

    pub fn revision(&self) -> ClassificationRevision {
        self.revision
    }

    pub fn description_revisions(&self) -> BTreeSet<ClassificationRevision> {
        self.categories
            .iter()
            .flat_map(|category| {
                category
                    .descriptions
                    .iter()
                    .map(|text| ClassificationRevision::description(text))
            })
            .collect()
    }
}
