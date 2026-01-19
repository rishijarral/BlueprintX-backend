pub mod ai;
pub mod bids;
pub mod documents;
pub mod health;
pub mod me;
pub mod projects;
pub mod tenders;

use axum::{routing::delete, routing::get, routing::post, Router};
use std::sync::Arc;

use crate::app::AppState;

/// Build the API router with all routes
pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        // Public routes
        .route("/health", get(health::health_check))
        // Protected routes
        .route("/me", get(me::get_me))
        // Projects
        .route("/projects", post(projects::create_project))
        .route("/projects", get(projects::list_projects))
        .route("/projects/:project_id", get(projects::get_project))
        // Documents (nested under projects)
        .route(
            "/projects/:project_id/documents",
            post(documents::create_document),
        )
        .route(
            "/projects/:project_id/documents",
            get(documents::list_documents),
        )
        // Tenders (nested under projects)
        .route(
            "/projects/:project_id/tenders",
            post(tenders::create_tender),
        )
        .route("/projects/:project_id/tenders", get(tenders::list_tenders))
        // Bids (nested under tenders)
        .route("/tenders/:tender_id/bids", post(bids::create_bid))
        .route("/tenders/:tender_id/bids", get(bids::list_bids))
        // AI endpoints (nested under projects)
        .route(
            "/projects/:project_id/ai/summary",
            post(ai::generate_plan_summary),
        )
        .route(
            "/projects/:project_id/ai/trade-scopes",
            post(ai::extract_trade_scopes),
        )
        .route(
            "/projects/:project_id/ai/tender-scope-doc",
            post(ai::generate_tender_scope_doc),
        )
        .route("/projects/:project_id/ai/qna", post(ai::ask_question))
        .route(
            "/projects/:project_id/ai/cache",
            delete(ai::invalidate_ai_cache),
        )
        // Global AI endpoints
        .route("/ai/trades", get(ai::get_standard_trades))
}
