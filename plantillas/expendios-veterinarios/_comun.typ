// Parte común de los certificados de inscripción de establecimientos de expendio de
// productos farmacéuticos de uso veterinario (F-RIS-AEM-PP-009), migrados desde v2
// (expendios-veterinarios/certificado-expendio-veterinario.html y su variante
// certificado-expendio-veterinario-ninguna.html, que sólo omite la línea
// "Este establecimiento está autorizado para comercializar").
//
// Variables de `DatosCredencial` (mismos nombres con guion que usa v2):
//   folio, razon-social, rut-del-establecimiento, direccion-establecimiento,
//   comuna-establecimiento, region-establecimiento, representante-legal,
//   rut-representante-legal, correo-representante-legal,
//   nombre-director-tecnico1, rut-director-tecnico1, profesion-1, correo-director-tecnico1,
//   nombre-director-tecnico2, rut-director-tecnico2, profesion-2, correo-director-tecnico2,
//   fecha-emision, horario-funcionamiento, tipo-de-establecimiento,
//   categoria-establecimiento, tipo-medicamento (sólo la variante completa),
//   region-rup, sector-rup, correlativo-fijo, correlativo-rup.
// Todas son texto plano; no hay imágenes en los datos (el logo es un asset de la plantilla).
//
// Defectos de v2 que aquí se corrigen: el bloque "Código/Versión/Folio/Página" tenía
// `top:-80px` y se salía de la hoja (en la página 2 quedaba cortado y su resto aparecía
// al pie de la página 1); el folio no cabía en su caja de 14 % y se cortaba en el borde;
// la barra bicolor del pie no tenía estilos y nunca se veía; `.bg-imagen` no se usaba.
// Aquí el encabezado es un encabezado de página real, con numeración por contador.

#import sys: inputs
#let d = inputs.at("datos", default: (:))

#let como-texto(x) = {
  if x == none { "" }
  else if type(x) == str { x }
  else if type(x) == bool { if x { "Sí" } else { "No" } }
  else if type(x) == int or type(x) == float { str(x) }
  else { repr(x) }
}
#let val(k) = como-texto(d.at(k, default: ""))

#let verde = rgb("#3BCF54")
#let gris-glosa = rgb(90, 90, 90)
#let serif = ("Times New Roman", "Times", "Liberation Serif", "DejaVu Serif")
#let sans = ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans")

// ---------- Encabezado (en v2 se repetía a mano en cada hoja) ----------
#let encabezado = context block(width: 100%, height: 112pt, {
  place(top + left, dx: 4pt, dy: 4pt, image("assets/logo-sag.png", width: 75pt, height: 75pt))
  place(top + left, dx: 34%, dy: 10pt,
    text(font: serif, size: 14pt, weight: "bold", fill: gray)[SERVICIO AGRÍCOLA Y GANADERO])
  place(top + left, dx: 44%, dy: 48pt,
    link("https://www.sag.gob.cl", text(font: serif, size: 12pt, fill: rgb("#0000ee"))[#underline[https://www.sag.gob.cl]]))
  place(top + left, dx: 80%, dy: 0pt, block(width: 20%, {
    set par(leading: 2pt, spacing: 0pt, justify: false)
    text(font: sans, size: 7.5pt, fill: gris-glosa)[Código: F-RIS-AEM-PP-009 \ Versión: 02 \ Fecha de Vigencia: 18/08/2021]
    v(7pt)
    text(font: serif, size: 12pt)[Folio:#val("folio") \ Página #counter(page).display() de #counter(page).final().first()]
  }))
})

// ---------- Componentes ----------
#let titulo-verde(t) = table(
  columns: (100%), stroke: 1pt + black, inset: (x: 3pt, y: 4.5pt), align: center,
  fill: verde, text(font: serif, size: 12pt, t),
)
#let fila(columnas, ..celdas) = table(
  columns: columnas, stroke: 1pt + black, inset: (x: 3pt, y: 4.5pt), align: left + horizon,
  ..celdas.pos().map(c => text(font: serif, size: 12pt, c)),
)
#let separacion = v(30pt)

#let pie = {
  line(length: 100%, stroke: 0.75pt + rgb("#a0a0a0"))
  v(2pt)
  text(font: serif, size: 12pt)[Fecha de Emisión: #h(19pt) #val("fecha-emision")]
  v(2pt)
  line(length: 100%, stroke: 0.75pt + rgb("#a0a0a0"))
  // Barra bicolor institucional (.bicolor .blue/.red): en v2 no tenía dimensiones y no se veía.
  v(6pt)
  stack(dir: ltr, rect(width: 64pt, height: 15pt, fill: rgb("#0168b3"), stroke: none),
    rect(width: 64pt, height: 15pt, fill: rgb("#ee3a43"), stroke: none))
}

#let certificado(con-tipo-medicamento: true) = {
  set page(
    paper: "us-letter",
    margin: (x: 30pt, top: 30pt + 112pt + 20pt, bottom: 30pt),
    header: encabezado,
    header-ascent: 20pt,
  )
  set text(font: serif, size: 12pt, hyphenate: false)
  set par(leading: 0.35em, spacing: 12pt, justify: true)
  // Las tablas de v2 van pegadas entre sí (border-collapse, sin márgenes).
  show table: set block(above: 0pt, below: 0pt)

  // ----- Hoja 1 -----
  block(width: 100%, stroke: (top: 2.25pt + black, bottom: 2.25pt + black), inset: (y: 14pt),
    align(center, text(font: sans, size: 14pt, weight: "bold")[
      Certificado de Inscripción de Establecimiento \
      de expendio de productos farmacéuticos de uso veterinario.
    ]))
  v(4pt)
  par[El/la Jefe/a de Oficina del Servicio Agrícola y Ganadero que suscribe, certifica que el siguiente
    establecimiento se encuentra inscrito como *Establecimiento de Expendio* de Productos Farmacéuticos de
    uso exclusivamente veterinario:]
  v(12pt)
  titulo-verde[Nombre o Razón Social y RUT del Establecimiento de Expendio]
  fila((1fr, auto), val("razon-social"), val("rut-del-establecimiento"))
  titulo-verde[Dirección (Comuna/Localidad/Región)]
  fila((auto, 1fr), val("direccion-establecimiento"), val("comuna-establecimiento") + " - " + val("region-establecimiento"))
  separacion
  titulo-verde[Nombre, RUT y correo electrónico Propietario o Representante Legal]
  fila((1fr, auto, auto), val("representante-legal"), val("rut-representante-legal"), val("correo-representante-legal"))
  separacion
  titulo-verde[Nombre, RUT, profesión y correo electrónico Director/a Técnico/a]
  fila((1fr, auto, auto, auto),
    val("nombre-director-tecnico1"), val("rut-director-tecnico1"), val("profesion-1"), val("correo-director-tecnico1"),
    val("nombre-director-tecnico2"), val("rut-director-tecnico2"), val("profesion-2"), val("correo-director-tecnico2"))
  v(34pt)
  pie

  // ----- Hoja 2 -----
  pagebreak()
  v(10pt)
  titulo-verde[Horario de funcionamiento del establecimiento]
  fila((100%), val("horario-funcionamiento"))
  v(12pt)
  par(justify: false)[
    *Tipo de Establecimiento:* #val("tipo-de-establecimiento") \
    *Categoría del Establecimiento:* #val("categoria-establecimiento") \
    #if con-tipo-medicamento [*Este establecimiento está autorizado para comercializar:* #val("tipo-medicamento") \ ]
  ]
  v(14pt)
  titulo-verde[*ROL ÚNICO PECUARIO (RUP)*]
  table(
    columns: (25%, 25%, 25%, 25%), stroke: 1pt + black, inset: (x: 3pt, y: 4.5pt), align: center + horizon,
    fill: (x, y) => if y == 0 { verde } else { none },
    table.cell(x: 0, y: 0)[Región], table.cell(x: 1, y: 0)[Sector], table.cell(x: 2, y: 0, colspan: 2)[Correlativo],
    val("region-rup"), val("sector-rup"), val("correlativo-fijo"), val("correlativo-rup"),
  )
  v(24pt)
  par[Por cuanto *da cumplimiento a los requisitos* señalados en el *Decreto del Ministerio de Agricultura Nº 25 de 2005*, Reglamento de Productos Farmacéuticas de Uso Exclusivamente Veterinario *y Resoluciones* del Servicio, *relacionados al expendio de productos farmacéuticos de uso veterinario*.]
  par[*Válido mientras se mantengan las condiciones que habilitaron su inscripción*.]
  v(20pt)
  pie
}
