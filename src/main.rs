use anyhow::Context;
use sag_transversal_api_generarcredencialv3::config::Config;
use sag_transversal_api_generarcredencialv3::http::{router, AppState, PREFIJO};
use sag_transversal_api_generarcredencialv3::render::typst_renderer::TypstRenderer;
use sag_transversal_api_generarcredencialv3::render::RenderPool;
use sag_transversal_api_generarcredencialv3::services::generar_credencial::GeneradorCredencial;
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
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_env("LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = Config::desde_entorno()?;

    // Base de datos + migraciones (idempotentes).
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await
        .context("no se pudo conectar a PostgreSQL")?;
    sqlx::migrate!("./migrations").run(&pool).await.context("fallaron las migraciones")?;
    tracing::info!("migraciones aplicadas");

    // Motor de render: se construye una vez y se precalienta.
    let renderer = TypstRenderer::nuevo(&cfg.plantillas_dir, cfg.fonts_dir.as_deref())?;
    let rutas: Vec<String> = sqlx::query_scalar("SELECT ruta FROM plantilla WHERE motor = 'typst'").fetch_all(&pool).await?;
    for ruta in &rutas {
        let t = Instant::now();
        match renderer.precalentar(ruta) {
            Ok(()) => tracing::info!(plantilla = %ruta, ms = t.elapsed().as_millis() as u64, "plantilla precalentada"),
            Err(e) => tracing::warn!(plantilla = %ruta, error = %e, "no se pudo precalentar la plantilla"),
        }
    }
    let render = RenderPool::nuevo(Arc::new(renderer), cfg.render_concurrency);

    // Storage: adaptador local (Azure Blob queda como siguiente adaptador, ver README).
    let local = Arc::new(LocalStorage::nuevo(cfg.storage_dir.clone(), cfg.storage_public_url.clone()));
    let storage: Arc<dyn Storage> = local.clone();

    let generador = GeneradorCredencial { pool: pool.clone(), render: render.clone(), storage };
    let state = AppState {
        app_name: cfg.app_name.clone(),
        pool,
        render,
        generador,
        storage_local: Some(local),
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
