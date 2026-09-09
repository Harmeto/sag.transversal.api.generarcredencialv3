//! Adaptador de Azure Blob Storage (equivalente a `CredencialService.store` de v2).
//!
//! Firma las peticiones con Shared Key a mano (`hmac` + `sha2` + `base64` sobre
//! `reqwest`) en vez de usar el SDK de Azure, que arrastra un árbol de
//! dependencias enorme para las tres operaciones que necesitamos: comprobar si el
//! contenedor existe, crearlo con acceso público de lectura y subir el PDF.
//!
//! La URL pública se mantiene byte a byte igual a la de v2
//! (`https://{cuenta}.blob.core.windows.net/{contenedor}/{documento}`, sin
//! extensión en el nombre del blob) porque esas URLs ya están impresas en los
//! códigos QR de credenciales emitidas.

use super::{Storage, StorageError};
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use futures::future::BoxFuture;
use hmac::{Hmac, Mac};
use reqwest::{Method, Response, StatusCode};
use sha2::Sha256;
use std::time::Duration;

/// Versión del API REST de Azure Storage que se declara en `x-ms-version`.
const VERSION_API: &str = "2021-08-06";
const TIPO_PDF: &str = "application/pdf";
const ESPERA_REINTENTO: Duration = Duration::from_millis(300);

// ---------------------------------------------------------------------------
// Connection string
// ---------------------------------------------------------------------------

/// Datos que se extraen del connection string de Azure Storage.
struct ConexionBlob {
    /// `AccountName`: identifica la cuenta al firmar.
    cuenta: String,
    /// `AccountKey` ya decodificada de base64.
    clave: Vec<u8>,
    /// Endpoint base sin `/` final: `BlobEndpoint` si viene (es lo que usa
    /// Azurite), si no `{protocolo}://{cuenta}.blob.{sufijo}`.
    endpoint: String,
}

// Debug manual: la AccountKey no puede terminar en un log.
impl std::fmt::Debug for ConexionBlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConexionBlob")
            .field("cuenta", &self.cuenta)
            .field("clave", &"***")
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

impl ConexionBlob {
    fn parsear(connection_string: &str) -> Result<Self> {
        let mut cuenta = None;
        let mut clave_b64 = None;
        let mut protocolo = None;
        let mut sufijo = None;
        let mut blob_endpoint = None;

        for parte in connection_string.split(';') {
            let parte = parte.trim();
            if parte.is_empty() {
                continue;
            }
            // La AccountKey es base64 y lleva '=' de relleno: se parte solo en el primero.
            let Some((clave_kv, valor)) = parte.split_once('=') else { continue };
            let valor = valor.trim().to_string();
            match clave_kv.trim() {
                k if k.eq_ignore_ascii_case("AccountName") => cuenta = Some(valor),
                k if k.eq_ignore_ascii_case("AccountKey") => clave_b64 = Some(valor),
                k if k.eq_ignore_ascii_case("DefaultEndpointsProtocol") => protocolo = Some(valor),
                k if k.eq_ignore_ascii_case("EndpointSuffix") => sufijo = Some(valor),
                k if k.eq_ignore_ascii_case("BlobEndpoint") => blob_endpoint = Some(valor),
                _ => {}
            }
        }

        let cuenta = cuenta.filter(|c| !c.is_empty()).ok_or_else(|| anyhow!("el connection string no trae AccountName"))?;
        let clave_b64 = clave_b64.filter(|c| !c.is_empty()).ok_or_else(|| anyhow!("el connection string no trae AccountKey"))?;
        let clave = B64.decode(clave_b64.as_bytes()).context("la AccountKey del connection string no es base64 válido")?;

        // BlobEndpoint manda sobre protocolo/sufijo (así se conecta a Azurite).
        let endpoint = match blob_endpoint.filter(|e| !e.is_empty()) {
            Some(e) => e,
            None => {
                let protocolo = protocolo.filter(|p| !p.is_empty()).unwrap_or_else(|| "https".to_string());
                let sufijo = sufijo.filter(|s| !s.is_empty()).unwrap_or_else(|| "core.windows.net".to_string());
                format!("{protocolo}://{cuenta}.blob.{sufijo}")
            }
        };

        Ok(Self { cuenta, clave, endpoint: endpoint.trim_end_matches('/').to_string() })
    }
}

// ---------------------------------------------------------------------------
// Firma Shared Key
// ---------------------------------------------------------------------------

/// `CanonicalizedResource` (esquema completo, el que exige Shared Key):
/// `/{cuenta}{ruta}` y, por cada parámetro de query ordenado, `\n{nombre}:{valor}`.
///
/// `ruta` es el path de la URL tal como viaja (con `/` inicial). Contra Azurite el
/// path incluye la cuenta (`/devstoreaccount1/cont/blob`), así que el recurso
/// canónico la repite: es lo mismo que hace el SDK oficial.
fn recurso_canonico(cuenta: &str, ruta: &str, consulta: &[(&str, &str)]) -> String {
    let mut out = format!("/{cuenta}{ruta}");
    let mut params: Vec<(String, &str)> = consulta.iter().map(|(k, v)| (k.to_ascii_lowercase(), *v)).collect();
    params.sort_by(|a, b| a.0.cmp(&b.0));
    for (k, v) in params {
        out.push('\n');
        out.push_str(&k);
        out.push(':');
        out.push_str(v);
    }
    out
}

/// `CanonicalizedHeaders`: cabeceras `x-ms-*` en minúsculas, ordenadas por nombre,
/// con el valor recortado y una línea por cabecera.
fn cabeceras_canonicas(headers_ms: &[(String, String)]) -> String {
    let mut hs: Vec<(String, String)> =
        headers_ms.iter().map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_string())).collect();
    hs.sort_by(|a, b| a.0.cmp(&b.0));
    hs.iter().map(|(k, v)| format!("{k}:{v}\n")).collect()
}

/// Cadena a firmar de Shared Key para Blob Storage. Solo se rellenan los campos
/// que usamos; el resto van vacíos. `Content-Length` va vacío cuando es 0.
fn cadena_a_firmar(
    verbo: &str,
    largo: usize,
    tipo_contenido: &str,
    headers_ms: &[(String, String)],
    recurso: &str,
) -> String {
    let largo = if largo == 0 { String::new() } else { largo.to_string() };
    format!(
        // VERB, Content-Encoding, Content-Language, Content-Length, Content-MD5,
        // Content-Type, Date, If-Modified-Since, If-Match, If-None-Match,
        // If-Unmodified-Since, Range, CanonicalizedHeaders, CanonicalizedResource
        "{verbo}\n\n\n{largo}\n\n{tipo_contenido}\n\n\n\n\n\n\n{}{recurso}",
        cabeceras_canonicas(headers_ms)
    )
}

/// `base64(HMAC-SHA256(clave, cadena))`.
fn firmar(clave: &[u8], cadena: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(clave).expect("HMAC-SHA256 acepta claves de cualquier largo");
    mac.update(cadena.as_bytes());
    B64.encode(mac.finalize().into_bytes())
}

/// Fecha en RFC 1123 GMT, el formato que exige `x-ms-date`.
fn fecha_rfc1123(ahora: chrono::DateTime<chrono::Utc>) -> String {
    ahora.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

/// Percent-encoding de un segmento de path (deja intactos los caracteres no
/// reservados). Los nombres que genera la API son `[a-z0-9_-]`, así que en la
/// práctica es la identidad; está por seguridad ante nombres inesperados.
fn codificar_segmento(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Adaptador
// ---------------------------------------------------------------------------

/// Guarda los PDF en Azure Blob Storage: contenedor por categoría (creado con
/// acceso público de lectura si no existe) y blob `application/pdf`.
/// Métodos en los que Azure exige la cabecera `Content-Length` aunque el cuerpo esté
/// vacío. La firma Shared Key, en cambio, lleva el largo vacío cuando vale 0.
fn exige_content_length(metodo: &Method) -> bool {
    matches!(*metodo, Method::PUT | Method::POST)
}

pub struct AzureBlobStorage {
    cliente: reqwest::Client,
    /// Cuenta con la que se firma (la del connection string).
    cuenta: String,
    /// Cuenta que aparece en la URL pública.
    cuenta_publica: String,
    clave: Vec<u8>,
    endpoint: String,
}

impl std::fmt::Debug for AzureBlobStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzureBlobStorage")
            .field("cuenta", &self.cuenta)
            .field("cuenta_publica", &self.cuenta_publica)
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

/// Petición ya resuelta, lista para firmarse en cada intento.
struct Peticion {
    metodo: Method,
    /// Path con `/` inicial, ya codificado.
    ruta: String,
    consulta: Vec<(&'static str, &'static str)>,
    /// Cabeceras `x-ms-*` propias de la operación (sin `x-ms-date`/`x-ms-version`).
    headers_ms: Vec<(String, String)>,
    tipo_contenido: &'static str,
    cuerpo: Option<Vec<u8>>,
}

impl AzureBlobStorage {
    /// `account_name` vacío significa "usa el AccountName del connection string".
    pub fn nuevo(connection_string: &str, account_name: &str) -> Result<Self> {
        let conexion = ConexionBlob::parsear(connection_string)?;
        let cuenta_publica =
            if account_name.trim().is_empty() { conexion.cuenta.clone() } else { account_name.trim().to_string() };
        let cliente = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .context("no se pudo construir el cliente HTTP para Azure Blob")?;
        Ok(Self {
            cliente,
            cuenta: conexion.cuenta,
            cuenta_publica,
            clave: conexion.clave,
            endpoint: conexion.endpoint,
        })
    }

    fn ruta(&self, contenedor: &str, nombre: Option<&str>) -> Result<String, StorageError> {
        let invalido = |s: &str| s.is_empty() || s.contains('/') || s.contains("..");
        if invalido(contenedor) || nombre.is_some_and(invalido) {
            return Err(StorageError("nombre de contenedor o documento inválido".into()));
        }
        Ok(match nombre {
            Some(n) => format!("/{}/{}", codificar_segmento(contenedor), codificar_segmento(n)),
            None => format!("/{}", codificar_segmento(contenedor)),
        })
    }

    /// Path completo tal como viaja en la URL: incluye el prefijo del endpoint
    /// (Azurite mete la cuenta en el path; Azure real no).
    fn path_url(&self, ruta: &str) -> String {
        let prefijo = self
            .endpoint
            .split_once("://")
            .map(|(_, resto)| resto)
            .and_then(|resto| resto.find('/').map(|i| &resto[i..]))
            .unwrap_or("");
        format!("{prefijo}{ruta}")
    }

    /// Manda la petición firmándola en el momento (`x-ms-date` se recalcula en
    /// cada intento, por eso la firma se arma aquí y no fuera).
    async fn enviar(&self, p: &Peticion) -> reqwest::Result<Response> {
        let mut headers_ms = p.headers_ms.clone();
        headers_ms.push(("x-ms-date".to_string(), fecha_rfc1123(chrono::Utc::now())));
        headers_ms.push(("x-ms-version".to_string(), VERSION_API.to_string()));

        let largo = p.cuerpo.as_ref().map(|c| c.len()).unwrap_or(0);
        let recurso = recurso_canonico(&self.cuenta, &self.path_url(&p.ruta), &p.consulta);
        let cadena = cadena_a_firmar(p.metodo.as_str(), largo, p.tipo_contenido, &headers_ms, &recurso);
        let autorizacion = format!("SharedKey {}:{}", self.cuenta, firmar(&self.clave, &cadena));

        let url = format!("{}{}", self.endpoint, p.ruta);
        let mut req = self.cliente.request(p.metodo.clone(), url).query(&p.consulta);
        for (k, v) in &headers_ms {
            req = req.header(k.as_str(), v.as_str());
        }
        if !p.tipo_contenido.is_empty() {
            req = req.header(reqwest::header::CONTENT_TYPE, p.tipo_contenido);
        }
        req = req.header(reqwest::header::AUTHORIZATION, autorizacion);
        if let Some(cuerpo) = &p.cuerpo {
            req = req.header(reqwest::header::CONTENT_LENGTH, cuerpo.len());
            req = req.body(cuerpo.clone());
        } else if exige_content_length(&p.metodo) {
            // Azure responde 411 a un PUT sin `Content-Length`, aunque no lleve cuerpo
            // (es el caso de crear el contenedor). Azurite lo acepta igual, así que este
            // fallo sólo aparece contra el servicio real.
            req = req.header(reqwest::header::CONTENT_LENGTH, 0);
            req = req.body(Vec::new());
        }
        req.send().await
    }

    /// Un reintento ante error de red o 5xx, con espera corta. Nada más.
    async fn enviar_con_reintento(&self, p: &Peticion, operacion: &str) -> Result<Response, StorageError> {
        match self.enviar(p).await {
            Ok(r) if !r.status().is_server_error() => Ok(r),
            resultado => {
                let motivo = match &resultado {
                    Ok(r) => format!("HTTP {}", r.status().as_u16()),
                    // El error de reqwest no incluye cabeceras: no filtra la firma.
                    Err(e) => e.to_string(),
                };
                tracing::warn!(operacion, motivo, "azure blob: reintentando una vez");
                tokio::time::sleep(ESPERA_REINTENTO).await;
                match self.enviar(p).await {
                    Ok(r) => Ok(r),
                    Err(e) => Err(StorageError(format!("azure blob: {operacion} falló: {e}"))),
                }
            }
        }
    }

    /// Mensaje de error con el código HTTP y el `x-ms-error-code` de Azure. No se
    /// incluye el cuerpo de la respuesta: ante `AuthenticationFailed` Azure hace
    /// eco de la firma recibida y no debe acabar en un log.
    fn error(operacion: &str, resp: &Response) -> StorageError {
        let codigo = resp.headers().get("x-ms-error-code").and_then(|v| v.to_str().ok()).unwrap_or("sin x-ms-error-code");
        StorageError(format!("azure blob: {operacion} devolvió HTTP {} ({codigo})", resp.status().as_u16()))
    }

    /// Cambia el nivel de acceso de un contenedor existente sin tocar su contenido.
    /// `Some("blob")` deja los blobs con lectura anónima; `None` lo deja privado.
    pub(crate) async fn fijar_acl(&self, contenedor: &str, acceso: Option<&str>) -> Result<(), StorageError> {
        let acl = Peticion {
            metodo: Method::PUT,
            ruta: self.ruta(contenedor, None)?,
            consulta: vec![("comp", "acl"), ("restype", "container")],
            headers_ms: match acceso {
                Some(v) => vec![("x-ms-blob-public-access".to_string(), v.to_string())],
                None => Vec::new(),
            },
            tipo_contenido: "",
            cuerpo: None,
        };
        let resp = self.enviar_con_reintento(&acl, "cambiar el acceso del contenedor").await?;
        if resp.status().is_success() {
            return Ok(());
        }
        Err(Self::error("cambiar el acceso del contenedor", &resp))
    }

    /// Nivel de acceso público del contenedor, para diagnóstico y pruebas.
    pub(crate) async fn acceso_publico(&self, contenedor: &str) -> Result<String, StorageError> {
        let p = Peticion {
            metodo: Method::HEAD,
            ruta: self.ruta(contenedor, None)?,
            consulta: vec![("restype", "container")],
            headers_ms: Vec::new(),
            tipo_contenido: "",
            cuerpo: None,
        };
        let resp = self.enviar_con_reintento(&p, "consultar contenedor").await?;
        if !resp.status().is_success() {
            return Err(Self::error("consultar contenedor", &resp));
        }
        Ok(resp
            .headers()
            .get("x-ms-blob-public-access")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("privado")
            .to_string())
    }

    /// Réplica del flujo de v2: si `?restype=container` da 404, se crea con
    /// `access: 'blob'`; un 409 `ContainerAlreadyExists` (otro pod se adelantó)
    /// también es éxito.
    async fn asegurar_contenedor(&self, contenedor: &str) -> Result<(), StorageError> {
        let ruta = self.ruta(contenedor, None)?;
        let existe = Peticion {
            metodo: Method::HEAD,
            ruta: ruta.clone(),
            consulta: vec![("restype", "container")],
            headers_ms: Vec::new(),
            tipo_contenido: "",
            cuerpo: None,
        };
        let resp = self.enviar_con_reintento(&existe, "consultar contenedor").await?;
        match resp.status() {
            s if s.is_success() => {
                // El contenedor ya existe. Se comprueba que tenga lectura pública: si no
                // la tiene, las URL que se guardan en base y se imprimen en los códigos QR
                // responden 404 a cualquiera que no traiga credenciales, y el problema no
                // se nota hasta que alguien intenta abrir su documento. Puede pasar con un
                // contenedor creado a mano o por una versión anterior del servicio.
                let acceso = resp
                    .headers()
                    .get("x-ms-blob-public-access")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                if acceso != "blob" && acceso != "container" {
                    // Si la corrección falla, se avisa y se sigue: el documento igual debe
                    // guardarse. Lo único afectado es la lectura anónima, que se puede
                    // arreglar después desde el portal.
                    if let Err(e) = self.fijar_acl(contenedor, Some("blob")).await {
                        tracing::warn!(contenedor, error = %e, "azure blob: no se pudo restablecer la lectura pública");
                    } else {
                        tracing::info!(contenedor, "azure blob: lectura pública restablecida");
                    }
                }
                return Ok(());
            }
            StatusCode::NOT_FOUND => {}
            _ => return Err(Self::error("consultar contenedor", &resp)),
        }

        let crear = Peticion {
            metodo: Method::PUT,
            ruta,
            consulta: vec![("restype", "container")],
            // Acceso público de lectura a los blobs (equivale a access: 'blob').
            headers_ms: vec![("x-ms-blob-public-access".to_string(), "blob".to_string())],
            tipo_contenido: "",
            cuerpo: None,
        };
        let resp = self.enviar_con_reintento(&crear, "crear contenedor").await?;
        if resp.status().is_success() {
            tracing::info!(contenedor, "azure blob: contenedor creado con acceso público de lectura");
            return Ok(());
        }
        let codigo = resp.headers().get("x-ms-error-code").and_then(|v| v.to_str().ok()).unwrap_or_default();
        if resp.status() == StatusCode::CONFLICT && (codigo.is_empty() || codigo == "ContainerAlreadyExists") {
            // Carrera con otra réplica: el contenedor ya existe, que es lo que queríamos.
            return Ok(());
        }
        Err(Self::error("crear contenedor", &resp))
    }

    async fn subir_pdf(&self, contenedor: &str, nombre: &str, bytes: Vec<u8>) -> Result<(), StorageError> {
        let peticion = Peticion {
            metodo: Method::PUT,
            ruta: self.ruta(contenedor, Some(nombre))?,
            consulta: Vec::new(),
            headers_ms: vec![("x-ms-blob-type".to_string(), "BlockBlob".to_string())],
            tipo_contenido: TIPO_PDF,
            cuerpo: Some(bytes),
        };
        let resp = self.enviar_con_reintento(&peticion, "subir blob").await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Self::error("subir blob", &resp))
        }
    }

    /// Descarga el blob y devuelve `(bytes, content-type)`. No forma parte del
    /// puerto `Storage`; sirve para verificar lo subido (lo usa el test contra
    /// Azurite) y para diagnósticos.
    pub async fn descargar(&self, contenedor: &str, nombre: &str) -> Result<(Vec<u8>, Option<String>), StorageError> {
        let peticion = Peticion {
            metodo: Method::GET,
            ruta: self.ruta(contenedor, Some(nombre))?,
            consulta: Vec::new(),
            headers_ms: Vec::new(),
            tipo_contenido: "",
            cuerpo: None,
        };
        let resp = self.enviar_con_reintento(&peticion, "descargar blob").await?;
        if !resp.status().is_success() {
            return Err(Self::error("descargar blob", &resp));
        }
        let tipo = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let bytes = resp.bytes().await.map_err(|e| StorageError(format!("azure blob: descargar blob falló: {e}")))?;
        Ok((bytes.to_vec(), tipo))
    }
}

impl Storage for AzureBlobStorage {
    fn nombre(&self) -> &'static str {
        "azure"
    }

    fn url_publica(&self, contenedor: &str, nombre: &str) -> String {
        // Formato exacto de v2: ya está impreso en los QR de credenciales emitidas.
        format!("https://{}.blob.core.windows.net/{contenedor}/{nombre}", self.cuenta_publica)
    }

    fn guardar<'a>(&'a self, contenedor: &'a str, nombre: &'a str, bytes: Vec<u8>) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            self.asegurar_contenedor(contenedor).await?;
            self.subir_pdf(contenedor, nombre, bytes).await
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn put_sin_cuerpo_igual_lleva_content_length() {
        // Regresión: crear el contenedor es un PUT sin cuerpo y Azure devuelve 411
        // si falta la cabecera. Azurite no lo exige, por eso no se detectó con él.
        use super::exige_content_length;
        assert!(exige_content_length(&reqwest::Method::PUT));
        assert!(exige_content_length(&reqwest::Method::POST));
        assert!(!exige_content_length(&reqwest::Method::GET));
        assert!(!exige_content_length(&reqwest::Method::HEAD));
    }

    use super::*;

    /// Cuenta y clave públicas de desarrollo de Azurite (no son un secreto).
    const CUENTA_DEV: &str = "devstoreaccount1";
    const CLAVE_DEV: &str = "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";

    fn conn_azurite() -> String {
        format!(
            "DefaultEndpointsProtocol=http;AccountName={CUENTA_DEV};AccountKey={CLAVE_DEV};BlobEndpoint=http://127.0.0.1:10000/{CUENTA_DEV};"
        )
    }

    #[test]
    fn parsea_connection_string_de_produccion() {
        let cs = "DefaultEndpointsProtocol=https;AccountName=sagcredenciales;AccountKey=a2V5MTIz;EndpointSuffix=core.windows.net";
        let c = ConexionBlob::parsear(cs).unwrap();
        assert_eq!(c.cuenta, "sagcredenciales");
        assert_eq!(c.endpoint, "https://sagcredenciales.blob.core.windows.net");
        assert_eq!(c.clave, b"key123");
    }

    #[test]
    fn blob_endpoint_manda_sobre_protocolo_y_sufijo() {
        let c = ConexionBlob::parsear(&conn_azurite()).unwrap();
        assert_eq!(c.cuenta, CUENTA_DEV);
        assert_eq!(c.endpoint, "http://127.0.0.1:10000/devstoreaccount1");
        assert_eq!(c.clave, B64.decode(CLAVE_DEV).unwrap());
    }

    #[test]
    fn endpoint_sin_barra_final_y_valores_por_defecto() {
        let c = ConexionBlob::parsear("AccountName=cta;AccountKey=a2V5MTIz;BlobEndpoint=https://cdn.ejemplo.cl/cta/").unwrap();
        assert_eq!(c.endpoint, "https://cdn.ejemplo.cl/cta");
        // Sin DefaultEndpointsProtocol ni EndpointSuffix se cae a https/core.windows.net.
        let c = ConexionBlob::parsear("AccountName=cta;AccountKey=a2V5MTIz").unwrap();
        assert_eq!(c.endpoint, "https://cta.blob.core.windows.net");
    }

    #[test]
    fn account_key_con_relleno_base64_no_se_corta() {
        // La clave lleva '=' de relleno: partir por el primer '=' es lo único correcto.
        let c = ConexionBlob::parsear(&conn_azurite()).unwrap();
        assert_eq!(c.clave.len(), 64);
    }

    #[test]
    fn connection_string_incompleto_falla() {
        assert!(ConexionBlob::parsear("AccountName=cta").is_err());
        assert!(ConexionBlob::parsear("AccountKey=a2V5MTIz").is_err());
        assert!(ConexionBlob::parsear("AccountName=cta;AccountKey=no-es-base64!!").is_err());
    }

    #[test]
    fn debug_no_filtra_la_clave() {
        let texto = format!("{:?}", ConexionBlob::parsear(&conn_azurite()).unwrap());
        assert!(texto.contains("***"));
        assert!(!texto.contains("Eby8vdM"));
    }

    #[test]
    fn cuenta_publica_cae_al_account_name_del_connection_string() {
        let a = AzureBlobStorage::nuevo(&conn_azurite(), "").unwrap();
        assert_eq!(a.url_publica("mosca_fruta", "doc"), "https://devstoreaccount1.blob.core.windows.net/mosca_fruta/doc");
        let b = AzureBlobStorage::nuevo(&conn_azurite(), "sagcredenciales").unwrap();
        assert_eq!(b.url_publica("mosca_fruta", "doc"), "https://sagcredenciales.blob.core.windows.net/mosca_fruta/doc");
    }

    #[test]
    fn recurso_canonico_ordena_query_y_baja_a_minusculas() {
        assert_eq!(recurso_canonico("cta", "/cont", &[("restype", "container")]), "/cta/cont\nrestype:container");
        assert_eq!(
            recurso_canonico("cta", "/cont/blob", &[("timeout", "30"), ("RESTYPE", "container")]),
            "/cta/cont/blob\nrestype:container\ntimeout:30"
        );
    }

    #[test]
    fn cabeceras_canonicas_ordenadas_en_minusculas_y_recortadas() {
        let hs = vec![
            ("X-Ms-Version".to_string(), " 2021-08-06 ".to_string()),
            ("x-ms-blob-type".to_string(), "BlockBlob".to_string()),
        ];
        assert_eq!(cabeceras_canonicas(&hs), "x-ms-blob-type:BlockBlob\nx-ms-version:2021-08-06\n");
    }

    #[test]
    fn content_length_va_vacio_cuando_es_cero() {
        let cadena = cadena_a_firmar("PUT", 0, "", &[], "/cta/cont");
        assert_eq!(cadena, "PUT\n\n\n\n\n\n\n\n\n\n\n\n/cta/cont");
        let cadena = cadena_a_firmar("PUT", 7, TIPO_PDF, &[], "/cta/cont/blob");
        assert_eq!(cadena, "PUT\n\n\n7\n\napplication/pdf\n\n\n\n\n\n\n/cta/cont/blob");
    }

    #[test]
    fn fecha_en_rfc1123_gmt() {
        let t = chrono::DateTime::parse_from_rfc3339("2026-09-08T12:00:00Z").unwrap().with_timezone(&chrono::Utc);
        assert_eq!(fecha_rfc1123(t), "Tue, 08 Sep 2026 12:00:00 GMT");
    }

    /// Firma completa de un PUT de blob. El valor esperado se calculó aparte con
    /// `hmac.new(key, s, sha256)` de Python sobre la misma cadena: si la firma se
    /// rompe, este test lo detecta sin necesidad de Azurite.
    #[test]
    fn firma_de_subida_de_blob() {
        let headers_ms = vec![
            ("x-ms-blob-type".to_string(), "BlockBlob".to_string()),
            ("x-ms-date".to_string(), "Tue, 08 Sep 2026 12:00:00 GMT".to_string()),
            ("x-ms-version".to_string(), VERSION_API.to_string()),
        ];
        let recurso = recurso_canonico(
            CUENTA_DEV,
            "/devstoreaccount1/mosca_fruta/mosca_2026-09-08_ea035152-0914-41b0-a616-00171980bbea",
            &[],
        );
        let cadena = cadena_a_firmar("PUT", 1024, TIPO_PDF, &headers_ms, &recurso);
        assert_eq!(
            cadena,
            "PUT\n\n\n1024\n\napplication/pdf\n\n\n\n\n\n\n\
             x-ms-blob-type:BlockBlob\nx-ms-date:Tue, 08 Sep 2026 12:00:00 GMT\nx-ms-version:2021-08-06\n\
             /devstoreaccount1/devstoreaccount1/mosca_fruta/mosca_2026-09-08_ea035152-0914-41b0-a616-00171980bbea"
        );
        let clave = B64.decode(CLAVE_DEV).unwrap();
        assert_eq!(firmar(&clave, &cadena), "OKgu9zqkOL9//SDBDeeQAyLjazzZ6vga8GjmWqvYfuM=");
    }

    /// Misma verificación para la creación del contenedor (query `restype` incluida).
    #[test]
    fn firma_de_creacion_de_contenedor() {
        let headers_ms = vec![
            ("x-ms-blob-public-access".to_string(), "blob".to_string()),
            ("x-ms-date".to_string(), "Tue, 08 Sep 2026 12:00:00 GMT".to_string()),
            ("x-ms-version".to_string(), VERSION_API.to_string()),
        ];
        let recurso = recurso_canonico(CUENTA_DEV, "/devstoreaccount1/mosca_fruta", &[("restype", "container")]);
        let cadena = cadena_a_firmar("PUT", 0, "", &headers_ms, &recurso);
        assert_eq!(
            cadena,
            "PUT\n\n\n\n\n\n\n\n\n\n\n\n\
             x-ms-blob-public-access:blob\nx-ms-date:Tue, 08 Sep 2026 12:00:00 GMT\nx-ms-version:2021-08-06\n\
             /devstoreaccount1/devstoreaccount1/mosca_fruta\nrestype:container"
        );
        let clave = B64.decode(CLAVE_DEV).unwrap();
        assert_eq!(firmar(&clave, &cadena), "mNbHLVJMsBaNnPT9C0IyI08I/Nhd6CsSOVW/N7pYLAc=");
    }

    #[test]
    fn path_url_repite_la_cuenta_solo_en_azurite() {
        let azurite = AzureBlobStorage::nuevo(&conn_azurite(), "").unwrap();
        assert_eq!(azurite.path_url("/cont/blob"), "/devstoreaccount1/cont/blob");
        let azure = AzureBlobStorage::nuevo("AccountName=cta;AccountKey=a2V5MTIz", "").unwrap();
        assert_eq!(azure.path_url("/cont/blob"), "/cont/blob");
    }

    #[test]
    fn rechaza_nombres_con_separadores() {
        let a = AzureBlobStorage::nuevo(&conn_azurite(), "").unwrap();
        assert!(a.ruta("cont", Some("../otro")).is_err());
        assert!(a.ruta("con/t", Some("doc")).is_err());
        assert!(a.ruta("", Some("doc")).is_err());
        assert_eq!(a.ruta("cont", Some("doc_1")).unwrap(), "/cont/doc_1");
    }

    #[test]
    fn codifica_caracteres_reservados() {
        assert_eq!(codificar_segmento("mosca_2026-09-08_ea03.5152~x"), "mosca_2026-09-08_ea03.5152~x");
        assert_eq!(codificar_segmento("a b?c#d"), "a%20b%3Fc%23d");
    }

    /// Verificación real contra Azurite. Requiere:
    ///   docker run -d -p 10000:10000 --name azurite-v3 \
    ///     mcr.microsoft.com/azure-storage/azurite azurite-blob --blobHost 0.0.0.0
    /// Un contenedor que quedó privado (creado a mano, por una versión anterior, o por
    /// un fallo puntual) hace que las URL impresas en los códigos QR respondan 404 sin
    /// que nadie se entere. El adaptador lo detecta al subir y lo corrige.
    #[tokio::test]
    #[ignore = "requiere Azurite en localhost:10000"]
    async fn repara_un_contenedor_que_quedo_privado() {
        let almacen = AzureBlobStorage::nuevo(&conn_azurite(), "").unwrap();
        let contenedor = crate::services::nombres::nombre_contenedor("Reparacion");
        let contenedor = contenedor.as_str();
        let pdf: Vec<u8> = b"%PDF-1.7\n%%EOF\n".to_vec();

        // Se crea (queda público) y luego se deja privado a propósito.
        let primero = crate::services::nombres::nombre_documento("Doc", uuid::Uuid::new_v4());
        almacen.guardar(contenedor, &primero, pdf.clone()).await.expect("subida inicial");
        almacen.fijar_acl(contenedor, None).await.expect("dejar privado");
        assert_eq!(almacen.acceso_publico(contenedor).await.unwrap(), "privado");
        let url_primero = format!("{}/{contenedor}/{primero}", almacen.endpoint);
        // Azure responde 404 y Azurite 403; lo que importa es que sin credenciales no se pueda leer.
        assert_ne!(reqwest::get(&url_primero).await.unwrap().status().as_u16(), 200);

        // La siguiente subida lo devuelve a lectura pública, y el documento anterior
        // vuelve a ser accesible sin credenciales.
        let segundo = crate::services::nombres::nombre_documento("Doc", uuid::Uuid::new_v4());
        almacen.guardar(contenedor, &segundo, pdf.clone()).await.expect("subida que repara");
        assert_eq!(almacen.acceso_publico(contenedor).await.unwrap(), "blob");
        assert_eq!(reqwest::get(&url_primero).await.unwrap().status().as_u16(), 200);
    }

    /// Se ejecuta con `cargo test -- --ignored` (fuera de CI, que no tiene Azurite).
    #[tokio::test]
    #[ignore = "requiere Azurite en localhost:10000"]
    async fn azurite_crea_contenedor_sube_y_descarga_pdf() {
        let almacen = AzureBlobStorage::nuevo(&conn_azurite(), "").unwrap();
        // Categoría de una sola palabra: `nombre_contenedor` la deja sin guiones bajos,
        // que es lo único que acepta Azure (ver `contenedor_con_guion_bajo_lo_rechaza_azure`).
        let contenedor = crate::services::nombres::nombre_contenedor("Mascotas");
        let contenedor = contenedor.as_str();
        let uuid = uuid::Uuid::new_v4();
        let documento = crate::services::nombres::nombre_documento("Datos CZE", uuid);
        // PDF mínimo válido: lo importante es que vuelvan los mismos bytes.
        let pdf: Vec<u8> = b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n".to_vec();

        // La primera vez crea el contenedor; la segunda lo encuentra existente.
        almacen.guardar(contenedor, &documento, pdf.clone()).await.expect("subida");
        let otro = crate::services::nombres::nombre_documento("Datos CZE", uuid::Uuid::new_v4());
        almacen.guardar(contenedor, &otro, pdf.clone()).await.expect("segunda subida");

        let (bytes, tipo) = almacen.descargar(contenedor, &documento).await.expect("descarga");
        assert_eq!(bytes, pdf);
        assert_eq!(tipo.as_deref(), Some(TIPO_PDF));

        // El contenedor quedó con acceso público de lectura: el blob se baja sin
        // firmar, que es de lo que dependen las URLs impresas en los QR.
        let anonima = reqwest::get(format!("{}/{contenedor}/{documento}", almacen.endpoint)).await.expect("GET anónimo");
        assert_eq!(anonima.status().as_u16(), 200);
        assert_eq!(anonima.headers().get(reqwest::header::CONTENT_TYPE).unwrap(), TIPO_PDF);
        assert_eq!(anonima.bytes().await.unwrap().to_vec(), pdf);

        // Un blob inexistente da un error con el código HTTP y el x-ms-error-code.
        let err = almacen.descargar(contenedor, "no_existe_9999").await.unwrap_err().to_string();
        assert!(err.contains("404"), "mensaje inesperado: {err}");
        assert!(err.contains("BlobNotFound"), "mensaje inesperado: {err}");
    }

    /// LIMITACIÓN HEREDADA DE v2, no del adaptador: `nombre_contenedor` es
    /// snake_case, pero Azure solo acepta nombres de contenedor con minúsculas,
    /// dígitos y guiones (`-`). Cualquier categoría de más de una palabra
    /// ("Emergencias Pecuarias", "Mosca Fruta", "Alimentación Animal",
    /// "Expendios Veterinarios") produce un guion bajo y Azure responde 400
    /// `InvalidResourceName`. Este test fija el comportamiento observado para que
    /// quede documentado; Azurite aplica exactamente la misma regla que Azure.
    #[tokio::test]
    #[ignore = "requiere Azurite en localhost:10000"]
    async fn contenedor_con_guion_bajo_lo_rechaza_azure() {
        let almacen = AzureBlobStorage::nuevo(&conn_azurite(), "").unwrap();
        // Azure sólo acepta minúsculas, dígitos y guion medio en el nombre del contenedor.
        let err = almacen.guardar("emergencias_pecuarias", "doc", b"%PDF-1.7\n".to_vec()).await.unwrap_err().to_string();
        assert!(err.contains("400"), "mensaje inesperado: {err}");
        assert!(err.contains("InvalidResourceName"), "mensaje inesperado: {err}");
        // Por eso `nombre_contenedor` traduce el guion bajo de la convención de v2.
        assert_eq!(
            crate::services::nombres::nombre_contenedor("Emergencias Pecuarias"),
            "emergencias-pecuarias"
        );
    }
}
