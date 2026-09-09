//! El documento OpenAPI y la página de Swagger UI viajan dentro del binario, así que
//! se pueden verificar sin base de datos ni servidor: basta con el documento incrustado
//! y con la función que arma la página.

use sag_transversal_api_generarcredencialv3::http::swagger::{pagina_html, OPENAPI_JSON};
use sag_transversal_api_generarcredencialv3::http::PREFIJO;
use serde_json::Value;

fn documento() -> Value {
    serde_json::from_str(OPENAPI_JSON).expect("assets/openapi.json no es JSON válido")
}

/// Las nueve operaciones que expone el router (`src/http/mod.rs`), con su método.
const OPERACIONES: &[(&str, &str)] = &[
    ("/health", "get"),
    ("/categoria", "get"),
    ("/categoria/{id}", "get"),
    ("/plantilla", "get"),
    ("/plantilla/{id}", "get"),
    ("/credencial", "get"),
    ("/credencial/{guid}", "get"),
    ("/generarCredencial", "post"),
    ("/generarPreview", "post"),
];

#[test]
fn el_documento_openapi_parsea_y_declara_todas_las_operaciones() {
    let doc = documento();

    assert!(
        doc["openapi"].as_str().unwrap_or_default().starts_with("3."),
        "se espera OpenAPI 3.x"
    );
    assert!(doc["info"]["title"].is_string());

    // `servers` relativo al prefijo: la instancia que sirve la documentación es la que
    // se prueba desde la UI, sin configuración por ambiente.
    assert_eq!(doc["servers"][0]["url"], Value::from(PREFIJO));

    let paths = doc["paths"].as_object().expect("el documento no tiene 'paths'");
    for (ruta, metodo) in OPERACIONES {
        let operacion = &paths[*ruta][*metodo];
        assert!(!operacion.is_null(), "falta {} {}", metodo.to_uppercase(), ruta);
        assert!(operacion["summary"].is_string(), "{ruta} sin summary");
        assert!(
            operacion["responses"].as_object().is_some_and(|r| !r.is_empty()),
            "{ruta} sin respuestas documentadas"
        );
    }
    assert_eq!(paths.len(), OPERACIONES.len(), "sobran o faltan rutas documentadas");
}

#[test]
fn el_documento_describe_las_respuestas_propias_de_la_api() {
    let doc = documento();
    let paths = &doc["paths"];

    // 201 con { guid, categoriaId, url, fechaCreacion }
    let creada = &doc["components"]["schemas"]["CredencialCreada"]["properties"];
    for campo in ["guid", "categoriaId", "url", "fechaCreacion"] {
        assert!(!creada[campo].is_null(), "CredencialCreada sin {campo}");
    }
    assert!(!paths["/generarCredencial"]["post"]["responses"]["201"].is_null());

    // El preview responde el PDF, no JSON.
    assert!(
        !paths["/generarPreview"]["post"]["responses"]["200"]["content"]["application/pdf"].is_null(),
        "generarPreview debería documentar application/pdf"
    );

    // Payload de v2.
    let solicitud = &doc["components"]["schemas"]["SolicitudGeneracion"];
    assert_eq!(solicitud["required"], serde_json::json!(["PlantillaId", "DatosCredencial"]));
    for campo in ["PlantillaId", "DatosCredencial", "GenerarQR", "QRTag"] {
        assert!(!solicitud["properties"][campo].is_null(), "SolicitudGeneracion sin {campo}");
    }

    // Query de los listados.
    let nombres: Vec<&str> = paths["/credencial"]["get"]["parameters"]
        .as_array()
        .expect("sin parámetros")
        .iter()
        .map(|p| {
            p["name"]
                .as_str()
                .or_else(|| p["$ref"].as_str().and_then(|r| r.rsplit('/').next()))
                .unwrap_or_default()
        })
        .collect();
    assert_eq!(nombres, vec!["sort", "Paginate", "Page", "Limit"]);

    // Formas de error de `src/error.rs`.
    assert_eq!(doc["components"]["schemas"]["ErroresValidacion"]["type"], "array");
    let item = &doc["components"]["schemas"]["ErroresValidacion"]["items"]["properties"];
    for campo in ["message", "rule", "field"] {
        assert!(!item[campo].is_null(), "el error de validación debería traer {campo}");
    }
    assert_eq!(
        doc["components"]["responses"]["NoEncontrado"]["content"]["application/json"]["example"],
        serde_json::json!({ "title": "Not Found", "status": 404 })
    );
}

#[test]
fn la_pagina_lleva_swagger_ui_incrustado_y_apunta_al_documento() {
    let url = "http://localhost:3345/api/v3/transversal/credencial/swagger/swagger.json";
    let html = pagina_html(url);

    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains(&format!("url: \"{url}\"")), "la página no referencia el swagger.json");

    // Sin CDN: el CSS, los dos bundles y los favicons van dentro de la propia página.
    assert!(html.contains("SwaggerUIBundle"));
    assert!(html.contains("SwaggerUIStandalonePreset"));
    assert!(html.contains("data:image/png;base64,"));
    assert!(
        !html.contains("https://unpkg.com") && !html.contains("cdn.jsdelivr.net"),
        "la página no debe depender de una CDN"
    );
    assert!(
        html.len() > 1_500_000,
        "la página debería traer los bundles incrustados (mide {} bytes)",
        html.len()
    );
}
