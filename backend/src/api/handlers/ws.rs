use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;

use crate::api::handlers::sse::SseEvent;
use crate::api::AppState;

pub async fn ws_logs_handler(
    ws: WebSocketUpgrade,
    Path(card_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, card_id, state))
}

async fn handle_ws(mut socket: WebSocket, card_id: String, state: AppState) {
    let mut rx = state.sse_tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if let Ok(event) = serde_json::from_str::<SseEvent>(&msg) {
            if let SseEvent::AgentLogCreated {
                card_id: event_card_id,
                ..
            } = event
            {
                if event_card_id == card_id && socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}
