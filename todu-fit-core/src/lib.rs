//! Todu Fit Core Library
//!
//! Shared types and logic for Todu Fit applications.

pub mod automerge;
pub mod document_id;
pub mod documents;
pub mod identity;
pub mod models;
pub mod sync;

pub use automerge::{
    delete_dish, delete_meallog, delete_mealplan, write_dish, write_meallog, write_mealplan,
    DocType, DocumentStorage, MultiDocStorage, MultiStorageError, StorageError,
};
pub use document_id::{DocumentId, DocumentIdError};
pub use documents::{GroupDocument, GroupRef, IdentityDocument};
pub use identity::{Identity, IdentityError, IdentityState};
pub use models::{Dish, Ingredient, MealLog, MealPlan, MealType, Nutrient};
pub use sync::{check_server, SyncClient, SyncError, SyncResult};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
