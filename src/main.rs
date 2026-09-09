use anyhow::Context;
use sag_transversal_api_generarcredencialv3::config::{Config, OrigenStorage};
use sag_transversal_api_generarcredencialv3::http::{router, AppState, PREFIJO};
use sag_transversal_api_generarcredencialv3::render::typst_renderer::TypstRenderer;
use sag_transversal_api_generarcredencialv3::render::RenderPool;
use sag_transversal_api_generarcredencialv3::services::generar_credencial::GeneradorCredencial;
use sag_transversal_api_generarcredencialv3::storage::azure::AzureBlobStorage;
use sag_transversal_api_generarcredencialv3::storage::local::LocalStorage;
use sag_transversal_api_generarcredencialv3::storage::Storage;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Instant;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let inicio = Instant::now();
    dotenvy::dotenv().ok();

    // La configuración se carga ANTES de inicializar los logs porque el nivel de
    // log vive en Consul (`logLevel`). Hasta entonces, los mensajes van a stderr.
    let cfg = Config::cargar().await?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new(&cfg.log_level).unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    tracing::info!(
        app = %cfg.app_name,
        entorno = %cfg.entorno,
        db = %cfg.destino_db(),
        "configuración cargada"
    );

    // Base de datos + migraciones (idempotentes).
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect_with(cfg.opciones_pg()?)
        .await
        .context("no se pudo conectar a PostgreSQL")?;
    sqlx::migrate!("./migrations").run(&pool).await.context("fallaron las migraciones")?;
    tracing::info!("migraciones aplicadas");

    // Motor de render: se construye una vez y se precalienta.
    let renderer = TypstRenderer::nuevo(&cfg.plantillas_dir, cfg.fonts_dir.as_deref())?;
    let rutas: Vec<String> =
        sqlx::query_scalar("SELECT ruta FROM plantilla WHERE motor = 'typst'").fetch_all(&pool).await?;
    for ruta in &rutas {
        let t = Instant::now();
        match renderer.precalentar(ruta) {
            Ok(()) => tracing::info!(plantilla = %ruta, ms = t.elapsed().as_millis() as u64, "plantilla precalentada"),
            Err(e) => tracing::warn!(plantilla = %ruta, error = %e, "no se pudo precalentar la plantilla"),
        }
    }
    let render = RenderPool::nuevo(Arc::new(renderer), cfg.render_concurrency);

    // Storage de documentos.
    let storage_local;
    let storage: Arc<dyn Storage> = match &cfg.storage {
        OrigenStorage::Local { dir, url_base } => {
            if !Config::es_local(&cfg.entorno) {
                tracing::warn!(
                    "storage en disco del pod: sólo es válido con una réplica. \
                     Configure 'blobStorageConfigValue' en Consul para usar Azure Blob"
                );
            }
            let local = Arc::new(LocalStorage::nuevo(dir.clone(), url_base.clone()));
            storage_local = Some(local.clone());
            local
        }
        OrigenStorage::Azure { connection_string, account_name } => {
            storage_local = None;
            Arc::new(AzureBlobStorage::nuevo(connection_string, account_name)?)
        }
    };
    tracing::info!(storage = storage.nombre(), "storage configurado");

    let generador = GeneradorCredencial { pool: pool.clone(), render: render.clone(), storage };
    let state = AppState {
        app_name: cfg.app_name.clone(),
        pool,
        render,
        generador,
        storage_local,
        inicio,
    };

    let direccion = format!("{}:{}", cfg.host, cfg.port);
    let listener = tokio::net::TcpListener::bind(&direccion).await.context("no se pudo abrir el puerto")?;
    tracing::info!(
        app = %cfg.app_name,
        %direccion,
        prefijo = PREFIJO,
        concurrencia_render = cfg.render_concurrency,
        "listening"
    );

    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("apagando");
        })
        .await?;
    Ok(())
}
