use super::{fecha_iso_santiago, Direccion, OpcionOrden, Pagina};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub const ORDENES: &[OpcionOrden] = &[
    OpcionOrden { id: "credencial_id_asc", columna: "id", direccion: Direccion::Asc },
    OpcionOrden { id: "credencial_id_desc", columna: "id", direccion: Direccion::Desc },
    OpcionOrden { id: "credencial_categoria_id_asc", columna: "categoria_id", direccion: Direccion::Asc },
    OpcionOrden { id: "credencial_categoria_id_desc", columna: "categoria_id", direccion: Direccion::Desc },
    OpcionOrden { id: "fecha_creacion_asc", columna: "fecha_creacion", direccion: Direccion::Asc },
    OpcionOrden { id: "fecha_creacion_desc", columna: "fecha_creacion", direccion: Direccion::Desc },
];

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CredencialFila {
    pub id: Uuid,
    pub categoria_id: i32,
    pub url: String,
    pub fecha_creacion: DateTime<Utc>,
}

/// Forma JSON de v2: `{ id, categoriaId, url, fechaCreacion }` (uuid en minúsculas,
/// fecha ISO con desfase de Santiago).
#[derive(Debug, Clone, Serialize)]
pub struct Credencial {
    pub id: String,
    #[serde(rename = "categoriaId")]
    pub categoria_id: i32,
    pub url: String,
    #[serde(rename = "fechaCreacion")]
    pub fecha_creacion: String,
}

impl From<CredencialFila> for Credencial {
    fn from(f: CredencialFila) -> Self {
        Self {
            id: f.id.hyphenated().to_string().to_lowercase(),
            categoria_id: f.categoria_id,
            url: f.url,
            fecha_creacion: fecha_iso_santiago(f.fecha_creacion),
        }
    }
}

const COLUMNAS: &str = "id, categoria_id, url, fecha_creacion";

pub async fn listar(pool: &PgPool, orden: OpcionOrden) -> sqlx::Result<Vec<Credencial>> {
    let sql = format!("SELECT {COLUMNAS} FROM credencial ORDER BY {} {}", orden.columna, orden.direccion.sql());
    let filas: Vec<CredencialFila> = sqlx::query_as(&sql).fetch_all(pool).await?;
    Ok(filas.into_iter().map(Into::into).collect())
}

pub async fn paginar(pool: &PgPool, orden: OpcionOrden, pagina: i64, limite: i64) -> sqlx::Result<Pagina<Credencial>> {
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM credencial").fetch_one(pool).await?;
    let sql = format!(
        "SELECT {COLUMNAS} FROM credencial ORDER BY {} {} LIMIT $1 OFFSET $2",
        orden.columna,
        orden.direccion.sql()
    );
    let filas: Vec<CredencialFila> =
        sqlx::query_as(&sql).bind(limite).bind((pagina - 1) * limite).fetch_all(pool).await?;
    Ok(Pagina { total, datos: filas.into_iter().map(Into::into).collect() })
}

pub async fn buscar(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Credencial>> {
    let sql = format!("SELECT {COLUMNAS} FROM credencial WHERE id = $1");
    let fila: Option<CredencialFila> = sqlx::query_as(&sql).bind(id).fetch_optional(pool).await?;
    Ok(fila.map(Into::into))
}

pub async fn crear(pool: &PgPool, id: Uuid, categoria_id: i32, url: &str) -> sqlx::Result<CredencialFila> {
    sqlx::query_as(
        "INSERT INTO credencial (id, categoria_id, url) VALUES ($1, $2, $3) \
         RETURNING id, categoria_id, url, fecha_creacion",
    )
    .bind(id)
    .bind(categoria_id)
    .bind(url)
    .fetch_one(pool)
    .await
}
