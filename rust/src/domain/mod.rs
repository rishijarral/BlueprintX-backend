//! Domain types and DTOs
//!
//! These types define the data structures for BlueprintX entities.
//! Many types are scaffolded for future use when implementing database logic.

#![allow(dead_code)]

pub mod ai;
pub mod auth;
pub mod bids;
pub mod documents;
pub mod profiles;
pub mod projects;
pub mod rfis;
pub mod settings;
pub mod subcontractors;
pub mod tasks;
pub mod tenders;

// Re-export commonly used types
pub use bids::*;
pub use documents::*;
pub use projects::*;
pub use tenders::*;

// Auth, AI, and other types are accessed via crate::domain::module:: to avoid namespace pollution
