//! QR de verificación. v2 entregaba a la plantilla un data URL PNG; v3 entrega dos
//! cosas: el data URL (SVG) bajo el `QRTag` pedido, por compatibilidad, y el SVG en
//! texto bajo `__qr_svg`, que es lo que una plantilla Typst puede incrustar
//! directamente con `image(bytes(...), format: "svg")`.

use base64::Engine;
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};

pub struct Qr {
    pub svg: String,
    pub data_url: String,
}

pub fn generar(url: &str) -> anyhow::Result<Qr> {
    let codigo = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::M)?;
    let svg = codigo
        .render::<svg::Color>()
        .min_dimensions(200, 200)
        .quiet_zone(true)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();
    let data_url = format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(svg.as_bytes())
    );
    Ok(Qr { svg, data_url })
}
