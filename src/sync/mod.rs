//! Sync module for offline-first data synchronization using Automerge CRDTs.
//!
//! This module provides the foundation for syncing data across devices using
//! Automerge, a CRDT library that enables automatic conflict resolution.
//!
//! # Document Structure
//!
//! Each entity type has its own Automerge document:
//! - `dishes`: Map of dish_id (UUID string) -> Dish object
//! - `mealplans`: Map of mealplan_id (UUID string) -> MealPlan object
//! - `meallogs`: Map of meallog_id (UUID string) -> MealLog object
//!
//! # Usage
//!
//! ```
//! use todufit::sync::{DishesDoc, MealPlansDoc, MealLogsDoc};
//!
//! // Create empty documents
//! let dishes = DishesDoc::new();
//! let mealplans = MealPlansDoc::new();
//! let meallogs = MealLogsDoc::new();
//! ```

pub mod dish_sync;
pub mod meallog_sync;
pub mod mealplan_sync;
pub mod projection;
pub mod schema;
pub mod storage;
pub mod writer;

pub use dish_sync::{SyncDishError, SyncDishRepository};
pub use meallog_sync::{SyncMealLogError, SyncMealLogRepository};
pub use mealplan_sync::{SyncMealPlanError, SyncMealPlanRepository};
pub use projection::{DishProjection, MealLogProjection, MealPlanProjection, ProjectionError};
pub use schema::{DishesDoc, MealLogsDoc, MealPlansDoc};
pub use storage::{DocType, DocumentStorage, StorageError};
