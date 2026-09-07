//! Query string de los listados (`sort`, `paginate`, `page`, `limit`) con las mismas
//! reglas de VineJS de v2, y la paginación con la forma de `SimplePaginator` de Lucid.

use crate::error::{ApiError, ErrorValidacion};
use crate::repos::{OpcionOrden, Pagina};
use axum::http::Uri;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Default, Deserialize)]
pub struct QueryListado {
    pub sort: Option<String>,
    pub paginate: Option<String>,
    pub page: Option<String>,
    pub limit: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListadoValidado {
    pub orden: OpcionOrden,
    pub paginar: bool,
    pub pagina: i64,
    pub limite: i64,
    /// Query string validada, en el orden del esquema (sort, paginate, page, limit),
    /// para reconstruir las URLs de paginación como lo hacía Lucid.
    pub qs: Vec<(String, String)>,
}

fn entero_positivo(campo: &str, valor: &Option<String>, errores: &mut Vec<ErrorValidacion>) -> Option<i64> {
    let v = valor.as_ref()?;
    match v.parse::<f64>() {
        Ok(n) if n.fract() == 0.0 && n > 0.0 => Some(n as i64),
        Ok(n) if n.fract() != 0.0 => {
            errores.push(ErrorValidacion::nuevo(campo, "withoutDecimals", format!("The {campo} field must be an integer")));
            None
        }
        Ok(_) => {
            errores.push(ErrorValidacion::nuevo(campo, "positive", format!("The {campo} field must be positive")));
            None
        }
        Err(_) => {
            errores.push(ErrorValidacion::nuevo(campo, "number", format!("The {campo} field must be a number")));
            None
        }
    }
}

pub fn validar(q: &QueryListado, ordenes: &'static [OpcionOrden]) -> Result<ListadoValidado, ApiError> {
    let mut errores = Vec::new();
    let mut qs = Vec::new();

    let orden = match &q.sort {
        None => ordenes[0],
        Some(id) => match ordenes.iter().find(|o| o.id == id) {
            Some(o) => {
                qs.push(("sort".to_string(), id.clone()));
                *o
            }
            None => {
                errores.push(ErrorValidacion::nuevo("sort", "exists", "The selected sort is invalid"));
                ordenes[0]
            }
        },
    };

    let paginar = match q.paginate.as_deref().map(|s| s.to_ascii_lowercase()) {
        None => false,
        Some(s) if ["true", "1", "on"].contains(&s.as_str()) => {
            qs.push(("paginate".into(), "true".into()));
            true
        }
        Some(s) if ["false", "0", "off"].contains(&s.as_str()) => {
            qs.push(("paginate".into(), "false".into()));
            false
        }
        Some(_) => {
            errores.push(ErrorValidacion::nuevo("paginate", "boolean", "The value must be a boolean"));
            false
        }
    };

    let pagina = entero_positivo("page", &q.page, &mut errores);
    if let Some(p) = pagina {
        qs.push(("page".into(), p.to_string()));
    }
    let limite = entero_positivo("limit", &q.limit, &mut errores);
    if let Some(l) = limite {
        qs.push(("limit".into(), l.to_string()));
    }

    if !errores.is_empty() {
        return Err(ApiError::Validacion(errores));
    }

    Ok(ListadoValidado { orden, paginar, pagina: pagina.unwrap_or(1), limite: limite.unwrap_or(10), qs })
}

/// `getBaseUrlByHost` de v2: fuerza https salvo en localhost, y descarta la query.
pub fn base_url(uri: &Uri, host: Option<&str>) -> String {
    let host = host.unwrap_or("localhost");
    let esquema = if host.starts_with("localhost") || host.starts_with("127.0.0.1") { "http" } else { "https" };
    format!("{esquema}://{host}{}", uri.path())
}

fn url_pagina(base: &str, qs: &[(String, String)], pagina: i64) -> String {
    let mut pares: Vec<(String, String)> = qs.to_vec();
    match pares.iter_mut().find(|(k, _)| k == "page") {
        Some(p) => p.1 = pagina.to_string(),
        None => pares.push(("page".into(), pagina.to_string())),
    }
    let query = pares.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
    format!("{base}?{query}")
}

/// Forma de `SimplePaginator.serialize()` de Lucid.
pub fn serializar_pagina<T: Serialize>(p: Pagina<T>, l: &ListadoValidado, base: &str) -> Value {
    let ultima = ((p.total as f64) / (l.limite as f64)).ceil().max(1.0) as i64;
    let siguiente = if l.pagina < ultima { Some(url_pagina(base, &l.qs, l.pagina + 1)) } else { None };
    let anterior = if l.pagina > 1 { Some(url_pagina(base, &l.qs, l.pagina - 1)) } else { None };
    json!({
        "meta": {
            "total": p.total,
            "perPage": l.limite,
            "currentPage": l.pagina,
            "lastPage": ultima,
            "firstPage": 1,
            "firstPageUrl": url_pagina(base, &l.qs, 1),
            "lastPageUrl": url_pagina(base, &l.qs, ultima),
            "nextPageUrl": siguiente,
            "previousPageUrl": anterior,
        },
        "data": p.datos,
    })
}

/// Extrae `Host` (o `X-Forwarded-Host`) para las URLs de paginación.
pub fn host_de(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

#[allow(dead_code)]
pub type Parametros = HashMap<String, String>;
