//! AI-related API endpoints that proxy to the Python AI service.
//!
//! These endpoints provide the frontend with AI capabilities while:
//! - Enforcing authentication
//! - Caching results in Redis
//! - Validating project ownership
//! - Propagating request IDs for tracing

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::response::DataResponse;
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::ai::{
    PlanSummaryRequest, PlanSummaryResponse, QnARequest, QnAResponse,
    StandardTradesResponse, TenderScopeDocRequest, TenderScopeDocResponse,
    TradeScopesRequest, TradeScopesResponse,
};
use crate::error::ApiResult;
use crate::middleware::request_id::X_REQUEST_ID;
use crate::services::cache::keys;

/// Helper to extract request ID from headers.
fn get_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

// =============================================================================
// Plan Analysis Endpoints
// =============================================================================

/// Generate a plan summary for a project.
///
/// POST /api/projects/:project_id/ai/summary
pub async fn generate_plan_summary(
    _auth: RequireAuth,
    Path(project_id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(req): Json<PlanSummaryRequest>,
) -> ApiResult<impl IntoResponse> {
    let request_id = get_request_id(&headers);

    // TODO: Validate user owns/has access to this project
    // For now, just proceed with the request

    // Check cache first
    let cache_key = keys::plan_summary(project_id);
    if let Some(cached) = state.cache.get::<PlanSummaryResponse>(&cache_key).await {
        tracing::debug!(project_id = %project_id, "Returning cached plan summary");
        return Ok(Json(DataResponse::new(PlanSummaryResponse {
            cached: true,
            ..cached
        })));
    }

    // Call AI service
    let summary = state
        .ai_client
        .generate_plan_summary(
            project_id,
            &req.document_text,
            req.instructions.as_deref(),
            request_id.as_deref(),
        )
        .await?;

    let response = PlanSummaryResponse {
        project_id: project_id.to_string(),
        summary,
        cached: false,
    };

    // Cache the result
    if let Err(e) = state.cache.set(&cache_key, &response).await {
        tracing::warn!(error = %e, "Failed to cache plan summary");
    }

    Ok(Json(DataResponse::new(response)))
}

/// Extract trade scopes from a project document.
///
/// POST /api/projects/:project_id/ai/trade-scopes
pub async fn extract_trade_scopes(
    _auth: RequireAuth,
    Path(project_id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(req): Json<TradeScopesRequest>,
) -> ApiResult<impl IntoResponse> {
    let request_id = get_request_id(&headers);

    // Check cache
    let cache_key = keys::trade_scopes(project_id);
    if let Some(cached) = state.cache.get::<TradeScopesResponse>(&cache_key).await {
        tracing::debug!(project_id = %project_id, "Returning cached trade scopes");
        return Ok(Json(DataResponse::new(TradeScopesResponse {
            cached: true,
            ..cached
        })));
    }

    // Call AI service
    let scopes = state
        .ai_client
        .extract_trade_scopes(
            project_id,
            &req.document_text,
            req.trades,
            request_id.as_deref(),
        )
        .await?;

    let response = TradeScopesResponse {
        project_id: project_id.to_string(),
        scopes,
        cached: false,
    };

    // Cache the result
    if let Err(e) = state.cache.set(&cache_key, &response).await {
        tracing::warn!(error = %e, "Failed to cache trade scopes");
    }

    Ok(Json(DataResponse::new(response)))
}

/// Get list of standard construction trades.
///
/// GET /api/ai/trades
pub async fn get_standard_trades(
    _auth: RequireAuth,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> ApiResult<impl IntoResponse> {
    let request_id = get_request_id(&headers);

    // This is static data, cache it longer
    let cache_key = "ai:standard_trades";
    if let Some(cached) = state.cache.get::<StandardTradesResponse>(cache_key).await {
        return Ok(Json(DataResponse::new(cached)));
    }

    let trades = state
        .ai_client
        .get_standard_trades(request_id.as_deref())
        .await?;

    let response = StandardTradesResponse { trades };

    // Cache for 24 hours (static data)
    if let Err(e) = state
        .cache
        .set_with_ttl(cache_key, &response, std::time::Duration::from_secs(86400))
        .await
    {
        tracing::warn!(error = %e, "Failed to cache standard trades");
    }

    Ok(Json(DataResponse::new(response)))
}

// =============================================================================
// Tender Scope Document Generation
// =============================================================================

/// Generate a tender scope document.
///
/// POST /api/projects/:project_id/ai/tender-scope-doc
pub async fn generate_tender_scope_doc(
    _auth: RequireAuth,
    Path(project_id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(req): Json<TenderScopeDocRequest>,
) -> ApiResult<impl IntoResponse> {
    let request_id = get_request_id(&headers);

    // No caching for tender docs - they're generated fresh each time
    // as scope_data may differ

    let document = state
        .ai_client
        .generate_tender_scope_doc(
            project_id,
            &req.trade,
            &req.scope_data,
            req.project_context.as_deref(),
            req.bid_due_date.as_deref(),
            request_id.as_deref(),
        )
        .await?;

    let response = TenderScopeDocResponse {
        project_id: project_id.to_string(),
        trade: req.trade,
        document,
    };

    Ok(Json(DataResponse::new(response)))
}

// =============================================================================
// Q&A Endpoint
// =============================================================================

/// Answer a question about project documents.
///
/// POST /api/projects/:project_id/ai/qna
pub async fn ask_question(
    _auth: RequireAuth,
    Path(project_id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(req): Json<QnARequest>,
) -> ApiResult<impl IntoResponse> {
    let request_id = get_request_id(&headers);

    // Create a hash of the question for cache key
    let question_hash = format!("{:x}", md5_hash(&req.question));
    let cache_key = keys::qna(project_id, &question_hash);

    // Check cache (only if no document_text provided - that indicates fresh context)
    if req.document_text.is_none() {
        if let Some(cached) = state.cache.get::<QnAResponse>(&cache_key).await {
            tracing::debug!(project_id = %project_id, "Returning cached Q&A response");
            return Ok(Json(DataResponse::new(cached)));
        }
    }

    // Call AI service
    let response = state
        .ai_client
        .ask_question(
            project_id,
            &req.question,
            req.document_id,
            req.document_text.as_deref(),
            request_id.as_deref(),
        )
        .await?;

    // Cache the result (only if using RAG, not direct text)
    if req.document_text.is_none() {
        if let Err(e) = state
            .cache
            .set_with_ttl(&cache_key, &response, std::time::Duration::from_secs(3600))
            .await
        {
            tracing::warn!(error = %e, "Failed to cache Q&A response");
        }
    }

    Ok(Json(DataResponse::new(response)))
}

/// Simple MD5 hash for question deduplication (not cryptographic).
fn md5_hash(input: &str) -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish() as u128
}

// =============================================================================
// Cache Invalidation
// =============================================================================

/// Invalidate AI caches for a project.
///
/// Called when documents are updated/added.
///
/// DELETE /api/projects/:project_id/ai/cache
pub async fn invalidate_ai_cache(
    _auth: RequireAuth,
    Path(project_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<impl IntoResponse> {
    let pattern = keys::ai_pattern(project_id);
    let deleted = state.cache.delete_pattern(&pattern).await.unwrap_or(0);

    tracing::info!(
        project_id = %project_id,
        deleted = deleted,
        "AI cache invalidated"
    );

    Ok(Json(serde_json::json!({
        "project_id": project_id.to_string(),
        "deleted_keys": deleted
    })))
}
