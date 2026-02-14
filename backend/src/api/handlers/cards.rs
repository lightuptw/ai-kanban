use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::dto::{BoardResponse, CardResponse, CreateCardRequest, MoveCardRequest, UpdateCardRequest};
use crate::api::AppState;
use crate::domain::KanbanError;
use crate::services::CardService;

pub async fn create_card(
    State(state): State<AppState>,
    Json(req): Json<CreateCardRequest>,
) -> Result<(StatusCode, Json<CardResponse>), KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::create_card(pool, req).await?;
    Ok((StatusCode::CREATED, Json(card)))
}

pub async fn get_board(
    State(state): State<AppState>,
) -> Result<Json<BoardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let board = CardService::get_board(pool).await?;
    Ok(Json(board))
}

pub async fn get_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_by_id(pool, &id).await?;
    Ok(Json(card))
}

pub async fn update_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCardRequest>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::update_card(pool, &id, req).await?;
    Ok(Json(card))
}

pub async fn move_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<MoveCardRequest>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::move_card(pool, &id, req).await?;
    Ok(Json(card))
}

pub async fn delete_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    CardService::delete_card(pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
