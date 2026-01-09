//! Document types for the multi-user, multi-group architecture.
//!
//! # Document Structure
//!
//! The application uses five document types:
//!
//! 1. **Identity Document** (personal): References personal meal logs and group memberships
//! 2. **Group Document** (shared): Contains group metadata and references to shared documents
//! 3. **Dishes Document** (shared): Map of dishes shared within a group
//! 4. **MealPlans Document** (shared): Map of meal plans shared within a group
//! 5. **MealLogs Document** (personal): Map of personal meal logs with dish snapshots

mod group;
mod identity;

pub use group::{GroupDocument, GroupRef};
pub use identity::IdentityDocument;
