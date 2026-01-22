//! Marketplace routes
//!
//! Enhanced marketplace endpoints for:
//! - Subcontractor directory with advanced search
//! - Subcontractor profile management (for subs)
//! - Portfolio management
//! - Saved searches
//! - Marketplace tenders (for subs to browse and bid)
//! - Bid management

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::pagination::PaginationParams;
use crate::api::response::{DataResponse, Paginated, PaginationMeta};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::marketplace::*;
use crate::error::ApiError;
use crate::services::notifications;

// ============================================================================
// Database Row Types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct MarketplaceSubRow {
    id: Uuid,
    profile_id: Option<Uuid>,
    name: String,
    trade: String,
    secondary_trades: serde_json::Value,
    headline: Option<String>,
    company_description: Option<String>,
    rating: f64,
    review_count: i32,
    location: Option<String>,
    contact_email: Option<String>,
    contact_phone: Option<String>,
    website: Option<String>,
    projects_completed: i32,
    average_bid_value: Option<i64>,
    response_time: Option<String>,
    response_time_hours: Option<i32>,
    verified: bool,
    verification_status: String,
    specialties: serde_json::Value,
    service_areas: serde_json::Value,
    certifications: serde_json::Value,
    insurance: serde_json::Value,
    license_info: serde_json::Value,
    year_established: Option<i32>,
    employee_count: Option<String>,
    min_project_value: Option<i64>,
    max_project_value: Option<i64>,
    availability_status: String,
    recent_projects: serde_json::Value,
    portfolio_count: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct TenderRow {
    id: Uuid,
    project_id: Uuid,
    project_name: Option<String>,
    gc_company_name: Option<String>,
    name: String,
    description: Option<String>,
    trade_category: String,
    scope_of_work: Option<String>,
    location: Option<String>,
    status: String,
    visibility: String,
    bid_due_date: Option<DateTime<Utc>>,
    estimated_value: Option<i64>,
    reserve_price: Option<i64>,
    requirements: serde_json::Value,
    bids_received: i64,
    priority: Option<String>,
    created_at: DateTime<Utc>,
    // User's bid info (from LEFT JOIN) - for N+1 optimization
    my_bid_id: Option<Uuid>,
    my_bid_amount: Option<sqlx::types::Decimal>,
    my_bid_status: Option<String>,
    my_bid_submitted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct BidRow {
    id: Uuid,
    tender_id: Uuid,
    tender_name: Option<String>,
    project_name: Option<String>,
    subcontractor_id: Option<Uuid>,
    company_name: String,
    bid_amount: sqlx::types::Decimal,
    breakdown: serde_json::Value,
    proposed_timeline_days: Option<i32>,
    proposed_start_date: Option<NaiveDate>,
    cover_letter: Option<String>,
    status: String,
    is_winning_bid: bool,
    notes: Option<String>,
    submitted_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct MarketplaceSubQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: MarketplaceSubcontractorQuery,
}

#[derive(Debug, Deserialize, Default)]
pub struct MarketplaceTenderQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: MarketplaceTenderQuery,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn decimal_to_i64(d: sqlx::types::Decimal) -> i64 {
    use std::str::FromStr;
    // Decimal in cents
    i64::from_str(&d.to_string().replace(".", "")).unwrap_or(0)
}

// ============================================================================
// Subcontractor Directory (Enhanced)
// ============================================================================

/// GET /api/marketplace/subcontractors
///
/// Enhanced subcontractor search with advanced filtering.
pub async fn list_marketplace_subcontractors(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MarketplaceSubQueryParams>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let verified_only = query.filter.verified_only.unwrap_or(false);
    let min_rating = query.filter.min_rating.unwrap_or(0.0);
    let has_insurance = query.filter.has_insurance.unwrap_or(false);

    // Count total
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM subcontractors s
        WHERE ($1::bool = false OR s.verified = true)
        AND s.rating >= $2
        AND ($3::text IS NULL OR s.trade ILIKE '%' || $3 || '%' OR 
             EXISTS (SELECT 1 FROM jsonb_array_elements_text(s.secondary_trades) t WHERE t ILIKE '%' || $3 || '%'))
        AND ($4::text IS NULL OR s.location ILIKE '%' || $4 || '%')
        AND ($5::text IS NULL OR s.name ILIKE '%' || $5 || '%' OR s.headline ILIKE '%' || $5 || '%')
        AND ($6::text IS NULL OR s.availability_status = $6)
        AND ($7::bigint IS NULL OR s.max_project_value >= $7)
        AND ($8::bigint IS NULL OR s.min_project_value <= $8)
        AND ($9::bool = false OR s.insurance IS NOT NULL AND s.insurance != '{}'::jsonb)
        "#,
    )
    .bind(verified_only)
    .bind(min_rating)
    .bind(&query.filter.trade)
    .bind(&query.filter.location)
    .bind(&query.filter.search)
    .bind(&query.filter.availability)
    .bind(query.filter.min_project_value)
    .bind(query.filter.max_project_value)
    .bind(has_insurance)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Determine sort
    let order_by = match query.filter.sort_by.as_deref() {
        Some("rating") => "s.rating",
        Some("reviews") => "s.review_count",
        Some("response_time") => "s.response_time_hours",
        Some("newest") => "s.created_at",
        _ => "s.rating",
    };
    let order_dir = match query.filter.sort_order.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let query_str = format!(
        r#"
        SELECT 
            s.id, s.profile_id, s.name, s.trade,
            COALESCE(to_jsonb(s.secondary_trades), '[]'::jsonb) as secondary_trades,
            s.headline, s.company_description, s.rating, s.review_count,
            s.location, s.contact_email, s.contact_phone, s.website,
            s.projects_completed, s.average_bid_value, s.response_time, s.response_time_hours,
            s.verified, COALESCE(s.verification_status, 'pending') as verification_status,
            COALESCE(to_jsonb(s.specialties), '[]'::jsonb) as specialties,
            COALESCE(s.service_areas, '[]'::jsonb) as service_areas,
            COALESCE(s.certifications, '[]'::jsonb) as certifications,
            COALESCE(s.insurance, '{{}}'::jsonb) as insurance,
            COALESCE(s.license_info, '{{}}'::jsonb) as license_info,
            s.year_established, s.employee_count,
            s.min_project_value, s.max_project_value,
            COALESCE(s.availability_status, 'available') as availability_status,
            COALESCE(s.recent_projects, '[]'::jsonb) as recent_projects,
            (SELECT COUNT(*) FROM portfolio_projects pp WHERE pp.subcontractor_id = s.id) as portfolio_count,
            s.created_at
        FROM subcontractors s
        WHERE ($1::bool = false OR s.verified = true)
        AND s.rating >= $2
        AND ($3::text IS NULL OR s.trade ILIKE '%' || $3 || '%' OR 
             EXISTS (SELECT 1 FROM jsonb_array_elements_text(s.secondary_trades) t WHERE t ILIKE '%' || $3 || '%'))
        AND ($4::text IS NULL OR s.location ILIKE '%' || $4 || '%')
        AND ($5::text IS NULL OR s.name ILIKE '%' || $5 || '%' OR s.headline ILIKE '%' || $5 || '%')
        AND ($6::text IS NULL OR s.availability_status = $6)
        AND ($7::bigint IS NULL OR s.max_project_value >= $7)
        AND ($8::bigint IS NULL OR s.min_project_value <= $8)
        AND ($9::bool = false OR s.insurance IS NOT NULL AND s.insurance != '{{}}'::jsonb)
        ORDER BY {} {} NULLS LAST
        LIMIT $10 OFFSET $11
        "#,
        order_by, order_dir
    );

    let rows = sqlx::query_as::<_, MarketplaceSubRow>(&query_str)
        .bind(verified_only)
        .bind(min_rating)
        .bind(&query.filter.trade)
        .bind(&query.filter.location)
        .bind(&query.filter.search)
        .bind(&query.filter.availability)
        .bind(query.filter.min_project_value)
        .bind(query.filter.max_project_value)
        .bind(has_insurance)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<SubcontractorProfile> = rows
        .into_iter()
        .map(|r| SubcontractorProfile {
            id: r.id,
            profile_id: r.profile_id,
            name: r.name,
            trade: r.trade,
            secondary_trades: serde_json::from_value(r.secondary_trades).unwrap_or_default(),
            headline: r.headline,
            company_description: r.company_description,
            rating: r.rating,
            review_count: r.review_count,
            location: r.location,
            contact_email: r.contact_email,
            contact_phone: r.contact_phone,
            website: r.website,
            projects_completed: r.projects_completed,
            average_bid_value: r.average_bid_value,
            response_time: r.response_time,
            response_time_hours: r.response_time_hours,
            verified: r.verified,
            verification_status: r.verification_status,
            specialties: serde_json::from_value(r.specialties).unwrap_or_default(),
            service_areas: serde_json::from_value(r.service_areas).unwrap_or_default(),
            certifications: serde_json::from_value(r.certifications).unwrap_or_default(),
            insurance: serde_json::from_value(r.insurance).ok(),
            license_info: serde_json::from_value(r.license_info).ok(),
            year_established: r.year_established,
            employee_count: r.employee_count,
            min_project_value: r.min_project_value,
            max_project_value: r.max_project_value,
            availability_status: r.availability_status,
            recent_projects: serde_json::from_value(r.recent_projects).unwrap_or_default(),
            portfolio_count: r.portfolio_count as i32,
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

/// GET /api/marketplace/subcontractors/:id
///
/// Get full subcontractor profile.
pub async fn get_marketplace_subcontractor(
    State(state): State<Arc<AppState>>,
    Path(sub_id): Path<Uuid>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let row = sqlx::query_as::<_, MarketplaceSubRow>(
        r#"
        SELECT 
            s.id, s.profile_id, s.name, s.trade,
            COALESCE(to_jsonb(s.secondary_trades), '[]'::jsonb) as secondary_trades,
            s.headline, s.company_description, s.rating, s.review_count,
            s.location, s.contact_email, s.contact_phone, s.website,
            s.projects_completed, s.average_bid_value, s.response_time, s.response_time_hours,
            s.verified, COALESCE(s.verification_status, 'pending') as verification_status,
            COALESCE(to_jsonb(s.specialties), '[]'::jsonb) as specialties,
            COALESCE(s.service_areas, '[]'::jsonb) as service_areas,
            COALESCE(s.certifications, '[]'::jsonb) as certifications,
            COALESCE(s.insurance, '{}'::jsonb) as insurance,
            COALESCE(s.license_info, '{}'::jsonb) as license_info,
            s.year_established, s.employee_count,
            s.min_project_value, s.max_project_value,
            COALESCE(s.availability_status, 'available') as availability_status,
            COALESCE(s.recent_projects, '[]'::jsonb) as recent_projects,
            (SELECT COUNT(*) FROM portfolio_projects pp WHERE pp.subcontractor_id = s.id) as portfolio_count,
            s.created_at
        FROM subcontractors s
        WHERE s.id = $1
        "#,
    )
    .bind(sub_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Subcontractor not found"))?;

    let profile = SubcontractorProfile {
        id: row.id,
        profile_id: row.profile_id,
        name: row.name,
        trade: row.trade,
        secondary_trades: serde_json::from_value(row.secondary_trades).unwrap_or_default(),
        headline: row.headline,
        company_description: row.company_description,
        rating: row.rating,
        review_count: row.review_count,
        location: row.location,
        contact_email: row.contact_email,
        contact_phone: row.contact_phone,
        website: row.website,
        projects_completed: row.projects_completed,
        average_bid_value: row.average_bid_value,
        response_time: row.response_time,
        response_time_hours: row.response_time_hours,
        verified: row.verified,
        verification_status: row.verification_status,
        specialties: serde_json::from_value(row.specialties).unwrap_or_default(),
        service_areas: serde_json::from_value(row.service_areas).unwrap_or_default(),
        certifications: serde_json::from_value(row.certifications).unwrap_or_default(),
        insurance: serde_json::from_value(row.insurance).ok(),
        license_info: serde_json::from_value(row.license_info).ok(),
        year_established: row.year_established,
        employee_count: row.employee_count,
        min_project_value: row.min_project_value,
        max_project_value: row.max_project_value,
        availability_status: row.availability_status,
        recent_projects: serde_json::from_value(row.recent_projects).unwrap_or_default(),
        portfolio_count: row.portfolio_count as i32,
        created_at: row.created_at,
    };

    Ok(Json(DataResponse::new(profile)))
}

/// GET /api/marketplace/subcontractors/:id/portfolio
///
/// Get subcontractor's portfolio projects.
pub async fn get_subcontractor_portfolio(
    State(state): State<Arc<AppState>>,
    Path(sub_id): Path<Uuid>,
    _auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let rows = sqlx::query_as::<_, PortfolioProject>(
        r#"
        SELECT id, subcontractor_id, title, description, project_type, trade_category,
               location, completion_date, project_value, client_name, client_testimonial,
               images, is_featured, display_order, created_at
        FROM portfolio_projects
        WHERE subcontractor_id = $1
        ORDER BY is_featured DESC, display_order ASC, created_at DESC
        "#,
    )
    .bind(sub_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<PortfolioProjectResponse> = rows.into_iter().map(Into::into).collect();

    Ok(Json(DataResponse::new(data)))
}

// ============================================================================
// My Profile (for Subcontractors)
// ============================================================================

/// GET /api/marketplace/profile
///
/// Get the current user's subcontractor profile.
pub async fn get_my_marketplace_profile(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let row = sqlx::query_as::<_, MarketplaceSubRow>(
        r#"
        SELECT 
            s.id, s.profile_id, s.name, s.trade,
            COALESCE(to_jsonb(s.secondary_trades), '[]'::jsonb) as secondary_trades,
            s.headline, s.company_description, s.rating, s.review_count,
            s.location, s.contact_email, s.contact_phone, s.website,
            s.projects_completed, s.average_bid_value, s.response_time, s.response_time_hours,
            s.verified, COALESCE(s.verification_status, 'pending') as verification_status,
            COALESCE(to_jsonb(s.specialties), '[]'::jsonb) as specialties,
            COALESCE(s.service_areas, '[]'::jsonb) as service_areas,
            COALESCE(s.certifications, '[]'::jsonb) as certifications,
            COALESCE(s.insurance, '{}'::jsonb) as insurance,
            COALESCE(s.license_info, '{}'::jsonb) as license_info,
            s.year_established, s.employee_count,
            s.min_project_value, s.max_project_value,
            COALESCE(s.availability_status, 'available') as availability_status,
            COALESCE(s.recent_projects, '[]'::jsonb) as recent_projects,
            (SELECT COUNT(*) FROM portfolio_projects pp WHERE pp.subcontractor_id = s.id) as portfolio_count,
            s.created_at
        FROM subcontractors s
        WHERE s.profile_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("No subcontractor profile found. Create one first."))?;

    let profile = SubcontractorProfile {
        id: row.id,
        profile_id: row.profile_id,
        name: row.name,
        trade: row.trade,
        secondary_trades: serde_json::from_value(row.secondary_trades).unwrap_or_default(),
        headline: row.headline,
        company_description: row.company_description,
        rating: row.rating,
        review_count: row.review_count,
        location: row.location,
        contact_email: row.contact_email,
        contact_phone: row.contact_phone,
        website: row.website,
        projects_completed: row.projects_completed,
        average_bid_value: row.average_bid_value,
        response_time: row.response_time,
        response_time_hours: row.response_time_hours,
        verified: row.verified,
        verification_status: row.verification_status,
        specialties: serde_json::from_value(row.specialties).unwrap_or_default(),
        service_areas: serde_json::from_value(row.service_areas).unwrap_or_default(),
        certifications: serde_json::from_value(row.certifications).unwrap_or_default(),
        insurance: serde_json::from_value(row.insurance).ok(),
        license_info: serde_json::from_value(row.license_info).ok(),
        year_established: row.year_established,
        employee_count: row.employee_count,
        min_project_value: row.min_project_value,
        max_project_value: row.max_project_value,
        availability_status: row.availability_status,
        recent_projects: serde_json::from_value(row.recent_projects).unwrap_or_default(),
        portfolio_count: row.portfolio_count as i32,
        created_at: row.created_at,
    };

    Ok(Json(DataResponse::new(profile)))
}

/// PUT /api/marketplace/profile
///
/// Update the current user's subcontractor profile.
pub async fn update_my_marketplace_profile(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
    Json(input): Json<UpdateMarketplaceProfileRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Check if profile exists
    let sub_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM subcontractors WHERE profile_id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let sub_id = sub_id.ok_or_else(|| {
        ApiError::not_found("No subcontractor profile found. Create one first.")
    })?;

    // Build update
    let secondary_trades = input.secondary_trades.map(|t| serde_json::to_value(t).unwrap_or_default());
    let specialties = input.specialties.map(|t| serde_json::to_value(t).unwrap_or_default());
    let service_areas = input.service_areas.map(|t| serde_json::to_value(t).unwrap_or_default());
    let certifications = input.certifications.map(|t| serde_json::to_value(t).unwrap_or_default());
    let insurance = input.insurance.map(|t| serde_json::to_value(t).unwrap_or_default());
    let license_info = input.license_info.map(|t| serde_json::to_value(t).unwrap_or_default());

    sqlx::query(
        r#"
        UPDATE subcontractors SET
            name = COALESCE($1, name),
            headline = COALESCE($2, headline),
            company_description = COALESCE($3, company_description),
            trade = COALESCE($4, trade),
            secondary_trades = COALESCE($5, secondary_trades),
            location = COALESCE($6, location),
            contact_email = COALESCE($7, contact_email),
            contact_phone = COALESCE($8, contact_phone),
            website = COALESCE($9, website),
            specialties = COALESCE($10, specialties),
            service_areas = COALESCE($11, service_areas),
            certifications = COALESCE($12, certifications),
            insurance = COALESCE($13, insurance),
            license_info = COALESCE($14, license_info),
            year_established = COALESCE($15, year_established),
            employee_count = COALESCE($16, employee_count),
            min_project_value = COALESCE($17, min_project_value),
            max_project_value = COALESCE($18, max_project_value),
            availability_status = COALESCE($19, availability_status),
            updated_at = NOW()
        WHERE id = $20
        "#,
    )
    .bind(&input.name)
    .bind(&input.headline)
    .bind(&input.company_description)
    .bind(&input.trade)
    .bind(secondary_trades)
    .bind(&input.location)
    .bind(&input.contact_email)
    .bind(&input.contact_phone)
    .bind(&input.website)
    .bind(specialties)
    .bind(service_areas)
    .bind(certifications)
    .bind(insurance)
    .bind(license_info)
    .bind(input.year_established)
    .bind(&input.employee_count)
    .bind(input.min_project_value)
    .bind(input.max_project_value)
    .bind(&input.availability_status)
    .bind(sub_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update profile: {}", e)))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// POST /api/marketplace/profile/request-verification
///
/// Request verification for the subcontractor profile.
pub async fn request_verification(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let result = sqlx::query(
        r#"
        UPDATE subcontractors 
        SET verification_status = 'pending', updated_at = NOW()
        WHERE profile_id = $1 AND verification_status != 'verified'
        "#,
    )
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::bad_request("Profile not found or already verified"));
    }

    Ok(Json(serde_json::json!({ "success": true, "message": "Verification request submitted" })))
}

// ============================================================================
// Portfolio Management
// ============================================================================

/// GET /api/marketplace/profile/portfolio
///
/// Get the current user's portfolio projects.
pub async fn get_my_portfolio(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let sub_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM subcontractors WHERE profile_id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let sub_id = sub_id.ok_or_else(|| ApiError::not_found("No subcontractor profile found"))?;

    let rows = sqlx::query_as::<_, PortfolioProject>(
        r#"
        SELECT id, subcontractor_id, title, description, project_type, trade_category,
               location, completion_date, project_value, client_name, client_testimonial,
               images, is_featured, display_order, created_at
        FROM portfolio_projects
        WHERE subcontractor_id = $1
        ORDER BY is_featured DESC, display_order ASC, created_at DESC
        "#,
    )
    .bind(sub_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<PortfolioProjectResponse> = rows.into_iter().map(Into::into).collect();

    Ok(Json(DataResponse::new(data)))
}

/// POST /api/marketplace/profile/portfolio
///
/// Add a new portfolio project.
pub async fn create_portfolio_project(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
    Json(input): Json<PortfolioProjectRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let sub_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM subcontractors WHERE profile_id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let sub_id = sub_id.ok_or_else(|| ApiError::not_found("No subcontractor profile found"))?;

    let id = Uuid::new_v4();
    let images = serde_json::to_value(input.images.unwrap_or_default()).unwrap_or(serde_json::json!([]));
    let display_order = input.display_order.unwrap_or(0);

    sqlx::query(
        r#"
        INSERT INTO portfolio_projects (
            id, subcontractor_id, title, description, project_type, trade_category,
            location, completion_date, project_value, client_name, client_testimonial,
            images, is_featured, display_order
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
    )
    .bind(id)
    .bind(sub_id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.project_type)
    .bind(&input.trade_category)
    .bind(&input.location)
    .bind(input.completion_date)
    .bind(input.project_value)
    .bind(&input.client_name)
    .bind(&input.client_testimonial)
    .bind(&images)
    .bind(input.is_featured.unwrap_or(false))
    .bind(display_order)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create portfolio project: {}", e)))?;

    Ok(Json(serde_json::json!({ "id": id, "success": true })))
}

/// PUT /api/marketplace/profile/portfolio/:id
///
/// Update a portfolio project.
pub async fn update_portfolio_project(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<PortfolioProjectRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Verify ownership
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM portfolio_projects pp
            JOIN subcontractors s ON pp.subcontractor_id = s.id
            WHERE pp.id = $1 AND s.profile_id = $2
        )
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !exists {
        return Err(ApiError::not_found("Portfolio project not found"));
    }

    let images = input.images.map(|i| serde_json::to_value(i).unwrap_or_default());

    sqlx::query(
        r#"
        UPDATE portfolio_projects SET
            title = $1,
            description = COALESCE($2, description),
            project_type = COALESCE($3, project_type),
            trade_category = COALESCE($4, trade_category),
            location = COALESCE($5, location),
            completion_date = COALESCE($6, completion_date),
            project_value = COALESCE($7, project_value),
            client_name = COALESCE($8, client_name),
            client_testimonial = COALESCE($9, client_testimonial),
            images = COALESCE($10, images),
            is_featured = COALESCE($11, is_featured),
            display_order = COALESCE($12, display_order),
            updated_at = NOW()
        WHERE id = $13
        "#,
    )
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.project_type)
    .bind(&input.trade_category)
    .bind(&input.location)
    .bind(input.completion_date)
    .bind(input.project_value)
    .bind(&input.client_name)
    .bind(&input.client_testimonial)
    .bind(images)
    .bind(input.is_featured)
    .bind(input.display_order)
    .bind(project_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update portfolio project: {}", e)))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/marketplace/profile/portfolio/:id
///
/// Delete a portfolio project.
pub async fn delete_portfolio_project(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let result = sqlx::query(
        r#"
        DELETE FROM portfolio_projects 
        WHERE id = $1 AND subcontractor_id IN (
            SELECT id FROM subcontractors WHERE profile_id = $2
        )
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Portfolio project not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================================
// Saved Searches
// ============================================================================

/// GET /api/marketplace/saved-searches
///
/// List user's saved searches.
pub async fn list_saved_searches(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let rows = sqlx::query_as::<_, SavedSearch>(
        r#"
        SELECT id, user_id, name, search_type, filters, notify_new_matches,
               last_run_at, created_at, updated_at
        FROM saved_searches
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<SavedSearchResponse> = rows.into_iter().map(Into::into).collect();

    Ok(Json(DataResponse::new(data)))
}

/// POST /api/marketplace/saved-searches
///
/// Create a new saved search.
pub async fn create_saved_search(
    State(state): State<Arc<AppState>>,
    auth: RequireAuth,
    Json(input): Json<CreateSavedSearchRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;
    let id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO saved_searches (id, user_id, name, search_type, filters, notify_new_matches)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(&input.name)
    .bind(&input.search_type)
    .bind(&input.filters)
    .bind(input.notify_new_matches)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create saved search: {}", e)))?;

    Ok(Json(serde_json::json!({ "id": id, "success": true })))
}

/// DELETE /api/marketplace/saved-searches/:id
///
/// Delete a saved search.
pub async fn delete_saved_search(
    State(state): State<Arc<AppState>>,
    Path(search_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let result = sqlx::query("DELETE FROM saved_searches WHERE id = $1 AND user_id = $2")
        .bind(search_id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Saved search not found"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================================
// Marketplace Tenders (for Subs to Browse)
// ============================================================================

/// GET /api/marketplace/tenders
///
/// List open tenders for subcontractors to browse and bid on.
pub async fn list_marketplace_tenders(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MarketplaceTenderQueryParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;
    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Get sub_id if user is a subcontractor (to show their bids)
    let sub_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM subcontractors WHERE profile_id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Count
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM tenders t
        JOIN projects p ON t.project_id = p.id
        WHERE t.status = 'open'
        AND t.visibility = 'public'
        AND (t.bid_due_date IS NULL OR t.bid_due_date > NOW())
        AND ($1::text IS NULL OR t.trade_category ILIKE '%' || $1 || '%')
        AND ($2::text IS NULL OR t.location ILIKE '%' || $2 || '%' OR p.location ILIKE '%' || $2 || '%')
        AND ($3::text IS NULL OR t.name ILIKE '%' || $3 || '%' OR t.description ILIKE '%' || $3 || '%')
        AND ($4::bigint IS NULL OR t.estimated_value >= $4)
        AND ($5::bigint IS NULL OR t.estimated_value <= $5)
        "#,
    )
    .bind(&query.filter.trade)
    .bind(&query.filter.location)
    .bind(&query.filter.search)
    .bind(query.filter.min_value)
    .bind(query.filter.max_value)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Determine sort
    let order_by = match query.filter.sort_by.as_deref() {
        Some("due_date") => "t.bid_due_date",
        Some("value") => "t.estimated_value",
        Some("newest") => "t.created_at",
        _ => "t.bid_due_date",
    };
    let order_dir = match query.filter.sort_order.as_deref() {
        Some("desc") => "DESC",
        _ => "ASC",
    };

    // Optimized query with LEFT JOIN to avoid N+1 for user's bid
    let query_str = format!(
        r#"
        SELECT 
            t.id, t.project_id, p.name as project_name, 
            pr.company_name as gc_company_name,
            t.name, t.description, t.trade_category, t.scope_of_work,
            COALESCE(t.location, p.location) as location,
            t.status, COALESCE(t.visibility, 'public') as visibility,
            t.bid_due_date, t.estimated_value, t.reserve_price,
            COALESCE(t.requirements, '{{}}'::jsonb) as requirements,
            (SELECT COUNT(*) FROM bids b WHERE b.tender_id = t.id) as bids_received,
            t.priority, t.created_at,
            -- User's bid info via LEFT JOIN (avoids N+1)
            my_bid.id as my_bid_id,
            my_bid.bid_amount as my_bid_amount,
            my_bid.status as my_bid_status,
            my_bid.submitted_at as my_bid_submitted_at
        FROM tenders t
        JOIN projects p ON t.project_id = p.id
        JOIN profiles pr ON p.owner_id = pr.id
        LEFT JOIN bids my_bid ON my_bid.tender_id = t.id AND my_bid.subcontractor_id = $8
        WHERE t.status = 'open'
        AND t.visibility = 'public'
        AND (t.bid_due_date IS NULL OR t.bid_due_date > NOW())
        AND ($1::text IS NULL OR t.trade_category ILIKE '%' || $1 || '%')
        AND ($2::text IS NULL OR t.location ILIKE '%' || $2 || '%' OR p.location ILIKE '%' || $2 || '%')
        AND ($3::text IS NULL OR t.name ILIKE '%' || $3 || '%' OR t.description ILIKE '%' || $3 || '%')
        AND ($4::bigint IS NULL OR t.estimated_value >= $4)
        AND ($5::bigint IS NULL OR t.estimated_value <= $5)
        ORDER BY {} {} NULLS LAST
        LIMIT $6 OFFSET $7
        "#,
        order_by, order_dir
    );

    let rows = sqlx::query_as::<_, TenderRow>(&query_str)
        .bind(&query.filter.trade)
        .bind(&query.filter.location)
        .bind(&query.filter.search)
        .bind(query.filter.min_value)
        .bind(query.filter.max_value)
        .bind(per_page as i64)
        .bind(offset)
        .bind(sub_id) // $8 for LEFT JOIN on user's bid
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Map rows to response - bid info already included via LEFT JOIN (no N+1!)
    let data: Vec<MarketplaceTender> = rows
        .into_iter()
        .map(|r| {
            // Extract user's bid from the LEFT JOIN columns
            let my_bid = r.my_bid_id.map(|id| MarketplaceBidSummary {
                id,
                bid_amount: r.my_bid_amount.map(decimal_to_i64).unwrap_or(0),
                status: r.my_bid_status.unwrap_or_default(),
                submitted_at: r.my_bid_submitted_at,
            });

            MarketplaceTender {
                id: r.id,
                project_id: r.project_id,
                project_name: r.project_name,
                gc_company_name: r.gc_company_name,
                name: r.name,
                description: r.description,
                trade_category: r.trade_category,
                scope_of_work: r.scope_of_work,
                location: r.location,
                status: r.status,
                visibility: r.visibility,
                bid_due_date: r.bid_due_date,
                estimated_value: r.estimated_value,
                reserve_price: r.reserve_price,
                requirements: r.requirements,
                bids_received: r.bids_received as i32,
                priority: r.priority,
                created_at: r.created_at,
                my_bid,
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

/// GET /api/marketplace/tenders/:id
///
/// Get a specific tender for bidding.
pub async fn get_marketplace_tender(
    State(state): State<Arc<AppState>>,
    Path(tender_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Get sub_id if user is a subcontractor
    let sub_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM subcontractors WHERE profile_id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let row = sqlx::query_as::<_, TenderRow>(
        r#"
        SELECT 
            t.id, t.project_id, p.name as project_name, 
            pr.company_name as gc_company_name,
            t.name, t.description, t.trade_category, t.scope_of_work,
            COALESCE(t.location, p.location) as location,
            t.status, COALESCE(t.visibility, 'public') as visibility,
            t.bid_due_date, t.estimated_value, t.reserve_price,
            COALESCE(t.requirements, '{}'::jsonb) as requirements,
            (SELECT COUNT(*) FROM bids b WHERE b.tender_id = t.id) as bids_received,
            t.priority, t.created_at
        FROM tenders t
        JOIN projects p ON t.project_id = p.id
        JOIN profiles pr ON p.owner_id = pr.id
        WHERE t.id = $1
        "#,
    )
    .bind(tender_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Tender not found"))?;

    // Check visibility
    if row.visibility == "invited_only" {
        // TODO: Check if user is invited
        // For now, allow access
    }

    // Get user's bid if they have one
    let my_bid = if let Some(sid) = sub_id {
        sqlx::query_as::<_, (Uuid, sqlx::types::Decimal, String, Option<DateTime<Utc>>)>(
            r#"
            SELECT id, bid_amount, status, submitted_at
            FROM bids
            WHERE tender_id = $1 AND subcontractor_id = $2
            "#,
        )
        .bind(tender_id)
        .bind(sid)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .map(|(id, amount, status, submitted_at)| MarketplaceBidSummary {
            id,
            bid_amount: decimal_to_i64(amount),
            status,
            submitted_at,
        })
    } else {
        None
    };

    let tender = MarketplaceTender {
        id: row.id,
        project_id: row.project_id,
        project_name: row.project_name,
        gc_company_name: row.gc_company_name,
        name: row.name,
        description: row.description,
        trade_category: row.trade_category,
        scope_of_work: row.scope_of_work,
        location: row.location,
        status: row.status,
        visibility: row.visibility,
        bid_due_date: row.bid_due_date,
        estimated_value: row.estimated_value,
        reserve_price: row.reserve_price,
        requirements: row.requirements,
        bids_received: row.bids_received as i32,
        priority: row.priority,
        created_at: row.created_at,
        my_bid,
    };

    Ok(Json(DataResponse::new(tender)))
}

// ============================================================================
// Bidding
// ============================================================================

/// POST /api/marketplace/tenders/:id/bid
///
/// Submit a bid on a tender.
pub async fn submit_bid(
    State(state): State<Arc<AppState>>,
    Path(tender_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<SubmitBidRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    // Get subcontractor ID
    let sub_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM subcontractors WHERE profile_id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let sub_id = sub_id.ok_or_else(|| {
        ApiError::forbidden("You need a subcontractor profile to submit bids")
    })?;

    // Check if tender exists and is open
    let tender: Option<(String, Option<DateTime<Utc>>, Uuid, String)> = sqlx::query_as(
        r#"
        SELECT t.status, t.bid_due_date, p.owner_id, t.name
        FROM tenders t
        JOIN projects p ON t.project_id = p.id
        WHERE t.id = $1
        "#,
    )
    .bind(tender_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let (status, due_date, gc_user_id, tender_name) = tender
        .ok_or_else(|| ApiError::not_found("Tender not found"))?;

    if status != "open" {
        return Err(ApiError::bad_request("This tender is no longer accepting bids"));
    }

    if let Some(due) = due_date {
        if due < Utc::now() {
            return Err(ApiError::bad_request("The bid deadline has passed"));
        }
    }

    // Check for existing bid
    let existing_bid: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM bids WHERE tender_id = $1 AND subcontractor_id = $2",
    )
    .bind(tender_id)
    .bind(sub_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if existing_bid.is_some() {
        return Err(ApiError::bad_request("You have already submitted a bid. Use PUT to update it."));
    }

    let id = Uuid::new_v4();
    let breakdown = serde_json::to_value(input.breakdown.unwrap_or_default()).unwrap_or(serde_json::json!([]));

    sqlx::query(
        r#"
        INSERT INTO bids (
            id, tender_id, subcontractor_id, bid_amount, breakdown,
            proposed_timeline_days, proposed_start_date, cover_letter, notes,
            status, submitted_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'submitted', NOW())
        "#,
    )
    .bind(id)
    .bind(tender_id)
    .bind(sub_id)
    .bind(input.bid_amount)
    .bind(&breakdown)
    .bind(input.proposed_timeline_days)
    .bind(input.proposed_start_date)
    .bind(&input.cover_letter)
    .bind(&input.notes)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to submit bid: {}", e)))?;

    // Get subcontractor name for notification
    let sub_name: Option<String> = sqlx::query_scalar("SELECT name FROM subcontractors WHERE id = $1")
        .bind(sub_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

    // Notify GC about new bid
    if let Err(e) = notifications::notify_bid_received(
        &state.db,
        gc_user_id,
        tender_id,
        &tender_name,
        &sub_name.unwrap_or_else(|| "A subcontractor".to_string()),
        input.bid_amount as f64 / 100.0, // Convert cents to dollars
    )
    .await
    {
        tracing::warn!(error = %e, "Failed to create bid notification");
    }

    Ok(Json(serde_json::json!({ "id": id, "success": true })))
}

/// PUT /api/marketplace/tenders/:id/bid
///
/// Update an existing bid.
pub async fn update_bid(
    State(state): State<Arc<AppState>>,
    Path(tender_id): Path<Uuid>,
    auth: RequireAuth,
    Json(input): Json<SubmitBidRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let sub_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM subcontractors WHERE profile_id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let sub_id = sub_id.ok_or_else(|| ApiError::forbidden("No subcontractor profile found"))?;

    // Check tender is still open
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM tenders WHERE id = $1")
        .bind(tender_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if status.as_deref() != Some("open") {
        return Err(ApiError::bad_request("This tender is no longer accepting bids"));
    }

    let breakdown = input.breakdown.map(|b| serde_json::to_value(b).unwrap_or_default());

    let result = sqlx::query(
        r#"
        UPDATE bids SET
            bid_amount = $1,
            breakdown = COALESCE($2, breakdown),
            proposed_timeline_days = COALESCE($3, proposed_timeline_days),
            proposed_start_date = COALESCE($4, proposed_start_date),
            cover_letter = COALESCE($5, cover_letter),
            notes = COALESCE($6, notes),
            updated_at = NOW()
        WHERE tender_id = $7 AND subcontractor_id = $8 AND status = 'submitted'
        "#,
    )
    .bind(input.bid_amount)
    .bind(breakdown)
    .bind(input.proposed_timeline_days)
    .bind(input.proposed_start_date)
    .bind(&input.cover_letter)
    .bind(&input.notes)
    .bind(tender_id)
    .bind(sub_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update bid: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Bid not found or cannot be updated"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/marketplace/tenders/:id/bid
///
/// Withdraw a bid.
pub async fn withdraw_bid(
    State(state): State<Arc<AppState>>,
    Path(tender_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;

    let sub_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM subcontractors WHERE profile_id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let sub_id = sub_id.ok_or_else(|| ApiError::forbidden("No subcontractor profile found"))?;

    let result = sqlx::query(
        "UPDATE bids SET status = 'withdrawn', updated_at = NOW() WHERE tender_id = $1 AND subcontractor_id = $2 AND status = 'submitted'",
    )
    .bind(tender_id)
    .bind(sub_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Bid not found or already processed"));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================================
// My Bids
// ============================================================================

/// GET /api/marketplace/my-bids
///
/// List the current user's submitted bids.
pub async fn list_my_bids(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaginationParams>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let sub_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM subcontractors WHERE profile_id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let sub_id = sub_id.ok_or_else(|| ApiError::forbidden("No subcontractor profile found"))?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bids WHERE subcontractor_id = $1")
        .bind(sub_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let rows = sqlx::query_as::<_, BidRow>(
        r#"
        SELECT 
            b.id, b.tender_id, t.name as tender_name, p.name as project_name,
            b.subcontractor_id, s.name as company_name,
            b.bid_amount, COALESCE(b.breakdown, '[]'::jsonb) as breakdown,
            b.proposed_timeline_days, b.proposed_start_date, b.cover_letter,
            b.status, COALESCE(b.is_winning_bid, false) as is_winning_bid,
            b.notes, b.submitted_at, b.created_at
        FROM bids b
        JOIN tenders t ON b.tender_id = t.id
        JOIN projects p ON t.project_id = p.id
        JOIN subcontractors s ON b.subcontractor_id = s.id
        WHERE b.subcontractor_id = $1
        ORDER BY b.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(sub_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let data: Vec<MarketplaceBidResponse> = rows
        .into_iter()
        .map(|r| MarketplaceBidResponse {
            id: r.id,
            tender_id: r.tender_id,
            tender_name: r.tender_name,
            project_name: r.project_name,
            subcontractor_id: r.subcontractor_id,
            company_name: r.company_name,
            bid_amount: decimal_to_i64(r.bid_amount),
            breakdown: serde_json::from_value(r.breakdown).unwrap_or_default(),
            proposed_timeline_days: r.proposed_timeline_days,
            proposed_start_date: r.proposed_start_date,
            cover_letter: r.cover_letter,
            status: r.status,
            is_winning_bid: r.is_winning_bid,
            notes: r.notes,
            submitted_at: r.submitted_at,
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
