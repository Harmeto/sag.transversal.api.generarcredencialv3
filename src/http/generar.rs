//! POST /generarCredencial y POST /generarPreview.

use super::AppState;
use crate::error::{ApiError, ErrorValidacion};
use crate::repos::fecha_iso_santiago;
use crate::services::generar_credencial::SolicitudGeneracion;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

/// Validación equivalente a `storeCredencial` (VineJS) de v2.
fn validar(cuerpo: &Bytes) -> Result<SolicitudGeneracion, ApiError> {
    let crudo: Value = serde_json::from_slice(cuerpo)
        .map_err(|_| ApiError::Validacion(vec![ErrorValidacion::nuevo("body", "object", "The body must be a JSON object")]))?;
    let obj = crudo.as_object().ok_or_else(|| {
        ApiError::Validacion(vec![ErrorValidacion::nuevo("body", "object", "The body must be a JSON object")])
    })?;

    let mut errores = Vec::new();
    match obj.get("PlantillaId") {
        None => errores.push(ErrorValidacion::nuevo("PlantillaId", "required", "The PlantillaId field must be defined")),
        Some(v) => match v.as_f64() {
            Some(n) if n > 0.0 && n.fract() == 0.0 => {}
            Some(_) => errores.push(ErrorValidacion::nuevo("PlantillaId", "positive", "The PlantillaId field must be a positive integer")),
            None => errores.push(ErrorValidacion::nuevo("PlantillaId", "number", "The PlantillaId field must be a number")),
        },
    }
    match obj.get("DatosCredencial") {
        None => errores.push(ErrorValidacion::nuevo("DatosCredencial", "required", "The DatosCredencial field must be defined")),
        Some(v) if !v.is_object() => errores.push(ErrorValidacion::nuevo("DatosCredencial", "object", "The DatosCredencial field must be an object")),
        _ => {}
    }
    if let Some(v) = obj.get("GenerarQR") {
        if !v.is_boolean() && !v.is_null() {
            errores.push(ErrorValidacion::nuevo("GenerarQR", "boolean", "The value must be a boolean"));
        }
    }
    if let Some(v) = obj.get("QRTag") {
        if !v.is_string() && !v.is_null() {
            errores.push(ErrorValidacion::nuevo("QRTag", "string", "The QRTag field must be a string"));
        }
    }
    if !errores.is_empty() {
        return Err(ApiError::Validacion(errores));
    }

    serde_json::from_value(crudo)
        .map_err(|e| ApiError::Validacion(vec![ErrorValidacion::nuevo("body", "object", e.to_string())]))
}

/// v2 responde `201 { guid, categoriaId, url, fechaCreacion }`.
pub async fn store(State(s): State<AppState>, cuerpo: Bytes) -> Result<Response, ApiError> {
    let solicitud = validar(&cuerpo)?;
    let generado = s.generador.generar_pdf(&solicitud).await?;
    let fila = s.generador.guardar(generado).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "guid": fila.id.hyphenated().to_string().to_lowercase(),
            "categoriaId": fila.categoria_id,
            "url": fila.url,
            "fechaCreacion": fecha_iso_santiago(fila.fecha_creacion),
        })),
    )
        .into_response())
}

/// v2 responde el PDF inline (`Content-Disposition: inline; filename="preview-<id>.pdf"`).
pub async fn preview(State(s): State<AppState>, cuerpo: Bytes) -> Result<Response, ApiError> {
    let solicitud = validar(&cuerpo)?;
    let generado = s.generador.generar_pdf(&solicitud).await?;
    let nombre = format!("preview-{}.pdf", uuid::Uuid::new_v4().simple());
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (header::CONTENT_DISPOSITION, format!("inline; filename=\"{nombre}\"")),
        ],
        generado.pdf,
    )
        .into_response())
}
