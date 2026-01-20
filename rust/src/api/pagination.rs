//! Pagination utilities for list endpoints

#![allow(dead_code)]

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// Pagination query parameters
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    pub page: Option<u32>,

    /// Items per page
    pub per_page: Option<u32>,
}

impl PaginationParams {
    /// Maximum allowed items per page
    pub const MAX_PER_PAGE: u32 = 100;

    /// Returns the clamped per_page value
    pub fn per_page(&self) -> u32 {
        self.per_page.unwrap_or(20).min(Self::MAX_PER_PAGE).max(1)
    }

    /// Returns the page (1-indexed, minimum 1)
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    /// Calculate SQL OFFSET
    pub fn offset(&self) -> u32 {
        (self.page() - 1) * self.per_page()
    }

    /// Calculate SQL LIMIT
    pub fn limit(&self) -> u32 {
        self.per_page()
    }
}

/// Pagination metadata
#[derive(Debug, Clone, Serialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub per_page: u32,
    pub total_items: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationMeta {
    pub fn new(params: &PaginationParams, total_items: u64) -> Self {
        let per_page = params.per_page();
        let page = params.page();
        let total_pages = ((total_items as f64) / (per_page as f64)).ceil() as u32;

        Self {
            page,
            per_page,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Serialize)]
pub struct Paginated<T: Serialize> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

impl<T: Serialize> Paginated<T> {
    pub fn new(data: Vec<T>, params: &PaginationParams, total_items: u64) -> Self {
        Self {
            data,
            pagination: PaginationMeta::new(params, total_items),
        }
    }
}

impl<T: Serialize> IntoResponse for Paginated<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
