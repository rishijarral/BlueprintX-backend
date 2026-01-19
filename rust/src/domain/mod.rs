//! Domain types and DTOs
//!
//! These types define the data structures for BlueprintX entities.
//! Many types are scaffolded for future use when implementing database logic.

#![allow(dead_code)]

pub mod ai;
pub mod bids;
pub mod documents;
pub mod projects;
pub mod tenders;

// Re-export commonly used types
pub use bids::*;
pub use documents::*;
pub use projects::*;
pub use tenders::*;

// AI types are accessed via crate::domain::ai:: to avoid namespace pollution
