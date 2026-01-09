//! Group document for shared data.
//!
//! A group document represents a shared context (e.g., family, household)
//! where multiple users can collaborate on dishes and meal plans.

use serde::{Deserialize, Serialize};

use crate::document_id::DocumentId;

/// Reference to a group, stored in identity documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupRef {
    /// Display name for the group
    pub name: String,

    /// Document ID of the group document
    pub doc_id: DocumentId,
}

impl GroupRef {
    /// Create a new group reference.
    pub fn new(name: impl Into<String>, doc_id: DocumentId) -> Self {
        Self {
            name: name.into(),
            doc_id,
        }
    }
}

/// Shared group document.
///
/// Contains group metadata and references to shared documents
/// (dishes and meal plans).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupDocument {
    /// Schema version for migration support
    pub schema_version: u32,

    /// Group display name
    pub name: String,

    /// Reference to shared dishes document
    pub dishes_doc_id: DocumentId,

    /// Reference to shared meal plans document
    pub mealplans_doc_id: DocumentId,
}

impl GroupDocument {
    /// Current schema version
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    /// Create a new group document with generated document IDs.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            name: name.into(),
            dishes_doc_id: DocumentId::new(),
            mealplans_doc_id: DocumentId::new(),
        }
    }

    /// Create a group document with specific document IDs.
    pub fn with_doc_ids(
        name: impl Into<String>,
        dishes_doc_id: DocumentId,
        mealplans_doc_id: DocumentId,
    ) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            name: name.into(),
            dishes_doc_id,
            mealplans_doc_id,
        }
    }

    /// Rename the group.
    pub fn rename(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_ref_new() {
        let doc_id = DocumentId::new();
        let group_ref = GroupRef::new("Family", doc_id);

        assert_eq!(group_ref.name, "Family");
        assert_eq!(group_ref.doc_id, doc_id);
    }

    #[test]
    fn test_new_group_document() {
        let group = GroupDocument::new("Family");

        assert_eq!(group.schema_version, GroupDocument::CURRENT_SCHEMA_VERSION);
        assert_eq!(group.name, "Family");
    }

    #[test]
    fn test_group_with_doc_ids() {
        let dishes_id = DocumentId::new();
        let mealplans_id = DocumentId::new();
        let group = GroupDocument::with_doc_ids("Family", dishes_id, mealplans_id);

        assert_eq!(group.dishes_doc_id, dishes_id);
        assert_eq!(group.mealplans_doc_id, mealplans_id);
    }

    #[test]
    fn test_rename_group() {
        let mut group = GroupDocument::new("Family");
        group.rename("Household");

        assert_eq!(group.name, "Household");
    }

    #[test]
    fn test_group_ref_serialization() {
        let group_ref = GroupRef::new("Family", DocumentId::new());

        let json = serde_json::to_string(&group_ref).unwrap();
        let parsed: GroupRef = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "Family");
        assert_eq!(parsed.doc_id, group_ref.doc_id);
    }

    #[test]
    fn test_group_document_serialization() {
        let group = GroupDocument::new("Family");

        let json = serde_json::to_string(&group).unwrap();
        let parsed: GroupDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.schema_version, group.schema_version);
        assert_eq!(parsed.name, group.name);
        assert_eq!(parsed.dishes_doc_id, group.dishes_doc_id);
        assert_eq!(parsed.mealplans_doc_id, group.mealplans_doc_id);
    }
}
