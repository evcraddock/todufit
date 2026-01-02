//! Automerge document handling for Todu Fit.
//!
//! This module provides the foundation for syncing data across devices using
//! Automerge, a CRDT library that enables automatic conflict resolution.
//!
//! # Document Structure
//!
//! Each entity type has its own Automerge document:
//! - `dishes.automerge`: Map of dish_id (UUID string) -> Dish object
//! - `mealplans.automerge`: Map of mealplan_id (UUID string) -> MealPlan object
//! - `meallogs.automerge`: Map of meallog_id (UUID string) -> MealLog object

mod doc_type;
mod storage;
mod writer;

pub use doc_type::DocType;
pub use storage::{DocumentStorage, StorageError};
pub use writer::{
    delete_dish, delete_meallog, delete_mealplan, write_dish, write_meallog, write_mealplan,
};
