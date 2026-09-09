//! Cliente de Consul KV para la configuración de runtime.
//!
//! Reemplaza a `@sag/consulconn` (la librería Node que usa v2) con las mismas
//! convenciones, para que la aplicación se despliegue con el pipeline estándar del
//! SAG sin cambiar nada del lado de la plataforma:
//!
//! - conexión desde `CONSUL_HOST`, `CONSUL_PORT`, `CONSUL_SECURE`,
//!   `CONSUL_ACL_TOKEN` y `CONSUL_REJECT_UNAUTHORIZED` (las entrega el ConfigMap
//!   que arma el release a partir de la key `{ambiente}/{proyecto}/deploy`);
//! - la configuración de la aplicación vive en `{NODE_ENV}/{APP_NAME}/config`;
//! - las sub-configuraciones (base de datos, storage) se referencian desde ahí y
//!   se leen como `{NODE_ENV}/{valor referenciado}`.
//!
//! Se usa la API HTTP de Consul con `?raw=true`, que devuelve el valor tal cual
//! (sin el sobre JSON con el valor en base64).

use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use std::time::Duration;

/// Tiempo máximo por lectura. Si Consul no responde, es preferible fallar el
/// arranque rápido y que OpenShift reinicie el pod, a quedar colgado sin servicio.
const TIMEOUT: Duration = Duration::from_secs(10);

pub struct ConsulClient {
    base: String,
    token: Option<String>,
    http: reqwest::Client,
}

impl ConsulClient {
    /// Construye el cliente desde las variables que entrega el ConfigMap.
    pub fn desde_entorno() -> Result<Self> {
        let host = std::env::var("CONSUL_HOST").context("falta CONSUL_HOST")?;
        let puerto = std::env::var("CONSUL_PORT").unwrap_or_else(|_| "443".into());
        let seguro = !matches!(std::env::var("CONSUL_SECURE").as_deref(), Ok("false") | Ok("0"));
        // Igual que la librería de v2: sólo el literal "false" desactiva la validación.
        let validar_tls = !matches!(
            std::env::var("CONSUL_REJECT_UNAUTHORIZED").as_deref(),
            Ok("false") | Ok("0")
        );
        let token = std::env::var("CONSUL_ACL_TOKEN").ok().filter(|t| !t.is_empty());

        let esquema = if seguro { "https" } else { "http" };
        // El host puede venir con esquema o con puerto incluido; se normaliza.
        let host = host
            .trim()
            .trim_end_matches('/')
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .to_string();
        let base = if host.contains(':') {
            format!("{esquema}://{host}")
        } else {
            format!("{esquema}://{host}:{puerto}")
        };

        let http = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .danger_accept_invalid_certs(!validar_tls)
            .build()
            .context("no se pudo construir el cliente HTTP para Consul")?;

        Ok(Self { base, token, http })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    /// Lee una key y la deserializa. Falla si no existe: en un pod, arrancar sin
    /// configuración sólo posterga el error hasta la primera solicitud.
    pub async fn obtener<T: DeserializeOwned>(&self, key: &str) -> Result<T> {
        let crudo = self.obtener_texto(key).await?;
        serde_json::from_str(&crudo)
            .with_context(|| format!("la key '{key}' de Consul no tiene el formato esperado"))
    }

    pub async fn obtener_texto(&self, key: &str) -> Result<String> {
        let key = key.trim_matches('/');
        let url = format!("{}/v1/kv/{key}?raw=true", self.base);

        let mut req = self.http.get(&url);
        if let Some(token) = &self.token {
            req = req.header("X-Consul-Token", token);
        }

        let resp = req
            .send()
            .await
            .with_context(|| format!("no se pudo consultar Consul en {}", self.base))?;

        let estado = resp.status();
        if estado == reqwest::StatusCode::NOT_FOUND {
            bail!("la key '{key}' no existe en Consul");
        }
        if estado == reqwest::StatusCode::FORBIDDEN || estado == reqwest::StatusCode::UNAUTHORIZED {
            bail!("Consul rechazó el token al leer '{key}' (HTTP {estado})");
        }
        if !estado.is_success() {
            bail!("Consul respondió HTTP {estado} al leer '{key}'");
        }

        let cuerpo = resp.text().await.context("no se pudo leer la respuesta de Consul")?;
        if cuerpo.trim().is_empty() {
            bail!("la key '{key}' existe pero está vacía");
        }
        Ok(cuerpo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializa el acceso a las variables de entorno entre tests.
    fn con_entorno<T>(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> T) -> T {
        static CANDADO: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = CANDADO.lock().unwrap_or_else(|e| e.into_inner());
        let previas: Vec<(String, Option<String>)> =
            vars.iter().map(|(k, _)| (k.to_string(), std::env::var(k).ok())) .collect();
        for (k, v) in vars {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        let r = f();
        for (k, v) in previas {
            match v {
                Some(v) => std::env::set_var(&k, v),
                None => std::env::remove_var(&k),
            }
        }
        r
    }

    #[test]
    fn arma_la_url_base_segun_el_entorno() {
        let base = con_entorno(
            &[
                ("CONSUL_HOST", Some("configms.sag.gob.cl")),
                ("CONSUL_PORT", Some("443")),
                ("CONSUL_SECURE", Some("true")),
                ("CONSUL_ACL_TOKEN", Some("token")),
                ("CONSUL_REJECT_UNAUTHORIZED", Some("false")),
            ],
            || ConsulClient::desde_entorno().unwrap().base().to_string(),
        );
        assert_eq!(base, "https://configms.sag.gob.cl:443");
    }

    #[test]
    fn normaliza_host_con_esquema_o_puerto() {
        let base = con_entorno(
            &[
                ("CONSUL_HOST", Some("http://localhost:8500")),
                ("CONSUL_PORT", Some("443")),
                ("CONSUL_SECURE", Some("false")),
                ("CONSUL_ACL_TOKEN", None),
                ("CONSUL_REJECT_UNAUTHORIZED", None),
            ],
            || ConsulClient::desde_entorno().unwrap().base().to_string(),
        );
        assert_eq!(base, "http://localhost:8500");
    }
}
