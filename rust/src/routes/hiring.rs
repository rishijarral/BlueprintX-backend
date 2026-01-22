//! Hiring routes
//!
//! Endpoints for marketplace hiring: external subs, hire requests, contracts, messages, team.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
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
use crate::domain::hiring::*;
use crate::error::ApiError;

// ============================================================================
// Database Row Types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct ExternalSubRow {
    id: Uuid,
    added_by: Uuid,
    company_name: String,
    contact_name: Option<String>,
    contact_email: Option<String>,
    contact_phone: Option<String>,
    trade: String,
    secondary_trades: serde_json::Value,
    location: Option<String>,
    address: Option<String>,
    license_number: Option<String>,
    insurance_info: Option<String>,
    notes: Option<String>,
    rating: sqlx::types::Decimal,
    projects_together: i32,
    is_preferred: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct HireRequestRow {
    id: Uuid,
    project_id: Uuid,
    project_name: String,
    tender_id: Option<Uuid>,
    gc_id: Uuid,
    gc_company_name: String,
    subcontractor_id: Option<Uuid>,
    external_sub_id: Option<Uuid>,
    sub_company_name: String,
    sub_contact_name: Option<String>,
    sub_contact_email: Option<String>,
    sub_contact_phone: Option<String>,
    sub_trade: String,
    sub_location: Option<String>,
    sub_rating: Option<sqlx::types::Decimal>,
    sub_verified: bool,
    status: String,
    trade: String,
    title: String,
    message: Option<String>,
    scope_description: Option<String>,
    proposed_amount: Option<sqlx::types::Decimal>,
    rate_type: Option<String>,
    unit_description: Option<String>,
    estimated_hours: Option<i32>,
    estimated_start_date: Option<DateTime<Utc>>,
    estimated_end_date: Option<DateTime<Utc>>,
    response_deadline: Option<DateTime<Utc>>,
    sub_response: Option<String>,
    sub_counter_amount: Option<sqlx::types::Decimal>,
    viewed_at: Option<DateTime<Utc>>,
    responded_at: Option<DateTime<Utc>>,
    hired_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct HireMessageRow {
    id: Uuid,
    hire_request_id: Uuid,
    sender_id: Uuid,
    sender_name: String,
    sender_type: String,
    message: String,
    message_type: String,
    metadata: serde_json::Value,
    is_read: bool,
    read_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
struct ContractTemplateRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    template_type: String,
    content: String,
    sections: serde_json::Value,
    variables: serde_json::Value,
    is_system: bool,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
struct ContractRow {
    id: Uuid,
    hire_request_id: Uuid,
    project_id: Uuid,
    project_name: String,
    template_id: Option<Uuid>,
    template_name: Option<String>,
    contract_number: Option<String>,
    title: String,
    content: String,
    sections: serde_json::Value,
    terms_summary: Option<String>,
    amount: sqlx::types::Decimal,
    payment_schedule: serde_json::Value,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    gc_signature: Option<String>,
    gc_signed_at: Option<DateTime<Utc>>,
    sub_signature: Option<String>,
    sub_signed_at: Option<DateTime<Utc>>,
    status: String,
    pdf_path: Option<String>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct TeamMemberRow {
    id: Uuid,
    project_id: Uuid,
    hire_request_id: Option<Uuid>,
    contract_id: Option<Uuid>,
    subcontractor_id: Option<Uuid>,
    external_sub_id: Option<Uuid>,
    sub_company_name: String,
    sub_contact_name: Option<String>,
    sub_contact_email: Option<String>,
    sub_contact_phone: Option<String>,
    sub_trade: String,
    sub_location: Option<String>,
    sub_rating: Option<sqlx::types::Decimal>,
    sub_verified: bool,
    role: Option<String>,
    trade: String,
    responsibilities: Option<String>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    hourly_rate: Option<sqlx::types::Decimal>,
    status: String,
    performance_rating: Option<sqlx::types::Decimal>,
    notes: Option<String>,
    joined_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn decimal_to_f64(d: sqlx::types::Decimal) -> f64 {
    use std::str::FromStr;
    f64::from_str(&d.to_string()).unwrap_or(0.0)
}

fn decimal_opt_to_f64(d: Option<sqlx::types::Decimal>) -> Option<f64> {
    d.map(decimal_to_f64)
}

fn make_subcontractor_info(
    id: Option<Uuid>,
    external_id: Option<Uuid>,
    company_name: String,
    contact_name: Option<String>,
    contact_email: Option<String>,
    contact_phone: Option<String>,
    trade: String,
    location: Option<String>,
    rating: Option<sqlx::types::Decimal>,
    verified: bool,
) -> HireRequestSubcontractor {
    HireRequestSubcontractor {
        id: id.or(external_id).unwrap_or(Uuid::nil()),
        is_external: external_id.is_some(),
        company_name,
        contact_name,
        contact_email,
        contact_phone,
        trade,
        location,
        rating: rating.map(decimal_to_f64),
        verified,
    }
}

// ============================================================================
// External Subcontractors
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct ExternalSubQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: ExternalSubcontractorQuery,
}

/// GET /api/my-subcontractors
///
/// List external subcontractors added by the current user.
pub async fn list_external_subcontractors(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ExternalSubQueryParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;
    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM external_subcontractors
        WHERE added_by = $1
        AND ($2::text IS NULL OR trade ILIKE '%' || $2 || '%')
        AND ($3::bool IS NULL OR is_preferred = $3)
        AND ($4::text IS NULL OR company_name ILIKE '%' || $4 || '%')
        "#,
    )
    .bind(user_id)
    .bind(&query.filter.trade)
    .bind(query.filter.is_preferred)
    .bind(&query.filter.search)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rows = sqlx::query_as::<_, ExternalSubRow>(
        r#"
        SELECT id, added_by, company_name, contact_name, contact_email, contact_phone,
               trade, secondary_trades, location, address, license_number, insurance_info,
               notes, rating, projects_together, is_preferred, created_at, updated_at
        FROM external_subcontractors
        WHERE added_by = $1
        AND ($2::text IS NULL OR trade ILIKE '%' || $2 || '%')
        AND ($3::bool IS NULL OR is_preferred = $3)
        AND ($4::text IS NULL OR company_name ILIKE '%' || $4 || '%')
        ORDER BY is_preferred DESC, company_name
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(user_id)
    .bind(&query.filter.trade)
    .bind(query.filter.is_preferred)
    .bind(&query.filter.search)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<ExternalSubcontractorResponse> = rows
        .into_iter()
        .map(|r| {
            let secondary_trades: Vec<String> =
                serde_json::from_value(r.secondary_trades).unwrap_or_default();
            ExternalSubcontractorResponse {
                id: r.id,
                added_by: r.added_by,
                company_name: r.company_name,
                contact_name: r.contact_name,
                contact_email: r.contact_email,
                contact_phone: r.contact_phone,
                trade: r.trade,
                secondary_trades,
                location: r.location,
                address: r.address,
                license_number: r.license_number,
                insurance_info: r.insurance_info,
                notes: r.notes,
                rating: decimal_to_f64(r.rating),
                projects_together: r.projects_together,
                is_preferred: r.is_preferred,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }
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

/// POST /api/my-subcontractors
pub async fn create_external_subcontractor(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
    Json(input): Json<CreateExternalSubcontractorInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;
    let id = Uuid::new_v4();
    let secondary_trades = serde_json::to_value(input.secondary_trades.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));

    sqlx::query(
        r#"
        INSERT INTO external_subcontractors (
            id, added_by, company_name, contact_name, contact_email, contact_phone,
            trade, secondary_trades, location, address, license_number, insurance_info,
            notes, is_preferred
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(&input.company_name)
    .bind(&input.contact_name)
    .bind(&input.contact_email)
    .bind(&input.contact_phone)
    .bind(&input.trade)
    .bind(&secondary_trades)
    .bind(&input.location)
    .bind(&input.address)
    .bind(&input.license_number)
    .bind(&input.insurance_info)
    .bind(&input.notes)
    .bind(input.is_preferred.unwrap_or(false))
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create subcontractor: {}", e)))?;

    Ok(Json(serde_json::json!({ "id": id, "success": true })))
}

/// GET /api/my-subcontractors/:id
pub async fn get_external_subcontractor(
    State(state): State<Arc<AppState>>,
    Path(sub_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let row = sqlx::query_as::<_, ExternalSubRow>(
        "SELECT * FROM external_subcontractors WHERE id = $1 AND added_by = $2",
    )
    .bind(sub_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Subcontractor not found"))?;

    let secondary_trades: Vec<String> =
        serde_json::from_value(row.secondary_trades).unwrap_or_default();

    let response = ExternalSubcontractorResponse {
        id: row.id,
        added_by: row.added_by,
        company_name: row.company_name,
        contact_name: row.contact_name,
        contact_email: row.contact_email,
        contact_phone: row.contact_phone,
        trade: row.trade,
        secondary_trades,
        location: row.location,
        address: row.address,
        license_number: row.license_number,
        insurance_info: row.insurance_info,
        notes: row.notes,
        rating: decimal_to_f64(row.rating),
        projects_together: row.projects_together,
        is_preferred: row.is_preferred,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(DataResponse::new(response)))
}

/// PUT /api/my-subcontractors/:id
pub async fn update_external_subcontractor(
    State(state): State<Arc<AppState>>,
    Path(sub_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<UpdateExternalSubcontractorInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;
    let secondary_trades = input.secondary_trades.map(|t| serde_json::to_value(t).unwrap_or(serde_json::json!([])));

    let result = sqlx::query(
        r#"
        UPDATE external_subcontractors SET
            company_name = COALESCE($1, company_name),
            contact_name = COALESCE($2, contact_name),
            contact_email = COALESCE($3, contact_email),
            contact_phone = COALESCE($4, contact_phone),
            trade = COALESCE($5, trade),
            secondary_trades = COALESCE($6, secondary_trades),
            location = COALESCE($7, location),
            address = COALESCE($8, address),
            license_number = COALESCE($9, license_number),
            insurance_info = COALESCE($10, insurance_info),
            notes = COALESCE($11, notes),
            rating = COALESCE($12, rating),
            is_preferred = COALESCE($13, is_preferred),
            updated_at = NOW()
        WHERE id = $14 AND added_by = $15
        "#,
    )
    .bind(&input.company_name)
    .bind(&input.contact_name)
    .bind(&input.contact_email)
    .bind(&input.contact_phone)
    .bind(&input.trade)
    .bind(secondary_trades)
    .bind(&input.location)
    .bind(&input.address)
    .bind(&input.license_number)
    .bind(&input.insurance_info)
    .bind(&input.notes)
    .bind(input.rating)
    .bind(input.is_preferred)
    .bind(sub_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update subcontractor: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Subcontractor not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/my-subcontractors/:id
pub async fn delete_external_subcontractor(
    State(state): State<Arc<AppState>>,
    Path(sub_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let result = sqlx::query("DELETE FROM external_subcontractors WHERE id = $1 AND added_by = $2")
        .bind(sub_id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete subcontractor: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Subcontractor not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================================
// Hire Requests
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct HireRequestQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: HireRequestQuery,
}

/// GET /api/hiring
///
/// List hire requests (as GC or as subcontractor).
pub async fn list_hire_requests(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HireRequestQueryParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;
    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Determine if viewing as GC or sub
    let as_gc = query.filter.as_gc.unwrap_or(true);

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) 
        FROM hire_requests hr
        JOIN projects p ON hr.project_id = p.id
        JOIN profiles gc ON hr.gc_id = gc.id
        LEFT JOIN subcontractors s ON hr.subcontractor_id = s.id
        LEFT JOIN external_subcontractors es ON hr.external_sub_id = es.id
        WHERE (($1 AND hr.gc_id = $2) OR (NOT $1 AND s.profile_id = $2))
        AND ($3::uuid IS NULL OR hr.project_id = $3)
        AND ($4::text IS NULL OR hr.status = $4)
        AND ($5::text IS NULL OR hr.trade ILIKE '%' || $5 || '%')
        "#,
    )
    .bind(as_gc)
    .bind(user_id)
    .bind(query.filter.project_id)
    .bind(&query.filter.status)
    .bind(&query.filter.trade)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rows = sqlx::query_as::<_, HireRequestRow>(
        r#"
        SELECT 
            hr.id, hr.project_id, p.name as project_name, hr.tender_id, hr.gc_id,
            gc.company_name as gc_company_name, hr.subcontractor_id, hr.external_sub_id,
            COALESCE(s.name, es.company_name) as sub_company_name,
            COALESCE(s.contact_email, es.contact_name) as sub_contact_name,
            COALESCE(s.contact_email, es.contact_email) as sub_contact_email,
            COALESCE(s.contact_phone, es.contact_phone) as sub_contact_phone,
            COALESCE(s.trade, es.trade) as sub_trade,
            COALESCE(s.location, es.location) as sub_location,
            s.rating as sub_rating,
            COALESCE(s.verified, false) as sub_verified,
            hr.status, hr.trade, hr.title, hr.message, hr.scope_description,
            hr.proposed_amount, hr.rate_type, hr.unit_description, hr.estimated_hours,
            hr.estimated_start_date, hr.estimated_end_date, hr.response_deadline,
            hr.sub_response, hr.sub_counter_amount, hr.viewed_at, hr.responded_at,
            hr.hired_at, hr.created_at, hr.updated_at
        FROM hire_requests hr
        JOIN projects p ON hr.project_id = p.id
        JOIN profiles gc ON hr.gc_id = gc.id
        LEFT JOIN subcontractors s ON hr.subcontractor_id = s.id
        LEFT JOIN external_subcontractors es ON hr.external_sub_id = es.id
        WHERE (($1 AND hr.gc_id = $2) OR (NOT $1 AND s.profile_id = $2))
        AND ($3::uuid IS NULL OR hr.project_id = $3)
        AND ($4::text IS NULL OR hr.status = $4)
        AND ($5::text IS NULL OR hr.trade ILIKE '%' || $5 || '%')
        ORDER BY hr.updated_at DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(as_gc)
    .bind(user_id)
    .bind(query.filter.project_id)
    .bind(&query.filter.status)
    .bind(&query.filter.trade)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<HireRequestResponse> = rows
        .into_iter()
        .map(|r| {
            let subcontractor = make_subcontractor_info(
                r.subcontractor_id,
                r.external_sub_id,
                r.sub_company_name,
                r.sub_contact_name,
                r.sub_contact_email,
                r.sub_contact_phone,
                r.sub_trade,
                r.sub_location,
                r.sub_rating,
                r.sub_verified,
            );

            HireRequestResponse {
                id: r.id,
                project_id: r.project_id,
                project_name: r.project_name,
                tender_id: r.tender_id,
                gc_id: r.gc_id,
                gc_company_name: r.gc_company_name,
                subcontractor,
                status: r.status,
                trade: r.trade,
                title: r.title,
                message: r.message,
                scope_description: r.scope_description,
                proposed_amount: decimal_opt_to_f64(r.proposed_amount),
                rate_type: r.rate_type,
                unit_description: r.unit_description,
                estimated_hours: r.estimated_hours,
                estimated_start_date: r.estimated_start_date,
                estimated_end_date: r.estimated_end_date,
                response_deadline: r.response_deadline,
                sub_response: r.sub_response,
                sub_counter_amount: decimal_opt_to_f64(r.sub_counter_amount),
                unread_messages: 0, // TODO: Calculate from messages
                contract_id: None,  // TODO: Fetch from contracts
                viewed_at: r.viewed_at,
                responded_at: r.responded_at,
                hired_at: r.hired_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }
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

/// POST /api/hiring
///
/// Create a new hire request.
pub async fn create_hire_request(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
    Json(input): Json<CreateHireRequestInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Verify project ownership
    let project_owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(input.project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if project_owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't own this project"));
    }

    // Validate that either subcontractor_id or external_sub_id is provided
    if input.subcontractor_id.is_none() && input.external_sub_id.is_none() {
        return Err(ApiError::bad_request(
            "Either subcontractor_id or external_sub_id must be provided",
        ));
    }
    if input.subcontractor_id.is_some() && input.external_sub_id.is_some() {
        return Err(ApiError::bad_request(
            "Cannot specify both subcontractor_id and external_sub_id",
        ));
    }

    let id = Uuid::new_v4();
    let status = if input.send_immediately.unwrap_or(false) {
        "sent"
    } else {
        "draft"
    };

    sqlx::query(
        r#"
        INSERT INTO hire_requests (
            id, project_id, tender_id, gc_id, subcontractor_id, external_sub_id,
            status, trade, title, message, scope_description, proposed_amount,
            rate_type, unit_description, estimated_hours, estimated_start_date,
            estimated_end_date, response_deadline
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        "#,
    )
    .bind(id)
    .bind(input.project_id)
    .bind(input.tender_id)
    .bind(user_id)
    .bind(input.subcontractor_id)
    .bind(input.external_sub_id)
    .bind(status)
    .bind(&input.trade)
    .bind(&input.title)
    .bind(&input.message)
    .bind(&input.scope_description)
    .bind(input.proposed_amount)
    .bind(&input.rate_type)
    .bind(&input.unit_description)
    .bind(input.estimated_hours)
    .bind(input.estimated_start_date)
    .bind(input.estimated_end_date)
    .bind(input.response_deadline)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create hire request: {}", e)))?;

    Ok(Json(serde_json::json!({ "id": id, "status": status, "success": true })))
}

/// GET /api/hiring/:id
pub async fn get_hire_request(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let row = sqlx::query_as::<_, HireRequestRow>(
        r#"
        SELECT 
            hr.id, hr.project_id, p.name as project_name, hr.tender_id, hr.gc_id,
            gc.company_name as gc_company_name, hr.subcontractor_id, hr.external_sub_id,
            COALESCE(s.name, es.company_name) as sub_company_name,
            COALESCE(s.contact_email, es.contact_name) as sub_contact_name,
            COALESCE(s.contact_email, es.contact_email) as sub_contact_email,
            COALESCE(s.contact_phone, es.contact_phone) as sub_contact_phone,
            COALESCE(s.trade, es.trade) as sub_trade,
            COALESCE(s.location, es.location) as sub_location,
            s.rating as sub_rating,
            COALESCE(s.verified, false) as sub_verified,
            hr.status, hr.trade, hr.title, hr.message, hr.scope_description,
            hr.proposed_amount, hr.rate_type, hr.unit_description, hr.estimated_hours,
            hr.estimated_start_date, hr.estimated_end_date, hr.response_deadline,
            hr.sub_response, hr.sub_counter_amount, hr.viewed_at, hr.responded_at,
            hr.hired_at, hr.created_at, hr.updated_at
        FROM hire_requests hr
        JOIN projects p ON hr.project_id = p.id
        JOIN profiles gc ON hr.gc_id = gc.id
        LEFT JOIN subcontractors s ON hr.subcontractor_id = s.id
        LEFT JOIN external_subcontractors es ON hr.external_sub_id = es.id
        WHERE hr.id = $1 AND (hr.gc_id = $2 OR s.profile_id = $2)
        "#,
    )
    .bind(request_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Hire request not found"))?;

    // Mark as viewed if sub is viewing
    if row.subcontractor_id.is_some() && row.viewed_at.is_none() {
        let _ = sqlx::query(
            "UPDATE hire_requests SET viewed_at = NOW(), status = CASE WHEN status = 'sent' THEN 'viewed' ELSE status END WHERE id = $1"
        )
        .bind(request_id)
        .execute(&state.db)
        .await;
    }

    let subcontractor = make_subcontractor_info(
        row.subcontractor_id,
        row.external_sub_id,
        row.sub_company_name,
        row.sub_contact_name,
        row.sub_contact_email,
        row.sub_contact_phone,
        row.sub_trade,
        row.sub_location,
        row.sub_rating,
        row.sub_verified,
    );

    let response = HireRequestResponse {
        id: row.id,
        project_id: row.project_id,
        project_name: row.project_name,
        tender_id: row.tender_id,
        gc_id: row.gc_id,
        gc_company_name: row.gc_company_name,
        subcontractor,
        status: row.status,
        trade: row.trade,
        title: row.title,
        message: row.message,
        scope_description: row.scope_description,
        proposed_amount: decimal_opt_to_f64(row.proposed_amount),
        rate_type: row.rate_type,
        unit_description: row.unit_description,
        estimated_hours: row.estimated_hours,
        estimated_start_date: row.estimated_start_date,
        estimated_end_date: row.estimated_end_date,
        response_deadline: row.response_deadline,
        sub_response: row.sub_response,
        sub_counter_amount: decimal_opt_to_f64(row.sub_counter_amount),
        unread_messages: 0,
        contract_id: None,
        viewed_at: row.viewed_at,
        responded_at: row.responded_at,
        hired_at: row.hired_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(DataResponse::new(response)))
}

/// PUT /api/hiring/:id
pub async fn update_hire_request(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<UpdateHireRequestInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let result = sqlx::query(
        r#"
        UPDATE hire_requests SET
            title = COALESCE($1, title),
            message = COALESCE($2, message),
            scope_description = COALESCE($3, scope_description),
            proposed_amount = COALESCE($4, proposed_amount),
            rate_type = COALESCE($5, rate_type),
            unit_description = COALESCE($6, unit_description),
            estimated_hours = COALESCE($7, estimated_hours),
            estimated_start_date = COALESCE($8, estimated_start_date),
            estimated_end_date = COALESCE($9, estimated_end_date),
            response_deadline = COALESCE($10, response_deadline),
            updated_at = NOW()
        WHERE id = $11 AND gc_id = $12 AND status IN ('draft', 'pending', 'sent')
        "#,
    )
    .bind(&input.title)
    .bind(&input.message)
    .bind(&input.scope_description)
    .bind(input.proposed_amount)
    .bind(&input.rate_type)
    .bind(&input.unit_description)
    .bind(input.estimated_hours)
    .bind(input.estimated_start_date)
    .bind(input.estimated_end_date)
    .bind(input.response_deadline)
    .bind(request_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update hire request: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Hire request not found or cannot be updated"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// POST /api/hiring/:id/status
///
/// Update hire request status (for status transitions).
pub async fn update_hire_request_status(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<HireRequestStatusInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Validate status transition
    let current: Option<(String, Uuid, Option<Uuid>)> = sqlx::query_as(
        r#"
        SELECT hr.status, hr.gc_id, s.profile_id
        FROM hire_requests hr
        LEFT JOIN subcontractors s ON hr.subcontractor_id = s.id
        WHERE hr.id = $1
        "#,
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let (current_status, gc_id, sub_profile_id) = current
        .ok_or_else(|| ApiError::not_found("Hire request not found"))?;

    let is_gc = gc_id == user_id;
    let is_sub = sub_profile_id == Some(user_id);

    if !is_gc && !is_sub {
        return Err(ApiError::forbidden("You don't have access to this hire request"));
    }

    // Validate status transition based on role
    let new_status = input.status.as_str();
    let valid_transition = match (current_status.as_str(), new_status, is_gc) {
        ("draft", "sent", true) => true,
        ("draft", "cancelled", true) => true,
        ("sent", "cancelled", true) => true,
        ("viewed", "interested", false) => true,
        ("viewed", "declined", false) => true,
        ("interested", "negotiating", _) => true,
        ("negotiating", "contract_sent", true) => true,
        ("contract_sent", "contract_signed", false) => true,
        ("contract_signed", "hired", true) => true,
        (_, "cancelled", true) => true,
        (_, "declined", false) => true,
        _ => false,
    };

    if !valid_transition {
        return Err(ApiError::bad_request(format!(
            "Cannot transition from '{}' to '{}'",
            current_status, new_status
        )));
    }

    let mut query = String::from("UPDATE hire_requests SET status = $1, updated_at = NOW()");
    
    if new_status == "interested" || new_status == "declined" {
        query.push_str(", responded_at = NOW(), sub_response = $3");
        if let Some(counter) = input.counter_amount {
            query.push_str(&format!(", sub_counter_amount = {}", counter));
        }
    }
    if new_status == "hired" {
        query.push_str(", hired_at = NOW()");
    }
    
    query.push_str(" WHERE id = $2");

    if input.response.is_some() {
        sqlx::query(&query)
            .bind(new_status)
            .bind(request_id)
            .bind(&input.response)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to update status: {}", e)))?;
    } else {
        sqlx::query("UPDATE hire_requests SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_status)
            .bind(request_id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to update status: {}", e)))?;
    }

    Ok(Json(serde_json::json!({ "success": true, "status": new_status })))
}

// ============================================================================
// Hire Messages
// ============================================================================

/// GET /api/hiring/:id/messages
pub async fn list_hire_messages(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Verify access
    let has_access: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM hire_requests hr
            LEFT JOIN subcontractors s ON hr.subcontractor_id = s.id
            WHERE hr.id = $1 AND (hr.gc_id = $2 OR s.profile_id = $2)
        )
        "#,
    )
    .bind(request_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !has_access {
        return Err(ApiError::forbidden("You don't have access to these messages"));
    }

    let rows = sqlx::query_as::<_, HireMessageRow>(
        r#"
        SELECT hm.id, hm.hire_request_id, hm.sender_id,
               COALESCE(p.company_name, p.first_name || ' ' || p.last_name) as sender_name,
               hm.sender_type, hm.message, hm.message_type, hm.metadata,
               hm.is_read, hm.read_at, hm.created_at
        FROM hire_messages hm
        JOIN profiles p ON hm.sender_id = p.id
        WHERE hm.hire_request_id = $1
        ORDER BY hm.created_at ASC
        "#,
    )
    .bind(request_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Mark unread messages as read
    sqlx::query(
        "UPDATE hire_messages SET is_read = true, read_at = NOW() WHERE hire_request_id = $1 AND sender_id != $2 AND is_read = false",
    )
    .bind(request_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    .ok();

    let messages: Vec<HireMessageResponse> = rows
        .into_iter()
        .map(|r| HireMessageResponse {
            id: r.id,
            hire_request_id: r.hire_request_id,
            sender_id: r.sender_id,
            sender_name: r.sender_name,
            sender_type: r.sender_type,
            message: r.message,
            message_type: r.message_type,
            metadata: r.metadata,
            is_read: r.is_read,
            read_at: r.read_at,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(DataResponse::new(messages)))
}

/// POST /api/hiring/:id/messages
pub async fn send_hire_message(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<SendMessageInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Get user type and verify access
    let access: Option<(Uuid, Option<Uuid>, String)> = sqlx::query_as(
        r#"
        SELECT hr.gc_id, s.profile_id, p.user_type
        FROM hire_requests hr
        LEFT JOIN subcontractors s ON hr.subcontractor_id = s.id
        JOIN profiles p ON p.id = $2
        WHERE hr.id = $1 AND (hr.gc_id = $2 OR s.profile_id = $2)
        "#,
    )
    .bind(request_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let (gc_id, _, _user_type) = access
        .ok_or_else(|| ApiError::forbidden("You don't have access to this conversation"))?;

    let sender_type = if gc_id == user_id { "gc" } else { "sub" };
    let id = Uuid::new_v4();
    let message_type = input.message_type.unwrap_or_else(|| "text".to_string());
    let metadata = input.metadata.unwrap_or(serde_json::json!({}));

    sqlx::query(
        r#"
        INSERT INTO hire_messages (id, hire_request_id, sender_id, sender_type, message, message_type, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(id)
    .bind(request_id)
    .bind(user_id)
    .bind(sender_type)
    .bind(&input.message)
    .bind(&message_type)
    .bind(&metadata)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to send message: {}", e)))?;

    // Update hire request to negotiating if applicable
    if sender_type == "sub" {
        let _ = sqlx::query(
            "UPDATE hire_requests SET status = 'negotiating', updated_at = NOW() WHERE id = $1 AND status IN ('viewed', 'interested')"
        )
        .bind(request_id)
        .execute(&state.db)
        .await;
    }

    Ok(Json(serde_json::json!({ "id": id, "success": true })))
}

// ============================================================================
// Contract Templates
// ============================================================================

/// GET /api/contract-templates
pub async fn list_contract_templates(
    State(state): State<Arc<AppState>>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let rows = sqlx::query_as::<_, ContractTemplateRow>(
        r#"
        SELECT id, name, description, template_type, content, sections, variables,
               is_system, is_active, created_at, updated_at
        FROM contract_templates
        WHERE is_active = true
        ORDER BY is_system DESC, name
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let templates: Vec<ContractTemplateResponse> = rows
        .into_iter()
        .map(|r| {
            let sections: Vec<ContractSection> =
                serde_json::from_value(r.sections).unwrap_or_default();
            let variables: Vec<TemplateVariable> =
                serde_json::from_value(r.variables).unwrap_or_default();

            ContractTemplateResponse {
                id: r.id,
                name: r.name,
                description: r.description,
                template_type: r.template_type,
                content: r.content,
                sections,
                variables,
                is_system: r.is_system,
                is_active: r.is_active,
                created_at: r.created_at,
            }
        })
        .collect();

    Ok(Json(DataResponse::new(templates)))
}

// ============================================================================
// Contracts
// ============================================================================

/// POST /api/hiring/:hire_request_id/contract
pub async fn create_contract(
    State(state): State<Arc<AppState>>,
    Path(hire_request_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<CreateContractInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Verify GC owns the hire request
    let hire_request: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT gc_id, project_id FROM hire_requests WHERE id = $1",
    )
    .bind(hire_request_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let (gc_id, project_id) = hire_request
        .ok_or_else(|| ApiError::not_found("Hire request not found"))?;

    if gc_id != user_id {
        return Err(ApiError::forbidden("Only the GC can create contracts"));
    }

    let id = Uuid::new_v4();
    let contract_number = format!("CON-{}", &id.to_string()[..8].to_uppercase());
    
    // Get template content if specified
    let (content, sections) = if let Some(template_id) = input.template_id {
        let template: Option<(String, serde_json::Value)> = sqlx::query_as(
            "SELECT content, sections FROM contract_templates WHERE id = $1",
        )
        .bind(template_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

        template.unwrap_or_else(|| {
            (
                input.content.clone().unwrap_or_default(),
                serde_json::to_value(input.sections.clone().unwrap_or_default()).unwrap_or(serde_json::json!([])),
            )
        })
    } else {
        (
            input.content.unwrap_or_default(),
            serde_json::to_value(input.sections.unwrap_or_default()).unwrap_or(serde_json::json!([])),
        )
    };

    let payment_schedule = serde_json::to_value(input.payment_schedule.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));

    sqlx::query(
        r#"
        INSERT INTO contracts (
            id, hire_request_id, project_id, template_id, contract_number, title,
            content, sections, terms_summary, amount, payment_schedule,
            start_date, end_date, notes, status
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'draft')
        "#,
    )
    .bind(id)
    .bind(hire_request_id)
    .bind(project_id)
    .bind(input.template_id)
    .bind(&contract_number)
    .bind(&input.title)
    .bind(&content)
    .bind(&sections)
    .bind(&input.terms_summary)
    .bind(input.amount)
    .bind(&payment_schedule)
    .bind(input.start_date)
    .bind(input.end_date)
    .bind(&input.notes)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create contract: {}", e)))?;

    Ok(Json(serde_json::json!({ "id": id, "contract_number": contract_number, "success": true })))
}

/// GET /api/contracts/:id
pub async fn get_contract(
    State(state): State<Arc<AppState>>,
    Path(contract_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // TODO: Full contract query with joins
    let row = sqlx::query_as::<_, ContractRow>(
        r#"
        SELECT c.id, c.hire_request_id, c.project_id, p.name as project_name,
               c.template_id, ct.name as template_name, c.contract_number, c.title,
               c.content, c.sections, c.terms_summary, c.amount, c.payment_schedule,
               c.start_date, c.end_date, c.gc_signature, c.gc_signed_at,
               c.sub_signature, c.sub_signed_at, c.status, c.pdf_path, c.notes,
               c.created_at, c.updated_at
        FROM contracts c
        JOIN projects p ON c.project_id = p.id
        LEFT JOIN contract_templates ct ON c.template_id = ct.id
        JOIN hire_requests hr ON c.hire_request_id = hr.id
        LEFT JOIN subcontractors s ON hr.subcontractor_id = s.id
        WHERE c.id = $1 AND (hr.gc_id = $2 OR s.profile_id = $2)
        "#,
    )
    .bind(contract_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Contract not found"))?;

    let sections: Vec<ContractSection> =
        serde_json::from_value(row.sections).unwrap_or_default();
    let payment_schedule: Vec<PaymentMilestone> =
        serde_json::from_value(row.payment_schedule).unwrap_or_default();

    // Get subcontractor info
    let sub_info: Option<(Option<Uuid>, Option<Uuid>, String, Option<String>, Option<String>, Option<String>, String, Option<String>, Option<sqlx::types::Decimal>, bool)> = sqlx::query_as(
        r#"
        SELECT hr.subcontractor_id, hr.external_sub_id,
               COALESCE(s.name, es.company_name) as company_name,
               COALESCE(s.contact_email, es.contact_name) as contact_name,
               COALESCE(s.contact_email, es.contact_email) as contact_email,
               COALESCE(s.contact_phone, es.contact_phone) as contact_phone,
               COALESCE(s.trade, es.trade) as trade,
               COALESCE(s.location, es.location) as location,
               s.rating,
               COALESCE(s.verified, false) as verified
        FROM hire_requests hr
        LEFT JOIN subcontractors s ON hr.subcontractor_id = s.id
        LEFT JOIN external_subcontractors es ON hr.external_sub_id = es.id
        WHERE hr.id = $1
        "#,
    )
    .bind(row.hire_request_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let subcontractor = if let Some((sub_id, ext_id, company, contact, email, phone, trade, loc, rating, verified)) = sub_info {
        make_subcontractor_info(sub_id, ext_id, company, contact, email, phone, trade, loc, rating, verified)
    } else {
        HireRequestSubcontractor {
            id: Uuid::nil(),
            is_external: false,
            company_name: "Unknown".to_string(),
            contact_name: None,
            contact_email: None,
            contact_phone: None,
            trade: "Unknown".to_string(),
            location: None,
            rating: None,
            verified: false,
        }
    };

    let response = ContractResponse {
        id: row.id,
        hire_request_id: row.hire_request_id,
        project_id: row.project_id,
        project_name: row.project_name,
        template_id: row.template_id,
        template_name: row.template_name,
        contract_number: row.contract_number,
        title: row.title,
        content: row.content,
        sections,
        terms_summary: row.terms_summary,
        amount: decimal_to_f64(row.amount),
        payment_schedule,
        start_date: row.start_date,
        end_date: row.end_date,
        gc_signed: row.gc_signed_at.is_some(),
        gc_signed_at: row.gc_signed_at,
        sub_signed: row.sub_signed_at.is_some(),
        sub_signed_at: row.sub_signed_at,
        status: row.status,
        pdf_path: row.pdf_path,
        notes: row.notes,
        subcontractor,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(DataResponse::new(response)))
}

/// POST /api/contracts/:id/sign
pub async fn sign_contract(
    State(state): State<Arc<AppState>>,
    Path(contract_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<SignContractInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    if !input.agreed_to_terms {
        return Err(ApiError::bad_request("Must agree to terms to sign"));
    }

    // Get contract and determine if user is GC or sub
    let contract_info: Option<(Uuid, Option<Uuid>, String)> = sqlx::query_as(
        r#"
        SELECT hr.gc_id, s.profile_id, c.status
        FROM contracts c
        JOIN hire_requests hr ON c.hire_request_id = hr.id
        LEFT JOIN subcontractors s ON hr.subcontractor_id = s.id
        WHERE c.id = $1
        "#,
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let (gc_id, sub_profile_id, current_status) = contract_info
        .ok_or_else(|| ApiError::not_found("Contract not found"))?;

    let is_gc = gc_id == user_id;
    let is_sub = sub_profile_id == Some(user_id);

    if !is_gc && !is_sub {
        return Err(ApiError::forbidden("You cannot sign this contract"));
    }

    let (column, new_status) = if is_gc {
        if current_status != "draft" && current_status != "pending_gc" {
            return Err(ApiError::bad_request("Contract cannot be signed by GC at this stage"));
        }
        ("gc", "pending_sub")
    } else {
        if current_status != "pending_sub" && current_status != "gc_signed" {
            return Err(ApiError::bad_request("Contract cannot be signed by sub at this stage"));
        }
        ("sub", "fully_signed")
    };

    let query = format!(
        "UPDATE contracts SET {}_signature = $1, {}_signed_at = NOW(), status = $2, updated_at = NOW() WHERE id = $3",
        column, column
    );

    sqlx::query(&query)
        .bind(&input.signature)
        .bind(new_status)
        .bind(contract_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to sign contract: {}", e)))?;

    // If fully signed, update hire request status
    if new_status == "fully_signed" {
        sqlx::query(
            "UPDATE hire_requests SET status = 'contract_signed', updated_at = NOW() WHERE id = (SELECT hire_request_id FROM contracts WHERE id = $1)"
        )
        .bind(contract_id)
        .execute(&state.db)
        .await
        .ok();
    }

    Ok(Json(serde_json::json!({ "success": true, "status": new_status })))
}

// ============================================================================
// Project Team
// ============================================================================

/// GET /api/projects/:project_id/team
pub async fn list_team_members(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Verify project access
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't have access to this project"));
    }

    let rows = sqlx::query_as::<_, TeamMemberRow>(
        r#"
        SELECT pt.id, pt.project_id, pt.hire_request_id, pt.contract_id,
               pt.subcontractor_id, pt.external_sub_id,
               COALESCE(s.name, es.company_name) as sub_company_name,
               COALESCE(s.contact_email, es.contact_name) as sub_contact_name,
               COALESCE(s.contact_email, es.contact_email) as sub_contact_email,
               COALESCE(s.contact_phone, es.contact_phone) as sub_contact_phone,
               COALESCE(s.trade, es.trade) as sub_trade,
               COALESCE(s.location, es.location) as sub_location,
               s.rating as sub_rating,
               COALESCE(s.verified, false) as sub_verified,
               pt.role, pt.trade, pt.responsibilities, pt.start_date, pt.end_date,
               pt.hourly_rate, pt.status, pt.performance_rating, pt.notes,
               pt.joined_at, pt.created_at, pt.updated_at
        FROM project_team pt
        LEFT JOIN subcontractors s ON pt.subcontractor_id = s.id
        LEFT JOIN external_subcontractors es ON pt.external_sub_id = es.id
        WHERE pt.project_id = $1
        ORDER BY pt.joined_at DESC
        "#,
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<TeamMemberResponse> = rows
        .into_iter()
        .map(|r| {
            let subcontractor = make_subcontractor_info(
                r.subcontractor_id,
                r.external_sub_id,
                r.sub_company_name,
                r.sub_contact_name,
                r.sub_contact_email,
                r.sub_contact_phone,
                r.sub_trade,
                r.sub_location,
                r.sub_rating,
                r.sub_verified,
            );

            TeamMemberResponse {
                id: r.id,
                project_id: r.project_id,
                hire_request_id: r.hire_request_id,
                contract_id: r.contract_id,
                subcontractor,
                role: r.role,
                trade: r.trade,
                responsibilities: r.responsibilities,
                start_date: r.start_date,
                end_date: r.end_date,
                hourly_rate: decimal_opt_to_f64(r.hourly_rate),
                status: r.status,
                performance_rating: decimal_opt_to_f64(r.performance_rating),
                notes: r.notes,
                joined_at: r.joined_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }
        })
        .collect();

    Ok(Json(DataResponse::new(data)))
}

/// POST /api/projects/:project_id/team
pub async fn add_team_member(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<AddTeamMemberInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Verify project ownership
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't own this project"));
    }

    let id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO project_team (
            id, project_id, hire_request_id, contract_id, subcontractor_id, external_sub_id,
            role, trade, responsibilities, start_date, end_date, hourly_rate, notes
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
    )
    .bind(id)
    .bind(project_id)
    .bind(input.hire_request_id)
    .bind(input.contract_id)
    .bind(input.subcontractor_id)
    .bind(input.external_sub_id)
    .bind(&input.role)
    .bind(&input.trade)
    .bind(&input.responsibilities)
    .bind(input.start_date)
    .bind(input.end_date)
    .bind(input.hourly_rate)
    .bind(&input.notes)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to add team member: {}", e)))?;

    Ok(Json(serde_json::json!({ "id": id, "success": true })))
}

/// PUT /api/projects/:project_id/team/:member_id
pub async fn update_team_member(
    State(state): State<Arc<AppState>>,
    Path((project_id, member_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
    Json(input): Json<UpdateTeamMemberInput>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Verify project ownership
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't own this project"));
    }

    let result = sqlx::query(
        r#"
        UPDATE project_team SET
            role = COALESCE($1, role),
            responsibilities = COALESCE($2, responsibilities),
            start_date = COALESCE($3, start_date),
            end_date = COALESCE($4, end_date),
            hourly_rate = COALESCE($5, hourly_rate),
            status = COALESCE($6, status),
            performance_rating = COALESCE($7, performance_rating),
            notes = COALESCE($8, notes),
            updated_at = NOW()
        WHERE id = $9 AND project_id = $10
        "#,
    )
    .bind(&input.role)
    .bind(&input.responsibilities)
    .bind(input.start_date)
    .bind(input.end_date)
    .bind(input.hourly_rate)
    .bind(&input.status)
    .bind(input.performance_rating)
    .bind(&input.notes)
    .bind(member_id)
    .bind(project_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update team member: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Team member not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/projects/:project_id/team/:member_id
pub async fn remove_team_member(
    State(state): State<Arc<AppState>>,
    Path((project_id, member_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Verify project ownership
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't own this project"));
    }

    let result = sqlx::query("DELETE FROM project_team WHERE id = $1 AND project_id = $2")
        .bind(member_id)
        .bind(project_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to remove team member: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Team member not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
