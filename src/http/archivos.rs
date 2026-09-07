//! Sirve los PDF del adaptador de storage local (`/archivos/{contenedor}/{nombre}`),
//! con `Content-Type: application/pdf` como lo hace el blob de Azure en v2.

use super::AppState;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

pub async fn servir(State(s): State<AppState>, Path((contenedor, nombre)): Path<(String, String)>) -> Response {
    let Some(local) = s.storage_local.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(ruta) = local.ruta(&contenedor, &nombre) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match tokio::fs::read(&ruta).await {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/pdf".to_string()),
                (header::CONTENT_DISPOSITION, format!("inline; filename=\"{nombre}.pdf\"")),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
