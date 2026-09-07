//! Conversión de `serde_json::Value` a valores de Typst (`sys.inputs`).
//!
//! Regla adicional: un string con forma de data URL de imagen
//! (`data:image/png;base64,...`) se entrega a la plantilla como diccionario
//! `(format: "png", mime: ..., bytes: <bytes>)`, porque Typst no decodifica
//! base64 pero sí incrusta bytes con `image(dict.bytes, format: dict.format)`.
//! Así una plantilla puede usar el QR (o cualquier imagen) que v2 entregaba
//! como data URL, sin cambiar el contrato del consumidor.

use base64::Engine;
use typst::foundations::{Array, Bytes, Dict, Str, Value};

fn data_url_a_imagen(s: &str) -> Option<Value> {
    let resto = s.strip_prefix("data:image/")?;
    let (subtipo, resto) = resto.split_once(';')?;
    let datos = resto.strip_prefix("base64,")?;
    let format = match subtipo.to_ascii_lowercase().as_str() {
        "png" => "png",
        "jpeg" | "jpg" => "jpg",
        "gif" => "gif",
        "svg+xml" | "svg" => "svg",
        "webp" => "webp",
        _ => return None,
    };
    let bytes = base64::engine::general_purpose::STANDARD.decode(datos.trim()).ok()?;
    let mut d = Dict::new();
    d.insert("format".into(), Value::Str(Str::from(format)));
    d.insert("mime".into(), Value::Str(Str::from(format!("image/{subtipo}").as_str())));
    d.insert("bytes".into(), Value::Bytes(Bytes::new(bytes)));
    Some(Value::Dict(d))
}

/// Base64 "crudo" de una imagen (sin prefijo data URL). v2 lo usaba en carnets:
/// `src="data:image/jpg;base64,{foto}"`, así que el consumidor envía sólo el base64.
/// Se reconoce por la firma del formato al inicio del base64.
fn base64_crudo_a_imagen(s: &str) -> Option<Value> {
    let t = s.trim();
    if t.len() < 64 || !t.is_ascii() {
        return None;
    }
    let (format, mime) = if t.starts_with("/9j/") {
        ("jpg", "image/jpeg")
    } else if t.starts_with("iVBOR") {
        ("png", "image/png")
    } else if t.starts_with("R0lGOD") {
        ("gif", "image/gif")
    } else if t.starts_with("UklGR") {
        ("webp", "image/webp")
    } else {
        return None;
    };
    let bytes = base64::engine::general_purpose::STANDARD.decode(t).ok()?;
    let mut d = Dict::new();
    d.insert("format".into(), Value::Str(Str::from(format)));
    d.insert("mime".into(), Value::Str(Str::from(mime)));
    d.insert("bytes".into(), Value::Bytes(Bytes::new(bytes)));
    Some(Value::Dict(d))
}

pub fn json_a_typst(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::None,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => {
            if s.starts_with("data:image/") {
                if let Some(img) = data_url_a_imagen(s) {
                    return img;
                }
            } else if let Some(img) = base64_crudo_a_imagen(s) {
                return img;
            }
            Value::Str(Str::from(s.as_str()))
        }
        serde_json::Value::Array(a) => Value::Array(a.iter().map(json_a_typst).collect::<Array>()),
        serde_json::Value::Object(o) => {
            let mut d = Dict::new();
            for (k, val) in o {
                d.insert(Str::from(k.as_str()), json_a_typst(val));
            }
            Value::Dict(d)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_url_se_convierte_en_bytes() {
        let v = serde_json::json!({ "QR": "data:image/png;base64,iVBORw0KGgo=" , "texto": "hola" });
        let t = json_a_typst(&v);
        let Value::Dict(d) = t else { panic!() };
        let Value::Dict(qr) = d.get("QR").unwrap() else { panic!("QR debía ser dict") };
        assert_eq!(qr.get("format").unwrap(), &Value::Str("png".into()));
        assert!(matches!(qr.get("bytes").unwrap(), Value::Bytes(_)));
        assert_eq!(d.get("texto").unwrap(), &Value::Str("hola".into()));
    }

    #[test]
    fn base64_crudo_de_imagen_se_convierte() {
        // PNG 1x1 válido, sin prefijo data URL (contrato de {foto} en v2)
        let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
        let v = serde_json::json!({ "foto": png, "nombre": "iVBORito" });
        let Value::Dict(d) = json_a_typst(&v) else { panic!() };
        let Value::Dict(foto) = d.get("foto").unwrap() else { panic!("foto debía ser dict") };
        assert_eq!(foto.get("format").unwrap(), &Value::Str("png".into()));
        assert!(matches!(d.get("nombre").unwrap(), Value::Str(_)));
    }
}
