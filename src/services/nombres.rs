//! Nombres de contenedor y documento. Deben ser idénticos a los de v2 porque forman
//! la URL pública ya impresa en QR de credenciales emitidas:
//!   contenedor = snake_case(sin_acentos(categoria).to_lowercase())
//!   documento  = snake_case(sin_acentos(plantilla).to_lowercase()) + "_" + aaaa-mm-dd (UTC) + "_" + uuid

use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

fn sin_diacriticos(s: &str) -> String {
    s.nfd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).collect()
}

/// Equivalente a `string.snakeCase(replaceSpecialCharacters(nombre).toLowerCase())` de v2.
pub fn snake_case(nombre: &str) -> String {
    sin_diacriticos(nombre)
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

pub fn nombre_contenedor(categoria: &str) -> String {
    snake_case(categoria)
}

pub fn nombre_documento(plantilla: &str, uuid: Uuid) -> String {
    let fecha = chrono::Utc::now().format("%Y-%m-%d");
    format!("{}_{}_{}", snake_case(plantilla), fecha, uuid.hyphenated())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_como_v2() {
        assert_eq!(snake_case("Datos CZE"), "datos_cze");
        assert_eq!(snake_case("Certificación Alcoholes"), "certificacion_alcoholes");
        assert_eq!(snake_case("Nombre del contenedor"), "nombre_del_contenedor");
        assert_eq!(snake_case("Mosca-Fruta  visita"), "mosca_fruta_visita");
    }

    #[test]
    fn documento_lleva_fecha_y_uuid() {
        let id = Uuid::parse_str("ea035152-0914-41b0-a616-00171980bbea").unwrap();
        let n = nombre_documento("Datos CZE", id);
        assert!(n.starts_with("datos_cze_"));
        assert!(n.ends_with("_ea035152-0914-41b0-a616-00171980bbea"));
    }
}
