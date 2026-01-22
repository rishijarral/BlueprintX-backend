//! Processing jobs routes
//!
//! Endpoints for managing document processing jobs with real-time progress.

use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    Json,
};
use chrono::{DateTime, Utc};
use futures::stream::{self, Stream};
use serde::Deserialize;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::api::pagination::PaginationParams;
use crate::api::response::{DataResponse, Paginated, PaginationMeta};
use crate::app::AppState;
use crate::auth::RequireAuth;
use crate::domain::jobs::{
    default_ingestion_steps, JobControlRequest, JobProgressEvent, ProcessingJobResponse,
    ProcessingStepResponse, StartProcessingRequest,
};
use crate::error::ApiError;

// ============================================================================
// Database Row Types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct ProcessingJobRow {
    id: Uuid,
    document_id: Uuid,
    project_id: Uuid,
    status: String,
    current_step: Option<String>,
    progress: sqlx::types::Decimal,
    total_steps: i32,
    completed_steps: i32,
    error_message: Option<String>,
    error_step: Option<String>,
    can_retry: bool,
    retry_count: i32,
    max_retries: i32,
    paused_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct ProcessingStepRow {
    id: Uuid,
    job_id: Uuid,
    step_name: String,
    step_key: String,
    step_order: i32,
    status: String,
    progress: sqlx::types::Decimal,
    message: Option<String>,
    details: serde_json::Value,
    items_total: i32,
    items_processed: i32,
    error_message: Option<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

// ============================================================================
// Conversion Functions
// ============================================================================

fn decimal_to_f64(d: sqlx::types::Decimal) -> f64 {
    use std::str::FromStr;
    f64::from_str(&d.to_string()).unwrap_or(0.0)
}

impl From<ProcessingStepRow> for ProcessingStepResponse {
    fn from(row: ProcessingStepRow) -> Self {
        Self {
            id: row.id,
            step_name: row.step_name,
            step_key: row.step_key,
            step_order: row.step_order,
            status: row.status,
            progress: decimal_to_f64(row.progress),
            message: row.message,
            details: row.details,
            items_total: row.items_total,
            items_processed: row.items_processed,
            error_message: row.error_message,
            started_at: row.started_at,
            completed_at: row.completed_at,
        }
    }
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct JobQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    pub status: Option<String>,
    pub document_id: Option<Uuid>,
}

// ============================================================================
// Route Handlers
// ============================================================================

/// POST /api/projects/:project_id/documents/:document_id/process
///
/// Start processing a document (triggers AI ingestion pipeline).
pub async fn start_processing(
    State(state): State<Arc<AppState>>,
    Path((project_id, document_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
    Json(input): Json<StartProcessingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    // Verify project ownership
    let project_owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if project_owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't have access to this project"));
    }

    // Verify document exists and belongs to project
    let doc_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND project_id = $2)",
    )
    .bind(document_id)
    .bind(project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !doc_exists {
        return Err(ApiError::not_found("Document not found"));
    }

    // Check if there's already an active job for this document
    let existing_job: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM processing_jobs WHERE document_id = $1 AND status IN ('queued', 'running', 'paused')",
    )
    .bind(document_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if let Some(job_id) = existing_job {
        return Err(ApiError::conflict(format!(
            "Document already has an active processing job: {}",
            job_id
        )));
    }

    // Create the processing job
    let job_id = Uuid::new_v4();
    let steps = default_ingestion_steps();
    let total_steps = steps.len() as i32;

    sqlx::query(
        r#"
        INSERT INTO processing_jobs (id, document_id, project_id, status, total_steps)
        VALUES ($1, $2, $3, 'queued', $4)
        "#,
    )
    .bind(job_id)
    .bind(document_id)
    .bind(project_id)
    .bind(total_steps)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create job: {}", e)))?;

    // Create processing steps
    for (step_key, step_name, step_order) in steps {
        sqlx::query(
            r#"
            INSERT INTO processing_steps (id, job_id, step_name, step_key, step_order, status)
            VALUES ($1, $2, $3, $4, $5, 'pending')
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(job_id)
        .bind(step_name)
        .bind(step_key.to_string())
        .bind(step_order)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create step: {}", e)))?;
    }

    // If auto_start is true (default), trigger the processing via the AI service
    if input.auto_start.unwrap_or(true) {
        // Update job status to running
        sqlx::query("UPDATE processing_jobs SET status = 'running', started_at = NOW() WHERE id = $1")
            .bind(job_id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to update job: {}", e)))?;

        // TODO: Trigger the AI service to start processing
        // This would be an async call to the Python AI service
        // For now, we just mark the job as running and the AI service will poll for jobs
    }

    // Fetch and return the created job
    let job = get_job_with_steps(&state, job_id).await?;
    Ok(Json(DataResponse::new(job)))
}

/// GET /api/projects/:project_id/jobs
///
/// List processing jobs for a project.
pub async fn list_project_jobs(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<JobQuery>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    // Verify project ownership
    let project_owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if project_owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't have access to this project"));
    }

    let page = query.pagination.page.unwrap_or(1).max(1);
    let per_page = query.pagination.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM processing_jobs 
        WHERE project_id = $1
        AND ($2::text IS NULL OR status = $2)
        AND ($3::uuid IS NULL OR document_id = $3)
        "#,
    )
    .bind(project_id)
    .bind(&query.status)
    .bind(query.document_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let jobs = sqlx::query_as::<_, ProcessingJobRow>(
        r#"
        SELECT id, document_id, project_id, status, current_step, progress,
               total_steps, completed_steps, error_message, error_step,
               can_retry, retry_count, max_retries, paused_at, started_at,
               completed_at, created_at, updated_at
        FROM processing_jobs
        WHERE project_id = $1
        AND ($2::text IS NULL OR status = $2)
        AND ($3::uuid IS NULL OR document_id = $3)
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(project_id)
    .bind(&query.status)
    .bind(query.document_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let mut responses = Vec::with_capacity(jobs.len());
    for job in jobs {
        let steps = get_job_steps(&state, job.id).await?;
        responses.push(ProcessingJobResponse {
            id: job.id,
            document_id: job.document_id,
            project_id: job.project_id,
            status: job.status,
            current_step: job.current_step,
            progress: decimal_to_f64(job.progress),
            total_steps: job.total_steps,
            completed_steps: job.completed_steps,
            error_message: job.error_message,
            error_step: job.error_step,
            can_retry: job.can_retry,
            retry_count: job.retry_count,
            steps,
            paused_at: job.paused_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            created_at: job.created_at,
        });
    }

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

    Ok(Json(Paginated {
        data: responses,
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

/// GET /api/projects/:project_id/jobs/:job_id
///
/// Get a specific processing job with all steps.
pub async fn get_job(
    State(state): State<Arc<AppState>>,
    Path((project_id, job_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    // Verify project ownership
    let project_owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if project_owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't have access to this project"));
    }

    let job = get_job_with_steps(&state, job_id).await?;
    
    if job.project_id != project_id {
        return Err(ApiError::not_found("Job not found"));
    }

    Ok(Json(DataResponse::new(job)))
}

/// POST /api/projects/:project_id/jobs/:job_id/control
///
/// Control a processing job (pause, resume, cancel, retry).
pub async fn control_job(
    State(state): State<Arc<AppState>>,
    Path((project_id, job_id)): Path<(Uuid, Uuid)>,
    auth: RequireAuth,
    Json(input): Json<JobControlRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = auth.user_id();

    // Verify project ownership
    let project_owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if project_owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't have access to this project"));
    }

    // Get current job status
    let job = sqlx::query_as::<_, ProcessingJobRow>(
        r#"
        SELECT id, document_id, project_id, status, current_step, progress,
               total_steps, completed_steps, error_message, error_step,
               can_retry, retry_count, max_retries, paused_at, started_at,
               completed_at, created_at, updated_at
        FROM processing_jobs WHERE id = $1 AND project_id = $2
        "#,
    )
    .bind(job_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Job not found"))?;

    use crate::domain::jobs::JobControlAction;
    match input.action {
        JobControlAction::Pause => {
            if job.status != "running" {
                return Err(ApiError::bad_request("Can only pause running jobs"));
            }
            sqlx::query("UPDATE processing_jobs SET status = 'paused', paused_at = NOW(), updated_at = NOW() WHERE id = $1")
                .bind(job_id)
                .execute(&state.db)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to pause job: {}", e)))?;
        }
        JobControlAction::Resume => {
            if job.status != "paused" {
                return Err(ApiError::bad_request("Can only resume paused jobs"));
            }
            sqlx::query("UPDATE processing_jobs SET status = 'running', paused_at = NULL, updated_at = NOW() WHERE id = $1")
                .bind(job_id)
                .execute(&state.db)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to resume job: {}", e)))?;
        }
        JobControlAction::Cancel => {
            if job.status == "completed" || job.status == "cancelled" {
                return Err(ApiError::bad_request("Cannot cancel completed or already cancelled jobs"));
            }
            sqlx::query("UPDATE processing_jobs SET status = 'cancelled', completed_at = NOW(), updated_at = NOW() WHERE id = $1")
                .bind(job_id)
                .execute(&state.db)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to cancel job: {}", e)))?;
        }
        JobControlAction::RetryStep { step_key } => {
            if job.status != "failed" && job.status != "paused" {
                return Err(ApiError::bad_request("Can only retry steps on failed or paused jobs"));
            }
            // Reset the failed step
            sqlx::query(
                "UPDATE processing_steps SET status = 'pending', error_message = NULL, progress = 0 WHERE job_id = $1 AND step_key = $2"
            )
            .bind(job_id)
            .bind(&step_key)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to reset step: {}", e)))?;

            // Update job status
            sqlx::query(
                "UPDATE processing_jobs SET status = 'running', error_message = NULL, error_step = NULL, retry_count = retry_count + 1, updated_at = NOW() WHERE id = $1"
            )
            .bind(job_id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to update job: {}", e)))?;
        }
        JobControlAction::RetryJob => {
            if job.status != "failed" {
                return Err(ApiError::bad_request("Can only retry failed jobs"));
            }
            if !job.can_retry || job.retry_count >= job.max_retries {
                return Err(ApiError::bad_request("Job has exceeded maximum retry attempts"));
            }
            // Reset all steps that are not completed
            sqlx::query(
                "UPDATE processing_steps SET status = 'pending', error_message = NULL, progress = 0 WHERE job_id = $1 AND status != 'completed'"
            )
            .bind(job_id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to reset steps: {}", e)))?;

            // Update job status
            sqlx::query(
                "UPDATE processing_jobs SET status = 'running', error_message = NULL, error_step = NULL, retry_count = retry_count + 1, updated_at = NOW() WHERE id = $1"
            )
            .bind(job_id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to update job: {}", e)))?;
        }
    }

    // Return updated job
    let job = get_job_with_steps(&state, job_id).await?;
    Ok(Json(DataResponse::new(job)))
}

/// GET /api/projects/:project_id/jobs/stream
///
/// SSE endpoint for real-time job progress updates.
pub async fn stream_job_progress(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    auth: RequireAuth,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let user_id = auth.user_id();

    // Verify project ownership
    let project_owner: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .flatten();

    if project_owner != Some(user_id) {
        return Err(ApiError::forbidden("You don't have access to this project"));
    }

    // Create a stream that polls the database for job updates
    // In production, this would be replaced with Redis pub/sub
    let db = state.db.clone();
    let stream = stream::unfold(
        (db, project_id, None::<DateTime<Utc>>),
        |(db, project_id, last_check)| async move {
            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_secs(1)).await;

            let now = Utc::now();

            // Fetch active jobs with recent updates
            let jobs: Vec<ProcessingJobRow> = sqlx::query_as(
                r#"
                SELECT id, document_id, project_id, status, current_step, progress,
                       total_steps, completed_steps, error_message, error_step,
                       can_retry, retry_count, max_retries, paused_at, started_at,
                       completed_at, created_at, updated_at
                FROM processing_jobs
                WHERE project_id = $1
                AND status IN ('queued', 'running', 'paused')
                ORDER BY updated_at DESC
                "#,
            )
            .bind(project_id)
            .fetch_all(&db)
            .await
            .unwrap_or_default();

            let events: Vec<Event> = if jobs.is_empty() {
                // Send heartbeat
                vec![Event::default()
                    .event("heartbeat")
                    .data(
                        serde_json::to_string(&JobProgressEvent::Heartbeat { timestamp: now })
                            .unwrap_or_default(),
                    )]
            } else {
                jobs.into_iter()
                    .map(|job| {
                        let event = JobProgressEvent::JobStatusChanged {
                            job_id: job.id,
                            status: job.status.clone(),
                            progress: decimal_to_f64(job.progress),
                            current_step: job.current_step.clone(),
                        };
                        Event::default()
                            .event("job_update")
                            .data(serde_json::to_string(&event).unwrap_or_default())
                    })
                    .collect()
            };

            Some((events, (db, project_id, Some(now))))
        },
    )
    .flat_map(|events| stream::iter(events.into_iter().map(Ok)));

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn get_job_steps(
    state: &AppState,
    job_id: Uuid,
) -> Result<Vec<ProcessingStepResponse>, ApiError> {
    let steps = sqlx::query_as::<_, ProcessingStepRow>(
        r#"
        SELECT id, job_id, step_name, step_key, step_order, status, progress,
               message, details, items_total, items_processed, error_message,
               started_at, completed_at, created_at
        FROM processing_steps
        WHERE job_id = $1
        ORDER BY step_order ASC
        "#,
    )
    .bind(job_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(steps.into_iter().map(Into::into).collect())
}

async fn get_job_with_steps(
    state: &AppState,
    job_id: Uuid,
) -> Result<ProcessingJobResponse, ApiError> {
    let job = sqlx::query_as::<_, ProcessingJobRow>(
        r#"
        SELECT id, document_id, project_id, status, current_step, progress,
               total_steps, completed_steps, error_message, error_step,
               can_retry, retry_count, max_retries, paused_at, started_at,
               completed_at, created_at, updated_at
        FROM processing_jobs WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Job not found"))?;

    let steps = get_job_steps(state, job_id).await?;

    Ok(ProcessingJobResponse {
        id: job.id,
        document_id: job.document_id,
        project_id: job.project_id,
        status: job.status,
        current_step: job.current_step,
        progress: decimal_to_f64(job.progress),
        total_steps: job.total_steps,
        completed_steps: job.completed_steps,
        error_message: job.error_message,
        error_step: job.error_step,
        can_retry: job.can_retry,
        retry_count: job.retry_count,
        steps,
        paused_at: job.paused_at,
        started_at: job.started_at,
        completed_at: job.completed_at,
        created_at: job.created_at,
    })
}
