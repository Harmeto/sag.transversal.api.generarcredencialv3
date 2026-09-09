//! Documentación OpenAPI y Swagger UI, con las mismas dos rutas que v2
//! (`/swagger` y `/swagger/swagger.json`) bajo el prefijo de v3.
//!
//! Todo va incrustado en el binario:
//!
//! - el documento OpenAPI es `assets/openapi.json`, escrito a mano (v2 lo transpilaba
//!   desde una colección Postman, que en v3 no existe). Se sirve tal cual con
//!   `include_str!`: así se edita y valida como JSON con herramientas estándar, el
//!   diff es legible y no hay que serializarlo en cada petición;
//! - Swagger UI son los archivos de `swagger-ui-dist` copiados a `assets/swagger-ui/`
//!   (Apache 2.0, ver el `LICENSE` de esa carpeta) e incrustados con `include_str!` /
//!   `include_bytes!`. La página es autocontenida: no pide nada a ninguna CDN, que es
//!   lo que ya hacía v2 y lo que exige la red de los ambientes.
//!
//! El documento declara `servers: [{ url: "/api/v3/transversal/credencial" }]`, una URL
//! relativa: Swagger UI la resuelve contra el origen desde el que cargó el
//! `swagger.json`, así que "Try it out" apunta siempre a la propia instancia (local,
//! dev, qa o producción) sin configuración por ambiente.

use super::listado::{base_url, host_de};
use axum::extract::OriginalUri;
use axum::http::{header, HeaderMap};
use axum::response::{IntoResponse, Response};
use base64::Engine;

/// Documento OpenAPI 3.0 de la API.
pub const OPENAPI_JSON: &str = include_str!("../../assets/openapi.json");

const SWAGGER_UI_CSS: &str = include_str!("../../assets/swagger-ui/swagger-ui.css");
const INDEX_CSS: &str = include_str!("../../assets/swagger-ui/index.css");
const SWAGGER_UI_BUNDLE_JS: &str = include_str!("../../assets/swagger-ui/swagger-ui-bundle.js");
const SWAGGER_UI_STANDALONE_PRESET_JS: &str =
    include_str!("../../assets/swagger-ui/swagger-ui-standalone-preset.js");
const FAVICON_16: &[u8] = include_bytes!("../../assets/swagger-ui/favicon-16x16.png");
const FAVICON_32: &[u8] = include_bytes!("../../assets/swagger-ui/favicon-32x32.png");

/// `GET {prefijo}/swagger/swagger.json`
pub async fn documento() -> Response {
    ([(header::CONTENT_TYPE, "application/json")], OPENAPI_JSON).into_response()
}

/// `GET {prefijo}/swagger`
///
/// La URL del documento se construye desde el host de la petición, como hacía
/// `getBaseUrlByHost` en v2 (http en localhost, https en el resto), de modo que la
/// página funcione igual detrás del ingress de cada ambiente.
pub async fn ui(OriginalUri(uri): OriginalUri, headers: HeaderMap) -> Response {
    let url = format!("{}/swagger.json", base_url(&uri, host_de(&headers).as_deref()));
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        pagina_html(&url),
    )
        .into_response()
}

/// Swagger UI autocontenido: CSS, JS y favicons van dentro de la propia página.
pub fn pagina_html(url_json: &str) -> String {
    let b64 = base64::engine::general_purpose::STANDARD;
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <title>SAG · Generación de credenciales v3</title>
    <style>
{css_swagger}
{css_index}
    </style>
    <link rel="icon" href="data:image/png;base64,{favicon32}" sizes="32x32" />
    <link rel="icon" href="data:image/png;base64,{favicon16}" sizes="16x16" />
</head>
<body>
    <div id="swagger-ui"></div>
    <script>{js_bundle}</script>
    <script>{js_preset}</script>
    <script>
    window.onload = function () {{
        window.ui = SwaggerUIBundle({{
            url: "{url_json}",
            dom_id: '#swagger-ui',
            presets: [SwaggerUIBundle.presets.apis, SwaggerUIStandalonePreset],
            layout: "BaseLayout"
        }});
    }};
    </script>
</body>
</html>
"#,
        css_swagger = SWAGGER_UI_CSS,
        css_index = INDEX_CSS,
        favicon32 = b64.encode(FAVICON_32),
        favicon16 = b64.encode(FAVICON_16),
        js_bundle = SWAGGER_UI_BUNDLE_JS,
        js_preset = SWAGGER_UI_STANDALONE_PRESET_JS,
        url_json = url_json,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Uri;

    #[test]
    fn la_url_del_documento_sale_del_host_de_la_peticion() {
        let uri: Uri = "/api/v3/transversal/credencial/swagger".parse().unwrap();
        assert_eq!(
            format!("{}/swagger.json", base_url(&uri, Some("localhost:3345"))),
            "http://localhost:3345/api/v3/transversal/credencial/swagger/swagger.json"
        );
        assert_eq!(
            format!("{}/swagger.json", base_url(&uri, Some("servicios.sag.gob.cl"))),
            "https://servicios.sag.gob.cl/api/v3/transversal/credencial/swagger/swagger.json"
        );
    }
}
