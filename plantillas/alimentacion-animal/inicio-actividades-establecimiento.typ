// Plantilla "Comunicación de Inicio de Actividades para establecimientos de productos
// destinados a la alimentación animal", migrada desde v2
// (alimentacion-animal/inicio-actividades-establecimiento.html) a Typst.
//
// Recibe en `sys.inputs.datos` el mismo `DatosCredencial` que v2 (63 variables):
//   - Encabezado: folio, folioBase64 (data URL PNG del código de barras del folio).
//   - Texto: nombreCompleto, rutRepresentanteLegal, nombreEstablecimiento, rutEstablecimiento,
//     giro, nombreCalle, numeroCalle, ciudad, nombreComuna, nombreRegion, destinoProducto,
//     actividadComercial.
//   - Tabla "Fábricas o elaboradoras": produce_* (alimentos_completos, suplementos, aditivos,
//     ingredientes_vegetal/mineral/animal, *_medicados), var_animales_productor_* y
//     var_otro_productor_*.
//   - Tabla "Comercialización": comercia_*, var_animales_comercia_*, var_otro_comercia_*
//     (ojo: v2 usa `var_otro_comercia_suplemento`, en singular; se respeta).
//   - Tabla "Bodegas": calle_empresa, numero_empresa, dpto_oficina_local, ciudad_empresa,
//     region_empresa, comuna_empresa y `otras_bodegas`, que en v2 es un fragmento HTML con
//     filas `<tr><td>…</td>…</tr>` (el consumidor manda "<tr></tr>" cuando no hay). Como la
//     clave no termina en `Html`, la API no lo estructura: aquí se acepta tanto el string HTML
//     (se extraen las celdas con expresiones regulares) como un arreglo de filas
//     (arreglo de arreglos de strings) o de diccionarios con las mismas claves de la matriz.
//   - Pie: fechaEmision, qrBase64 (data URL PNG), urlVerificacion, nombreFirmante, cargoFirmante.
//
// Defectos de v2 corregidos: el `.footer` era `position:absolute; bottom:0` sin contenedor
// posicionado, así que los tres pies se apilaban al final de la página 1 (se veían en
// "negrita" por el triple dibujado) y las páginas 2 y 3 quedaban sin pie. Aquí el pie va en
// todas las páginas. La numeración "Pagina N de 3" estaba escrita a mano; aquí usa contadores.

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
#let imagen(k) = {
  let img = d.at(k, default: none)
  if type(img) == dictionary and "bytes" in img { img } else { none }
}
#let decodificar(s) = (
  s.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<")
    .replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").trim()
)
// Fragmento HTML "<tr><td>a</td><td>b</td></tr>…" -> ((a, b), …). Ignora filas sin celdas.
#let filas-desde-html(html) = {
  html.matches(regex("(?is)<tr[^>]*>(.*?)</tr>")).map(fila =>
    fila.captures.at(0).matches(regex("(?is)<t[dh][^>]*>(.*?)</t[dh]>"))
      .map(c => decodificar(c.captures.at(0).replace(regex("<[^>]*>"), "")))
  ).filter(f => f.len() > 0)
}
#let claves-bodega = ("nombre", "calle", "numero", "dpto", "ciudad", "region", "comuna")
#let otras-bodegas = {
  let ob = d.at("otras_bodegas", default: none)
  if ob == none { () }
  else if type(ob) == str { filas-desde-html(ob) }
  else if type(ob) == array {
    ob.map(f => if type(f) == array { f.map(como-texto) }
      else if type(f) == dictionary { claves-bodega.map(k => como-texto(f.at(k, default: ""))) }
      else { (como-texto(f),) })
  } else { () }
}

// ---------- Estilos ----------
#let sans = ("Helvetica", "Arial", "Liberation Sans", "DejaVu Sans")
#let gris-titulo = rgb(177, 179, 191)
#let barra(grosor) = block(width: 100%, height: grosor, fill: black, above: 0pt, below: 0pt)

// ---------- Encabezado (idéntico en las tres páginas de v2) ----------
#let encabezado = context block(width: 100%, height: 208pt, {
  set par(leading: 0.3em, spacing: 0pt)
  let folio = imagen("folioBase64")
  grid(
    columns: (80%, 20%),
    align: (left + horizon, right + top),
    // .leftDiv: h2 con margin-left 20 % (del 80 %), centrado verticalmente respecto de la columna derecha
    pad(left: 20%, text(size: 15.75pt, weight: "bold")[Servicio Agrícola y Ganadero]),
    // .rightDiv: hr1, "Folio:", código de barras, número
    {
      barra(4.5pt)
      v(4pt)
      pad(right: 45pt, text(size: 10.5pt)[Folio:])
      v(2pt)
      if folio != none { image(folio.bytes, format: folio.format, width: 120pt, height: 52.5pt, fit: "contain") } else { v(52.5pt) }
      v(2pt)
      align(center, text(size: 10.5pt, weight: "bold", val("folio")))
    },
  )
  v(6pt)
  grid(
    columns: (80%, 20%),
    align: (left + horizon, right + top),
    pad(left: 20%, y: 14pt, text(size: 10.5pt, weight: "bold")[https://www.sag.gob.cl]),
    pad(right: 26pt, y: 14pt, text(size: 9pt)[Pagina #counter(page).display() de #counter(page).final().first()]),
  )
  v(8pt)
  barra(4.5pt)
  v(14pt)
  align(center, {
    text(size: 15.75pt, weight: "bold")[Comunicación de Inicio de Actividades]
    v(12pt)
    text(size: 10.5pt)[para establecimientos de productos destinados a la alimentación animal]
  })
  v(12pt)
  barra(3pt)
})

// ---------- Pie (en v2 sólo se veía en la página 1 por el defecto descrito arriba) ----------
#let pie = {
  set par(leading: 0.3em, spacing: 0pt)
  set text(size: 10.5pt)
  let qr = imagen("qrBase64")
  line(length: 100%, stroke: 0.75pt + rgb("#9a9a9a"))
  v(8pt)
  [Fecha de Emisión: #h(19pt) #val("fechaEmision")]
  v(8pt)
  line(length: 100%, stroke: 0.75pt + rgb("#9a9a9a"))
  v(6pt)
  grid(
    columns: (auto, 1fr),
    align: (left + top, left + top),
    {
      [Código Verificación:]
      v(3pt)
      if qr != none { image(qr.bytes, format: qr.format, width: 90pt, height: 90pt) } else { v(90pt) }
    },
    // .divVerifyCodeRight: nombre (14px) y cargo (12px) con margin-left 7cm
    pad(left: 7cm - 90pt - 20pt, top: 6pt, {
      text(size: 10.5pt, val("nombreFirmante"))
      v(6pt)
      text(size: 9pt, val("cargoFirmante"))
    }),
  )
  v(6pt)
  line(length: 100%, stroke: 0.75pt + rgb("#9a9a9a"))
  v(8pt)
  text(size: 9pt)[Verifique la validez de este documento en: \ #val("urlVerificacion")]
  v(6pt)
  stack(dir: ltr, rect(width: 63.75pt, height: 15pt, fill: rgb("#0168b3"), stroke: none),
    rect(width: 63.75pt, height: 15pt, fill: rgb("#ee3a43"), stroke: none))
}

#set page(
  paper: "us-letter",
  margin: (x: 8pt, top: 8pt + 208pt + 10pt, bottom: 8pt + 205pt),
  header: encabezado,
  header-ascent: 10pt,
  footer: pie,
  footer-descent: 8pt,
)
// Sin partición de palabras: Chromium tampoco partía, y la silabación de Typst es lo más caro
// del render (con ella el documento tarda 2-3 veces más).
#set text(font: sans, size: 10.5pt, hyphenate: false)
#set par(leading: 0.3em, spacing: 12pt, justify: true)

// ---------- Componentes de tabla ----------
#let etiqueta(t) = text(weight: "bold", t)
// Las celdas de v2 no justifican ni parten palabras (text-align: center)
#let celda(..partes) = align(center + horizon, par(justify: false, partes.pos().join(" ")))
#let tabla(titulo, columnas, ..celdas) = table(
  columns: columnas,
  stroke: 1pt + black,
  inset: (x: 2pt, y: 2pt),
  align: center + horizon,
  table.header(table.cell(colspan: columnas.len(), fill: gris-titulo, inset: (x: 3.75pt, y: 7.5pt),
    text(fill: white, weight: "bold", titulo))),
  ..celdas.pos(),
)
#let seis = (1fr,) * 6

// ---------- Página 1: texto y fábricas ----------
#par[Por medio del presente documento, el Servicio Agrícola y Ganadero hace constar que el/la
  sr(a): #val("nombreCompleto") rut: #val("rutRepresentanteLegal") representante legal del
  establecimiento: #val("nombreEstablecimiento") rut: #val("rutEstablecimiento"), con giro #val("giro"),
  ubicado en la calle: #val("nombreCalle") número: #val("numeroCalle") en la ciudad: #val("ciudad") comuna:
  #val("nombreComuna") de la región: #val("nombreRegion"), ha presentado ante esta institución la
  comunicación de inicio de actividades para establecimientos de productos destinados a la
  alimentación animal. En el mismo acto, el/la sr(a): #val("nombreCompleto") también ha
  comunicado que el destino de los productos fabricados o elaborados es para:
  #val("destinoProducto") y que la actividad comercial de la empresa es: #val("actividadComercial").]

#tabla("FÁBRICAS O ELABORADORAS", seis,
  celda(etiqueta[Alimentos completos:], val("produce_alimentos_completos")),
  celda(etiqueta[Suplementos:], val("produce_suplementos")),
  celda(etiqueta[Aditivos], val("produce_aditivos")),
  celda(etiqueta[Ingredientes de origen vegetal:], val("produce_ingredientes_vegetal")),
  celda(etiqueta[Ingredientes de origen mineral:], val("produce_ingredientes_mineral")),
  celda(etiqueta[Ingredientes de origen animal:], val("produce_ingredientes_animal")),

  celda(etiqueta[¿Producto medicado?], val("produce_alimentos_medicados")),
  celda(etiqueta[¿Producto medicado?], val("produce_suplementos_medicados")),
  celda(etiqueta[¿Producto medicado?], val("produce_aditivos_medicados")),
  celda[N/A], celda[N/A], celda[N/A],

  celda(etiqueta[Animal(es) de destino:], val("var_animales_productor_alimentos"), val("var_otro_productor_alimentos")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_productor_suplementos"), val("var_otro_productor_suplementos")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_productor_aditivos"), val("var_otro_productor_aditivos")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_productor_vegetal"), val("var_otro_productor_vegetal")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_productor_mineral"), val("var_otro_productor_mineral")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_productor_animal")),
)

// ---------- Página 2: comercialización ----------
#pagebreak()
#tabla("COMERCIALIZACIÓN DEL PRODUCTO", seis,
  celda(etiqueta[Alimentos completos:], val("comercia_alimentos_completos")),
  celda(etiqueta[Suplementos:], val("comercia_suplementos")),
  celda(etiqueta[Aditivos:], val("comercia_aditivos")),
  celda(etiqueta[Ingredientes de origen vegetal:], val("comercia_ingredientes_vegetal")),
  celda(etiqueta[Ingredientes de origen mineral:], val("comercia_ingredientes_mineral")),
  celda(etiqueta[Ingredientes de origen animal:], val("comercia_ingredientes_animal")),

  celda(etiqueta[Animal(es) de destino:], val("var_animales_comercia_alimentos"), val("var_otro_comercia_alimentos")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_comercia_suplementos"), val("var_otro_comercia_suplemento")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_comercia_aditivos"), val("var_otro_comercia_aditivos")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_comercia_vegetal"), val("var_otro_comercia_vegetal")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_comercia_mineral"), val("var_otro_comercia_mineral")),
  celda(etiqueta[Animal(es) de destino:], val("var_animales_comercia_animal")),
)

// ---------- Página 3: bodegas ----------
#pagebreak()
#let fila-bodega(f) = range(7).map(i => celda(if i < f.len() { f.at(i) } else { "" }))
#tabla("BODEGAS", (auto, auto, auto, 1fr, auto, auto, auto),
  ..("Nombre bodega", "Calle", "Número", "Depto / Oficina / Local / Otro", "Ciudad", "Región", "Comuna").map(t => celda(etiqueta(t))),
  ..fila-bodega(("Matriz", val("calle_empresa"), val("numero_empresa"), val("dpto_oficina_local"),
    val("ciudad_empresa"), val("region_empresa"), val("comuna_empresa"))),
  ..otras-bodegas.map(fila-bodega).flatten(),
)
