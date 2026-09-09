//! Capa HTTP (axum). Rutas bajo `/api/v3/transversal/credencial`, mismas operaciones
//! y formas de respuesta que v2.

pub mod archivos;
pub mod catalogos;
pub mod generar;
pub mod health;
pub mod listado;
pub mod swagger;

use crate::render::RenderPool;
use crate::services::generar_credencial::GeneradorCredencial;
use crate::storage::local::LocalStorage;
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::{routing::get, routing::post, Json, Router};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub app_name: String,
    pub pool: PgPool,
    pub render: RenderPool,
    pub generador: GeneradorCredencial,
    pub storage_local: Option<Arc<LocalStorage>>,
    pub inicio: Instant,
}

pub const PREFIJO: &str = "/api/v3/transversal/credencial";

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health::check))
        .route("/categoria", get(catalogos::categoria_index))
        .route("/categoria/{id}", get(catalogos::categoria_show))
        .route("/plantilla", get(catalogos::plantilla_index))
        .route("/plantilla/{id}", get(catalogos::plantilla_show))
        .route("/credencial", get(catalogos::credencial_index))
        .route("/credencial/{guid}", get(catalogos::credencial_show))
        .route("/generarCredencial", post(generar::store))
        .route("/generarPreview", post(generar::preview))
        .route("/swagger", get(swagger::ui))
        .route("/swagger/swagger.json", get(swagger::documento));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    Router::new()
        .nest(PREFIJO, api)
        .route("/archivos/{contenedor}/{nombre}", get(archivos::servir))
        .fallback(no_encontrado)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// v2: cualquier ruta no registrada responde `404 { msg: "Not found" }`.
async fn no_encontrado() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({ "msg": "Not found" })))
}
