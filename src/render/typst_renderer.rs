//! Adaptador Typst: compila `plantillas/<ruta>.typ` con `sys.inputs.datos` y exporta PDF.
//!
//! Un único `TypstEngine` compartido entre hilos: los archivos de plantilla y las
//! fuentes se cargan una vez; cada render construye un `World` liviano con sus inputs.

use super::{valores::json_a_typst, RenderError, Renderer};
use std::path::{Path, PathBuf};
use typst::foundations::Dict;
use typst_as_lib::typst_kit_options::TypstKitFontOptions;
use typst_as_lib::{TypstEngine, TypstTemplateCollection};
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

pub struct TypstRenderer {
    engine: TypstEngine<TypstTemplateCollection>,
    raiz: PathBuf,
}

impl TypstRenderer {
    pub fn nuevo(raiz_plantillas: &Path, fuentes_dir: Option<&Path>) -> anyhow::Result<Self> {
        anyhow::ensure!(
            raiz_plantillas.is_dir(),
            "el directorio de plantillas no existe: {}",
            raiz_plantillas.display()
        );
        let raiz = raiz_plantillas.canonicalize()?;

        let mut opciones = TypstKitFontOptions::default().include_system_fonts(true).include_embedded_fonts(true);
        if let Some(dir) = fuentes_dir.filter(|d| d.is_dir()) {
            opciones = opciones.include_dirs([dir.to_path_buf()]);
        }

        let engine = TypstEngine::builder()
            .search_fonts_with(opciones)
            .with_file_system_resolver(raiz.clone())
            .build();

        Ok(Self { engine, raiz })
    }

    /// Precalienta la caché de compilación de una plantilla (fuentes, parseo) para
    /// que el primer request no pague ese costo.
    pub fn precalentar(&self, ruta: &str) -> Result<(), RenderError> {
        self.renderizar(ruta, &serde_json::json!({})).map(|_| ())
    }
}

impl Renderer for TypstRenderer {
    fn motor(&self) -> &'static str {
        "typst"
    }

    fn renderizar(&self, ruta: &str, datos: &serde_json::Value) -> Result<Vec<u8>, RenderError> {
        let archivo = self.raiz.join(ruta);
        if !archivo.is_file() {
            return Err(RenderError::PlantillaNoDisponible(ruta.to_string()));
        }

        let mut inputs = Dict::new();
        inputs.insert("datos".into(), json_a_typst(datos));

        let id_virtual = format!("/{}", ruta.trim_start_matches('/'));
        let salida = self.engine.compile_with_input::<_, _, PagedDocument>(id_virtual.as_str(), inputs);

        for advertencia in &salida.warnings {
            // Las familias de respaldo declaradas en la plantilla (Liberation, DejaVu...) no
            // existen en todos los sistemas; eso es esperable y no merece un warning por render.
            if advertencia.message.contains("unknown font family") {
                tracing::debug!(plantilla = ruta, "{}", advertencia.message);
            } else {
                tracing::warn!(plantilla = ruta, "{}", advertencia.message);
            }
        }

        let documento = salida.output.map_err(|e| RenderError::Compilacion(e.to_string()))?;

        typst_pdf::pdf(&documento, &PdfOptions::default())
            .map_err(|errores| RenderError::Exportacion(format!("{errores:?}")))
    }
}
