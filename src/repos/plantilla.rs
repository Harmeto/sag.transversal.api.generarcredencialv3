use super::{Direccion, OpcionOrden, Pagina};
use serde::Serialize;
use sqlx::PgPool;

pub const ORDENES: &[OpcionOrden] = &[
    OpcionOrden { id: "plantilla_id_asc", columna: "id", direccion: Direccion::Asc },
    OpcionOrden { id: "plantilla_id_desc", columna: "id", direccion: Direccion::Desc },
    OpcionOrden { id: "plantilla_nombre_asc", columna: "nombre", direccion: Direccion::Asc },
    OpcionOrden { id: "plantilla_nombre_desc", columna: "nombre", direccion: Direccion::Desc },
    OpcionOrden { id: "plantilla_ruta_asc", columna: "ruta", direccion: Direccion::Asc },
    OpcionOrden { id: "plantilla_ruta_desc", columna: "ruta", direccion: Direccion::Desc },
];

/// Forma JSON de v2: `{ id, nombre, ruta, categoriaId }`.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Plantilla {
    pub id: i32,
    pub nombre: String,
    pub ruta: String,
    #[serde(rename = "categoriaId")]
    #[sqlx(rename = "categoriaId")]
    pub categoria_id: i32,
}

/// Plantilla con su categoría precargada, para el flujo de generación.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlantillaConCategoria {
    pub id: i32,
    pub nombre: String,
    pub ruta: String,
    pub motor: String,
    pub categoria_id: i32,
    pub categoria_nombre: String,
}

const COLUMNAS: &str = "id, nombre, ruta, categoria_id AS \"categoriaId\"";

pub async fn listar(pool: &PgPool, orden: OpcionOrden) -> sqlx::Result<Vec<Plantilla>> {
    let sql = format!("SELECT {COLUMNAS} FROM plantilla ORDER BY {} {}", orden.columna, orden.direccion.sql());
    sqlx::query_as(&sql).fetch_all(pool).await
}

pub async fn paginar(pool: &PgPool, orden: OpcionOrden, pagina: i64, limite: i64) -> sqlx::Result<Pagina<Plantilla>> {
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM plantilla").fetch_one(pool).await?;
    let sql = format!(
        "SELECT {COLUMNAS} FROM plantilla ORDER BY {} {} LIMIT $1 OFFSET $2",
        orden.columna,
        orden.direccion.sql()
    );
    let datos = sqlx::query_as(&sql).bind(limite).bind((pagina - 1) * limite).fetch_all(pool).await?;
    Ok(Pagina { total, datos })
}

pub async fn buscar(pool: &PgPool, id: i32) -> sqlx::Result<Option<Plantilla>> {
    let sql = format!("SELECT {COLUMNAS} FROM plantilla WHERE id = $1");
    sqlx::query_as(&sql).bind(id).fetch_optional(pool).await
}

pub async fn buscar_con_categoria(pool: &PgPool, id: i32) -> sqlx::Result<Option<PlantillaConCategoria>> {
    sqlx::query_as(
        "SELECT p.id, p.nombre, p.ruta, p.motor, p.categoria_id, c.nombre AS categoria_nombre \
         FROM plantilla p JOIN categoria c ON c.id = p.categoria_id WHERE p.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
