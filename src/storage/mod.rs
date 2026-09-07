//! Puerto de almacenamiento de documentos. v2 sube a Azure Blob Storage con
//! contenedor = categoría (snake_case) y nombre = `<plantilla>_<fecha>_<uuid>`.
//! v3 conserva esa convención; el adaptador local sirve los archivos por HTTP
//! para que la URL devuelta funcione en desarrollo.

pub mod local;

use futures::future::BoxFuture;

#[derive(Debug, thiserror::Error)]
#[error("storage: {0}")]
pub struct StorageError(pub String);

pub trait Storage: Send + Sync + 'static {
    fn nombre(&self) -> &'static str;
    /// URL pública que tendrá el documento. Se puede calcular antes de guardarlo
    /// (v2 la necesita anticipada para incrustarla en el QR).
    fn url_publica(&self, contenedor: &str, nombre: &str) -> String;
    fn guardar<'a>(&'a self, contenedor: &'a str, nombre: &'a str, bytes: Vec<u8>) -> BoxFuture<'a, Result<(), StorageError>>;
}
