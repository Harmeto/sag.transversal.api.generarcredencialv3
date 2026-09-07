//! Repositorios sobre PostgreSQL (sqlx). Los modelos conservan la semántica de
//! las tablas Categoria / Plantilla / Credencial de v2 y se serializan con la
//! misma forma JSON que exponía Lucid (camelCase).

pub mod categoria;
pub mod credencial;
pub mod plantilla;

use serde::Serialize;

/// Dirección de orden, validada contra el catálogo de `sort` de cada recurso.
#[derive(Debug, Clone, Copy)]
pub enum Direccion {
    Asc,
    Desc,
}

impl Direccion {
    pub fn sql(self) -> &'static str {
        match self {
            Direccion::Asc => "ASC",
            Direccion::Desc => "DESC",
        }
    }
}

/// Opción de orden equivalente a `sortOptions` de los servicios de v2.
#[derive(Debug, Clone, Copy)]
pub struct OpcionOrden {
    pub id: &'static str,
    pub columna: &'static str,
    pub direccion: Direccion,
}

#[derive(Debug, Clone, Serialize)]
pub struct Pagina<T> {
    pub total: i64,
    pub datos: Vec<T>,
}

/// Fecha serializada como la entregaba Luxon en v2: ISO 8601 con milisegundos y
/// desfase de America/Santiago (p. ej. `2023-04-23T20:00:00.000-04:00`).
pub fn fecha_iso_santiago(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.with_timezone(&chrono_tz::America::Santiago)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
}
