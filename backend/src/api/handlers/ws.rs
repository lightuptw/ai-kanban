use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use tokio::sync::broadcast::error::RecvError;

use crate::api::handlers::sse::WsEvent;
use crate::api::AppState;
use crate::domain::KanbanError;

pub async fn ws_logs_handler(
    ws: WebSocketUpgrade,
    Path(card_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, card_id, state))
}

#[derive(serde::Deserialize)]
pub struct WsEventsQuery {
    pub token: String,
}

pub async fn ws_events_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<WsEventsQuery>,
) -> Result<impl IntoResponse, KanbanError> {
    let pool = state.require_db()?;
    let signing_key = crate::auth::jwt::get_or_create_signing_key(pool)
        .await
        .map_err(|e| KanbanError::Unauthorized(format!("JWT key error: {e}")))?;

    let _auth_user = crate::auth::jwt::verify_token(&signing_key, &params.token)
        .map_err(|e| KanbanError::Unauthorized(format!("Invalid token: {e}")))?;

    Ok(ws.on_upgrade(move |socket| handle_ws_events(socket, state)))
}

async fn handle_ws(mut socket: WebSocket, card_id: String, state: AppState) {
    let mut rx = state.sse_tx.subscribe();

    loop {
        match rx.recv().await {
            Ok(msg) => {
                if let Ok(WsEvent::AgentLogCreated {
                    card_id: event_card_id,
                    ..
                }) = serde_json::from_str::<WsEvent>(&msg)
                {
                    if event_card_id == card_id
                        && socket.send(Message::Text(msg.into())).await.is_err()
                    {
                        break;
                    }
                }
            }
            Err(RecvError::Lagged(n)) => {
                tracing::debug!(skipped = n, "WS log receiver lagged, continuing");
                continue;
            }
            Err(RecvError::Closed) => break,
        }
    }
}

async fn handle_ws_events(mut socket: WebSocket, state: AppState) {
    let mut rx = state.sse_tx.subscribe();

    let _ = socket
        .send(Message::Text(r#"{"type":"connected"}"#.into()))
        .await;

    loop {
        match rx.recv().await {
            Ok(msg) => {
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
            Err(RecvError::Lagged(n)) => {
                tracing::debug!(skipped = n, "WS events receiver lagged, continuing");
                continue;
            }
            Err(RecvError::Closed) => break,
        }
    }
}
