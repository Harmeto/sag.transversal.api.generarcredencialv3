//! Migración de datos de v2 (SQL Server) a v3 (PostgreSQL).
//!
//! Consume el JSON que produce `scripts/exportar-v2.mjs` y carga las tres tablas
//! conservando los identificadores originales. Eso es obligatorio: los consumidores
//! envían `PlantillaId` (el BFF de Emergencias Pecuarias, por ejemplo, lo lleva en una
//! variable de entorno), y los `guid` de las credenciales ya están impresos en códigos QR.
//!
//! Uso:
//!   migrar-datos --archivo tmp/export-v2.json [opciones]
//!
//!   --archivo <ruta>       JSON exportado desde v2 (obligatorio)
//!   --zona <tz>            zona en que se interpretan las fechas de v2
//!                          (por omisión America/Santiago, el TZ del contenedor de v2)
//!   --dry-run              hace todo el trabajo y deshace la transacción al final
//!   --limpiar-catalogos    borra las categorías y plantillas de la semilla antes de
//!                          cargar las de v2. Es el modo normal en un traspaso real:
//!                          sin esto, los ids de la semilla chocan con los de v2
//!
//! La conexión sale de la misma configuración que usa la API (`.env` en local, Consul
//! en los ambientes), así que el binario sirve tanto desde un puesto de trabajo con
//! port-forward como desde dentro del pod.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use sag_transversal_api_generarcredencialv3::config::Config;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Postgres, Transaction};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct Exportacion {
    #[serde(default)]
    origen: serde_json::Value,
    categorias: Vec<CategoriaV2>,
    plantillas: Vec<PlantillaV2>,
    credenciales: Vec<CredencialV2>,
}

#[derive(Debug, Deserialize)]
struct CategoriaV2 {
    id: i32,
    nombre: String,
}

#[derive(Debug, Deserialize)]
struct PlantillaV2 {
    id: i32,
    nombre: String,
    ruta: String,
    #[serde(rename = "categoriaId")]
    categoria_id: i32,
}

#[derive(Debug, Deserialize)]
struct CredencialV2 {
    id: String,
    #[serde(rename = "categoriaId")]
    categoria_id: i32,
    url: String,
    /// "aaaa-mm-ddThh:mm:ss.mmm", sin zona: es la hora de pared que guardó v2.
    #[serde(rename = "fechaCreacion")]
    fecha_creacion: Option<String>,
}

const AYUDA: &str = "\
Migra los datos de v2 (SQL Server) a v3 (PostgreSQL) conservando los identificadores.

Uso: migrar-datos --archivo <ruta.json> [opciones]

  --archivo <ruta>      JSON exportado con scripts/exportar-v2.mjs (obligatorio)
  --zona <tz>           Zona de las fechas de v2 (por omisión America/Santiago)
  --dry-run             Hace todo y deshace la transacción al final
  --limpiar-catalogos   Reemplaza las categorías y plantillas de la semilla por las de v2

La conexión sale de la configuración de la API: .env en local, Consul en los ambientes.
Detalle completo en docs/MIGRACION-DATOS.md";

struct Opciones {
    archivo: String,
    zona: Tz,
    dry_run: bool,
    limpiar_catalogos: bool,
}

fn leer_opciones() -> Result<Opciones> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut archivo = None;
    let mut zona = "America/Santiago".to_string();
    let mut dry_run = false;
    let mut limpiar_catalogos = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--archivo" => {
                archivo = args.get(i + 1).cloned();
                i += 2;
            }
            "--zona" => {
                zona = args.get(i + 1).cloned().context("--zona necesita un valor")?;
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--limpiar-catalogos" => {
                limpiar_catalogos = true;
                i += 1;
            }
            "--help" | "-h" => {
                println!("{AYUDA}");
                std::process::exit(0);
            }
            otro => bail!("opción desconocida: {otro}"),
        }
    }

    Ok(Opciones {
        archivo: archivo.context("falta --archivo con el JSON exportado desde v2")?,
        zona: zona.parse().map_err(|_| anyhow::anyhow!("zona horaria desconocida: {zona}"))?,
        dry_run,
        limpiar_catalogos,
    })
}

/// La ruta de v2 apunta al HTML; en v3 la misma plantilla es un `.typ`.
fn ruta_v3(ruta_v2: &str) -> String {
    match ruta_v2.strip_suffix(".html") {
        Some(base) => format!("{base}.typ"),
        None => ruta_v2.to_string(),
    }
}

/// Convierte la hora de pared que guardó v2 al instante UTC que espera `timestamptz`.
///
/// Los dos casos límite son reales en Chile: en el cambio de septiembre hay horas
/// locales que no existen, y en el de abril hay horas que ocurren dos veces.
fn a_utc(naive: NaiveDateTime, zona: Tz) -> (DateTime<Utc>, Option<&'static str>) {
    use chrono::offset::LocalResult;
    match zona.from_local_datetime(&naive) {
        LocalResult::Single(dt) => (dt.with_timezone(&Utc), None),
        // Hora repetida (vuelta a horario de invierno): se toma la primera ocurrencia.
        LocalResult::Ambiguous(primera, _) => {
            (primera.with_timezone(&Utc), Some("hora ambigua, se tomó la primera ocurrencia"))
        }
        // Hora inexistente (adelanto de horario de verano): se corre una hora.
        LocalResult::None => {
            let corrida = naive + Duration::hours(1);
            match zona.from_local_datetime(&corrida) {
                LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => {
                    (dt.with_timezone(&Utc), Some("hora inexistente por el cambio de hora, se corrió 1 hora"))
                }
                LocalResult::None => (Utc.from_utc_datetime(&naive), Some("no se pudo ubicar en la zona, se tomó como UTC")),
            }
        }
    }
}

fn parsear_naive(s: &str) -> Result<NaiveDateTime> {
    for formato in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, formato) {
            return Ok(dt);
        }
    }
    bail!("no se pudo interpretar la fecha '{s}'")
}

async fn cargar(
    tx: &mut Transaction<'_, Postgres>,
    exp: &Exportacion,
    op: &Opciones,
    plantillas_dir: &Path,
) -> Result<()> {
    // ---- Comprobaciones previas -------------------------------------------------
    let ids_categoria: std::collections::HashSet<i32> = exp.categorias.iter().map(|c| c.id).collect();
    for p in &exp.plantillas {
        if !ids_categoria.contains(&p.categoria_id) {
            bail!("la plantilla {} apunta a la categoría {} que no está en el export", p.id, p.categoria_id);
        }
    }
    for c in &exp.credenciales {
        if !ids_categoria.contains(&c.categoria_id) {
            bail!("la credencial {} apunta a la categoría {} que no está en el export", c.id, c.categoria_id);
        }
    }

    // Plantillas de v2 que todavía no tienen equivalente `.typ` en v3.
    let mut sin_typ = Vec::new();
    for p in &exp.plantillas {
        let ruta = ruta_v3(&p.ruta);
        if !plantillas_dir.join(&ruta).is_file() {
            sin_typ.push(format!("  - id {} '{}' -> {ruta}", p.id, p.nombre));
        }
    }
    if !sin_typ.is_empty() {
        println!(
            "AVISO: {} plantilla(s) de v2 no tienen archivo .typ en {}. Se cargan igual (para no \n\
             romper los ids que usan los consumidores), pero generarán 500 hasta que se migren:\n{}",
            sin_typ.len(),
            plantillas_dir.display(),
            sin_typ.join("\n")
        );
    }

    // Choque de ids entre la semilla de v3 y los datos de v2.
    if !op.limpiar_catalogos {
        let choques: Vec<(i32, String, String)> = sqlx::query_as(
            "SELECT c.id, c.nombre, x.nombre FROM categoria c \
             JOIN UNNEST($1::int[], $2::text[]) AS x(id, nombre) ON x.id = c.id \
             WHERE c.nombre <> x.nombre",
        )
        .bind(exp.categorias.iter().map(|c| c.id).collect::<Vec<_>>())
        .bind(exp.categorias.iter().map(|c| c.nombre.clone()).collect::<Vec<_>>())
        .fetch_all(&mut **tx)
        .await?;
        if !choques.is_empty() {
            let detalle: Vec<String> = choques
                .iter()
                .map(|(id, actual, nuevo)| format!("  - id {id}: '{actual}' (destino) vs '{nuevo}' (v2)"))
                .collect();
            bail!(
                "el destino ya tiene categorías con esos ids y otro nombre (son las de la semilla).\n{}\n\
                 Use --limpiar-catalogos para reemplazar los catálogos por los de v2.",
                detalle.join("\n")
            );
        }
    }

    // ---- Limpieza opcional de la semilla ----------------------------------------
    if op.limpiar_catalogos {
        let plantillas_borradas = sqlx::query("DELETE FROM plantilla").execute(&mut **tx).await?.rows_affected();
        // Sólo se pueden borrar categorías sin credenciales: si hay, es que el destino
        // ya tiene datos reales y no corresponde limpiarlo.
        let categorias_borradas = sqlx::query(
            "DELETE FROM categoria c WHERE NOT EXISTS (SELECT 1 FROM credencial cr WHERE cr.categoria_id = c.id)",
        )
        .execute(&mut **tx)
        .await?
        .rows_affected();
        println!("limpieza: {plantillas_borradas} plantillas y {categorias_borradas} categorías eliminadas del destino");
    }

    // ---- Categorías --------------------------------------------------------------
    for c in &exp.categorias {
        sqlx::query("INSERT INTO categoria (id, nombre) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET nombre = EXCLUDED.nombre")
            .bind(c.id)
            .bind(&c.nombre)
            .execute(&mut **tx)
            .await
            .with_context(|| format!("categoría {}", c.id))?;
    }

    // ---- Plantillas --------------------------------------------------------------
    for p in &exp.plantillas {
        sqlx::query(
            "INSERT INTO plantilla (id, nombre, ruta, categoria_id, motor) VALUES ($1, $2, $3, $4, 'typst') \
             ON CONFLICT (id) DO UPDATE SET nombre = EXCLUDED.nombre, ruta = EXCLUDED.ruta, categoria_id = EXCLUDED.categoria_id",
        )
        .bind(p.id)
        .bind(&p.nombre)
        .bind(ruta_v3(&p.ruta))
        .bind(p.categoria_id)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("plantilla {}", p.id))?;
    }

    // ---- Credenciales ------------------------------------------------------------
    let mut avisos_fecha = Vec::new();
    let mut nuevas = 0u64;
    for c in &exp.credenciales {
        let uuid = Uuid::parse_str(&c.id).with_context(|| format!("uuid inválido: {}", c.id))?;
        let fecha = match &c.fecha_creacion {
            Some(s) => {
                let naive = parsear_naive(s)?;
                let (dt, aviso) = a_utc(naive, op.zona);
                if let Some(a) = aviso {
                    avisos_fecha.push(format!("  - {} ({s}): {a}", c.id));
                }
                dt
            }
            None => Utc::now(),
        };
        let r = sqlx::query(
            "INSERT INTO credencial (id, categoria_id, url, fecha_creacion) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING",
        )
        .bind(uuid)
        .bind(c.categoria_id)
        .bind(&c.url)
        .bind(fecha)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("credencial {}", c.id))?;
        nuevas += r.rows_affected();
    }
    if !avisos_fecha.is_empty() {
        println!("AVISO: {} fecha(s) en el borde del cambio de hora:\n{}", avisos_fecha.len(), avisos_fecha.join("\n"));
    }

    // ---- Secuencias --------------------------------------------------------------
    // Tras insertar ids explícitos, el contador de las columnas serial queda atrás y el
    // siguiente INSERT normal chocaría con una clave existente.
    for tabla in ["categoria", "plantilla"] {
        sqlx::query(&format!(
            "SELECT setval(pg_get_serial_sequence('{tabla}', 'id'), GREATEST((SELECT COALESCE(MAX(id), 0) FROM {tabla}), 1))"
        ))
        .execute(&mut **tx)
        .await?;
    }

    println!(
        "cargados: {} categorías, {} plantillas, {} credenciales nuevas ({} ya estaban)",
        exp.categorias.len(),
        exp.plantillas.len(),
        nuevas,
        exp.credenciales.len() as u64 - nuevas
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let op = leer_opciones()?;

    let contenido = std::fs::read_to_string(&op.archivo)
        .with_context(|| format!("no se pudo leer {}", op.archivo))?;
    let exp: Exportacion = serde_json::from_str(&contenido)
        .with_context(|| format!("{} no tiene el formato que produce exportar-v2.mjs", op.archivo))?;

    let cfg = Config::cargar().await?;
    println!(
        "origen: {}\ndestino: {} (ambiente {})\nzona de las fechas de v2: {}{}",
        exp.origen,
        cfg.destino_db(),
        cfg.entorno,
        op.zona,
        if op.dry_run { "\nmodo: dry-run (no se confirma nada)" } else { "" }
    );

    let pool = PgPoolOptions::new().max_connections(2).connect_with(cfg.opciones_pg()?).await?;
    let mut tx = pool.begin().await?;

    let resultado = cargar(&mut tx, &exp, &op, &cfg.plantillas_dir).await;

    match resultado {
        Ok(()) if op.dry_run => {
            tx.rollback().await?;
            println!("dry-run: transacción deshecha, el destino quedó igual que antes");
        }
        Ok(()) => {
            tx.commit().await?;
            let (cat, pla, cre): (i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM categoria), (SELECT count(*) FROM plantilla), (SELECT count(*) FROM credencial)",
            )
            .fetch_one(&pool)
            .await?;
            println!("destino tras la migración: {cat} categorías, {pla} plantillas, {cre} credenciales");
        }
        Err(e) => {
            tx.rollback().await?;
            return Err(e.context("la migración se deshizo por completo; el destino quedó intacto"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapea_la_ruta_del_html_al_typ() {
        assert_eq!(ruta_v3("mascotas/datos-cze.html"), "mascotas/datos-cze.typ");
        assert_eq!(
            ruta_v3("expendios-veterinarios/ninguno/certificado-expendio-veterinario.html"),
            "expendios-veterinarios/ninguno/certificado-expendio-veterinario.typ"
        );
        // Si ya viene sin extensión conocida, se respeta.
        assert_eq!(ruta_v3("algo/raro.txt"), "algo/raro.txt");
    }

    #[test]
    fn convierte_hora_de_pared_de_santiago_a_utc() {
        let zona: Tz = "America/Santiago".parse().unwrap();

        // Invierno (UTC-4): 10:15 local son las 14:15 UTC.
        let (dt, aviso) = a_utc(parsear_naive("2023-04-24T10:15:30.123").unwrap(), zona);
        assert_eq!(dt.to_rfc3339(), "2023-04-24T14:15:30.123+00:00");
        assert!(aviso.is_none());

        // Verano (UTC-3): 23:59 local del 2 de noviembre son las 02:59 UTC del 3.
        let (dt, _) = a_utc(parsear_naive("2024-11-02T23:59:59.997").unwrap(), zona);
        assert_eq!(dt.to_rfc3339(), "2024-11-03T02:59:59.997+00:00");
    }

    #[test]
    fn resuelve_las_horas_del_cambio_de_hora() {
        let zona: Tz = "America/Santiago".parse().unwrap();

        // Adelanto de septiembre: las 00:30 del 6/9/2026 no existen en Santiago.
        let (dt, aviso) = a_utc(parsear_naive("2026-09-06T00:30:00.000").unwrap(), zona);
        assert!(aviso.unwrap().contains("inexistente"));
        assert_eq!(dt.to_rfc3339(), "2026-09-06T04:30:00+00:00");

        // Atraso de abril: las 23:30 del 4/4/2026 ocurren dos veces; se toma la primera.
        let (dt, aviso) = a_utc(parsear_naive("2026-04-04T23:30:00.000").unwrap(), zona);
        assert!(aviso.unwrap().contains("ambigua"));
        assert_eq!(dt.to_rfc3339(), "2026-04-05T02:30:00+00:00");
    }

    #[test]
    fn acepta_los_formatos_de_fecha_del_export() {
        assert!(parsear_naive("2023-04-24T10:15:30.123").is_ok());
        assert!(parsear_naive("2023-04-24 10:15:30").is_ok());
        assert!(parsear_naive("no es una fecha").is_err());
    }
}
