// Plantilla "Registro de Mortalidad de Aves Silvestres" migrada desde v2
// (emergencias-pecuarias/registro-mortalidad-aves-silvestres.html) a Typst.
//
// Recibe en `sys.inputs.datos` el mismo `DatosCredencial` que arma
// sag.emergenciaspecuarias.api (ApiGenerarCredencialService.generarComprobanteRegistroMortalidad):
//   numeroRegistro (uuid), fecha (dd-MM-yyyy), usuario, rup, nombreEstablecimiento,
//   codigoZona (uuid de la zona), nombreZona, observaciones,
//   especies = fragmento HTML: <table> con <th> de encabezado y un <tr> por especie
//              (nombre, muertos, eutanasiados, muestras), o el texto
//              "No se registraron especies silvestres" cuando no hay filas.
//
// Como la clave `especies` no lleva sufijo `Html`, la API no la estructura: la
// plantilla trae un parser mínimo de <tr>/<th>/<td> (y acepta también un arreglo de
// filas por si la API lo estructura en el futuro). La variable
// {titularEstablecimiento} del HTML original estaba comentada y no se muestra.
// El pie con el nombre del usuario, que en v2 iba fijo al final de la primera
// página, aquí se repite en cada página.

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

// Texto plano de un trozo de HTML: sin tags, entidades comunes decodificadas,
// espacios colapsados.
#let texto-html(s) = (s
  .replace(regex("<[^>]*>"), "")
  .replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
  .replace("&quot;", "\"").replace("&#39;", "'").replace("&#x27;", "'")
  .replace(regex("\\s+"), " ").trim())

// Filas de un fragmento HTML con <tr>/<th>/<td>: arreglo de filas, cada fila es un
// arreglo de (texto, es-encabezado). Un <thead> con <th> sueltos (sin <tr>), como
// el que genera el consumidor, se toma como fila de encabezado.
#let filas-html(html) = {
  let filas = ()
  for seg in html.split(regex("<tr[^>]*>")) {
    let celdas = (seg
      .matches(regex("(?s)<(td|th)[^>]*>(.*?)</(?:td|th)>"))
      .map(m => (texto-html(m.captures.at(1)), m.captures.at(0) == "th")))
    if celdas.len() > 0 { filas.push(celdas) }
  }
  filas
}

// Paleta del CSS original
#let gris-borde = rgb("#cccccc")
#let gris-fondo = rgb("#f2f2f2")
#let gris-titulo = rgb("#333333")
#let gris-pie = rgb("#666666")

// ---------- Página ----------
// @page: letter, margin 0.5cm. Marca de agua: .container::before, 70% x 70% centrado en
// (50%, 38%), opacity 0.1 (el PNG ya trae la opacidad); tamaño y posición medidos sobre el
// PDF de v2. Pie: .footer absoluto al fondo.
#set page(
  paper: "us-letter",
  margin: (x: 0.5cm, top: 0.5cm, bottom: 0.5cm + 50pt),
  background: place(center + horizon, dy: -60pt, image("assets/sag-marca-agua.png", width: 14.4cm)),
  footer: align(center, text(size: 10pt, fill: gris-pie, val("usuario"))),
  footer-descent: 31pt,   // el pie queda a la altura medida en el PDF de v2
)
// Cajas de línea como en CSS (line-height normal ≈ 1.15em): 0.92em sobre la base y 0.23em
// bajo ella, sin interlineado extra; así no dependemos de las métricas de la fuente.
#set text(font: ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans"), size: 12pt,
  top-edge: 0.92em, bottom-edge: -0.23em)
#set par(leading: 0em, spacing: 12pt)

// h1 de 14pt #333 con márgenes 0.67em
#let titulo(t) = block(above: 9.4pt, below: 9.4pt, sticky: true,
  text(size: 14pt, weight: "bold", fill: gris-titulo, t))

// .normal-table: sólo borde exterior #ccc; celdas con padding 8px, claves con fondo #f2f2f2
#let tabla-normal(columnas, alineacion, ancho: auto, ..celdas) = block(
  width: ancho, stroke: 0.75pt + gris-borde,
  table(columns: columnas, stroke: none, inset: 6pt, align: alineacion, ..celdas),
)
#let clave(t) = table.cell(fill: gris-fondo, t)

// ---------- Encabezado: logo, título y tabla de registro (.header, flex) ----------
#grid(
  columns: (62pt, 1fr, 225pt),
  column-gutter: 11pt,
  align: (left + horizon, left + horizon, left + horizon),
  image("assets/logo-sag-cuadrado.png", width: 62pt),
  text(size: 14pt, weight: "bold", fill: gris-titulo)[Registro de Mortalidad de Aves Silvestres],
  // .header-table (max-width 300px, margin-bottom 20px dentro del flex centrado)
  pad(bottom: 15pt, table(
    columns: (1fr, 1fr, 1fr),
    stroke: 0.75pt + gris-borde,
    inset: 6pt,
    align: left + horizon,
    table.header(clave[*Número registro*], clave[*Fecha*], clave[*Usuario*]),
    // el uuid no tiene espacios: se permite cortar tras cada guion, como hace Chromium
    text(size: 9.75pt, val("numeroRegistro").replace("-", "-\u{200b}")),
    text(size: 9.75pt, val("fecha")),
    text(size: 9.75pt, val("usuario")),
  )),
)

// ---------- Antecedentes del evento ----------
#titulo[Antecedentes del evento]
#tabla-normal((auto, auto), left + horizon,
  clave[RUP], val("rup"),
  clave[Nombre Establecimiento], val("nombreEstablecimiento"),
  clave[Código Zona], val("codigoZona"),
  clave[Nombre Zona], val("nombreZona"),
)
#v(22.5pt)

// ---------- Especies ----------
#titulo[Especies]
#let especies = d.at("especies", default: "")
#let filas = if type(especies) == array {
  especies.map(f => if type(f) == array { f.map(c => (como-texto(c), false)) } else { ((como-texto(f), false),) })
} else if type(especies) == str and especies.contains("<tr") {
  filas-html(especies)
} else { () }
#if filas.len() == 0 [
  #texto-html(como-texto(especies))
] else {
  let n = calc.max(..filas.map(f => f.len()))
  let alineacion = (x, y) => if x == 0 { left + horizon } else { center + horizon }
  // como en Chromium: la tabla ocupa el ancho completo y la primera columna es la más ancha
  tabla-normal((2.4fr,) + range(n - 1).map(i => 1fr), alineacion, ancho: 100%,
    ..filas.map(f => range(n).map(i => {
      let c = f.at(i, default: ("", false))
      if c.at(1) { clave(strong(c.at(0))) } else { c.at(0) }
    })).flatten(),
  )
}
#v(22.5pt)

// ---------- Observaciones ----------
#titulo[Observaciones]
#texto-html(val("observaciones"))
