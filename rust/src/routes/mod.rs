pub mod admin;
pub mod ai;
pub mod auth;
pub mod bids;
pub mod documents;
pub mod extraction;
pub mod health;
pub mod hiring;
pub mod jobs;
pub mod marketplace;
pub mod me;
pub mod notifications;
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
        // Processing Jobs (nested under projects)
        .route(
            "/projects/:project_id/documents/:document_id/process",
            post(jobs::start_processing),
        )
        .route("/projects/:project_id/jobs", get(jobs::list_project_jobs))
        .route("/projects/:project_id/jobs/stream", get(jobs::stream_job_progress))
        .route("/projects/:project_id/jobs/:job_id", get(jobs::get_job))
        .route(
            "/projects/:project_id/jobs/:job_id/control",
            post(jobs::control_job),
        )
        // Extraction endpoints (nested under projects)
        .route(
            "/projects/:project_id/extraction",
            get(extraction::get_extraction_summary),
        )
        .route(
            "/projects/:project_id/extraction/materials",
            get(extraction::list_materials),
        )
        .route(
            "/projects/:project_id/extraction/materials",
            post(extraction::create_material),
        )
        .route(
            "/projects/:project_id/extraction/materials/:material_id",
            put(extraction::update_material),
        )
        .route(
            "/projects/:project_id/extraction/materials/:material_id",
            delete(extraction::delete_material),
        )
        .route(
            "/projects/:project_id/extraction/materials/:material_id/verify",
            post(extraction::verify_material),
        )
        .route(
            "/projects/:project_id/extraction/rooms",
            get(extraction::list_rooms),
        )
        .route(
            "/projects/:project_id/extraction/rooms",
            post(extraction::create_room),
        )
        .route(
            "/projects/:project_id/extraction/rooms/:room_id",
            put(extraction::update_room),
        )
        .route(
            "/projects/:project_id/extraction/rooms/:room_id",
            delete(extraction::delete_room),
        )
        .route(
            "/projects/:project_id/extraction/milestones",
            get(extraction::list_milestones),
        )
        .route(
            "/projects/:project_id/extraction/milestones",
            post(extraction::create_milestone),
        )
        .route(
            "/projects/:project_id/extraction/milestones/:milestone_id",
            put(extraction::update_milestone),
        )
        .route(
            "/projects/:project_id/extraction/milestones/:milestone_id",
            delete(extraction::delete_milestone),
        )
        .route(
            "/projects/:project_id/extraction/trade-scopes",
            get(extraction::list_trade_scopes),
        )
        .route(
            "/projects/:project_id/extraction/trade-scopes",
            post(extraction::create_trade_scope),
        )
        .route(
            "/projects/:project_id/extraction/trade-scopes/:scope_id",
            put(extraction::update_trade_scope),
        )
        .route(
            "/projects/:project_id/extraction/trade-scopes/:scope_id",
            delete(extraction::delete_trade_scope),
        )
        // Project Team (nested under projects)
        .route("/projects/:project_id/team", get(hiring::list_team_members))
        .route("/projects/:project_id/team", post(hiring::add_team_member))
        .route(
            "/projects/:project_id/team/:member_id",
            put(hiring::update_team_member),
        )
        .route(
            "/projects/:project_id/team/:member_id",
            delete(hiring::remove_team_member),
        )
        // External Subcontractors (my-subcontractors)
        .route(
            "/my-subcontractors",
            get(hiring::list_external_subcontractors),
        )
        .route(
            "/my-subcontractors",
            post(hiring::create_external_subcontractor),
        )
        .route(
            "/my-subcontractors/:id",
            get(hiring::get_external_subcontractor),
        )
        .route(
            "/my-subcontractors/:id",
            put(hiring::update_external_subcontractor),
        )
        .route(
            "/my-subcontractors/:id",
            delete(hiring::delete_external_subcontractor),
        )
        // Hire Requests
        .route("/hiring", get(hiring::list_hire_requests))
        .route("/hiring", post(hiring::create_hire_request))
        .route("/hiring/:id", get(hiring::get_hire_request))
        .route("/hiring/:id", put(hiring::update_hire_request))
        .route("/hiring/:id/status", post(hiring::update_hire_request_status))
        .route("/hiring/:id/messages", get(hiring::list_hire_messages))
        .route("/hiring/:id/messages", post(hiring::send_hire_message))
        .route("/hiring/:id/contract", post(hiring::create_contract))
        // Contracts
        .route("/contracts/:id", get(hiring::get_contract))
        .route("/contracts/:id/sign", post(hiring::sign_contract))
        // Contract Templates
        .route("/contract-templates", get(hiring::list_contract_templates))
        // Notifications
        .route("/notifications", get(notifications::list_notifications))
        .route("/notifications", delete(notifications::delete_all_read))
        .route(
            "/notifications/unread-count",
            get(notifications::get_unread_count),
        )
        .route(
            "/notifications/read-all",
            put(notifications::mark_all_read),
        )
        .route(
            "/notifications/mark-read",
            post(notifications::mark_batch_read),
        )
        .route(
            "/notifications/:notification_id",
            get(notifications::get_notification),
        )
        .route(
            "/notifications/:notification_id",
            delete(notifications::delete_notification),
        )
        .route(
            "/notifications/:notification_id/read",
            put(notifications::mark_notification_read),
        )
        // Marketplace - Subcontractor Directory (Enhanced)
        .route(
            "/marketplace/subcontractors",
            get(marketplace::list_marketplace_subcontractors),
        )
        .route(
            "/marketplace/subcontractors/:sub_id",
            get(marketplace::get_marketplace_subcontractor),
        )
        .route(
            "/marketplace/subcontractors/:sub_id/portfolio",
            get(marketplace::get_subcontractor_portfolio),
        )
        // Marketplace - My Profile (for Subs)
        .route(
            "/marketplace/profile",
            get(marketplace::get_my_marketplace_profile),
        )
        .route(
            "/marketplace/profile",
            put(marketplace::update_my_marketplace_profile),
        )
        .route(
            "/marketplace/profile/request-verification",
            post(marketplace::request_verification),
        )
        .route(
            "/marketplace/profile/portfolio",
            get(marketplace::get_my_portfolio),
        )
        .route(
            "/marketplace/profile/portfolio",
            post(marketplace::create_portfolio_project),
        )
        .route(
            "/marketplace/profile/portfolio/:project_id",
            put(marketplace::update_portfolio_project),
        )
        .route(
            "/marketplace/profile/portfolio/:project_id",
            delete(marketplace::delete_portfolio_project),
        )
        // Marketplace - Saved Searches
        .route(
            "/marketplace/saved-searches",
            get(marketplace::list_saved_searches),
        )
        .route(
            "/marketplace/saved-searches",
            post(marketplace::create_saved_search),
        )
        .route(
            "/marketplace/saved-searches/:search_id",
            delete(marketplace::delete_saved_search),
        )
        // Marketplace - Tenders (for Subs to browse/bid)
        .route(
            "/marketplace/tenders",
            get(marketplace::list_marketplace_tenders),
        )
        .route(
            "/marketplace/tenders/:tender_id",
            get(marketplace::get_marketplace_tender),
        )
        .route(
            "/marketplace/tenders/:tender_id/bid",
            post(marketplace::submit_bid),
        )
        .route(
            "/marketplace/tenders/:tender_id/bid",
            put(marketplace::update_bid),
        )
        .route(
            "/marketplace/tenders/:tender_id/bid",
            delete(marketplace::withdraw_bid),
        )
        // Marketplace - My Bids
        .route("/marketplace/my-bids", get(marketplace::list_my_bids))
        // Admin routes (protected by RequireAdmin middleware)
        .route("/admin/check", get(admin::check_admin))
        .route("/admin/stats", get(admin::get_admin_stats))
        .route(
            "/admin/verifications",
            get(admin::list_pending_verifications),
        )
        .route(
            "/admin/verifications/:sub_id",
            get(admin::get_verification),
        )
        .route(
            "/admin/verifications/:sub_id/approve",
            post(admin::approve_verification),
        )
        .route(
            "/admin/verifications/:sub_id/reject",
            post(admin::reject_verification),
        )
        .route("/admin/audit-log", get(admin::list_audit_log))
}
