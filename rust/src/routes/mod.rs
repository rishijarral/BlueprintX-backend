pub mod ai;
pub mod auth;
pub mod bids;
pub mod documents;
pub mod health;
pub mod me;
pub mod profiles;
pub mod projects;
pub mod rfis;
pub mod settings;
pub mod subcontractors;
pub mod tasks;
pub mod tenders;

use axum::{routing::delete, routing::get, routing::post, routing::put, Router};
use std::sync::Arc;

use crate::app::AppState;

/// Build the API router with all routes
pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        // Public routes
        .route("/health", get(health::health_check))
        // Auth routes (public)
        .route("/auth/signup", post(auth::sign_up))
        .route("/auth/signin", post(auth::sign_in))
        .route("/auth/refresh", post(auth::refresh_token))
        // Auth routes (protected)
        .route("/auth/signout", post(auth::sign_out))
        .route("/auth/session", get(auth::get_session))
        // Protected routes
        .route("/me", get(me::get_me))
        // Profile routes
        .route("/profiles/me", get(profiles::get_my_profile))
        .route("/profiles/me", put(profiles::update_my_profile))
        // Settings routes
        .route("/settings", get(settings::get_settings))
        .route("/settings", put(settings::update_settings))
        // Projects
        .route("/projects", post(projects::create_project))
        .route("/projects", get(projects::list_projects))
        .route("/projects/:project_id", get(projects::get_project))
        .route("/projects/:project_id", put(projects::update_project))
        .route("/projects/:project_id", delete(projects::delete_project))
        // Documents (nested under projects)
        .route(
            "/projects/:project_id/documents",
            post(documents::create_document),
        )
        .route(
            "/projects/:project_id/documents",
            get(documents::list_documents),
        )
        .route(
            "/projects/:project_id/documents/upload",
            post(documents::upload_document),
        )
        .route(
            "/projects/:project_id/documents/:document_id",
            get(documents::get_document),
        )
        .route(
            "/projects/:project_id/documents/:document_id",
            delete(documents::delete_document),
        )
        // Tenders (nested under projects)
        .route(
            "/projects/:project_id/tenders",
            post(tenders::create_tender),
        )
        .route("/projects/:project_id/tenders", get(tenders::list_tenders))
        .route("/projects/:project_id/tenders/:tender_id", get(tenders::get_tender))
        .route("/projects/:project_id/tenders/:tender_id", put(tenders::update_tender))
        .route("/projects/:project_id/tenders/:tender_id", delete(tenders::delete_tender))
        // All tenders (for flat access)
        .route("/tenders", get(tenders::list_all_tenders))
        // Bids (nested under tenders)
        .route("/tenders/:tender_id/bids", post(bids::create_bid))
        .route("/tenders/:tender_id/bids", get(bids::list_bids))
        // Tasks (nested under projects)
        .route("/projects/:project_id/tasks", post(tasks::create_task))
        .route("/projects/:project_id/tasks", get(tasks::list_tasks))
        .route("/projects/:project_id/tasks/:task_id", get(tasks::get_task))
        .route("/projects/:project_id/tasks/:task_id", put(tasks::update_task))
        .route("/projects/:project_id/tasks/:task_id", delete(tasks::delete_task))
        // All tasks (for flat access)
        .route("/tasks", get(tasks::list_all_tasks))
        // RFIs (nested under projects)
        .route("/projects/:project_id/rfis", post(rfis::create_rfi))
        .route("/projects/:project_id/rfis", get(rfis::list_rfis))
        .route("/projects/:project_id/rfis/:rfi_id", get(rfis::get_rfi))
        .route("/projects/:project_id/rfis/:rfi_id", put(rfis::update_rfi))
        .route("/projects/:project_id/rfis/:rfi_id", delete(rfis::delete_rfi))
        .route("/projects/:project_id/rfis/:rfi_id/responses", post(rfis::add_rfi_response))
        .route("/projects/:project_id/rfis/:rfi_id/responses", get(rfis::get_rfi_responses))
        // All RFIs (for flat access)
        .route("/rfis", get(rfis::list_all_rfis))
        // Subcontractors (marketplace)
        .route("/subcontractors", get(subcontractors::list_subcontractors))
        .route("/subcontractors/:subcontractor_id", get(subcontractors::get_subcontractor))
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
