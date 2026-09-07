//! GET de catálogos: categoria, plantilla y credencial (index + show), con las
//! mismas respuestas que v2.

use super::listado::{base_url, host_de, serializar_pagina, validar, QueryListado};
use super::AppState;
use crate::error::{ApiError, ErrorValidacion};
use crate::repos::{categoria, credencial, plantilla};
use axum::extract::{OriginalUri, Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};
use uuid::Uuid;

fn id_positivo(valor: &str) -> Result<i32, ApiError> {
    match valor.parse::<i64>() {
        Ok(n) if n > 0 => i32::try_from(n).map_err(|_| ApiError::NoEncontrado),
        _ => Err(ApiError::Validacion(vec![ErrorValidacion::nuevo("id", "number", "The id field must be a number")])),
    }
}

pub async fn categoria_index(
    State(s): State<AppState>,
    Query(q): Query<QueryListado>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let l = validar(&q, categoria::ORDENES)?;
    if l.paginar {
        let p = categoria::paginar(&s.pool, l.orden, l.pagina, l.limite).await?;
        Ok(Json(serializar_pagina(p, &l, &base_url(&uri, host_de(&headers).as_deref()))))
    } else {
        Ok(Json(json!(categoria::listar(&s.pool, l.orden).await?)))
    }
}

pub async fn categoria_show(State(s): State<AppState>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let c = categoria::buscar(&s.pool, id_positivo(&id)?).await?.ok_or(ApiError::NoEncontrado)?;
    Ok(Json(json!(c)))
}

pub async fn plantilla_index(
    State(s): State<AppState>,
    Query(q): Query<QueryListado>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let l = validar(&q, plantilla::ORDENES)?;
    if l.paginar {
        let p = plantilla::paginar(&s.pool, l.orden, l.pagina, l.limite).await?;
        Ok(Json(serializar_pagina(p, &l, &base_url(&uri, host_de(&headers).as_deref()))))
    } else {
        Ok(Json(json!(plantilla::listar(&s.pool, l.orden).await?)))
    }
}

pub async fn plantilla_show(State(s): State<AppState>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let p = plantilla::buscar(&s.pool, id_positivo(&id)?).await?.ok_or(ApiError::NoEncontrado)?;
    Ok(Json(json!(p)))
}

pub async fn credencial_index(
    State(s): State<AppState>,
    Query(q): Query<QueryListado>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let l = validar(&q, credencial::ORDENES)?;
    if l.paginar {
        let p = credencial::paginar(&s.pool, l.orden, l.pagina, l.limite).await?;
        Ok(Json(serializar_pagina(p, &l, &base_url(&uri, host_de(&headers).as_deref()))))
    } else {
        Ok(Json(json!(credencial::listar(&s.pool, l.orden).await?)))
    }
}

pub async fn credencial_show(State(s): State<AppState>, Path(guid): Path<String>) -> Result<Json<Value>, ApiError> {
    let id = Uuid::parse_str(&guid).map_err(|_| {
        ApiError::Validacion(vec![ErrorValidacion::nuevo("guid", "uuid", "The guid field must be a valid UUID")])
    })?;
    let c = credencial::buscar(&s.pool, id).await?.ok_or(ApiError::NoEncontrado)?;
    Ok(Json(json!(c)))
}
