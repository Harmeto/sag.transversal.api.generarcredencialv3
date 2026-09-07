// Plantilla "Bioseguridad Traspatio" (Pauta Bioseguridad en Avicultura de productores
// de la AFC o Traspatio, F-VYC-VIS-PP-005) migrada desde v2
// (emergencias-pecuarias/bioseguridad-traspatio.html + JavaScript) a Typst.
//
// Recibe en `sys.inputs.datos` el mismo `DatosCredencial` que arma el BFF de
// Emergencias Pecuarias (BioseguridadMapper.toDatosCredencialTraspatio):
//   nombreCompleto, nombreEstablecimiento, rupEstablecimiento, actividad, contacto,
//   nombreCalle, nombreRegion, tipoProduccion, cantidadAves,
//   secciones (arreglo o string JSON) = [{ titulo, preguntas: [{ numero, pregunta, respuesta }] } | { titulo, observacion }],
//   QR (data URL, generado por la API con GenerarQR/QRTag = "QR").
//
// Lo que en HTML hacía el JavaScript (recorrer `secciones`, colorear la respuesta,
// mostrar u ocultar el QR) lo hace la plantilla. Además se corrige un defecto de v2:
// los <span class="pag-actual"/"pag-total"> nunca se rellenaban, así que el PDF decía
// "Página  de ". Aquí se numera con contadores reales.

#import sys: inputs
#let d = inputs.at("datos", default: (:))

// ---------- Utilidades ----------
#let como-texto(x) = {
  if x == none { "" }
  else if type(x) == str { x }
  else if type(x) == bool { if x { "Sí" } else { "No" } }
  else if type(x) == int or type(x) == float { str(x) }
  else { repr(x) }
}
#let val(k) = como-texto(d.at(k, default: ""))
#let secciones = {
  let s = d.at("secciones", default: ())
  if s == none { () }
  else if type(s) == str { if s.trim() == "" { () } else { json(bytes(s)) } }
  else if type(s) == array { s }
  else { () }
}
// QR: la API convierte el data URL en (format, bytes); si viene otra cosa, no se muestra.
#let qr = {
  let q = d.at("QR", default: none)
  if type(q) == dictionary and "bytes" in q { q } else { none }
}

// Colores y etiquetas de respuesta (getRespuestaClass / getRespuestaLabel del script)
#let verde = rgb("#538135")
#let rojo = rgb("#c00000")
#let gris = rgb("#666666")
#let ambar = rgb("#bf8f00")
#let respuesta(r) = {
  let t = como-texto(r)
  if t == "SI" { text(fill: verde, weight: "bold", t) }
  else if t == "NO" { text(fill: rojo, weight: "bold", t) }
  else if t == "NA" or t == "No aplica" { text(fill: gris)[No aplica] }
  else if t == "PARCIAL" or t == "Cumple parcialmente" { text(fill: ambar, weight: "bold")[Cumple parcialmente] }
  else { t }
}

// ---------- Encabezado (se repite en cada página, como el <thead> del print-wrapper) ----------
#let encabezado = context table(
  columns: (68pt, 1fr, 120pt),
  rows: 58pt,
  stroke: 1pt + black,
  inset: 0pt,
  align: (center + horizon, center + horizon, left + horizon),
  // logo
  pad(3pt, image("assets/logo-sag-minagri.jpg", width: 52pt, height: 41pt, fit: "contain")),
  // título
  pad(y: 5pt, {
    set align(center)
    block(width: 100%, inset: (x: 9pt, bottom: 3pt), stroke: (bottom: 1pt + black), below: 4pt,
      text(size: 9pt, weight: "bold")[FORMULARIO])
    block(inset: (x: 9pt), below: 2pt, text(size: 7.5pt, weight: "bold")[SISTEMA OFICIAL DE BIOSEGURIDAD])
    block(inset: (x: 9pt), text(size: 7pt)[Pauta Bioseguridad en Avicultura de productores de la AFC o Traspatio.])
  }),
  // código / versión / vigencia / página
  pad(x: 8pt, y: 5pt, {
    set text(size: 7pt)
    set par(leading: 4.5pt)
    [*Código:* F-VYC-VIS-PP-005 \
     *Versión:* 02 \
     *Fecha de vigencia:* 13/11/2024 \
     *Página* #counter(page).display() de #counter(page).final().first()]
  }),
)

// ---------- Página ----------
// @page: letter, margin 1.5cm, margin-top 2.5cm. El encabezado vive en el margen superior.
#set page(
  paper: "us-letter",
  margin: (x: 1.5cm, top: 2.5cm + 58pt + 14pt, bottom: 1.5cm),
  header: encabezado,
  header-ascent: 14pt,
  // Marca de agua fija al centro (.watermark, 370px, opacity 0.08: el PNG ya trae la opacidad)
  background: place(center + horizon, dy: 5%, image("assets/sag-marca-agua.png", width: 278pt)),
)
#set text(font: ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans"), size: 7.5pt)
#set par(leading: 3pt, spacing: 0pt)

// ---------- Datos del establecimiento + QR (.layout-row) ----------
#let fila-dato(etiqueta, valor) = (
  align(center, text(weight: "bold", etiqueta)),
  valor,
)
#grid(
  columns: (1fr, 68pt),
  column-gutter: 8pt,
  align: (left + top, center + top),
  table(
    columns: (45%, 55%),
    stroke: 0.75pt + rgb("#333333"),
    inset: (x: 6pt, y: 4pt),
    align: (center + horizon, left + horizon),
    ..fila-dato("Titular", val("nombreCompleto")),
    ..fila-dato("Nombre del establecimiento", val("nombreEstablecimiento")),
    ..fila-dato("RUP establecimiento", val("rupEstablecimiento")),
    ..fila-dato("Actividad", val("actividad")),
    ..fila-dato("Contacto", val("contacto")),
    ..fila-dato("Dirección", val("nombreCalle")),
    ..fila-dato("Región", val("nombreRegion")),
    ..fila-dato("Tipo de Producción", val("tipoProduccion")),
    ..fila-dato("Cantidad de Aves", val("cantidadAves")),
  ),
  if qr != none { image(qr.bytes, format: qr.format, width: 60pt, height: 60pt) } else { [] },
)
#v(12pt)

// ---------- Secciones (.section-title + .questions-table) ----------
#let titulo-seccion(t) = block(
  width: 100%, inset: (left: 24pt, top: 10pt, bottom: 4pt), stroke: (bottom: 1.1pt + rgb("#333333")),
  above: 0pt, below: 0pt, sticky: true,
  text(size: 8.5pt, weight: "bold", t),
)
#let tabla-preguntas(filas) = table(
  columns: (24pt, 1fr, 98pt),
  stroke: (x, y) => (bottom: 0.75pt + rgb("#cccccc")),
  inset: (x: 4pt, y: 4pt),
  align: (left + top, left + top, right + top),
  ..filas.flatten(),
)

#for s in secciones {
  titulo-seccion(como-texto(s.at("titulo", default: "")))
  let preguntas = s.at("preguntas", default: none)
  if type(preguntas) == array and preguntas.len() > 0 {
    tabla-preguntas(preguntas.map(p => (
      como-texto(p.at("numero", default: "")),
      como-texto(p.at("pregunta", default: "")),
      respuesta(p.at("respuesta", default: "")),
    )))
  } else if "observacion" in s {
    let obs = como-texto(s.at("observacion", default: ""))
    let vacia = obs.trim() == "" or obs.trim() in ("null", "undefined")
    tabla-preguntas((("", if vacia { "Sin observaciones" } else { obs }, ""),))
  }
  v(12pt)
}
