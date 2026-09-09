//! Configuración de la aplicación.
//!
//! Dos orígenes, según `NODE_ENV` (la variable que el ConfigMap del release fija
//! con el ambiente: `dev`, `test`, `qa` o `prod`):
//!
//! - **`local`** (o sin `NODE_ENV`): todo sale de variables de entorno / `.env`.
//!   Es el modo de desarrollo: no requiere Consul ni VPN.
//! - **cualquier otro**: la configuración sensible sale de **Consul**, con las
//!   mismas convenciones que usa v2 a través de `@sag/consulconn`:
//!
//!   ```text
//!   {NODE_ENV}/{APP_NAME}/config        -> { port, host, logLevel,
//!                                            dbConfigValue, blobStorageConfigValue, ... }
//!   {NODE_ENV}/{dbConfigValue}          -> { server, port, user, password, database }
//!   {NODE_ENV}/{blobStorageConfigValue} -> { blobStorageConnectionString, blobStorageAccountName }
//!   ```
//!
//! Así una sola imagen sirve para todos los ambientes: lo único que cambia entre
//! ellos es el ConfigMap (`NODE_ENV`, `APP_NAME`, `CONSUL_*`) y lo que hay en
//! Consul bajo ese ambiente.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::path::PathBuf;
use std::str::FromStr;

use crate::consul::ConsulClient;

// ---------------------------------------------------------------------------
// Estructuras que se leen desde Consul
// ---------------------------------------------------------------------------

/// `{NODE_ENV}/{APP_NAME}/config`. Se conservan los nombres de campo de v2 y se
/// agregan, opcionales, los propios de v3. Los campos que sólo usa v2 (`appKey`)
/// se ignoran sin fallar.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConfigApp {
    pub port: Option<u16>,
    pub host: Option<String>,
    pub log_level: Option<String>,
    /// Key (sin el prefijo del ambiente) con la configuración de base de datos.
    pub db_config_value: Option<String>,
    /// Key (sin el prefijo del ambiente) con la configuración del blob storage.
    pub blob_storage_config_value: Option<String>,
    /// Renders simultáneos. Si no viene, se usan los núcleos disponibles.
    pub render_concurrency: Option<usize>,
}

/// `{NODE_ENV}/{dbConfigValue}`. Mismos campos que la configuración de v2, más
/// `sslMode` y `url`, opcionales, propios de PostgreSQL.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DbConfig {
    pub server: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    /// `disable`, `allow`, `prefer` (por omisión), `require`, `verify-ca`, `verify-full`.
    pub ssl_mode: Option<String>,
    /// Cadena de conexión completa. Si viene, manda sobre los campos anteriores.
    pub url: Option<String>,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            port: 5432,
            user: String::new(),
            password: String::new(),
            database: String::new(),
            ssl_mode: None,
            url: None,
        }
    }
}

/// `{NODE_ENV}/{blobStorageConfigValue}`. Idéntica a la de v2.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BlobConfig {
    pub blob_storage_connection_string: String,
    pub blob_storage_account_name: String,
}

// ---------------------------------------------------------------------------
// Configuración resuelta
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum OrigenDb {
    /// Desarrollo local: `DATABASE_URL`.
    Url(String),
    /// Ambientes desplegados: campos sueltos desde Consul.
    Partes(DbConfig),
}

#[derive(Debug, Clone)]
pub enum OrigenStorage {
    /// Disco del contenedor, servido por la propia API en `/archivos`.
    /// Sólo para desarrollo: con varias réplicas cada pod vería sus propios archivos.
    Local { dir: PathBuf, url_base: String },
    /// Azure Blob Storage, igual que v2.
    Azure { connection_string: String, account_name: String },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub app_name: String,
    pub entorno: String,
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub db: OrigenDb,
    pub plantillas_dir: PathBuf,
    pub fonts_dir: Option<PathBuf>,
    pub render_concurrency: usize,
    pub storage: OrigenStorage,
}

fn var(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("falta la variable de entorno {name}"))
}

fn var_opt(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn var_or(name: &str, default: &str) -> String {
    var_opt(name).unwrap_or_else(|| default.to_string())
}

fn usize_env(name: &str) -> Option<usize> {
    var_opt(name).and_then(|v| v.parse::<usize>().ok()).filter(|n| *n > 0)
}

/// Núcleos disponibles para el proceso. En OpenShift el límite de CPU del pod no
/// se refleja acá, así que `RENDER_CONCURRENCY` (vía `envVars` del deploy o la
/// propia key de Consul) permite ajustarlo por ambiente sin reconstruir la imagen.
fn nucleos() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

impl Config {
    /// Ambiente actual. `NODE_ENV` es el nombre que usa toda la plataforma del SAG.
    pub fn entorno_actual() -> String {
        var_or("NODE_ENV", "local").to_lowercase()
    }

    pub fn es_local(entorno: &str) -> bool {
        matches!(entorno, "local" | "")
    }

    pub async fn cargar() -> Result<Self> {
        let entorno = Self::entorno_actual();
        if Self::es_local(&entorno) {
            Self::desde_entorno(entorno)
        } else {
            Self::desde_consul(entorno).await
        }
    }

    /// Desarrollo local: todo desde `.env` / variables de entorno.
    pub fn desde_entorno(entorno: String) -> Result<Self> {
        let storage = match var_or("STORAGE_DRIVER", "local").to_lowercase().as_str() {
            "azure" => OrigenStorage::Azure {
                connection_string: var("BLOB_STORAGE_CONNECTION_STRING")?,
                account_name: var_or("BLOB_STORAGE_ACCOUNT_NAME", ""),
            },
            _ => OrigenStorage::Local {
                dir: PathBuf::from(var_or("STORAGE_DIR", "./storage")),
                url_base: var_or("STORAGE_PUBLIC_URL", "http://localhost:3345/archivos")
                    .trim_end_matches('/')
                    .to_string(),
            },
        };

        Ok(Self {
            app_name: var_or("APP_NAME", "sag.transversal.api.generarcredencialv3"),
            entorno,
            host: var_or("HOST", "0.0.0.0"),
            port: Self::puerto(None)?,
            log_level: var_or("LOG_LEVEL", "info"),
            db: OrigenDb::Url(var("DATABASE_URL")?),
            plantillas_dir: PathBuf::from(var_or("PLANTILLAS_DIR", "./plantillas")),
            fonts_dir: var_opt("FONTS_DIR").map(PathBuf::from),
            render_concurrency: usize_env("RENDER_CONCURRENCY").unwrap_or_else(nucleos),
            storage,
        })
    }

    /// Ambientes desplegados: la configuración sensible vive en Consul.
    pub async fn desde_consul(entorno: String) -> Result<Self> {
        let app_name = var("APP_NAME")
            .context("APP_NAME es obligatorio fuera de local: define el prefijo de las keys de Consul")?;

        let consul = ConsulClient::desde_entorno()?;
        let key_config = format!("{entorno}/{app_name}/config");
        eprintln!("[config] leyendo {key_config} desde {}", consul.base());

        let app: ConfigApp = consul
            .obtener(&key_config)
            .await
            .with_context(|| format!("no se pudo leer la configuración de la aplicación ({key_config})"))?;

        // Base de datos
        let db_key = app
            .db_config_value
            .clone()
            .filter(|k| !k.is_empty())
            .context("la configuración de la aplicación no define 'dbConfigValue'")?;
        let db: DbConfig = consul
            .obtener(&format!("{entorno}/{db_key}"))
            .await
            .with_context(|| format!("no se pudo leer la configuración de base de datos ({entorno}/{db_key})"))?;
        if db.url.is_none() && (db.server.is_empty() || db.database.is_empty()) {
            bail!("la configuración de base de datos en '{entorno}/{db_key}' está incompleta");
        }

        // Storage: Azure Blob si hay configuración. Si no, disco del pod, que sólo
        // sirve con una réplica; se advierte al arrancar.
        let storage = match app.blob_storage_config_value.clone().filter(|k| !k.is_empty()) {
            Some(blob_key) => {
                let blob: BlobConfig = consul
                    .obtener(&format!("{entorno}/{blob_key}"))
                    .await
                    .with_context(|| format!("no se pudo leer la configuración de storage ({entorno}/{blob_key})"))?;
                if blob.blob_storage_connection_string.is_empty() {
                    bail!(
                        "la configuración de storage en '{entorno}/{blob_key}' no trae 'blobStorageConnectionString'"
                    );
                }
                OrigenStorage::Azure {
                    connection_string: blob.blob_storage_connection_string,
                    account_name: blob.blob_storage_account_name,
                }
            }
            None => OrigenStorage::Local {
                dir: PathBuf::from(var_or("STORAGE_DIR", "/tmp/storage")),
                url_base: var_or("STORAGE_PUBLIC_URL", "").trim_end_matches('/').to_string(),
            },
        };

        Ok(Self {
            app_name,
            entorno,
            host: app.host.clone().unwrap_or_else(|| var_or("HOST", "0.0.0.0")),
            port: Self::puerto(app.port)?,
            log_level: app.log_level.clone().unwrap_or_else(|| var_or("LOG_LEVEL", "info")),
            db: OrigenDb::Partes(db),
            plantillas_dir: PathBuf::from(var_or("PLANTILLAS_DIR", "/app/plantillas")),
            fonts_dir: Some(PathBuf::from(var_or("FONTS_DIR", "/app/fonts"))),
            render_concurrency: usize_env("RENDER_CONCURRENCY")
                .or(app.render_concurrency)
                .unwrap_or_else(nucleos),
            storage,
        })
    }

    /// Puerto de escucha. `APP_PORT` manda porque es el que el Deployment declara
    /// como `containerPort` y al que apunta el Service: si la aplicación escuchara
    /// en otro, el pod quedaría arriba pero inalcanzable.
    fn puerto(puerto_consul: Option<u16>) -> Result<u16> {
        if let Some(v) = var_opt("APP_PORT") {
            let p: u16 = v.parse().context("APP_PORT inválido")?;
            if let Some(c) = puerto_consul {
                if c != p {
                    eprintln!(
                        "[config] aviso: Consul indica el puerto {c} pero se usará APP_PORT={p}, \
                         que es el que expone el Deployment"
                    );
                }
            }
            return Ok(p);
        }
        if let Some(c) = puerto_consul {
            return Ok(c);
        }
        var_or("PORT", "3345").parse().context("PORT inválido")
    }

    /// Opciones de conexión a PostgreSQL. Se construyen campo a campo (no como
    /// cadena) para no tener que escapar la contraseña.
    pub fn opciones_pg(&self) -> Result<PgConnectOptions> {
        match &self.db {
            OrigenDb::Url(url) => {
                PgConnectOptions::from_str(url).context("DATABASE_URL no es una cadena de conexión válida")
            }
            OrigenDb::Partes(db) => {
                if let Some(url) = db.url.as_deref().filter(|u| !u.is_empty()) {
                    return PgConnectOptions::from_str(url)
                        .context("la 'url' de la configuración de base de datos no es válida");
                }
                let ssl = match db.ssl_mode.as_deref().unwrap_or("prefer").to_lowercase().as_str() {
                    "disable" => PgSslMode::Disable,
                    "allow" => PgSslMode::Allow,
                    "require" => PgSslMode::Require,
                    "verify-ca" => PgSslMode::VerifyCa,
                    "verify-full" => PgSslMode::VerifyFull,
                    _ => PgSslMode::Prefer,
                };
                Ok(PgConnectOptions::new()
                    .host(&db.server)
                    .port(db.port)
                    .username(&db.user)
                    .password(&db.password)
                    .database(&db.database)
                    .ssl_mode(ssl))
            }
        }
    }

    /// Destino de la base de datos para los logs, sin credenciales.
    pub fn destino_db(&self) -> String {
        match &self.db {
            OrigenDb::Url(_) => "DATABASE_URL".to_string(),
            OrigenDb::Partes(db) => format!("{}:{}/{}", db.server, db.port, db.database),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_json_de_consul_usa_camel_case_como_en_v2() {
        let app: ConfigApp = serde_json::from_str(
            r#"{"port":3000,"host":"0.0.0.0","appKey":"x","logLevel":"debug",
                "dbConfigValue":"sag.transversal.api.generarcredencialv3/db",
                "blobStorageConfigValue":"sag.transversal.api.generarcredencialv3/blob",
                "renderConcurrency":6}"#,
        )
        .unwrap();
        assert_eq!(app.port, Some(3000));
        assert_eq!(app.log_level.as_deref(), Some("debug"));
        assert_eq!(app.render_concurrency, Some(6));
        assert!(app.db_config_value.unwrap().ends_with("/db"));

        let db: DbConfig = serde_json::from_str(
            r#"{"server":"pg.sag.gob.cl","port":5432,"user":"u","password":"p","database":"d"}"#,
        )
        .unwrap();
        assert_eq!(db.server, "pg.sag.gob.cl");
        assert_eq!(db.ssl_mode, None);

        let blob: BlobConfig =
            serde_json::from_str(r#"{"blobStorageConnectionString":"cs","blobStorageAccountName":"cuenta"}"#)
                .unwrap();
        assert_eq!(blob.blob_storage_account_name, "cuenta");
    }

    #[test]
    fn campos_desconocidos_no_rompen_la_carga() {
        // La key de Consul puede traer campos que sólo usa v2 (appKey, etc.).
        let app: ConfigApp =
            serde_json::from_str(r#"{"appKey":"secreto","otroCampo":123,"port":8080}"#).unwrap();
        assert_eq!(app.port, Some(8080));
    }

    #[test]
    fn opciones_pg_desde_partes() {
        let cfg = Config {
            app_name: "app".into(),
            entorno: "dev".into(),
            host: "0.0.0.0".into(),
            port: 3000,
            log_level: "info".into(),
            db: OrigenDb::Partes(DbConfig {
                server: "pg.interno".into(),
                port: 5433,
                user: "usuario".into(),
                // Contraseña con caracteres que romperían una URL sin escapar.
                password: "p@ss/word:#".into(),
                database: "credenciales".into(),
                ssl_mode: Some("require".into()),
                url: None,
            }),
            plantillas_dir: PathBuf::from("/app/plantillas"),
            fonts_dir: None,
            render_concurrency: 4,
            storage: OrigenStorage::Local { dir: PathBuf::from("/tmp"), url_base: String::new() },
        };
        let o = cfg.opciones_pg().unwrap();
        assert_eq!(o.get_host(), "pg.interno");
        assert_eq!(o.get_port(), 5433);
        assert_eq!(o.get_database(), Some("credenciales"));
        assert_eq!(cfg.destino_db(), "pg.interno:5433/credenciales");
    }

    #[test]
    fn entorno_local_es_el_unico_que_no_usa_consul() {
        assert!(Config::es_local("local"));
        assert!(!Config::es_local("dev"));
        assert!(!Config::es_local("prod"));
    }
}
