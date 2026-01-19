//! AI Service client for communicating with the Python LLM service.
//!
//! Provides type-safe methods for:
//! - Plan summary generation
//! - Trade scope extraction
//! - Tender scope document generation
//! - RAG-based Q&A
//! - Document ingestion job management

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, instrument};
use uuid::Uuid;

use crate::domain::ai::{
    PlanSummary, QnAResponse, TenderScopeDoc, TradeScopesOutput,
};
use crate::error::ApiError;

/// Client for the AI service.
#[derive(Clone)]
pub struct AiClient {
    client: Client,
    base_url: String,
    token: String,
}

/// Error response from AI service.
#[derive(Debug, Deserialize)]
struct AiErrorResponse {
    #[allow(dead_code)]
    code: String,
    message: String,
    #[allow(dead_code)]
    request_id: Option<String>,
}

impl AiClient {
    /// Create a new AI service client.
    pub fn new(base_url: &str, token: &str, timeout_seconds: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        tracing::info!(base_url = base_url, "AI client initialized");

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
        })
    }

    /// Make a POST request to the AI service.
    async fn post<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
        request_id: Option<&str>,
    ) -> Result<R, ApiError> {
        let url = format!("{}{}", self.base_url, path);

        let mut req = self
            .client
            .post(&url)
            .header("X-Internal-Token", &self.token)
            .header("Content-Type", "application/json");

        if let Some(rid) = request_id {
            req = req.header("x-request-id", rid);
        }

        debug!(url = %url, "AI service request");

        let response = req
            .json(body)
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "AI service request failed");
                ApiError::Internal(anyhow::anyhow!("AI service unavailable: {}", e))
            })?;

        let status = response.status();

        if status.is_success() {
            response.json::<R>().await.map_err(|e| {
                error!(error = %e, "Failed to parse AI service response");
                ApiError::Internal(anyhow::anyhow!("Invalid AI service response: {}", e))
            })
        } else {
            let error_body = response
                .json::<AiErrorResponse>()
                .await
                .ok();

            let message = error_body
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| format!("AI service error: {}", status));

            match status {
                StatusCode::BAD_REQUEST => Err(ApiError::BadRequest(message)),
                StatusCode::UNAUTHORIZED => {
                    error!("AI service authentication failed");
                    Err(ApiError::Internal(anyhow::anyhow!("AI service auth error")))
                }
                StatusCode::NOT_FOUND => Err(ApiError::NotFound(message)),
                _ => {
                    error!(status = %status, message = %message, "AI service error");
                    Err(ApiError::Internal(anyhow::anyhow!(message)))
                }
            }
        }
    }

    /// Check AI service health.
    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/health", self.base_url);

        self.client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .context("AI service health check failed")?
            .error_for_status()
            .context("AI service unhealthy")?;

        Ok(())
    }

    // =========================================================================
    // Plan Analysis Endpoints
    // =========================================================================

    /// Generate a plan summary from document text.
    #[instrument(skip(self, document_text))]
    pub async fn generate_plan_summary(
        &self,
        project_id: Uuid,
        document_text: &str,
        instructions: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<PlanSummary, ApiError> {
        #[derive(Serialize)]
        struct Request<'a> {
            project_id: String,
            document_text: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            instructions: Option<&'a str>,
        }

        #[derive(Deserialize)]
        struct Response {
            summary: PlanSummary,
        }

        let response: Response = self
            .post(
                "/v1/plan/summary",
                &Request {
                    project_id: project_id.to_string(),
                    document_text,
                    instructions,
                },
                request_id,
            )
            .await?;

        Ok(response.summary)
    }

    /// Extract trade scopes from document text.
    #[instrument(skip(self, document_text))]
    pub async fn extract_trade_scopes(
        &self,
        project_id: Uuid,
        document_text: &str,
        trades: Option<Vec<String>>,
        request_id: Option<&str>,
    ) -> Result<TradeScopesOutput, ApiError> {
        #[derive(Serialize)]
        struct Request<'a> {
            project_id: String,
            document_text: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            trades: Option<Vec<String>>,
        }

        #[derive(Deserialize)]
        struct Response {
            scopes: TradeScopesOutput,
        }

        let response: Response = self
            .post(
                "/v1/plan/trade-scopes",
                &Request {
                    project_id: project_id.to_string(),
                    document_text,
                    trades,
                },
                request_id,
            )
            .await?;

        Ok(response.scopes)
    }

    /// Get list of standard trades.
    pub async fn get_standard_trades(&self, request_id: Option<&str>) -> Result<Vec<String>, ApiError> {
        let url = format!("{}/v1/plan/trades", self.base_url);

        let mut req = self
            .client
            .get(&url)
            .header("X-Internal-Token", &self.token);

        if let Some(rid) = request_id {
            req = req.header("x-request-id", rid);
        }

        let response = req.send().await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("AI service unavailable: {}", e))
        })?;

        if response.status().is_success() {
            response.json().await.map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Invalid response: {}", e))
            })
        } else {
            Err(ApiError::Internal(anyhow::anyhow!("Failed to get trades")))
        }
    }

    // =========================================================================
    // Tender Endpoints
    // =========================================================================

    /// Generate a tender scope document.
    #[instrument(skip(self, scope_data))]
    pub async fn generate_tender_scope_doc(
        &self,
        project_id: Uuid,
        trade: &str,
        scope_data: &serde_json::Value,
        project_context: Option<&str>,
        bid_due_date: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<TenderScopeDoc, ApiError> {
        #[derive(Serialize)]
        struct Request<'a> {
            project_id: String,
            trade: &'a str,
            scope_data: &'a serde_json::Value,
            #[serde(skip_serializing_if = "Option::is_none")]
            project_context: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            bid_due_date: Option<&'a str>,
        }

        #[derive(Deserialize)]
        struct Response {
            document: TenderScopeDoc,
        }

        let response: Response = self
            .post(
                "/v1/tenders/scope-doc",
                &Request {
                    project_id: project_id.to_string(),
                    trade,
                    scope_data,
                    project_context,
                    bid_due_date,
                },
                request_id,
            )
            .await?;

        Ok(response.document)
    }

    // =========================================================================
    // Q&A Endpoints
    // =========================================================================

    /// Answer a question about project documents.
    #[instrument(skip(self, document_text))]
    pub async fn ask_question(
        &self,
        project_id: Uuid,
        question: &str,
        document_id: Option<Uuid>,
        document_text: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<QnAResponse, ApiError> {
        #[derive(Serialize)]
        struct Request<'a> {
            project_id: String,
            question: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            document_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            document_text: Option<&'a str>,
        }

        let response: QnAResponse = self
            .post(
                "/v1/qna",
                &Request {
                    project_id: project_id.to_string(),
                    question,
                    document_id: document_id.map(|id| id.to_string()),
                    document_text,
                },
                request_id,
            )
            .await?;

        Ok(response)
    }

    // =========================================================================
    // Job Management Endpoints (scaffolded for future use)
    // =========================================================================

    /// Create an ingestion job for a document.
    #[allow(dead_code)]
    #[instrument(skip(self))]
    pub async fn create_ingest_job(
        &self,
        project_id: Uuid,
        document_id: Uuid,
        file_path: &str,
        request_id: Option<&str>,
    ) -> Result<JobResponse, ApiError> {
        #[derive(Serialize)]
        struct Request<'a> {
            r#type: &'static str,
            input: JobInput<'a>,
            project_id: String,
            document_id: String,
        }

        #[derive(Serialize)]
        struct JobInput<'a> {
            file_path: &'a str,
        }

        let response: JobResponse = self
            .post(
                "/v1/jobs",
                &Request {
                    r#type: "document_ingest",
                    input: JobInput { file_path },
                    project_id: project_id.to_string(),
                    document_id: document_id.to_string(),
                },
                request_id,
            )
            .await?;

        Ok(response)
    }

    /// Get job status.
    #[allow(dead_code)]
    pub async fn get_job(&self, job_id: &str, request_id: Option<&str>) -> Result<JobResponse, ApiError> {
        let url = format!("{}/v1/jobs/{}", self.base_url, job_id);

        let mut req = self
            .client
            .get(&url)
            .header("X-Internal-Token", &self.token);

        if let Some(rid) = request_id {
            req = req.header("x-request-id", rid);
        }

        let response = req.send().await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("AI service unavailable: {}", e))
        })?;

        if response.status().is_success() {
            response.json().await.map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Invalid response: {}", e))
            })
        } else if response.status() == StatusCode::NOT_FOUND {
            Err(ApiError::NotFound("Job not found".to_string()))
        } else {
            Err(ApiError::Internal(anyhow::anyhow!("Failed to get job")))
        }
    }

    /// Run a job (synchronous).
    #[allow(dead_code)]
    pub async fn run_job(&self, job_id: &str, request_id: Option<&str>) -> Result<JobResponse, ApiError> {
        #[derive(Serialize)]
        struct Empty {}

        self.post(&format!("/v1/jobs/{}/run", job_id), &Empty {}, request_id).await
    }

    /// Run a job (asynchronous).
    #[allow(dead_code)]
    pub async fn run_job_async(&self, job_id: &str, request_id: Option<&str>) -> Result<JobResponse, ApiError> {
        #[derive(Serialize)]
        struct Empty {}

        self.post(&format!("/v1/jobs/{}/run-async", job_id), &Empty {}, request_id).await
    }
}

/// Job response from AI service.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResponse {
    pub job_id: String,
    pub r#type: String,
    pub status: String,
    pub progress: f64,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub project_id: Option<String>,
    pub document_id: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}
