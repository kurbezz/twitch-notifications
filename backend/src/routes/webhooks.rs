use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};

use crate::error::AppError;
use crate::services::webhooks::{EventSubPayload, WebhookService};
use crate::AppState;

const MESSAGE_TYPE_VERIFICATION: &str = "webhook_callback_verification";
const MESSAGE_TYPE_NOTIFICATION: &str = "notification";
const MESSAGE_TYPE_REVOCATION: &str = "revocation";

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/twitch", post(handle_twitch_webhook))
}

async fn handle_twitch_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, String), AppError> {
    let (message_id, timestamp, signature, message_type) =
        WebhookService::extract_headers(&headers)?;

    WebhookService::verify_signature(&state, &message_id, &timestamp, &body, &signature)?;

    let payload: EventSubPayload = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("Invalid payload: {}", e)))?;

    tracing::info!(
        "Received EventSub webhook: message_type={}, subscription_type={}, subscription_id={}",
        message_type,
        payload.subscription.subscription_type,
        payload.subscription.id
    );

    match message_type.as_str() {
        MESSAGE_TYPE_VERIFICATION => {
            let challenge = WebhookService::handle_verification(&state, &payload).await?;
            Ok((StatusCode::OK, challenge))
        }
        MESSAGE_TYPE_NOTIFICATION => {
            WebhookService::handle_notification(&state, &payload).await?;
            Ok((StatusCode::OK, "OK".to_string()))
        }
        MESSAGE_TYPE_REVOCATION => {
            tracing::warn!(
                "Subscription revoked: id={}, type={}, reason={}",
                payload.subscription.id,
                payload.subscription.subscription_type,
                payload.subscription.status
            );
            Ok((StatusCode::OK, "OK".to_string()))
        }
        _ => {
            tracing::warn!("Unknown message type: {}", message_type);
            Ok((StatusCode::OK, "OK".to_string()))
        }
    }
}
