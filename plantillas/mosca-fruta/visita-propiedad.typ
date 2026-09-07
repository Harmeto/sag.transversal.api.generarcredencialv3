// Plantilla "Registro de Visita a Propiedad" (Campaña Erradicación Mosca de la Fruta,
// PlantillaId 15) migrada desde v2 (mosca-fruta/visita-propiedad.html) a Typst.
//
// Recibe en `sys.inputs.datos` el `DatosCredencial` que arman sag.movil.api.moscafruta
// (CredencialTransformer) y sag.movil.api (MoscaFrutaController):
//   folio, fecha, numeroResolucion, jefeEquipo, numeroCuadrante, patenteVehiculo,
//   integrantesHtml -> la API agrega `integrantes` (arreglo de strings),
//   nadieEnCasaChecked / noDejoEntrarChecked ("checked" | ""),
//   direccion, nombrePropietario, rutPropietario, telefonoPropietario, correoPropietario,
//   latitude, longitude, accuracy, captureMethod, observacionesPropiedad,
//   hospedantesHtml -> la API agrega `hospedantes` (arreglo de filas; ver `hospedantes` abajo),
//   totalKilosDescargados, banco, tipoCuenta, numeroCuentaBancaria, telefonoContacto,
//   correoComprobante, firmaUrl (data URL -> se muestra; URL http -> no, v3 no hace red),
//   firmaPropietarioFecha, firmaSupervisorStatus, firmaSupervisorEmail, firmaSupervisorFecha,
//   codigoQR (QRTag del consumidor; también se acepta QR).
// status/statusText/fechaGeneracion/userName/userEmail/approvedBy/approvedAt/documentId
// llegan en el payload pero v2 no los imprimía (la .title-section está display:none).
//
// Imágenes externas de v2 (statics/logos/logo_sag.jpg y statics/timbres/TimbreFirmaSimple.png
// en storagestaticprod.blob.core.windows.net) pasan a ser assets locales, porque v3 no hace
// red al renderizar: assets/logo-sag.jpg (misma marca SAG/Minagri, copia del asset de
// emergencias-pecuarias) y assets/timbre-firma-simple.png (timbre "Firma Electrónica Simple"
// recuperado del PDF que Chromium generó en v2; conviene reemplazarlo por el PNG original).
//
// Diferencias deliberadas con v2: la cabecera de sección no queda huérfana al pie de página,
// una sección se parte sólo si es más alta que una página (como break-inside: avoid) y la
// jerarquía de `hospedantes` se reconstruye a partir del aplanado que hace la API.

#import sys: inputs
#let d = inputs.at("datos", default: (:))

// ---------- Paleta (misma que el CSS) ----------
#let texto = rgb("#333333")
#let gris = rgb("#666666")
#let gris-claro = rgb("#999999")
#let verde = rgb("#1a5f2a")
#let verde-claro = rgb("#2d8f47")
#let rojo = rgb("#c41e3a")
#let borde = rgb("#dddddd")
#let fondo = rgb("#f8f9fa")
#let fondo-verde = rgb("#e8f5e9")
#let azul = rgb("#1565c0")
#let azul-fondo = rgb("#e3f2fd")
#let azul-loc-fondo = rgb("#f0f7ff")
#let azul-loc-borde = rgb("#bbdefb")
#let indigo = rgb("#5c6bc0")
#let borde-tabla = rgb("#eeeeee")
#let amarillo-fondo = rgb("#fff8e1")
#let amarillo-borde = rgb("#ffca28")
#let ok-fondo = rgb("#d4edda")
#let ok-texto = rgb("#155724")

// ---------- Utilidades ----------
#let como-texto(x) = {
  if x == none { "" }
  else if type(x) == str { x }
  else if type(x) == bool { if x { "Sí" } else { "No" } }
  else if type(x) == int or type(x) == float { str(x) }
  else { repr(x) }
}
#let val(k) = como-texto(d.at(k, default: ""))
#let imagen-datos(k) = {
  let x = d.at(k, default: none)
  if type(x) == dictionary and "bytes" in x { x } else { none }
}
#let firma = imagen-datos("firmaUrl")
#let qr = {
  let q = imagen-datos("codigoQR")
  if q == none { imagen-datos("QR") } else { q }
}
#let marcado(k) = val(k).trim() == "checked"

// integrantes: arreglo de strings (la API lo deriva de integrantesHtml); "" si no hay.
#let integrantes = {
  let i = d.at("integrantes", default: ())
  if type(i) == array { i.map(como-texto).filter(t => t.trim() != "") }
  else if type(i) == str and i.trim() != "" { (i,) }
  else { () }
}

// hospedantes: la API convierte el fragmento hospedantesHtml en filas de celdas y
// conserva la tabla anidada de tratamientos de la 4ª celda como arreglo de filas:
//   [nombre, sinFrutos, kilos, [["Tipo","Litros","m²","Situación","Obs."], [tipo, litros, m2, situacion, obs], ...]]
//   [nombre, sinFrutos, kilos, [["Sin tratamientos"]]]   <- fila única de una celda (colspan) = nota
//   [nombre, sinFrutos, kilos, "Sin tratamientos"]       <- también aceptado como texto
// Cada hospedante = (nombre, sin-frutos, kilos, nota, tratamientos). Por robustez se
// acepta además la forma aplanada antigua ([tipo, litros, m2, situacion, obs] como fila suelta).
#let hospedantes = {
  let h = d.at("hospedantes", default: ())
  let out = ()
  if type(h) == array {
    for fila in h {
      if type(fila) != array { continue }
      let n = fila.len()
      let primera = if n > 0 { como-texto(fila.at(0)).trim() } else { "" }
      if n == 5 and out.len() > 0 {
        if primera != "Tipo" {
          let ultimo = out.pop()
          ultimo.tratamientos.push(fila.map(como-texto))
          out.push(ultimo)
        }
      } else if n >= 3 {
        let item = (
          nombre: primera,
          sin-frutos: como-texto(fila.at(1)).trim(),
          kilos: como-texto(fila.at(2)).trim(),
          nota: "",
          tratamientos: (),
        )
        if n == 4 {
          let cuarta = fila.at(3)
          if type(cuarta) == array {
            let filas = cuarta
              .filter(r => type(r) == array and r.len() > 0 and como-texto(r.at(0)).trim() != "Tipo")
              .map(r => r.map(como-texto))
            // una sola fila de una celda (colspan) es una nota, no un tratamiento
            if filas.len() == 1 and filas.at(0).len() == 1 {
              item.nota = filas.at(0).at(0).trim()
            } else {
              item.tratamientos = filas
            }
          } else {
            item.nota = como-texto(cuarta).trim()
          }
        }
        out.push(item)
      }
    }
  }
  out
}

// ---------- Página ----------
// @page: letter, margin 15mm; body Arial 10pt #333, line-height 1.4
#set page(paper: "us-letter", margin: 15mm)
// Con top-edge/bottom-edge = ascender/descender la caja de una línea mide ≈ 0.94em (métricas
// tipográficas de Arial); CSS usa line-height 1.4. Por eso el interlineado es 0.46em y los
// insets verticales de cada caja llevan además ≈ 0.23em por línea de texto.
#set text(font: ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans"), size: 10pt, fill: texto,
  top-edge: "ascender", bottom-edge: "descender")
#set par(leading: 0.46em, spacing: 0.46em)
#let mono = ("Courier New", "Liberation Mono", "Courier", "DejaVu Sans Mono")

// ---------- Componentes ----------
// .info-label
#let etiqueta(t) = block(below: 6pt, text(size: 8pt, fill: gris, upper(t)))
// .info-value (span dentro de .info-item flex column: ocupa todo el ancho)
#let valor(t, resaltado: false, tamano: 10pt, peso: "regular") = block(
  width: 100%, inset: (x: 7.5pt, y: 6.8pt), radius: 3pt,
  fill: if resaltado { fondo-verde } else { fondo },
  stroke: (left: 2.25pt + (if resaltado { verde-claro } else { verde })),
  text(size: tamano, weight: peso, t),
)
// .info-value con min-height (observaciones)
#let valor-min(t, alto) = layout(size => {
  let cuerpo = block(width: size.width, inset: (x: 7.5pt, y: 6.8pt), t)
  let h = measure(cuerpo).height
  block(
    width: 100%, height: calc.max(h, alto), inset: (x: 7.5pt, y: 6.8pt), radius: 3pt,
    fill: fondo, stroke: (left: 2.25pt + verde), t,
  )
})
// .info-item
#let dato(etq, v, resaltado: false) = { etiqueta(etq); valor(v, resaltado: resaltado) }

// .section: borde redondeado, cabecera gris con número en círculo verde.
// `breakable: auto` imita break-inside: avoid: la sección se mueve entera a la página
// siguiente si cabe en una página; si es más alta que una página, se parte.
#let caja-seccion(n, titulo, cuerpo, partible) = block(
  width: 100%, stroke: 0.75pt + borde, radius: 6pt, clip: true, breakable: partible,
  above: 0pt, below: 0pt,
  {
    block(
      width: 100%, fill: fondo, inset: (x: 11.25pt, y: 7.75pt), stroke: (bottom: 0.75pt + borde),
      below: 0pt, sticky: true, // la cabecera no queda huérfana al pie de página
      grid(
        columns: (15pt, auto), column-gutter: 7.5pt, align: horizon,
        box(width: 15pt, height: 15pt, fill: verde, radius: 50%,
          align(center + horizon, text(size: 10pt, fill: white, str(n)))),
        text(size: 11pt, weight: "bold", titulo),
      ),
    )
    block(width: 100%, inset: 11.25pt, above: 0pt, cuerpo)
  },
)
#let seccion(n, titulo, cuerpo, breakable: auto) = {
  if breakable == auto {
    layout(size => {
      let alto = measure(width: size.width, caja-seccion(n, titulo, cuerpo, true)).height
      caja-seccion(n, titulo, cuerpo, alto > size.height - 6pt)
    })
  } else { caja-seccion(n, titulo, cuerpo, breakable) }
  v(15pt, weak: true) // margin-bottom: 20px
}

// .checkbox-item
#let casilla(activa, etq) = grid(
  columns: (12pt, auto), column-gutter: 6pt, align: horizon,
  box(
    width: 12pt, height: 12pt, stroke: 1.5pt + verde, radius: 2.25pt,
    fill: if activa { verde } else { none },
    if activa {
      place(curve(
        stroke: 1.4pt + white,
        curve.move((2.6pt, 6.2pt)), curve.line((5pt, 8.6pt)), curve.line((9.6pt, 3.4pt)),
      ))
    },
  ),
  text(etq),
)

// .tag
#let tag(t) = box(inset: (x: 7.5pt, y: 5.1pt), fill: azul-fondo, radius: 11pt, baseline: 6.5pt,
  text(size: 9pt, fill: azul, t))

// Tabla anidada de tratamientos (.treatments-table)
#let tabla-tratamientos(filas) = pad(top: 6pt, block(
  width: 100%, stroke: 0.75pt + borde, radius: 3pt, clip: true, breakable: false,
  table(
    columns: (1.1fr, 0.8fr, 0.7fr, 1.2fr, 2fr),
    inset: (x: 4.5pt, y: 6.2pt),
    stroke: (x, y) => if y > 0 { (bottom: 0.75pt + borde-tabla) } else { none },
    fill: (x, y) => if y == 0 { indigo } else if calc.even(y) { fondo } else { none },
    align: left + top,
    table.header(..("Tipo", "Litros", "m²", "Situación", "Obs.").map(h =>
      text(size: 7pt, fill: white, weight: "bold", upper(h)))),
    ..filas.map(f => range(5).map(i => text(size: 8pt, if i < f.len() { f.at(i) } else { "" }))).flatten(),
  ),
))
#let nota-tratamientos(t) = pad(top: 6pt, block(
  width: 100%, stroke: 0.75pt + borde, radius: 3pt, inset: (x: 4.5pt, y: 6.2pt),
  text(size: 8pt, t),
))

// ---------- Encabezado ----------
#block(
  width: 100%, inset: (bottom: 11.25pt), stroke: (bottom: 2.25pt + rojo), below: 15pt,
  grid(
    columns: (auto, 1fr, auto), align: (left + top, center + top, right + top),
    // el <img> es inline: la caja de línea deja ~4.5pt bajo la línea base
    pad(bottom: 4.5pt, image("assets/logo-sag.jpg", height: 41.25pt)),
    pad(x: 7.5pt, top: 2.75pt, {
      block(below: 6.5pt, text(size: 12pt, weight: "bold", fill: verde)[Campaña Erradicación Mosca de la Fruta])
      text(size: 10pt)[Registro de Visita a Propiedad]
    }),
    pad(top: 1.85pt, stack(dir: ttb, spacing: 5pt,
      text(size: 8pt, fill: gris)[FOLIO],
      text(size: 14pt, weight: "bold", fill: rojo, font: mono, val("folio")),
    )),
  ),
)

// ---------- 1. Información General ----------
#seccion(1, "Información General", {
  grid(
    columns: (1fr, 1fr), gutter: 9pt,
    dato("Fecha de Visita", val("fecha"), resaltado: true),
    dato("N° Resolución", val("numeroResolucion")),
    dato("Jefe de Equipo", val("jefeEquipo")),
    dato("N° Cuadrante", val("numeroCuadrante")),
    dato("Patente Vehículo", val("patenteVehiculo")),
    {
      etiqueta("Integrantes del Equipo")
      set par(leading: 7pt)
      integrantes.map(tag).join([ #h(2pt) ])
    },
  )
  v(11.25pt)
  grid(
    columns: (auto, auto), column-gutter: 15pt, inset: (y: 6pt),
    casilla(marcado("nadieEnCasaChecked"), "No había nadie en casa"),
    casilla(marcado("noDejoEntrarChecked"), "No dejó entrar"),
  )
})

// ---------- 2. Datos de la Propiedad ----------
#seccion(2, "Datos de la Propiedad", {
  grid(
    columns: (1fr, 1fr), gutter: 9pt,
    grid.cell(colspan: 2, dato("Dirección", val("direccion"))),
    dato("Nombre Propietario", val("nombrePropietario")),
    dato("RUT Propietario", val("rutPropietario")),
    dato("Teléfono", val("telefonoPropietario")),
    dato("Correo Electrónico", val("correoPropietario")),
  )
  v(11.25pt)
  etiqueta("Ubicación GPS")
  block(
    width: 100%, fill: azul-loc-fondo, stroke: 0.75pt + azul-loc-borde, radius: 4.5pt, inset: (x: 7.5pt, y: 11pt),
    grid(
      columns: (18pt, auto, auto), column-gutter: 11.25pt, align: horizon,
      image("assets/pin-ubicacion.svg", width: 18pt),
      text(font: mono, size: 9pt, fill: azul)[Latitud: #val("latitude") \ Longitud: #val("longitude")],
      text(size: 8pt, fill: gris)[Precisión: #val("accuracy")m | Método: #val("captureMethod")],
    ),
  )
  v(11.25pt)
  etiqueta("Observaciones de la Propiedad")
  valor-min(val("observacionesPropiedad"), 30pt)
})

// ---------- 3. Hospedantes y Tratamientos ----------
#seccion(3, "Hospedantes y Tratamientos", {
  table(
    columns: (20%, 12%, 13%, 55%),
    inset: (x: 6pt, y: 9.5pt),
    stroke: (x, y) => if y > 0 { (bottom: 0.75pt + borde-tabla) } else { none },
    fill: (x, y) => if y == 0 { verde } else if calc.even(y) { fondo } else { none },
    align: (left + top, center + top, center + top, left + top),
    table.header(..("Hospedante", "Sin Frutos", "Kg Descargados", "Tratamientos Aplicados").map(h =>
      text(size: 8pt, fill: white, weight: "bold", upper(h)))),
    ..hospedantes.map(h => (
      text(size: 9pt, weight: "bold", h.nombre),
      text(size: 9pt, h.sin-frutos),
      text(size: 9pt, h.kilos),
      if h.tratamientos.len() > 0 { tabla-tratamientos(h.tratamientos) }
      else if h.nota == "" or h.nota == "-" { text(size: 9pt, if h.nota == "" { "" } else { "-" }) }
      else { nota-tratamientos(h.nota) },
    )).flatten(),
  )
  v(11.25pt)
  align(right, {
    text(size: 8pt, fill: gris)[TOTAL KILOS DESCARGADOS]
    h(4pt)
    box(inset: (x: 7.5pt, y: 7.3pt), radius: 3pt, fill: fondo-verde, stroke: (left: 2.25pt + verde-claro),
      baseline: 9pt, text(size: 12pt, weight: "bold")[#val("totalKilosDescargados") kg])
  })
})

// ---------- Última hoja ----------
#pagebreak()

// ---------- 4. Datos de Contacto y Bancarios ----------
#seccion(4, "Datos de Contacto y Bancarios", {
  block(
    width: 100%, fill: amarillo-fondo, stroke: 0.75pt + amarillo-borde, radius: 4.5pt, inset: 7.5pt,
    grid(
      columns: (1fr, 1fr, 1fr), gutter: 7.5pt,
      dato("Banco", val("banco")),
      dato("Tipo de Cuenta", val("tipoCuenta")),
      dato("N° Cuenta", val("numeroCuentaBancaria")),
    ),
  )
  v(11.25pt)
  grid(
    columns: (1fr, 1fr), gutter: 9pt,
    dato("Teléfono de Contacto", val("telefonoContacto")),
    dato("Correo para Comprobante", val("correoComprobante")),
  )
})

// ---------- 5. Firmas Digitales ----------
#let caja-firma(cuerpo, alto: auto) = block(
  width: 100%, height: alto, inset: 15pt, radius: 6pt,
  stroke: (paint: rgb("#cccccc"), thickness: 0.75pt, dash: "dashed"),
  align(center, cuerpo),
)
#let linea-firma = block(above: 3.75pt, below: 7.5pt, line(length: 80%, stroke: 0.75pt + texto))
#let nota-firma(t, arriba) = block(above: arriba, below: 0pt, text(size: 8pt, fill: gris, t))
#let etiqueta-firma(t) = block(inset: (y: 2pt), above: 0pt, below: 0pt, text(size: 9pt, fill: gris, t))

#let firma-propietario = {
      pad(y: 7.5pt,
        if firma != none { image(firma.bytes, format: firma.format, width: 112.5pt, height: 60pt, fit: "contain") }
        else { box(width: 112.5pt, height: 60pt) })
      linea-firma
      etiqueta-firma[Firma Propietario/a]
      nota-firma(val("nombrePropietario"), 7.4pt)
      nota-firma(val("firmaPropietarioFecha"), 6pt)
}
#let firma-supervisor = {
      pad(y: 7.5pt, image("assets/timbre-firma-simple.png", width: 112.5pt, height: 60pt, fit: "contain"))
      linea-firma
      etiqueta-firma[Firma Supervisor/a]
      block(width: 100%, above: 7.5pt, below: 0pt, inset: (x: 7.5pt, y: 5.6pt), radius: 3pt, fill: ok-fondo,
        text(size: 8pt, fill: ok-texto, val("firmaSupervisorStatus")))
      nota-firma(val("firmaSupervisorEmail"), 7.4pt)
      nota-firma(val("firmaSupervisorFecha"), 6pt)
}
// .signatures-grid: grid CSS de 2 columnas; las cajas se estiran al alto de la más alta
#seccion(5, "Firmas Digitales", {
  v(15pt)
  layout(size => {
    let ancho = (size.width - 22.5pt) / 2
    let alto = calc.max(
      measure(width: ancho, caja-firma(firma-propietario)).height,
      measure(width: ancho, caja-firma(firma-supervisor)).height,
    )
    grid(
      columns: (1fr, 1fr), column-gutter: 22.5pt,
      caja-firma(firma-propietario, alto: alto),
      caja-firma(firma-supervisor, alto: alto),
    )
  })
})

// ---------- Pie con QR ----------
#block(
  width: 100%, above: 22.5pt, inset: (top: 11.25pt), stroke: (top: 1.5pt + verde),
  grid(
    columns: (1fr, auto), align: (left + top, center + top),
    {
      set text(size: 8pt, fill: gris)
      set par(leading: 6pt)
      [Documento generado electrónicamente \
       Servicio Agrícola y Ganadero - SAG \
       Campaña Erradicación Mosca de la Fruta]
    },
    {
      if qr != none { image(qr.bytes, format: qr.format, width: 82.5pt, height: 82.5pt) }
      else { box(width: 82.5pt, height: 82.5pt) }
      block(above: 2.25pt, text(size: 7pt, fill: gris)[Escanee para descargar])
    },
  ),
)
