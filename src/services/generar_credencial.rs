//! Caso de uso central: plantilla + datos -> PDF (-> storage + registro).
//! No conoce HTTP: lo usan los handlers hoy y podría usarlo un worker asíncrono mañana.

use crate::error::ApiError;
use crate::render::RenderPool;
use crate::repos::{credencial, plantilla, plantilla::PlantillaConCategoria};
use crate::services::{nombres, qr};
use crate::storage::Storage;
use serde::Deserialize;
use serde_json::{Map, Value};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Payload de v2: `PlantillaId`, `DatosCredencial`, `GenerarQR`, `QRTag`.
#[derive(Debug, Clone, Deserialize)]
pub struct SolicitudGeneracion {
    #[serde(rename = "PlantillaId")]
    pub plantilla_id: i64,
    #[serde(rename = "DatosCredencial")]
    pub datos_credencial: Map<String, Value>,
    #[serde(rename = "GenerarQR", default)]
    pub generar_qr: bool,
    #[serde(rename = "QRTag")]
    pub qr_tag: Option<String>,
}

pub struct PdfGenerado {
    pub plantilla: PlantillaConCategoria,
    pub pdf: Vec<u8>,
    /// UUID reservado cuando se generó QR (la URL ya quedó incrustada en el documento).
    pub uuid_reservado: Option<Uuid>,
}

#[derive(Clone)]
pub struct GeneradorCredencial {
    pub pool: PgPool,
    pub render: RenderPool,
    pub storage: Arc<dyn Storage>,
}

impl GeneradorCredencial {
    /// Paso 1 de v2 (`CredencialService.generatePdf`): busca la plantilla, arma los
    /// datos (con QR anticipado si se pide) y renderiza.
    pub async fn generar_pdf(&self, solicitud: &SolicitudGeneracion) -> Result<PdfGenerado, ApiError> {
        let id = i32::try_from(solicitud.plantilla_id).map_err(|_| ApiError::NoEncontrado)?;
        let plantilla = plantilla::buscar_con_categoria(&self.pool, id).await?.ok_or(ApiError::NoEncontrado)?;

        let mut datos = solicitud.datos_credencial.clone();
        preparar_datos(&mut datos);

        let mut uuid_reservado = None;
        if solicitud.generar_qr {
            let uuid = Uuid::new_v4();
            let url = self.storage.url_publica(
                &nombres::nombre_contenedor(&plantilla.categoria_nombre),
                &nombres::nombre_documento(&plantilla.nombre, uuid),
            );
            let qr = qr::generar(&url).map_err(|e| ApiError::Interno(e.to_string()))?;
            let tag = solicitud.qr_tag.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| "QR".to_string());
            datos.insert(tag, Value::String(qr.data_url));
            datos.insert("__qr_svg".to_string(), Value::String(qr.svg));
            datos.insert("__qr_url".to_string(), Value::String(url));
            uuid_reservado = Some(uuid);
        }

        let inicio = Instant::now();
        let pdf = self
            .render
            .renderizar(plantilla.ruta.clone(), Value::Object(datos))
            .await
            .map_err(|e| ApiError::Render(e.to_string()))?;
        tracing::info!(
            plantilla = %plantilla.nombre,
            motor = %plantilla.motor,
            bytes = pdf.len(),
            ms = inicio.elapsed().as_millis() as u64,
            "PDF generado"
        );

        Ok(PdfGenerado { plantilla, pdf, uuid_reservado })
    }

    /// Paso 2 de v2 (`CredencialService.store`): guarda el PDF y registra la credencial.
    pub async fn guardar(&self, generado: PdfGenerado) -> Result<credencial::CredencialFila, ApiError> {
        let uuid = generado.uuid_reservado.unwrap_or_else(Uuid::new_v4);
        let contenedor = nombres::nombre_contenedor(&generado.plantilla.categoria_nombre);
        let documento = nombres::nombre_documento(&generado.plantilla.nombre, uuid);

        let inicio = Instant::now();
        self.storage
            .guardar(&contenedor, &documento, generado.pdf)
            .await
            .map_err(|e| ApiError::Guardar(e.to_string()))?;
        let url = self.storage.url_publica(&contenedor, &documento);

        let fila = credencial::crear(&self.pool, uuid, generado.plantilla.categoria_id, &url)
            .await
            .map_err(|e| ApiError::Guardar(e.to_string()))?;

        tracing::info!(guid = %fila.id, %url, ms = inicio.elapsed().as_millis() as u64, "credencial guardada");
        Ok(fila)
    }
}

/// Normalizaciones de compatibilidad que se aplican a `DatosCredencial` antes de
/// entregarlo a la plantilla (las usa también `bench-render` para que el
/// benchmark vea exactamente lo mismo que la API).
pub fn preparar_datos(datos: &mut Map<String, Value>) {
    normalizar_json_embebido(datos);
    estructurar_fragmentos_html(datos);
}

/// Compatibilidad con consumidores que hoy envían estructuras serializadas como
/// string (p. ej. `mascotas: "[{...}]"` para el script de la plantilla HTML de v2).
/// Si un valor string parece JSON de objeto/arreglo, se parsea para que la
/// plantilla reciba datos estructurados.
fn normalizar_json_embebido(datos: &mut Map<String, Value>) {
    for valor in datos.values_mut() {
        if let Value::String(s) = valor {
            let t = s.trim();
            if (t.starts_with('[') && t.ends_with(']')) || (t.starts_with('{') && t.ends_with('}')) {
                if let Ok(parseado) = serde_json::from_str::<Value>(t) {
                    *valor = parseado;
                }
            }
        }
    }
}

/// Compatibilidad con consumidores que envían fragmentos HTML (`integrantesHtml`,
/// `hospedantesHtml` en mosca de la fruta). Typst no interpreta HTML, así que por
/// cada clave `xHtml` cuyo valor es string se agrega una clave `x` con el contenido
/// estructurado: filas de celdas si trae `<tr>`, lista de textos si trae `<span>`,
/// `<li>` o `<p>`, o el texto plano en otro caso. La clave original se conserva.
fn estructurar_fragmentos_html(datos: &mut Map<String, Value>) {
    let claves: Vec<String> = datos
        .keys()
        .filter(|k| k.len() > 4 && (k.ends_with("Html") || k.ends_with("HTML") || k.ends_with("_html")))
        .cloned()
        .collect();
    for clave in claves {
        let Some(Value::String(html)) = datos.get(&clave) else { continue };
        let base = clave.trim_end_matches("Html").trim_end_matches("HTML").trim_end_matches("_html").to_string();
        if datos.contains_key(&base) {
            continue;
        }
        datos.insert(base, html_fragmento::estructurar(html));
    }
}

/// Parser mínimo de fragmentos HTML (sin dependencias): suficiente para los
/// fragmentos que generan los consumidores (tags simples con texto).
pub mod html_fragmento {
    use serde_json::Value;

    fn decodificar(s: &str) -> String {
        s.replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&#x27;", "'")
    }

    /// Recorre el fragmento y devuelve (nombre_tag_en_minúsculas, es_cierre) por cada tag
    /// y el texto entre tags.
    enum Tok {
        Abre(String),
        Cierra(String),
        Texto(String),
    }

    fn tokens(html: &str) -> Vec<Tok> {
        let mut out = Vec::new();
        let mut resto = html;
        while let Some(i) = resto.find('<') {
            if i > 0 {
                out.push(Tok::Texto(resto[..i].to_string()));
            }
            let Some(j) = resto[i..].find('>') else { break };
            let tag = &resto[i + 1..i + j];
            let cierre = tag.starts_with('/');
            let nombre = tag
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !nombre.is_empty() && !nombre.starts_with('!') {
                out.push(if cierre { Tok::Cierra(nombre) } else { Tok::Abre(nombre) });
            }
            resto = &resto[i + j + 1..];
        }
        if !resto.is_empty() {
            out.push(Tok::Texto(resto.to_string()));
        }
        out
    }

    fn limpiar(t: &str) -> String {
        decodificar(t).split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Parsea celdas de una tabla desde `i` (justo después de `<table>` o al inicio si la
    /// tabla es implícita) hasta el `</table>` correspondiente. Una celda que contiene
    /// una tabla anidada se convierte en el arreglo de filas de esa tabla.
    fn parse_tabla(toks: &[Tok], mut i: usize) -> (Vec<Value>, usize) {
        let mut filas: Vec<Value> = Vec::new();
        let mut fila: Vec<Value> = Vec::new();
        let mut celda = String::new();
        let mut anidada: Option<Value> = None;
        let mut en_celda = false;
        while i < toks.len() {
            match &toks[i] {
                Tok::Abre(n) if n == "table" && en_celda => {
                    let (sub, j) = parse_tabla(toks, i + 1);
                    anidada = Some(Value::Array(sub));
                    i = j;
                    continue;
                }
                Tok::Cierra(n) if n == "table" => {
                    i += 1;
                    break;
                }
                Tok::Abre(n) if n == "td" || n == "th" => {
                    en_celda = true;
                    celda.clear();
                    anidada = None;
                }
                Tok::Cierra(n) if n == "td" || n == "th" => {
                    fila.push(match anidada.take() {
                        Some(sub) => sub,
                        None => Value::String(limpiar(&celda)),
                    });
                    en_celda = false;
                }
                Tok::Abre(n) if n == "br" && en_celda => celda.push(' '),
                Tok::Cierra(n) if n == "tr" => {
                    if !fila.is_empty() {
                        filas.push(Value::Array(std::mem::take(&mut fila)));
                    }
                }
                Tok::Texto(x) if en_celda => celda.push_str(x),
                _ => {}
            }
            i += 1;
        }
        if !fila.is_empty() {
            filas.push(Value::Array(fila));
        }
        (filas, i)
    }

    pub fn estructurar(html: &str) -> Value {
        let toks = tokens(html);
        let tiene = |n: &str| toks.iter().any(|t| matches!(t, Tok::Abre(x) if x == n));

        if tiene("tr") {
            // Tabla explícita (<table>...) o implícita (fragmento que empieza en <tr>):
            // si el primer <tr> aparece antes que cualquier <table>, la tabla es implícita
            // y los <table> posteriores son anidados.
            let pos_table = toks.iter().position(|t| matches!(t, Tok::Abre(x) if x == "table"));
            let pos_tr = toks.iter().position(|t| matches!(t, Tok::Abre(x) if x == "tr"));
            let inicio = match (pos_table, pos_tr) {
                (Some(t), Some(r)) if t < r => t + 1,
                (Some(t), None) => t + 1,
                _ => 0,
            };
            let (filas, _) = parse_tabla(&toks, inicio);
            return Value::Array(filas);
        }

        let contenedor = ["span", "li", "p", "div"].into_iter().find(|n| tiene(n));
        if let Some(n) = contenedor {
            let mut items = Vec::new();
            let mut actual = String::new();
            let mut dentro = 0usize;
            for t in &toks {
                match t {
                    Tok::Abre(x) if x == n => {
                        dentro += 1;
                        actual.clear();
                    }
                    Tok::Cierra(x) if x == n && dentro > 0 => {
                        dentro -= 1;
                        let texto = limpiar(&actual);
                        if !texto.is_empty() {
                            items.push(Value::String(texto));
                        }
                    }
                    Tok::Texto(x) if dentro > 0 => actual.push_str(x),
                    _ => {}
                }
            }
            return Value::Array(items);
        }

        let texto: String = toks
            .iter()
            .filter_map(|t| match t {
                Tok::Texto(x) => Some(x.as_str()),
                Tok::Abre(n) if n == "br" => Some(" "),
                _ => None,
            })
            .collect();
        Value::String(limpiar(&texto))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragmentos_html_se_estructuran() {
        let mut m = Map::new();
        m.insert("integrantesHtml".into(), Value::String("<span class=\"tag\">Guido</span><span class=\"tag\">Juan &amp; Ana</span>".into()));
        m.insert("hospedantesHtml".into(), Value::String("<tr><td><strong>Durazno</strong></td><td>5 kg</td></tr><tr><td>Ciruelo</td><td>2 kg</td></tr>".into()));
        m.insert("notaHtml".into(), Value::String("Texto <b>simple</b><br>segunda".into()));
        estructurar_fragmentos_html(&mut m);
        assert_eq!(m["integrantes"], serde_json::json!(["Guido", "Juan & Ana"]));
        assert_eq!(m["hospedantes"], serde_json::json!([["Durazno", "5 kg"], ["Ciruelo", "2 kg"]]));
        assert_eq!(m["nota"], serde_json::json!("Texto simple segunda"));
        assert!(m.contains_key("integrantesHtml"));
    }

    #[test]
    fn tablas_anidadas_se_conservan() {
        let html = "<tr><td><strong>Durazno</strong></td><td>No</td><td>5</td><td><table><thead><tr><th>Tipo</th><th>Litros</th></tr></thead><tbody><tr><td>Cebo</td><td>2</td></tr><tr><td>Aspersión</td><td>10</td></tr></tbody></table></td></tr><tr><td>Ciruelo</td><td>Sí</td><td>0</td><td></td></tr>";
        let v = html_fragmento::estructurar(html);
        assert_eq!(
            v,
            serde_json::json!([
                ["Durazno", "No", "5", [["Tipo", "Litros"], ["Cebo", "2"], ["Aspersión", "10"]]],
                ["Ciruelo", "Sí", "0", ""]
            ])
        );
    }

    #[test]
    fn parsea_estructuras_serializadas() {
        let mut m = Map::new();
        m.insert("mascotas".into(), Value::String("[{\"nombre\":\"Rocky\"}]".into()));
        m.insert("texto".into(), Value::String("[no es json".into()));
        normalizar_json_embebido(&mut m);
        assert!(m["mascotas"].is_array());
        assert!(m["texto"].is_string());
    }
}
