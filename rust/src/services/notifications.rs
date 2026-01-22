//! Notification service
//!
//! Provides functions to create notifications from other parts of the application.
//! This service is called by routes when events occur that should trigger notifications.

#![allow(dead_code)]

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::notifications::NotificationType;

/// Create a notification for a user
pub async fn create_notification(
    db: &PgPool,
    user_id: Uuid,
    notification_type: NotificationType,
    title: &str,
    message: Option<&str>,
    data: Option<serde_json::Value>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    let type_str = notification_type.to_string();
    let data = data.unwrap_or(serde_json::json!({}));

    sqlx::query(
        r#"
        INSERT INTO notifications (id, user_id, type, title, message, data)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(&type_str)
    .bind(title)
    .bind(message)
    .bind(&data)
    .execute(db)
    .await?;

    tracing::info!(
        user_id = %user_id,
        notification_type = %type_str,
        notification_id = %id,
        "Notification created"
    );

    Ok(id)
}

/// Create a bid received notification for a GC
pub async fn notify_bid_received(
    db: &PgPool,
    gc_user_id: Uuid,
    tender_id: Uuid,
    tender_title: &str,
    subcontractor_name: &str,
    bid_amount: f64,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        gc_user_id,
        NotificationType::BidReceived,
        &format!("New bid on {}", tender_title),
        Some(&format!(
            "{} submitted a bid of ${:.2}",
            subcontractor_name, bid_amount
        )),
        Some(serde_json::json!({
            "tender_id": tender_id,
            "tender_title": tender_title,
            "subcontractor_name": subcontractor_name,
            "bid_amount": bid_amount,
        })),
    )
    .await
}

/// Create a bid awarded notification for a subcontractor
pub async fn notify_bid_awarded(
    db: &PgPool,
    sub_user_id: Uuid,
    tender_id: Uuid,
    tender_title: &str,
    project_name: &str,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        sub_user_id,
        NotificationType::BidAwarded,
        &format!("Your bid was accepted!"),
        Some(&format!(
            "Congratulations! Your bid for '{}' on project '{}' has been selected.",
            tender_title, project_name
        )),
        Some(serde_json::json!({
            "tender_id": tender_id,
            "tender_title": tender_title,
            "project_name": project_name,
        })),
    )
    .await
}

/// Create a bid rejected notification for a subcontractor
pub async fn notify_bid_rejected(
    db: &PgPool,
    sub_user_id: Uuid,
    tender_id: Uuid,
    tender_title: &str,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        sub_user_id,
        NotificationType::BidRejected,
        &format!("Bid not selected"),
        Some(&format!(
            "Your bid for '{}' was not selected. Keep bidding on other opportunities!",
            tender_title
        )),
        Some(serde_json::json!({
            "tender_id": tender_id,
            "tender_title": tender_title,
        })),
    )
    .await
}

/// Create a hire request received notification for a subcontractor
pub async fn notify_hire_request_received(
    db: &PgPool,
    sub_user_id: Uuid,
    hire_request_id: Uuid,
    gc_company_name: &str,
    project_name: &str,
    trade: &str,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        sub_user_id,
        NotificationType::HireRequestReceived,
        &format!("New hire request from {}", gc_company_name),
        Some(&format!(
            "{} would like to hire you for {} work on project '{}'",
            gc_company_name, trade, project_name
        )),
        Some(serde_json::json!({
            "hire_request_id": hire_request_id,
            "gc_company_name": gc_company_name,
            "project_name": project_name,
            "trade": trade,
        })),
    )
    .await
}

/// Create a hire request accepted notification for a GC
pub async fn notify_hire_request_accepted(
    db: &PgPool,
    gc_user_id: Uuid,
    hire_request_id: Uuid,
    subcontractor_name: &str,
    project_name: &str,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        gc_user_id,
        NotificationType::HireRequestAccepted,
        &format!("{} accepted your hire request!", subcontractor_name),
        Some(&format!(
            "{} has accepted your hire request for project '{}'",
            subcontractor_name, project_name
        )),
        Some(serde_json::json!({
            "hire_request_id": hire_request_id,
            "subcontractor_name": subcontractor_name,
            "project_name": project_name,
        })),
    )
    .await
}

/// Create a hire request declined notification for a GC
pub async fn notify_hire_request_declined(
    db: &PgPool,
    gc_user_id: Uuid,
    hire_request_id: Uuid,
    subcontractor_name: &str,
    project_name: &str,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        gc_user_id,
        NotificationType::HireRequestDeclined,
        &format!("{} declined your hire request", subcontractor_name),
        Some(&format!(
            "{} has declined your hire request for project '{}'",
            subcontractor_name, project_name
        )),
        Some(serde_json::json!({
            "hire_request_id": hire_request_id,
            "subcontractor_name": subcontractor_name,
            "project_name": project_name,
        })),
    )
    .await
}

/// Create a contract sent notification for a subcontractor
pub async fn notify_contract_sent(
    db: &PgPool,
    sub_user_id: Uuid,
    contract_id: Uuid,
    gc_company_name: &str,
    project_name: &str,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        sub_user_id,
        NotificationType::ContractSent,
        &format!("Contract received from {}", gc_company_name),
        Some(&format!(
            "{} has sent you a contract for project '{}'. Please review and sign.",
            gc_company_name, project_name
        )),
        Some(serde_json::json!({
            "contract_id": contract_id,
            "gc_company_name": gc_company_name,
            "project_name": project_name,
        })),
    )
    .await
}

/// Create a contract signed notification
pub async fn notify_contract_signed(
    db: &PgPool,
    recipient_user_id: Uuid,
    contract_id: Uuid,
    signer_name: &str,
    project_name: &str,
    is_fully_signed: bool,
) -> Result<Uuid, sqlx::Error> {
    let notification_type = if is_fully_signed {
        NotificationType::ContractFullySigned
    } else {
        NotificationType::ContractSigned
    };

    let title = if is_fully_signed {
        format!("Contract fully signed!")
    } else {
        format!("{} signed the contract", signer_name)
    };

    let message = if is_fully_signed {
        format!(
            "The contract for project '{}' has been signed by all parties.",
            project_name
        )
    } else {
        format!(
            "{} has signed the contract for project '{}'. Awaiting remaining signatures.",
            signer_name, project_name
        )
    };

    create_notification(
        db,
        recipient_user_id,
        notification_type,
        &title,
        Some(&message),
        Some(serde_json::json!({
            "contract_id": contract_id,
            "signer_name": signer_name,
            "project_name": project_name,
            "is_fully_signed": is_fully_signed,
        })),
    )
    .await
}

/// Create a review received notification for a subcontractor
pub async fn notify_review_received(
    db: &PgPool,
    sub_user_id: Uuid,
    review_id: Uuid,
    reviewer_name: &str,
    rating: f64,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        sub_user_id,
        NotificationType::ReviewReceived,
        &format!("New review from {}", reviewer_name),
        Some(&format!(
            "{} left you a {:.1}-star review. View and respond to this feedback.",
            reviewer_name, rating
        )),
        Some(serde_json::json!({
            "review_id": review_id,
            "reviewer_name": reviewer_name,
            "rating": rating,
        })),
    )
    .await
}

/// Create a profile verified notification for a subcontractor
pub async fn notify_profile_verified(db: &PgPool, sub_user_id: Uuid) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        sub_user_id,
        NotificationType::ProfileVerified,
        "Your profile has been verified!",
        Some("Congratulations! Your profile has been verified. You now have a verification badge visible to GCs."),
        None,
    )
    .await
}

/// Create a profile rejected notification for a subcontractor
pub async fn notify_profile_rejected(
    db: &PgPool,
    sub_user_id: Uuid,
    reason: &str,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        sub_user_id,
        NotificationType::ProfileRejected,
        "Profile verification not approved",
        Some(&format!(
            "Your profile verification was not approved. Reason: {}. Please update your profile and resubmit.",
            reason
        )),
        Some(serde_json::json!({
            "reason": reason,
        })),
    )
    .await
}

/// Create a new message notification
pub async fn notify_new_message(
    db: &PgPool,
    recipient_user_id: Uuid,
    hire_request_id: Uuid,
    sender_name: &str,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        recipient_user_id,
        NotificationType::NewMessage,
        &format!("New message from {}", sender_name),
        Some(&format!("{} sent you a message.", sender_name)),
        Some(serde_json::json!({
            "hire_request_id": hire_request_id,
            "sender_name": sender_name,
        })),
    )
    .await
}

/// Create a tender closing soon notification for interested subcontractors
pub async fn notify_tender_closing_soon(
    db: &PgPool,
    sub_user_id: Uuid,
    tender_id: Uuid,
    tender_title: &str,
    hours_remaining: i32,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        sub_user_id,
        NotificationType::TenderClosingSoon,
        &format!("Tender closing in {} hours", hours_remaining),
        Some(&format!(
            "The tender '{}' is closing in {} hours. Submit or update your bid now!",
            tender_title, hours_remaining
        )),
        Some(serde_json::json!({
            "tender_id": tender_id,
            "tender_title": tender_title,
            "hours_remaining": hours_remaining,
        })),
    )
    .await
}

/// Create a system notification
pub async fn notify_system(
    db: &PgPool,
    user_id: Uuid,
    title: &str,
    message: &str,
) -> Result<Uuid, sqlx::Error> {
    create_notification(
        db,
        user_id,
        NotificationType::System,
        title,
        Some(message),
        None,
    )
    .await
}

/// Batch create notifications for multiple users
pub async fn create_notifications_batch(
    db: &PgPool,
    user_ids: &[Uuid],
    notification_type: NotificationType,
    title: &str,
    message: Option<&str>,
    data: Option<serde_json::Value>,
) -> Result<Vec<Uuid>, sqlx::Error> {
    let type_str = notification_type.to_string();
    let data = data.unwrap_or(serde_json::json!({}));
    let mut ids = Vec::with_capacity(user_ids.len());

    for user_id in user_ids {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO notifications (id, user_id, type, title, message, data)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&type_str)
        .bind(title)
        .bind(message)
        .bind(&data)
        .execute(db)
        .await?;
        ids.push(id);
    }

    tracing::info!(
        count = user_ids.len(),
        notification_type = %type_str,
        "Batch notifications created"
    );

    Ok(ids)
}
