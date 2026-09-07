use super::{Direccion, OpcionOrden, Pagina};
use crate::repos::plantilla::Plantilla;
use serde::Serialize;
use sqlx::PgPool;

pub const ORDENES: &[OpcionOrden] = &[
    OpcionOrden { id: "categoria_id_asc", columna: "id", direccion: Direccion::Asc },
    OpcionOrden { id: "categoria_id_desc", columna: "id", direccion: Direccion::Desc },
    OpcionOrden { id: "categoria_nombre_asc", columna: "nombre", direccion: Direccion::Asc },
    OpcionOrden { id: "categoria_nombre_desc", columna: "nombre", direccion: Direccion::Desc },
];

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CategoriaFila {
    pub id: i32,
    pub nombre: String,
}

/// Forma JSON de v2: `{ id, nombre, plantillas: [...] }`.
#[derive(Debug, Clone, Serialize)]
pub struct Categoria {
    pub id: i32,
    pub nombre: String,
    pub plantillas: Vec<Plantilla>,
}

async fn con_plantillas(pool: &PgPool, filas: Vec<CategoriaFila>) -> sqlx::Result<Vec<Categoria>> {
    let ids: Vec<i32> = filas.iter().map(|c| c.id).collect();
    let plantillas: Vec<Plantilla> = sqlx::query_as(
        "SELECT id, nombre, ruta, categoria_id AS \"categoriaId\" FROM plantilla WHERE categoria_id = ANY($1) ORDER BY id",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;

    Ok(filas
        .into_iter()
        .map(|c| Categoria {
            id: c.id,
            nombre: c.nombre,
            plantillas: plantillas.iter().filter(|p| p.categoria_id == c.id).cloned().collect(),
        })
        .collect())
}

pub async fn listar(pool: &PgPool, orden: OpcionOrden) -> sqlx::Result<Vec<Categoria>> {
    let sql = format!("SELECT id, nombre FROM categoria ORDER BY {} {}", orden.columna, orden.direccion.sql());
    let filas: Vec<CategoriaFila> = sqlx::query_as(&sql).fetch_all(pool).await?;
    con_plantillas(pool, filas).await
}

pub async fn paginar(pool: &PgPool, orden: OpcionOrden, pagina: i64, limite: i64) -> sqlx::Result<Pagina<Categoria>> {
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM categoria").fetch_one(pool).await?;
    let sql = format!(
        "SELECT id, nombre FROM categoria ORDER BY {} {} LIMIT $1 OFFSET $2",
        orden.columna,
        orden.direccion.sql()
    );
    let filas: Vec<CategoriaFila> =
        sqlx::query_as(&sql).bind(limite).bind((pagina - 1) * limite).fetch_all(pool).await?;
    Ok(Pagina { total, datos: con_plantillas(pool, filas).await? })
}

pub async fn buscar(pool: &PgPool, id: i32) -> sqlx::Result<Option<Categoria>> {
    let fila: Option<CategoriaFila> =
        sqlx::query_as("SELECT id, nombre FROM categoria WHERE id = $1").bind(id).fetch_optional(pool).await?;
    match fila {
        None => Ok(None),
        Some(f) => Ok(con_plantillas(pool, vec![f]).await?.pop()),
    }
}
