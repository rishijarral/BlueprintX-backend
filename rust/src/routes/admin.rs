//! Admin routes
//!
//! Protected admin endpoints for:
//! - Dashboard statistics
//! - Verification management (approve/reject subcontractors)
//! - Audit log viewing
//!
//! All routes require admin privileges (is_admin flag on profile).

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, Query, State},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::pagination::PaginationParams;
use crate::api::response::{DataResponse, Paginated, PaginationMeta};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::admin::*;
use crate::error::{ApiError, ErrorResponse};
use crate::services::notifications;

// ============================================================================
// RequireAdmin Middleware
// ============================================================================

/// Extractor that requires admin privileges.
/// Uses RequireAuth internally and additionally checks is_admin flag.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RequireAdmin {
    pub auth: RequireAuth,
    pub admin_id: Uuid,
}

impl RequireAdmin {
    pub fn user_id(&self) -> Uuid {
        self.auth.user_id
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AdminAuthError {
    NotAuthenticated,
    NotAdmin,
    DatabaseError(String),
}

impl IntoResponse for AdminAuthError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AdminAuthError::NotAuthenticated => (StatusCode::UNAUTHORIZED, "Authentication required"),
            AdminAuthError::NotAdmin => (StatusCode::FORBIDDEN, "Admin privileges required"),
            AdminAuthError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = ErrorResponse {
            code: if status == StatusCode::FORBIDDEN {
                "FORBIDDEN".to_string()
            } else {
                "UNAUTHORIZED".to_string()
            },
            message: message.to_string(),
            request_id: None,
        };

        (status, Json(body)).into_response()
    }
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for RequireAdmin {
    type Rejection = AdminAuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // First, require authentication
        let auth = RequireAuth::from_request_parts(parts, state)
            .await
            .map_err(|_| AdminAuthError::NotAuthenticated)?;

        let user_id = auth.user_id;

        // Check if user has admin privileges
        let is_admin: Option<bool> = sqlx::query_scalar(
            "SELECT is_admin FROM profiles WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AdminAuthError::DatabaseError(e.to_string()))?
        .flatten();

        if is_admin != Some(true) {
            tracing::warn!(user_id = %user_id, "Non-admin user attempted to access admin route");
            return Err(AdminAuthError::NotAdmin);
        }

        Ok(RequireAdmin {
            auth,
            admin_id: user_id,
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Log an admin action to the audit log
async fn log_admin_action(
    db: &sqlx::PgPool,
    admin_id: Uuid,
    action: AdminAction,
    target_type: AuditTargetType,
    target_id: Option<Uuid>,
    details: serde_json::Value,
    ip_address: Option<String>,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO admin_audit_log (id, admin_id, action, target_type, target_id, details, ip_address)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(id)
    .bind(admin_id)
    .bind(action.to_string())
    .bind(target_type.to_string())
    .bind(target_id)
    .bind(&details)
    .bind(ip_address)
    .execute(db)
    .await?;

    tracing::info!(
        admin_id = %admin_id,
        action = %action,
        target_type = %target_type,
        target_id = ?target_id,
        "Admin action logged"
    );

    Ok(())
}

// ============================================================================
// Database Row Types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct PendingVerificationRow {
    id: Uuid,
    profile_id: Option<Uuid>,
    name: String,
    trade: String,
    location: Option<String>,
    contact_email: Option<String>,
    headline: Option<String>,
    company_description: Option<String>,
    year_established: Option<i32>,
    employee_count: Option<String>,
    certifications: serde_json::Value,
    insurance: serde_json::Value,
    license_info: serde_json::Value,
    created_at: DateTime<Utc>,
    profile_email: Option<String>,
    profile_name: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct AuditLogRow {
    id: Uuid,
    admin_id: Uuid,
    admin_name: Option<String>,
    action: String,
    target_type: String,
    target_id: Option<Uuid>,
    details: serde_json::Value,
    ip_address: Option<String>,
    created_at: DateTime<Utc>,
}

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct VerificationQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: VerificationQuery,
}

#[derive(Debug, Deserialize, Default)]
pub struct AuditLogQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: AuditLogQuery,
}

// ============================================================================
// Admin Dashboard
// ============================================================================

/// GET /api/admin/stats
///
/// Get dashboard statistics.
pub async fn get_admin_stats(
    State(state): State<Arc<AppState>>,
    admin: RequireAdmin,
) -> Result<impl IntoResponse, ApiError> {
    // Log the view action
    let _ = log_admin_action(
        &state.db,
        admin.user_id(),
        AdminAction::ViewSensitiveData,
        AuditTargetType::SystemSetting,
        None,
        serde_json::json!({"viewed": "dashboard_stats"}),
        None,
    )
    .await;

    // Fetch all stats
    let pending_verifications: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subcontractors WHERE verification_status = 'pending'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let total_subcontractors: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM subcontractors")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let verified_subcontractors: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subcontractors WHERE verification_status = 'verified'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let total_tenders: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenders")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let open_tenders: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tenders WHERE status = 'open'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let total_bids: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bids")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let total_contracts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let active_contracts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contracts WHERE status IN ('draft', 'pending_sub', 'pending_gc', 'gc_signed')",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profiles")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let gc_users: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM profiles WHERE user_type = 'general_contractor'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let sub_users: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM profiles WHERE user_type = 'subcontractor'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let recent_signups_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM profiles WHERE created_at > NOW() - INTERVAL '7 days'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let recent_verifications_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subcontractors WHERE verified_at > NOW() - INTERVAL '7 days'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let stats = AdminDashboardStats {
        pending_verifications,
        total_subcontractors,
        verified_subcontractors,
        total_tenders,
        open_tenders,
        total_bids,
        total_contracts,
        active_contracts,
        total_users,
        gc_users,
        sub_users,
        recent_signups_7d,
        recent_verifications_7d,
    };

    Ok(Json(DataResponse::new(stats)))
}

// ============================================================================
// Verification Management
// ============================================================================

/// GET /api/admin/verifications
///
/// List pending verification requests.
pub async fn list_pending_verifications(
    State(state): State<Arc<AppState>>,
    Query(query): Query<VerificationQueryParams>,
    _admin: RequireAdmin,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM subcontractors s
        WHERE s.verification_status = 'pending'
        AND ($1::text IS NULL OR s.trade ILIKE '%' || $1 || '%')
        "#,
    )
    .bind(&query.filter.trade)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Determine sort
    let order_by = match query.filter.sort_by.as_deref() {
        Some("name") => "s.name",
        _ => "s.created_at",
    };
    let order_dir = match query.filter.sort_order.as_deref() {
        Some("desc") => "DESC",
        _ => "ASC",
    };

    let query_str = format!(
        r#"
        SELECT 
            s.id, s.profile_id, s.name, s.trade, s.location, s.contact_email,
            s.headline, s.company_description, s.year_established, s.employee_count,
            COALESCE(s.certifications, '[]'::jsonb) as certifications,
            COALESCE(s.insurance, '{{}}'::jsonb) as insurance,
            COALESCE(s.license_info, '{{}}'::jsonb) as license_info,
            s.created_at,
            p.email as profile_email,
            COALESCE(p.company_name, p.first_name || ' ' || p.last_name) as profile_name
        FROM subcontractors s
        LEFT JOIN profiles p ON s.profile_id = p.id
        WHERE s.verification_status = 'pending'
        AND ($1::text IS NULL OR s.trade ILIKE '%' || $1 || '%')
        ORDER BY {} {}
        LIMIT $2 OFFSET $3
        "#,
        order_by, order_dir
    );

    let rows = sqlx::query_as::<_, PendingVerificationRow>(&query_str)
        .bind(&query.filter.trade)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<PendingVerification> = rows
        .into_iter()
        .map(|r| PendingVerification {
            id: r.id,
            profile_id: r.profile_id,
            name: r.name,
            trade: r.trade,
            location: r.location,
            contact_email: r.contact_email,
            headline: r.headline,
            company_description: r.company_description,
            year_established: r.year_established,
            employee_count: r.employee_count,
            certifications: serde_json::from_value(r.certifications).unwrap_or_default(),
            insurance: serde_json::from_value(r.insurance).ok(),
            license_info: serde_json::from_value(r.license_info).ok(),
            created_at: r.created_at,
            profile_email: r.profile_email,
            profile_name: r.profile_name,
        })
        .collect();

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

    Ok(Json(Paginated {
        data,
        pagination: PaginationMeta {
            page,
            per_page,
            total_items: total as u64,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
    }))
}

/// GET /api/admin/verifications/:id
///
/// Get a specific verification request details.
pub async fn get_verification(
    State(state): State<Arc<AppState>>,
    Path(sub_id): Path<Uuid>,
    _admin: RequireAdmin,
) -> Result<impl IntoResponse, ApiError> {
    let row = sqlx::query_as::<_, PendingVerificationRow>(
        r#"
        SELECT 
            s.id, s.profile_id, s.name, s.trade, s.location, s.contact_email,
            s.headline, s.company_description, s.year_established, s.employee_count,
            COALESCE(s.certifications, '[]'::jsonb) as certifications,
            COALESCE(s.insurance, '{}'::jsonb) as insurance,
            COALESCE(s.license_info, '{}'::jsonb) as license_info,
            s.created_at,
            p.email as profile_email,
            COALESCE(p.company_name, p.first_name || ' ' || p.last_name) as profile_name
        FROM subcontractors s
        LEFT JOIN profiles p ON s.profile_id = p.id
        WHERE s.id = $1
        "#,
    )
    .bind(sub_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Subcontractor not found"))?;

    let verification = PendingVerification {
        id: row.id,
        profile_id: row.profile_id,
        name: row.name,
        trade: row.trade,
        location: row.location,
        contact_email: row.contact_email,
        headline: row.headline,
        company_description: row.company_description,
        year_established: row.year_established,
        employee_count: row.employee_count,
        certifications: serde_json::from_value(row.certifications).unwrap_or_default(),
        insurance: serde_json::from_value(row.insurance).ok(),
        license_info: serde_json::from_value(row.license_info).ok(),
        created_at: row.created_at,
        profile_email: row.profile_email,
        profile_name: row.profile_name,
    };

    Ok(Json(DataResponse::new(verification)))
}

/// POST /api/admin/verifications/:id/approve
///
/// Approve a subcontractor's verification request.
pub async fn approve_verification(
    State(state): State<Arc<AppState>>,
    Path(sub_id): Path<Uuid>,
    admin: RequireAdmin,
    Json(input): Json<ApproveVerificationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Get subcontractor and profile info
    let sub_info: Option<(String, Option<Uuid>, String)> = sqlx::query_as(
        r#"
        SELECT s.verification_status, s.profile_id, s.name
        FROM subcontractors s
        WHERE s.id = $1
        "#,
    )
    .bind(sub_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let (current_status, profile_id, sub_name) = sub_info
        .ok_or_else(|| ApiError::not_found("Subcontractor not found"))?;

    if current_status == "verified" {
        return Err(ApiError::bad_request("Subcontractor is already verified"));
    }

    // Update verification status
    sqlx::query(
        r#"
        UPDATE subcontractors SET
            verification_status = 'verified',
            verified = true,
            verified_at = NOW(),
            verified_by = $1,
            verification_notes = $2,
            updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(admin.user_id())
    .bind(&input.notes)
    .bind(sub_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to approve verification: {}", e)))?;

    // Log the action
    let _ = log_admin_action(
        &state.db,
        admin.user_id(),
        AdminAction::VerifySubcontractor,
        AuditTargetType::Subcontractor,
        Some(sub_id),
        serde_json::json!({
            "subcontractor_name": sub_name,
            "notes": input.notes,
        }),
        None,
    )
    .await;

    // Send notification to subcontractor
    if let Some(profile_id) = profile_id {
        if let Err(e) = notifications::notify_profile_verified(&state.db, profile_id).await {
            tracing::warn!(error = %e, "Failed to send verification notification");
        }
    }

    Ok(Json(serde_json::json!({ 
        "success": true, 
        "message": "Subcontractor verified successfully" 
    })))
}

/// POST /api/admin/verifications/:id/reject
///
/// Reject a subcontractor's verification request.
pub async fn reject_verification(
    State(state): State<Arc<AppState>>,
    Path(sub_id): Path<Uuid>,
    admin: RequireAdmin,
    Json(input): Json<RejectVerificationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if input.reason.is_empty() {
        return Err(ApiError::bad_request("Rejection reason is required"));
    }

    // Get subcontractor info
    let sub_info: Option<(String, Option<Uuid>, String)> = sqlx::query_as(
        r#"
        SELECT s.verification_status, s.profile_id, s.name
        FROM subcontractors s
        WHERE s.id = $1
        "#,
    )
    .bind(sub_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let (current_status, profile_id, sub_name) = sub_info
        .ok_or_else(|| ApiError::not_found("Subcontractor not found"))?;

    if current_status == "verified" {
        return Err(ApiError::bad_request(
            "Cannot reject an already verified subcontractor",
        ));
    }

    // Update verification status
    sqlx::query(
        r#"
        UPDATE subcontractors SET
            verification_status = 'rejected',
            verified = false,
            verification_notes = $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(format!("Rejected: {}. Notes: {}", input.reason, input.notes.clone().unwrap_or_default()))
    .bind(sub_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to reject verification: {}", e)))?;

    // Log the action
    let _ = log_admin_action(
        &state.db,
        admin.user_id(),
        AdminAction::RejectSubcontractor,
        AuditTargetType::Subcontractor,
        Some(sub_id),
        serde_json::json!({
            "subcontractor_name": sub_name,
            "reason": input.reason,
            "notes": input.notes,
        }),
        None,
    )
    .await;

    // Send notification to subcontractor
    if let Some(profile_id) = profile_id {
        if let Err(e) =
            notifications::notify_profile_rejected(&state.db, profile_id, &input.reason).await
        {
            tracing::warn!(error = %e, "Failed to send rejection notification");
        }
    }

    Ok(Json(serde_json::json!({ 
        "success": true, 
        "message": "Verification rejected" 
    })))
}

// ============================================================================
// Audit Log
// ============================================================================

/// GET /api/admin/audit-log
///
/// View admin action audit log.
pub async fn list_audit_log(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditLogQueryParams>,
    _admin: RequireAdmin,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(50).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM admin_audit_log a
        WHERE ($1::uuid IS NULL OR a.admin_id = $1)
        AND ($2::text IS NULL OR a.action = $2)
        AND ($3::text IS NULL OR a.target_type = $3)
        AND ($4::uuid IS NULL OR a.target_id = $4)
        AND ($5::timestamptz IS NULL OR a.created_at >= $5)
        AND ($6::timestamptz IS NULL OR a.created_at <= $6)
        "#,
    )
    .bind(query.filter.admin_id)
    .bind(&query.filter.action)
    .bind(&query.filter.target_type)
    .bind(query.filter.target_id)
    .bind(query.filter.from_date)
    .bind(query.filter.to_date)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rows = sqlx::query_as::<_, AuditLogRow>(
        r#"
        SELECT 
            a.id, a.admin_id,
            COALESCE(p.company_name, p.first_name || ' ' || p.last_name) as admin_name,
            a.action, a.target_type, a.target_id, a.details, a.ip_address, a.created_at
        FROM admin_audit_log a
        LEFT JOIN profiles p ON a.admin_id = p.id
        WHERE ($1::uuid IS NULL OR a.admin_id = $1)
        AND ($2::text IS NULL OR a.action = $2)
        AND ($3::text IS NULL OR a.target_type = $3)
        AND ($4::uuid IS NULL OR a.target_id = $4)
        AND ($5::timestamptz IS NULL OR a.created_at >= $5)
        AND ($6::timestamptz IS NULL OR a.created_at <= $6)
        ORDER BY a.created_at DESC
        LIMIT $7 OFFSET $8
        "#,
    )
    .bind(query.filter.admin_id)
    .bind(&query.filter.action)
    .bind(&query.filter.target_type)
    .bind(query.filter.target_id)
    .bind(query.filter.from_date)
    .bind(query.filter.to_date)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<AdminAuditLogResponse> = rows
        .into_iter()
        .map(|r| AdminAuditLogResponse {
            id: r.id,
            admin_id: r.admin_id,
            admin_name: r.admin_name,
            action: r.action,
            target_type: r.target_type,
            target_id: r.target_id,
            details: r.details,
            ip_address: r.ip_address,
            created_at: r.created_at,
        })
        .collect();

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

    Ok(Json(Paginated {
        data,
        pagination: PaginationMeta {
            page,
            per_page,
            total_items: total as u64,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
    }))
}

// ============================================================================
// Admin User Check (for frontend)
// ============================================================================

/// GET /api/admin/check
///
/// Check if the current user is an admin. Returns 200 if admin, 403 if not.
pub async fn check_admin(
    _admin: RequireAdmin,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(serde_json::json!({ "is_admin": true })))
}
