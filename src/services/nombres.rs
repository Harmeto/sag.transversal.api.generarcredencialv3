//! Nombres de contenedor y documento. Forman la URL pública que queda impresa en los
//! códigos QR de las credenciales emitidas, así que se conserva la convención de v2:
//!   contenedor = snake_case(sin_acentos(categoria).to_lowercase())
//!   documento  = snake_case(sin_acentos(plantilla).to_lowercase()) + "_" + aaaa-mm-dd (UTC) + "_" + uuid
//!
//! Con una corrección obligada: **Azure no acepta guion bajo en el nombre de un
//! contenedor** (sólo minúsculas, dígitos y guion medio, entre 3 y 63 caracteres, sin
//! guion al principio ni al final ni repetido). La convención de v2 produce guion bajo
//! en cuanto la categoría tiene más de una palabra, y Azure responde 400
//! `InvalidResourceName`, así que esos contenedores nunca pudieron existir: no hay URL
//! emitida que preservar. Por eso, y sólo para el nombre del contenedor, el guion bajo
//! se convierte en guion medio. Las categorías de una sola palabra no cambian, que son
//! justamente aquellas para las que sí puede haber credenciales emitidas.
//!
//! El nombre del documento no se toca: en un blob el guion bajo es válido.

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

/// Nombre de contenedor válido para Azure Blob Storage.
pub fn nombre_contenedor(categoria: &str) -> String {
    let base = snake_case(categoria).replace('_', "-");

    // Colapsa guiones repetidos y recorta los de los extremos.
    let mut limpio = String::with_capacity(base.len());
    for c in base.chars() {
        if c == '-' && (limpio.is_empty() || limpio.ends_with('-')) {
            continue;
        }
        limpio.push(c);
    }
    while limpio.ends_with('-') {
        limpio.pop();
    }

    // Azure exige entre 3 y 63 caracteres.
    if limpio.len() > 63 {
        limpio.truncate(63);
        while limpio.ends_with('-') {
            limpio.pop();
        }
    }
    while limpio.len() < 3 {
        limpio.push('0');
    }
    limpio
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
    fn contenedor_valido_para_azure() {
        // Una sola palabra: idéntico a v2, así que las URL ya emitidas siguen sirviendo.
        assert_eq!(nombre_contenedor("Mascotas"), "mascotas");
        assert_eq!(nombre_contenedor("Plaguicidas"), "plaguicidas");
        // Varias palabras: guion medio en vez de guion bajo, que Azure rechaza.
        assert_eq!(nombre_contenedor("Emergencias Pecuarias"), "emergencias-pecuarias");
        assert_eq!(nombre_contenedor("Alimentación Animal"), "alimentacion-animal");
        assert_eq!(nombre_contenedor("Mosca  de   la Fruta"), "mosca-de-la-fruta");
        // Casos límite del formato que exige Azure.
        assert_eq!(nombre_contenedor("  --Al--  "), "al0");
        assert_eq!(nombre_contenedor("A"), "a00");
        let largo = nombre_contenedor(&"palabra ".repeat(20));
        assert!(largo.len() <= 63 && !largo.ends_with('-'));
    }

    #[test]
    fn el_nombre_del_documento_conserva_el_guion_bajo() {
        let id = Uuid::nil();
        assert!(nombre_documento("Bioseguridad Traspatio", id).starts_with("bioseguridad_traspatio_"));
    }

    #[test]
    fn documento_lleva_fecha_y_uuid() {
        let id = Uuid::parse_str("ea035152-0914-41b0-a616-00171980bbea").unwrap();
        let n = nombre_documento("Datos CZE", id);
        assert!(n.starts_with("datos_cze_"));
        assert!(n.ends_with("_ea035152-0914-41b0-a616-00171980bbea"));
    }
}
