//! Extraction routes
//!
//! Endpoints for managing AI-extracted data: materials, rooms, milestones, trade scopes.

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
use crate::domain::extraction::*;
use crate::error::ApiError;

// ============================================================================
// Database Row Types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct ExtractedMaterialRow {
    id: Uuid,
    project_id: Uuid,
    document_id: Option<Uuid>,
    name: String,
    description: Option<String>,
    quantity: Option<sqlx::types::Decimal>,
    unit: Option<String>,
    unit_cost: Option<sqlx::types::Decimal>,
    total_cost: Option<sqlx::types::Decimal>,
    location: Option<String>,
    room: Option<String>,
    specification: Option<String>,
    trade_category: Option<String>,
    csi_division: Option<String>,
    source_page: Option<i32>,
    confidence: sqlx::types::Decimal,
    is_verified: bool,
    verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct ExtractedRoomRow {
    id: Uuid,
    project_id: Uuid,
    document_id: Option<Uuid>,
    room_name: String,
    room_number: Option<String>,
    room_type: Option<String>,
    floor: Option<String>,
    area_sqft: Option<sqlx::types::Decimal>,
    ceiling_height: Option<sqlx::types::Decimal>,
    perimeter_ft: Option<sqlx::types::Decimal>,
    finishes: serde_json::Value,
    fixtures: serde_json::Value,
    notes: Option<String>,
    source_page: Option<i32>,
    confidence: sqlx::types::Decimal,
    is_verified: bool,
    verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct MilestoneRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: Option<String>,
    phase: Option<String>,
    phase_order: i32,
    estimated_duration_days: Option<i32>,
    estimated_start_date: Option<DateTime<Utc>>,
    estimated_end_date: Option<DateTime<Utc>>,
    actual_start_date: Option<DateTime<Utc>>,
    actual_end_date: Option<DateTime<Utc>>,
    dependencies: serde_json::Value,
    trades_involved: serde_json::Value,
    deliverables: serde_json::Value,
    status: String,
    progress: sqlx::types::Decimal,
    is_ai_generated: bool,
    is_verified: bool,
    verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct TradeScopeRow {
    id: Uuid,
    project_id: Uuid,
    document_id: Option<Uuid>,
    trade: String,
    trade_display_name: Option<String>,
    csi_division: Option<String>,
    inclusions: serde_json::Value,
    exclusions: serde_json::Value,
    required_sheets: serde_json::Value,
    spec_sections: serde_json::Value,
    rfi_needed: serde_json::Value,
    assumptions: serde_json::Value,
    estimated_value: Option<sqlx::types::Decimal>,
    confidence: sqlx::types::Decimal,
    is_verified: bool,
    verified_at: Option<DateTime<Utc>>,
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

async fn verify_project_access(
    state: &AppState,
    project_id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't have access to this project"));
    }
    Ok(())
}

// ============================================================================
// Extraction Summary
// ============================================================================

/// GET /api/projects/:project_id/extraction
///
/// Get extraction summary for a project.
pub async fn get_extraction_summary(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let materials_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM extracted_materials WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rooms_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM extracted_rooms WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let milestones_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM project_milestones WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let trade_scopes_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM extracted_trade_scopes WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let verified_materials: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM extracted_materials WHERE project_id = $1 AND is_verified = true",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let verified_rooms: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM extracted_rooms WHERE project_id = $1 AND is_verified = true",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let verified_milestones: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM project_milestones WHERE project_id = $1 AND is_verified = true",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let verified_trade_scopes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM extracted_trade_scopes WHERE project_id = $1 AND is_verified = true",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Get latest processing job
    let (processing_job_id, processing_status): (Option<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT id, status FROM processing_jobs WHERE project_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .unwrap_or((None, None));

    let last_extraction_at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT MAX(completed_at) FROM processing_jobs WHERE project_id = $1 AND status = 'completed'",
    )
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let summary = ExtractionSummary {
        project_id,
        materials_count,
        rooms_count,
        milestones_count,
        trade_scopes_count,
        verified_materials,
        verified_rooms,
        verified_milestones,
        verified_trade_scopes,
        last_extraction_at,
        processing_job_id,
        processing_status,
    };

    Ok(Json(DataResponse::new(summary)))
}

// ============================================================================
// Materials CRUD
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct MaterialQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: MaterialQuery,
}

/// GET /api/projects/:project_id/extraction/materials
pub async fn list_materials(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<MaterialQueryParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(50).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM extracted_materials 
        WHERE project_id = $1
        AND ($2::text IS NULL OR trade_category ILIKE '%' || $2 || '%')
        AND ($3::text IS NULL OR room ILIKE '%' || $3 || '%')
        AND ($4::bool IS NULL OR is_verified = $4)
        AND ($5::text IS NULL OR name ILIKE '%' || $5 || '%')
        "#,
    )
    .bind(project_id)
    .bind(&query.filter.trade_category)
    .bind(&query.filter.room)
    .bind(query.filter.is_verified)
    .bind(&query.filter.search)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rows = sqlx::query_as::<_, ExtractedMaterialRow>(
        r#"
        SELECT id, project_id, document_id, name, description, quantity, unit,
               unit_cost, total_cost, location, room, specification, trade_category,
               csi_division, source_page, confidence, is_verified, verified_at,
               created_at, updated_at
        FROM extracted_materials
        WHERE project_id = $1
        AND ($2::text IS NULL OR trade_category ILIKE '%' || $2 || '%')
        AND ($3::text IS NULL OR room ILIKE '%' || $3 || '%')
        AND ($4::bool IS NULL OR is_verified = $4)
        AND ($5::text IS NULL OR name ILIKE '%' || $5 || '%')
        ORDER BY trade_category, name
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(project_id)
    .bind(&query.filter.trade_category)
    .bind(&query.filter.room)
    .bind(query.filter.is_verified)
    .bind(&query.filter.search)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<ExtractedMaterialResponse> = rows
        .into_iter()
        .map(|r| ExtractedMaterialResponse {
            id: r.id,
            project_id: r.project_id,
            document_id: r.document_id,
            name: r.name,
            description: r.description,
            quantity: decimal_opt_to_f64(r.quantity),
            unit: r.unit,
            unit_cost: decimal_opt_to_f64(r.unit_cost),
            total_cost: decimal_opt_to_f64(r.total_cost),
            location: r.location,
            room: r.room,
            specification: r.specification,
            trade_category: r.trade_category,
            csi_division: r.csi_division,
            source_page: r.source_page,
            confidence: decimal_to_f64(r.confidence),
            is_verified: r.is_verified,
            verified_at: r.verified_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
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

/// POST /api/projects/:project_id/extraction/materials
pub async fn create_material(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<MaterialInput>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let id = Uuid::new_v4();
    let total_cost = input.quantity.zip(input.unit_cost).map(|(q, c)| q * c);

    sqlx::query(
        r#"
        INSERT INTO extracted_materials (
            id, project_id, name, description, quantity, unit, unit_cost, total_cost,
            location, room, specification, trade_category, csi_division, source_page,
            confidence, is_verified
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 1.0, false)
        "#,
    )
    .bind(id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.quantity)
    .bind(&input.unit)
    .bind(input.unit_cost)
    .bind(total_cost)
    .bind(&input.location)
    .bind(&input.room)
    .bind(&input.specification)
    .bind(&input.trade_category)
    .bind(&input.csi_division)
    .bind(input.source_page)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create material: {}", e)))?;

    let row = sqlx::query_as::<_, ExtractedMaterialRow>(
        "SELECT * FROM extracted_materials WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let response = ExtractedMaterialResponse {
        id: row.id,
        project_id: row.project_id,
        document_id: row.document_id,
        name: row.name,
        description: row.description,
        quantity: decimal_opt_to_f64(row.quantity),
        unit: row.unit,
        unit_cost: decimal_opt_to_f64(row.unit_cost),
        total_cost: decimal_opt_to_f64(row.total_cost),
        location: row.location,
        room: row.room,
        specification: row.specification,
        trade_category: row.trade_category,
        csi_division: row.csi_division,
        source_page: row.source_page,
        confidence: decimal_to_f64(row.confidence),
        is_verified: row.is_verified,
        verified_at: row.verified_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(DataResponse::new(response)))
}

/// PUT /api/projects/:project_id/extraction/materials/:material_id
pub async fn update_material(
    State(state): State<Arc<AppState>>,
    Path((project_id, material_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
    Json(input): Json<MaterialInput>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let total_cost = input.quantity.zip(input.unit_cost).map(|(q, c)| q * c);

    let result = sqlx::query(
        r#"
        UPDATE extracted_materials SET
            name = $1, description = $2, quantity = $3, unit = $4, unit_cost = $5,
            total_cost = $6, location = $7, room = $8, specification = $9,
            trade_category = $10, csi_division = $11, source_page = $12, updated_at = NOW()
        WHERE id = $13 AND project_id = $14
        "#,
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.quantity)
    .bind(&input.unit)
    .bind(input.unit_cost)
    .bind(total_cost)
    .bind(&input.location)
    .bind(&input.room)
    .bind(&input.specification)
    .bind(&input.trade_category)
    .bind(&input.csi_division)
    .bind(input.source_page)
    .bind(material_id)
    .bind(project_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update material: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Material not found"));
    }

    let row = sqlx::query_as::<_, ExtractedMaterialRow>(
        "SELECT * FROM extracted_materials WHERE id = $1",
    )
    .bind(material_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let response = ExtractedMaterialResponse {
        id: row.id,
        project_id: row.project_id,
        document_id: row.document_id,
        name: row.name,
        description: row.description,
        quantity: decimal_opt_to_f64(row.quantity),
        unit: row.unit,
        unit_cost: decimal_opt_to_f64(row.unit_cost),
        total_cost: decimal_opt_to_f64(row.total_cost),
        location: row.location,
        room: row.room,
        specification: row.specification,
        trade_category: row.trade_category,
        csi_division: row.csi_division,
        source_page: row.source_page,
        confidence: decimal_to_f64(row.confidence),
        is_verified: row.is_verified,
        verified_at: row.verified_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(DataResponse::new(response)))
}

/// DELETE /api/projects/:project_id/extraction/materials/:material_id
pub async fn delete_material(
    State(state): State<Arc<AppState>>,
    Path((project_id, material_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let result = sqlx::query("DELETE FROM extracted_materials WHERE id = $1 AND project_id = $2")
        .bind(material_id)
        .bind(project_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete material: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Material not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// POST /api/projects/:project_id/extraction/materials/:material_id/verify
pub async fn verify_material(
    State(state): State<Arc<AppState>>,
    Path((project_id, material_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
    Json(input): Json<VerifyItemRequest>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let result = sqlx::query(
        r#"
        UPDATE extracted_materials SET
            is_verified = $1,
            verified_by = CASE WHEN $1 THEN $2 ELSE NULL END,
            verified_at = CASE WHEN $1 THEN NOW() ELSE NULL END,
            updated_at = NOW()
        WHERE id = $3 AND project_id = $4
        "#,
    )
    .bind(input.is_verified)
    .bind(auth.user_id)
    .bind(material_id)
    .bind(project_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to verify material: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Material not found"));
    }

    Ok(Json(serde_json::json!({ "success": true, "is_verified": input.is_verified })))
}

// ============================================================================
// Rooms CRUD
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct RoomQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: RoomQuery,
}

/// GET /api/projects/:project_id/extraction/rooms
pub async fn list_rooms(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<RoomQueryParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(50).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM extracted_rooms 
        WHERE project_id = $1
        AND ($2::text IS NULL OR floor = $2)
        AND ($3::text IS NULL OR room_type = $3)
        AND ($4::bool IS NULL OR is_verified = $4)
        AND ($5::text IS NULL OR room_name ILIKE '%' || $5 || '%')
        "#,
    )
    .bind(project_id)
    .bind(&query.filter.floor)
    .bind(&query.filter.room_type)
    .bind(query.filter.is_verified)
    .bind(&query.filter.search)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rows = sqlx::query_as::<_, ExtractedRoomRow>(
        r#"
        SELECT id, project_id, document_id, room_name, room_number, room_type,
               floor, area_sqft, ceiling_height, perimeter_ft, finishes, fixtures,
               notes, source_page, confidence, is_verified, verified_at,
               created_at, updated_at
        FROM extracted_rooms
        WHERE project_id = $1
        AND ($2::text IS NULL OR floor = $2)
        AND ($3::text IS NULL OR room_type = $3)
        AND ($4::bool IS NULL OR is_verified = $4)
        AND ($5::text IS NULL OR room_name ILIKE '%' || $5 || '%')
        ORDER BY floor, room_number, room_name
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(project_id)
    .bind(&query.filter.floor)
    .bind(&query.filter.room_type)
    .bind(query.filter.is_verified)
    .bind(&query.filter.search)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<ExtractedRoomResponse> = rows
        .into_iter()
        .map(|r| {
            let finishes: RoomFinishes =
                serde_json::from_value(r.finishes).unwrap_or_default();
            let fixtures: Vec<String> =
                serde_json::from_value(r.fixtures).unwrap_or_default();

            ExtractedRoomResponse {
                id: r.id,
                project_id: r.project_id,
                document_id: r.document_id,
                room_name: r.room_name,
                room_number: r.room_number,
                room_type: r.room_type,
                floor: r.floor,
                area_sqft: decimal_opt_to_f64(r.area_sqft),
                ceiling_height: decimal_opt_to_f64(r.ceiling_height),
                perimeter_ft: decimal_opt_to_f64(r.perimeter_ft),
                finishes,
                fixtures,
                notes: r.notes,
                source_page: r.source_page,
                confidence: decimal_to_f64(r.confidence),
                is_verified: r.is_verified,
                verified_at: r.verified_at,
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

/// POST /api/projects/:project_id/extraction/rooms
pub async fn create_room(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<RoomInput>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let id = Uuid::new_v4();
    let finishes = serde_json::to_value(input.finishes.unwrap_or_default())
        .unwrap_or(serde_json::json!({}));
    let fixtures = serde_json::to_value(input.fixtures.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));

    sqlx::query(
        r#"
        INSERT INTO extracted_rooms (
            id, project_id, room_name, room_number, room_type, floor,
            area_sqft, ceiling_height, perimeter_ft, finishes, fixtures,
            notes, source_page, confidence, is_verified
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 1.0, false)
        "#,
    )
    .bind(id)
    .bind(project_id)
    .bind(&input.room_name)
    .bind(&input.room_number)
    .bind(&input.room_type)
    .bind(&input.floor)
    .bind(input.area_sqft)
    .bind(input.ceiling_height)
    .bind(input.perimeter_ft)
    .bind(&finishes)
    .bind(&fixtures)
    .bind(&input.notes)
    .bind(input.source_page)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create room: {}", e)))?;

    Ok(Json(serde_json::json!({ "id": id, "success": true })))
}

/// PUT /api/projects/:project_id/extraction/rooms/:room_id
pub async fn update_room(
    State(state): State<Arc<AppState>>,
    Path((project_id, room_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
    Json(input): Json<RoomInput>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let finishes = input.finishes.map(|f| serde_json::to_value(f).unwrap_or(serde_json::json!({})));
    let fixtures = input.fixtures.map(|f| serde_json::to_value(f).unwrap_or(serde_json::json!([])));

    let result = sqlx::query(
        r#"
        UPDATE extracted_rooms SET
            room_name = $1, room_number = $2, room_type = $3, floor = $4,
            area_sqft = $5, ceiling_height = $6, perimeter_ft = $7,
            finishes = COALESCE($8, finishes), fixtures = COALESCE($9, fixtures),
            notes = $10, source_page = $11, updated_at = NOW()
        WHERE id = $12 AND project_id = $13
        "#,
    )
    .bind(&input.room_name)
    .bind(&input.room_number)
    .bind(&input.room_type)
    .bind(&input.floor)
    .bind(input.area_sqft)
    .bind(input.ceiling_height)
    .bind(input.perimeter_ft)
    .bind(finishes)
    .bind(fixtures)
    .bind(&input.notes)
    .bind(input.source_page)
    .bind(room_id)
    .bind(project_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update room: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Room not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/projects/:project_id/extraction/rooms/:room_id
pub async fn delete_room(
    State(state): State<Arc<AppState>>,
    Path((project_id, room_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let result = sqlx::query("DELETE FROM extracted_rooms WHERE id = $1 AND project_id = $2")
        .bind(room_id)
        .bind(project_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete room: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Room not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================================
// Milestones CRUD
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct MilestoneQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: MilestoneQuery,
}

/// GET /api/projects/:project_id/extraction/milestones
pub async fn list_milestones(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<MilestoneQueryParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(50).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM project_milestones 
        WHERE project_id = $1
        AND ($2::text IS NULL OR phase = $2)
        AND ($3::text IS NULL OR status = $3)
        AND ($4::bool IS NULL OR is_verified = $4)
        "#,
    )
    .bind(project_id)
    .bind(&query.filter.phase)
    .bind(&query.filter.status)
    .bind(query.filter.is_verified)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rows = sqlx::query_as::<_, MilestoneRow>(
        r#"
        SELECT id, project_id, name, description, phase, phase_order,
               estimated_duration_days, estimated_start_date, estimated_end_date,
               actual_start_date, actual_end_date, dependencies, trades_involved,
               deliverables, status, progress, is_ai_generated, is_verified,
               verified_at, created_at, updated_at
        FROM project_milestones
        WHERE project_id = $1
        AND ($2::text IS NULL OR phase = $2)
        AND ($3::text IS NULL OR status = $3)
        AND ($4::bool IS NULL OR is_verified = $4)
        ORDER BY phase_order, estimated_start_date
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(project_id)
    .bind(&query.filter.phase)
    .bind(&query.filter.status)
    .bind(query.filter.is_verified)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<MilestoneResponse> = rows
        .into_iter()
        .map(|r| {
            let dependencies: Vec<String> =
                serde_json::from_value(r.dependencies).unwrap_or_default();
            let trades_involved: Vec<String> =
                serde_json::from_value(r.trades_involved).unwrap_or_default();
            let deliverables: Vec<String> =
                serde_json::from_value(r.deliverables).unwrap_or_default();

            MilestoneResponse {
                id: r.id,
                project_id: r.project_id,
                name: r.name,
                description: r.description,
                phase: r.phase,
                phase_order: r.phase_order,
                estimated_duration_days: r.estimated_duration_days,
                estimated_start_date: r.estimated_start_date,
                estimated_end_date: r.estimated_end_date,
                actual_start_date: r.actual_start_date,
                actual_end_date: r.actual_end_date,
                dependencies,
                trades_involved,
                deliverables,
                status: r.status,
                progress: decimal_to_f64(r.progress),
                is_ai_generated: r.is_ai_generated,
                is_verified: r.is_verified,
                verified_at: r.verified_at,
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

/// POST /api/projects/:project_id/extraction/milestones
pub async fn create_milestone(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<MilestoneInput>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let id = Uuid::new_v4();
    let dependencies = serde_json::to_value(input.dependencies.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));
    let trades_involved = serde_json::to_value(input.trades_involved.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));
    let deliverables = serde_json::to_value(input.deliverables.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));

    sqlx::query(
        r#"
        INSERT INTO project_milestones (
            id, project_id, name, description, phase, phase_order,
            estimated_duration_days, estimated_start_date, estimated_end_date,
            dependencies, trades_involved, deliverables, status, progress,
            is_ai_generated, is_verified
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, false, false)
        "#,
    )
    .bind(id)
    .bind(project_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.phase)
    .bind(input.phase_order.unwrap_or(0))
    .bind(input.estimated_duration_days)
    .bind(input.estimated_start_date)
    .bind(input.estimated_end_date)
    .bind(&dependencies)
    .bind(&trades_involved)
    .bind(&deliverables)
    .bind(input.status.as_deref().unwrap_or("pending"))
    .bind(input.progress.unwrap_or(0.0))
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create milestone: {}", e)))?;

    Ok(Json(serde_json::json!({ "id": id, "success": true })))
}

/// PUT /api/projects/:project_id/extraction/milestones/:milestone_id
pub async fn update_milestone(
    State(state): State<Arc<AppState>>,
    Path((project_id, milestone_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
    Json(input): Json<MilestoneInput>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let dependencies = input.dependencies.map(|d| serde_json::to_value(d).unwrap_or(serde_json::json!([])));
    let trades_involved = input.trades_involved.map(|t| serde_json::to_value(t).unwrap_or(serde_json::json!([])));
    let deliverables = input.deliverables.map(|d| serde_json::to_value(d).unwrap_or(serde_json::json!([])));

    let result = sqlx::query(
        r#"
        UPDATE project_milestones SET
            name = $1, description = $2, phase = COALESCE($3, phase),
            phase_order = COALESCE($4, phase_order),
            estimated_duration_days = COALESCE($5, estimated_duration_days),
            estimated_start_date = COALESCE($6, estimated_start_date),
            estimated_end_date = COALESCE($7, estimated_end_date),
            dependencies = COALESCE($8, dependencies),
            trades_involved = COALESCE($9, trades_involved),
            deliverables = COALESCE($10, deliverables),
            status = COALESCE($11, status),
            progress = COALESCE($12, progress),
            updated_at = NOW()
        WHERE id = $13 AND project_id = $14
        "#,
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.phase)
    .bind(input.phase_order)
    .bind(input.estimated_duration_days)
    .bind(input.estimated_start_date)
    .bind(input.estimated_end_date)
    .bind(dependencies)
    .bind(trades_involved)
    .bind(deliverables)
    .bind(&input.status)
    .bind(input.progress)
    .bind(milestone_id)
    .bind(project_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update milestone: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Milestone not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/projects/:project_id/extraction/milestones/:milestone_id
pub async fn delete_milestone(
    State(state): State<Arc<AppState>>,
    Path((project_id, milestone_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let result = sqlx::query("DELETE FROM project_milestones WHERE id = $1 AND project_id = $2")
        .bind(milestone_id)
        .bind(project_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete milestone: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Milestone not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================================
// Trade Scopes CRUD
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct TradeScopeQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: TradeScopeQuery,
}

/// GET /api/projects/:project_id/extraction/trade-scopes
pub async fn list_trade_scopes(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<TradeScopeQueryParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(50).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM extracted_trade_scopes 
        WHERE project_id = $1
        AND ($2::text IS NULL OR trade ILIKE '%' || $2 || '%')
        AND ($3::bool IS NULL OR is_verified = $3)
        "#,
    )
    .bind(project_id)
    .bind(&query.filter.trade)
    .bind(query.filter.is_verified)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rows = sqlx::query_as::<_, TradeScopeRow>(
        r#"
        SELECT id, project_id, document_id, trade, trade_display_name, csi_division,
               inclusions, exclusions, required_sheets, spec_sections, rfi_needed,
               assumptions, estimated_value, confidence, is_verified, verified_at,
               created_at, updated_at
        FROM extracted_trade_scopes
        WHERE project_id = $1
        AND ($2::text IS NULL OR trade ILIKE '%' || $2 || '%')
        AND ($3::bool IS NULL OR is_verified = $3)
        ORDER BY csi_division, trade
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(project_id)
    .bind(&query.filter.trade)
    .bind(query.filter.is_verified)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<TradeScopeResponse> = rows
        .into_iter()
        .map(|r| {
            let inclusions: Vec<ScopeItem> =
                serde_json::from_value(r.inclusions).unwrap_or_default();
            let exclusions: Vec<ScopeItem> =
                serde_json::from_value(r.exclusions).unwrap_or_default();
            let required_sheets: Vec<String> =
                serde_json::from_value(r.required_sheets).unwrap_or_default();
            let spec_sections: Vec<String> =
                serde_json::from_value(r.spec_sections).unwrap_or_default();
            let rfi_needed: Vec<String> =
                serde_json::from_value(r.rfi_needed).unwrap_or_default();
            let assumptions: Vec<String> =
                serde_json::from_value(r.assumptions).unwrap_or_default();

            TradeScopeResponse {
                id: r.id,
                project_id: r.project_id,
                document_id: r.document_id,
                trade: r.trade,
                trade_display_name: r.trade_display_name,
                csi_division: r.csi_division,
                inclusions,
                exclusions,
                required_sheets,
                spec_sections,
                rfi_needed,
                assumptions,
                estimated_value: decimal_opt_to_f64(r.estimated_value),
                confidence: decimal_to_f64(r.confidence),
                is_verified: r.is_verified,
                verified_at: r.verified_at,
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

/// POST /api/projects/:project_id/extraction/trade-scopes
pub async fn create_trade_scope(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<TradeScopeInput>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let id = Uuid::new_v4();
    let inclusions = serde_json::to_value(input.inclusions.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));
    let exclusions = serde_json::to_value(input.exclusions.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));
    let required_sheets = serde_json::to_value(input.required_sheets.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));
    let spec_sections = serde_json::to_value(input.spec_sections.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));
    let rfi_needed = serde_json::to_value(input.rfi_needed.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));
    let assumptions = serde_json::to_value(input.assumptions.unwrap_or_default())
        .unwrap_or(serde_json::json!([]));

    sqlx::query(
        r#"
        INSERT INTO extracted_trade_scopes (
            id, project_id, trade, trade_display_name, csi_division,
            inclusions, exclusions, required_sheets, spec_sections,
            rfi_needed, assumptions, estimated_value, confidence, is_verified
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 1.0, false)
        "#,
    )
    .bind(id)
    .bind(project_id)
    .bind(&input.trade)
    .bind(&input.trade_display_name)
    .bind(&input.csi_division)
    .bind(&inclusions)
    .bind(&exclusions)
    .bind(&required_sheets)
    .bind(&spec_sections)
    .bind(&rfi_needed)
    .bind(&assumptions)
    .bind(input.estimated_value)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create trade scope: {}", e)))?;

    Ok(Json(serde_json::json!({ "id": id, "success": true })))
}

/// PUT /api/projects/:project_id/extraction/trade-scopes/:scope_id
pub async fn update_trade_scope(
    State(state): State<Arc<AppState>>,
    Path((project_id, scope_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
    Json(input): Json<TradeScopeInput>,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let inclusions = input.inclusions.map(|i| serde_json::to_value(i).unwrap_or(serde_json::json!([])));
    let exclusions = input.exclusions.map(|e| serde_json::to_value(e).unwrap_or(serde_json::json!([])));
    let required_sheets = input.required_sheets.map(|r| serde_json::to_value(r).unwrap_or(serde_json::json!([])));
    let spec_sections = input.spec_sections.map(|s| serde_json::to_value(s).unwrap_or(serde_json::json!([])));
    let rfi_needed = input.rfi_needed.map(|r| serde_json::to_value(r).unwrap_or(serde_json::json!([])));
    let assumptions = input.assumptions.map(|a| serde_json::to_value(a).unwrap_or(serde_json::json!([])));

    let result = sqlx::query(
        r#"
        UPDATE extracted_trade_scopes SET
            trade = $1, trade_display_name = COALESCE($2, trade_display_name),
            csi_division = COALESCE($3, csi_division),
            inclusions = COALESCE($4, inclusions),
            exclusions = COALESCE($5, exclusions),
            required_sheets = COALESCE($6, required_sheets),
            spec_sections = COALESCE($7, spec_sections),
            rfi_needed = COALESCE($8, rfi_needed),
            assumptions = COALESCE($9, assumptions),
            estimated_value = COALESCE($10, estimated_value),
            updated_at = NOW()
        WHERE id = $11 AND project_id = $12
        "#,
    )
    .bind(&input.trade)
    .bind(&input.trade_display_name)
    .bind(&input.csi_division)
    .bind(inclusions)
    .bind(exclusions)
    .bind(required_sheets)
    .bind(spec_sections)
    .bind(rfi_needed)
    .bind(assumptions)
    .bind(input.estimated_value)
    .bind(scope_id)
    .bind(project_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update trade scope: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Trade scope not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/projects/:project_id/extraction/trade-scopes/:scope_id
pub async fn delete_trade_scope(
    State(state): State<Arc<AppState>>,
    Path((project_id, scope_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    verify_project_access(&state, project_id, auth.user_id).await?;

    let result = sqlx::query("DELETE FROM extracted_trade_scopes WHERE id = $1 AND project_id = $2")
        .bind(scope_id)
        .bind(project_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete trade scope: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Trade scope not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
