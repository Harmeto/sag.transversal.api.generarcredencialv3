use super::{Storage, StorageError};
use futures::future::BoxFuture;
use std::path::PathBuf;

/// Escribe en disco (`<raiz>/<contenedor>/<nombre>`) y publica bajo `<url_base>/<contenedor>/<nombre>`.
pub struct LocalStorage {
    raiz: PathBuf,
    url_base: String,
}

impl LocalStorage {
    pub fn nuevo(raiz: PathBuf, url_base: String) -> Self {
        Self { raiz, url_base }
    }

    pub fn ruta(&self, contenedor: &str, nombre: &str) -> Option<PathBuf> {
        // Los nombres los genera la propia API; igual se rechaza cualquier intento de salir de la raíz.
        if [contenedor, nombre].iter().any(|s| s.is_empty() || s.contains('/') || s.contains("..")) {
            return None;
        }
        Some(self.raiz.join(contenedor).join(nombre))
    }
}

impl Storage for LocalStorage {
    fn nombre(&self) -> &'static str {
        "local"
    }

    fn url_publica(&self, contenedor: &str, nombre: &str) -> String {
        format!("{}/{contenedor}/{nombre}", self.url_base)
    }

    fn guardar<'a>(&'a self, contenedor: &'a str, nombre: &'a str, bytes: Vec<u8>) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let ruta = self.ruta(contenedor, nombre).ok_or_else(|| StorageError("nombre inválido".into()))?;
            if let Some(dir) = ruta.parent() {
                tokio::fs::create_dir_all(dir).await.map_err(|e| StorageError(e.to_string()))?;
            }
            tokio::fs::write(&ruta, bytes).await.map_err(|e| StorageError(e.to_string()))
        })
    }
}
