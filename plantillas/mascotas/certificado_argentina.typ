// Plantilla "Certificado Argentina" (Certificado zoosanitario para la exportación
// temporal o definitiva de perros y gatos domésticos a la República Argentina)
// migrada desde v2 (mascotas/certificado_argentina.html) a Typst.
//
// Recibe en `sys.inputs.datos` el mismo `DatosCredencial` que arma
// sag.tramite.api.mascotas (CertificadoService.prepareDataCredencialStore). Ojo: varias
// claves llevan guion, no guion bajo:
//   folio, nombre-exportador, direccion-exportador, rut-exportador, especie,
//   nombre_mascota, numero_microchip, fecha_lugar_microchip ("xxxxx" cuando hay registro
//   municipal), fecha_nacimiento_mascota, sexo, raza, color, medio-transporte,
//   control-fronterizo, pais-escala, control-fronterizo-argentino, observaciones
//   (un NBSP cuando viene vacía), fecha-emision (dd-MM-yyyy HH:mm), parrafo-vacunas,
//   vacunacion_antirrabica y tratamiento_antiparasitario = fragmentos HTML con un <tr>
//   por fila (6 y 5 <td> respectivamente), generados por certificado_helper.ts.
//
// Como esas dos claves no llevan sufijo `Html`, la API no las estructura: la plantilla
// trae un parser mínimo de <tr>/<td> (y acepta un arreglo de filas por si la API lo
// estructura en el futuro).
//
// Sobre las medidas: el HTML de v2 no tiene @page y el folio está en `left: 750px`, lo
// que desborda la hoja carta; Chromium encoge toda la página (~0.9) para que quepa. Aquí
// se reproduce ese factor (`k`) para que tamaños y posiciones coincidan con el PDF de v2.
//
// Diferencias deliberadas con v2:
//   - v2 tenía un `.bg-imagen` (patrón "SAG") definido en CSS pero nunca usado; no se
//     migró esa imagen.
//   - "Página N de 2" se numera con contadores reales (v2 lo tenía escrito a mano).
//   - El documento es siempre de dos páginas, como en v2 (salto forzado).

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
#let val(k) = como-texto(d.at(k, default: "")).replace("\u{a0}", " ").trim()

// Texto plano de un trozo de HTML: sin tags, entidades comunes decodificadas, espacios colapsados.
#let texto-html(s) = (s
  .replace(regex("<[^>]*>"), "")
  .replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
  .replace("&quot;", "\"").replace("&#39;", "'").replace("&#x27;", "'")
  .replace(regex("\\s+"), " ").trim())

// Filas (arreglo de arreglos de `n` strings) desde un fragmento HTML con <tr>/<td>,
// o desde un arreglo ya estructurado.
#let filas-de(k, n) = {
  let x = d.at(k, default: "")
  let filas = if type(x) == array {
    x.map(f => if type(f) == array { f.map(como-texto) } else { (como-texto(f),) })
  } else if type(x) == str and x.contains("<tr") {
    (x.split(regex("<tr[^>]*>"))
      .map(seg => seg.matches(regex("(?s)<(?:td|th)[^>]*>(.*?)</(?:td|th)>")).map(m => texto-html(m.captures.at(0))))
      .filter(f => f.len() > 0))
  } else { () }
  filas.map(f => range(n).map(i => f.at(i, default: "")))
}

// ---------- Medidas ----------
// k = factor con que Chromium encogió la página de v2; px(n) convierte px CSS a pt.
#let k = 0.9
#let px(n) = n * 0.75pt * k
#let cuerpo = px(16)      // 16px Times por defecto del navegador
#let tam-h3 = px(18.72)   // h3 = 1.17em

#let serif = ("Times New Roman", "Liberation Serif", "Libertinus Serif", "DejaVu Serif")
#let sans = ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans")
#let gris = rgb("#808080")

// ---------- Página ----------
// Sin @page en v2: carta, márgenes 0 de Puppeteer + body margin 20px + padding 20px.
#set page(paper: "us-letter", margin: (x: px(40), top: px(40), bottom: px(40)))
// Cajas de línea como en CSS (line-height normal ≈ 1.15em): 0.92em sobre la base y 0.23em
// bajo ella, sin interlineado extra; así no dependemos de las métricas de la fuente.
#set text(font: serif, size: cuerpo, top-edge: 0.92em, bottom-edge: -0.23em)
#set par(leading: 0em, spacing: cuerpo, justify: false)

// h3: 1.17em bold con márgenes 1em
#let h3(t) = block(above: tam-h3, below: tam-h3, text(size: tam-h3, weight: "bold", t))
// hr por defecto del navegador (1px inset, margen 0.5em)
#let hr = block(above: 0.5 * cuerpo, below: 0.5 * cuerpo, line(length: 100%, stroke: 0.7pt + rgb("#9a9a9a")))

// Encabezado de cada hoja (.header, 150px de alto): logo absoluto, título gris corrido
// 250px a la derecha, folio absoluto a la derecha, enlace centrado y "Página N de 2".
// `folio-dy`: en v2 el folio de la hoja 2 estaba en `top: 1300px`, más abajo que en la 1.
#let encabezado(folio-dy) = context block(width: 100%, height: px(150), {
  // el logo queda unos 15pt más abajo de lo que dice el CSS (así lo pinta Chromium)
  place(top + left, dx: px(5), dy: 15pt, image("assets/logo-sag-minagri.png", width: px(100), height: px(100)))
  place(top + left, dx: px(250), dy: 23pt, block(width: 60%,
    text(size: tam-h3, weight: "bold", fill: gris)[MINISTERIO DE AGRICULTURA \ SERVICIO AGRÍCOLA Y GANADERO]))
  place(top + left, dx: px(710), dy: folio-dy, val("folio"))
  place(top + left, dy: 65pt, stack(dir: ttb, spacing: 1.5pt,
    align(center, link("https://www.sag.gob.cl", text(fill: rgb("#0000ee"), underline[https://www.sag.gob.cl]))),
    align(right)[Página #counter(page).display() de #counter(page).final().first()],
  ))
})

// Tabla con bordes negros (border-collapse, th/td border 1px)
#let tabla(columnas, inset, ..celdas) = table(
  columns: columnas, stroke: 0.7pt + black, inset: inset, align: left + horizon, ..celdas,
)
#let th(t) = table.cell(align: center + horizon, strong(t))

// ======================= Hoja 1 =======================
#encabezado(-4pt)
#v(1.1 * cuerpo)
#block(width: 100%, stroke: (top: px(3) + black, bottom: px(3) + black), inset: (y: tam-h3 + 3.5pt),
  above: 0pt, below: 18pt,
  align(center, text(font: sans, size: tam-h3, weight: "bold")[
    CERTIFICADO ZOOSANITARIO PARA LA EXPORTACIÓN TEMPORAL O \
    DEFINITIVA, DE PERROS Y GATOS DOMÉSTICOS A LA REPÚBLICA ARGENTINA.
  ]))

#h3[1. ANTECEDENTES GENERALES]
*NOMBRE Y DIRECCIÓN DE EXPORTADOR:* #val("nombre-exportador") - #val("direccion-exportador") \
*RUT DEL EXPORTADOR:* #val("rut-exportador") \
*ESPECIE(CANINO O FELINO):* #val("especie")
#v(1.1 * cuerpo)

*DATOS DE LA MASCOTA*
#block(above: 0pt, tabla((9fr, 14.8fr, 32.5fr, 19.2fr, 5.6fr, 11.5fr, 7.4fr), 2pt,
  th[NOMBRE], th[N° MICROCHIP], th[FECHA Y LUGAR DE APLICACIÓN DE MICROCHIP], th[FECHA DE NACIMIENTO], th[SEXO], th[RAZA], th[COLOR],
  val("nombre_mascota"), val("numero_microchip"), val("fecha_lugar_microchip"), val("fecha_nacimiento_mascota"),
  val("sexo"), val("raza"), val("color"),
))

*MEDIO DE TRANSPORTE(AVIÓN, VEHÍCULO O BARCO):* #val("medio-transporte") \
*CONTROL FRONTERIZO DE SALIDA DE CHILE:* #val("control-fronterizo") \
*PAÍS DE ESCALA O TRÁNSITO(SI CORRESPONDE):* #val("pais-escala") \
*CONTROL FRONTERIZO DE INGRESO A ARGENTINA:* #val("control-fronterizo-argentino")

#h3[2. RESPECTO AL EXAMEN CLÍNICO]
#par(justify: true)[
  El animal fue inspeccionado dentro de los 10 días antes de su salida de Chile por un Médico
  Veterinario, quien lo ha encontrado en condición normal de salud y apto para el viaje, sin
  presentar TUMORACIONES, HERIDAS FRESCAS o EN PROCESO DE CICATRIZACIÓN, ni signo alguno de
  ENFERMEDADES CUARENTENABLES O TRANSMISIBLES O PRESENCIA DE ECTOPARÁSITOS.
]

#h3[3. OBSERVACIONES]
#texto-html(val("observaciones"))

#hr
Fecha de emisión: #val("fecha-emision")
#hr

// ======================= Hoja 2 =======================
#pagebreak()
#encabezado(59pt)
#v(1.1 * cuerpo)
*4. INFORMACIÓN DE VACUNA ANTIRRÁBICA*

La vacunación antirrábica se encuentra dentro de su fecha de vigencia
#v(1.1 * cuerpo)

#block(above: 0pt, tabla((22.3fr, 16.8fr, 16fr, 18.4fr, 12.8fr, 13.8fr), px(4),
  th[Identificación de la mascota], th[Fecha de vacunación], th[Nombre de la vacuna],
  th[Laboratorio productor], th[Número de lote], th[Fecha de vigencia],
  ..filas-de("vacunacion_antirrabica", 6).flatten(),
))

#par(justify: true, texto-html(val("parrafo-vacunas")))

*5. INFORMACIÓN DEL TRATAMIENTO ANTIPARASITARIO*

#par(justify: true)[
  El animal ha sido sometido dentro de los quince (15) días previos a la fecha de emisión del
  presente certificado, a un tratamiento de amplio espectro contra parásitos internos y externos
  con productos autorizados por la Autoridad Veterinaria Competente de Chile:
]

#tabla((22.6fr, 20.8fr, 16.6fr, 15.3fr, 24.6fr), px(4),
  th[Tratamiento antiparasitario], th[Fecha de administración], th[Laboratorio],
  th[Nombre comercial], th[Principio activo del producto],
  ..filas-de("tratamiento_antiparasitario", 5).flatten(),
)

#par(justify: true, text(size: px(10))[
  Notas: \
  (1): SENASA considera temporal a una permanencia de los animales igual o menor a sesenta (60)
  días. \
  (2): Para amparar el retorno de animales a Chile, este documento tendrá una vigencia de
  sesenta (60) días a contar de la fecha de emisión. Siempre que se mantenga la vigencia de la
  vacuna antirrábica y el estado de salud de los animales.
])

#hr
Fecha de emisión: #val("fecha-emision")
#hr
