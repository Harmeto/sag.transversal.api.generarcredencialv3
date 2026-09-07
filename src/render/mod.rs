//! Puerto de render: convierte (plantilla, datos) en bytes de PDF.
//!
//! El motor es intercambiable. v3 nace con Typst (`typst_renderer`); un adaptador
//! Chromium podría convivir para migrar plantilla a plantilla, eligiendo el motor
//! por la columna `plantilla.motor`.

pub mod typst_renderer;
mod valores;

use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("plantilla no disponible para el motor: {0}")]
    PlantillaNoDisponible(String),
    #[error("error de compilación de la plantilla: {0}")]
    Compilacion(String),
    #[error("error al exportar el PDF: {0}")]
    Exportacion(String),
}

/// Render síncrono y CPU-bound. Se ejecuta fuera del runtime async (ver [`RenderPool`]).
pub trait Renderer: Send + Sync + 'static {
    fn motor(&self) -> &'static str;
    fn renderizar(&self, ruta_plantilla: &str, datos: &serde_json::Value) -> Result<Vec<u8>, RenderError>;
}

/// Ejecuta renders en hilos bloqueantes con concurrencia acotada. A diferencia de v2,
/// un pico de tráfico encola en vez de abrir N pestañas de Chromium: la memoria tiene
/// techo y el throughput no se degrada por contención de CPU.
#[derive(Clone)]
pub struct RenderPool {
    renderer: Arc<dyn Renderer>,
    permisos: Arc<Semaphore>,
}

impl RenderPool {
    pub fn nuevo(renderer: Arc<dyn Renderer>, concurrencia: usize) -> Self {
        Self { renderer, permisos: Arc::new(Semaphore::new(concurrencia.max(1))) }
    }

    pub fn motor(&self) -> &'static str {
        self.renderer.motor()
    }

    pub fn disponibles(&self) -> usize {
        self.permisos.available_permits()
    }

    pub async fn renderizar(&self, ruta: String, datos: serde_json::Value) -> Result<Vec<u8>, RenderError> {
        let _permiso = self
            .permisos
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| RenderError::Compilacion(e.to_string()))?;
        let renderer = self.renderer.clone();
        tokio::task::spawn_blocking(move || renderer.renderizar(&ruta, &datos))
            .await
            .map_err(|e| RenderError::Compilacion(format!("la tarea de render falló: {e}")))?
    }
}
