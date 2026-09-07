//! Errores de la API y su traducción a las respuestas HTTP de v2.
//!
//! Formas conservadas de v2:
//! - validación: `400` con `[{ message, rule, field }]` (formato VineJS)
//! - no encontrado: `404` con `{ title: "Not Found", status: 404 }`
//! - fallo al guardar: `400` con `{ title: "Bad Request", status: 400 }`
//!
//! Diferencia deliberada respecto de v2: un fallo del motor de render responde
//! `500 { title: "Internal Server Error", status: 500 }`. En v2 se devolvía 404,
//! que confundía "la plantilla no existe" con "el render falló".

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, Serialize)]
pub struct ErrorValidacion {
    pub message: String,
    pub rule: String,
    pub field: String,
}

impl ErrorValidacion {
    pub fn nuevo(field: &str, rule: &str, message: impl Into<String>) -> Self {
        Self { message: message.into(), rule: rule.to_string(), field: field.to_string() }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("validación")]
    Validacion(Vec<ErrorValidacion>),
    #[error("no encontrado")]
    NoEncontrado,
    #[error("fallo al guardar: {0}")]
    Guardar(String),
    #[error("fallo de render: {0}")]
    Render(String),
    #[error("error interno: {0}")]
    Interno(String),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Interno(e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Validacion(errores) => (StatusCode::BAD_REQUEST, Json(errores)).into_response(),
            ApiError::NoEncontrado => (
                StatusCode::NOT_FOUND,
                Json(json!({ "title": "Not Found", "status": 404 })),
            )
                .into_response(),
            ApiError::Guardar(detalle) => {
                tracing::error!(detalle, "fallo al guardar la credencial");
                (StatusCode::BAD_REQUEST, Json(json!({ "title": "Bad Request", "status": 400 })))
                    .into_response()
            }
            ApiError::Render(detalle) => {
                tracing::error!(detalle, "fallo del motor de render");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "title": "Internal Server Error", "status": 500 })),
                )
                    .into_response()
            }
            ApiError::Interno(detalle) => {
                tracing::error!(detalle, "error interno");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "title": "Internal Server Error", "status": 500 })),
                )
                    .into_response()
            }
        }
    }
}
