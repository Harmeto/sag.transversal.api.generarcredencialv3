// Plantilla "Comunicación Inicio de Actividades Ley de Alcoholes" migrada desde v2
// (alcoholes/certificado_alcoholes.html, un formulario exportado desde Word con
// tablas anidadas) a Typst. Una sola página carta.
//
// OJO con el contrato: el extractor de v3 sólo reconoce variables `{a_b}` y por eso
// informó una única variable (`core`); en realidad la plantilla de v2 usa 63 variables
// con guiones (`{region-empresa}`, `{giro-vino-vinifero}`, ...). Todas llegan como
// texto plano dentro de `DatosCredencial` y se leen con su mismo nombre:
//   - Empresa: region-empresa, provincia-empresa, comuna-empresa, tipo-nombre, rut-tipo,
//     calle-empresa, codigo-comuna-empresa, telefono-celular-contacto.
//   - Género / tipo de persona (casillas): genero-masculino, genero-femenino, tipo-persona.
//   - Representante legal: representante-nombre, rut-representante, representante-ciudad,
//     representante-direccion.
//   - Giros (casillas): giro-negocio-productor/elaborador/distribuidor, giro-vino-vinifero,
//     giro-vino-uva-mesa, giro-vino-pipeno-elaboradorvp/-elaboradorye, giro-chicha-chicha-cruda/
//     -chicha-cocida/-manzana, giro-fabricas-vino-espumante-o-espumozo/-vino-gasificado/-sidra/
//     -vinagre/-cerveza/-licores/-bebidas-alcoholicas-vinagres, giro-expendio-publico-bar-restaurant-
//     boite-hotel/-botilleria-supermercado/-fuente-de-soda-cerveceria, giro-comercio-exterior-
//     importador-ext/-exportador-ext, giro-alcoholes-alcoholes-importador/-estilatorio/-distribuidor/
//     -distribuidor-envasador, giro-usuarios-farmacia/-ferreteria/-hospital/-industria-desnaturalizada/
//     -laboratorio/-expendio/-industria-naturalizada/-otros-giros-no-incluidos.
//   - Capacidad: capacidad-litros-madera/-barrica/-cemento/-acero/-otros, capacidad-total-litros.
//   - Cierre: giro-usuarios-descripcion-giros-no-incluidos (observaciones), fecha-emision-documento,
//     firmante-rut, core, firmante-nombre, firmante-cargo, firmante-region.
// Las casillas muestran el valor tal cual (v2 imprimía el texto, normalmente "X" o vacío);
// por robustez un booleano `true` se muestra como "X" y `false` como vacío.
//
// Defectos de v2 corregidos: las tablas anidadas con rowspan escalonados de Word hacían que
// Chromium desalineara algunas filas (83/84, 43/44) y dibujara bordes dobles; aquí el giro es
// una sola grilla. Se corrige la errata "DESMATIRALIZADO" -> "DESNATURALIZADO".

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
#let marca(k) = {
  let x = d.at(k, default: "")
  if type(x) == bool { if x { "X" } else { "" } } else { como-texto(x) }
}

// ---------- Estilos ----------
#let serif = ("Times New Roman", "Times", "Liberation Serif", "DejaVu Serif")
#let sans = ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans")
#let b = 0.75pt + black
#let ancho = 600pt   // carta (612pt) menos los 8px de margen del body a cada lado

#set page(paper: "us-letter", margin: 6pt)
#set text(font: serif, size: 6pt, hyphenate: false)
#set par(leading: 1pt, spacing: 0pt)
#show table: set block(above: 0pt, below: 0pt)

// Tabla con bordes completos y celdas compactas. Typst mide la línea por la altura de las
// mayúsculas (~4pt a 6pt), así que el inset vertical de 1,7pt deja filas de ~7,4pt como las
// de Chromium (line-height 6,55pt + márgenes de párrafo de Word).
#let inset-celda = (x: 1pt, y: 1.7pt)
#let tabla(columnas, ..celdas) = table(
  columns: columnas, stroke: b, inset: inset-celda, align: left + top, ..celdas,
)
#let centro(t) = align(center, t)
#let casilla(k) = h(4pt) + box(width: 8pt, height: 8pt, stroke: b, baseline: 2pt,
  align(center + horizon, text(size: 6.5pt, marca(k))))

// ---------- Encabezado ----------
#place(top + left, dx: 3.75pt, dy: 3.75pt, image("assets/logo-sag.png", width: 49.5pt, height: 49.5pt))
#v(6pt)
#align(center, text(size: 14pt, weight: "bold", fill: gray)[SERVICIO AGRICOLA Y GANADERO])
#v(18pt)
#align(center, text(size: 8pt)[COMUNICACIÓN INICIO DE ACTIVIDADES LEY DE ALCOHOLES])
#v(37pt)

// ---------- Región / Provincia / Comuna (tabla de 218px al 75,5 % del ancho) ----------
#align(right, tabla((50pt, 98pt),
  [REGION], val("region-empresa"),
  [PROVINCIA], val("provincia-empresa"),
  [COMUNA], val("comuna-empresa"),
))
#v(16pt)

// ---------- Nombre o razón social ----------
#tabla((1fr, 44.5pt, 86.5pt),
  centro[NOMBRE O RAZON SOCIAL], centro[N° RUT], centro[CIUDAD],
  val("tipo-nombre"), centro(val("rut-tipo")), val("comuna-empresa"),
)
#v(9pt)

// ---------- Dirección + género / tipo de persona ----------
#tabla((1fr, 44.5pt, 86.5pt),
  centro[DIRECCIÓN], [COD.COMUNA], centro[TELEFONO],
  val("calle-empresa"), text(size: 5pt, val("codigo-comuna-empresa")), val("telefono-celular-contacto"),
)
#v(8pt)
#pad(left: 2pt, grid(
  columns: (30pt, 12pt, 14pt, 30pt, 12pt, 14pt, 28pt, 12pt),
  align: (right + horizon, left + horizon),
  [Masculino], casilla("genero-masculino"), [],
  [Femenino], casilla("genero-femenino"), [],
  align(right)[Persona \ Jurídica], casilla("tipo-persona"),
))
#v(21pt)

// ---------- Representante legal ----------
#tabla((1fr, 44.5pt, 86.5pt),
  centro[NOMBRE REPRESENTANTE LEGAL], centro(text(size: 7pt)[RUT]), centro(text(size: 7pt)[CIUDAD]),
  text(size: 7pt, val("representante-nombre")), centro(val("rut-representante")), text(size: 7pt, val("representante-ciudad")),
)
#v(9pt)

// ---------- Formulario principal (una caja continua formada por tablas apiladas) ----------
#tabla((1fr, 44.5pt, 86.5pt),
  centro[DIRECCIÓN], centro[CASILLA], centro[TELEFONO],
  val("representante-direccion"), [], val("telefono-celular-contacto"),
)
#tabla((100%),
  centro(text(font: sans, weight: "bold")[Si se trata de una modificación o eliminación del Inicio de Actividades ya registrado en el Servicio, debe especificarse en Observaciones]),
  centro(text(font: sans, weight: "bold", size: 7pt)[GIRO DEL NEGOCIO]),
)

// Grilla de giros: código | casilla | descripción, a la izquierda y a la derecha, con una
// columna separadora sin bordes. Los títulos de grupo no llevan bordes internos.
#let fila-giro(codigo, k, etiqueta) = (
  table.cell(stroke: b, codigo),
  table.cell(stroke: b, centro(marca(k))),
  table.cell(stroke: (top: b, bottom: b, right: b), etiqueta),
)
#let separador = table.cell(stroke: none)[]
#let titulo-izq(t, ..args) = table.cell(colspan: 3, stroke: (left: b), ..args, pad(left: 62pt, top: 3.5pt, t))
#let titulo-der(t, ..args) = table.cell(colspan: 3, stroke: (right: b), ..args, pad(left: 80pt, top: 3.5pt, t))
#let fila-vino = table.cell(colspan: 3, stroke: (left: b), pad(top: 2pt, bottom: 4pt, grid(
  columns: (34pt, 60pt, 16pt, 86pt, 16pt),
  align: (left + top, right + top, left + top, right + top, left + top),
  [VINO:], [VINIFERO], casilla("giro-vino-vinifero"), align(right)[UVA \ MESA], casilla("giro-vino-uva-mesa"),
)))

#table(
  columns: (16pt, 50pt, 213pt, 25pt, 22pt, 58pt, 1fr),
  inset: inset-celda,
  align: left + top,
  stroke: none,
  // 1
  fila-vino, separador, titulo-der[EXPENDIO A PUBLICO],
  // 2-4
  ..fila-giro("11", "giro-negocio-productor", "PRODUCTOR"), separador,
  ..fila-giro("61", "giro-expendio-publico-bar-restaurant-boite-hotel", "BAR, RESTAURANT, BOITE, HOTEL"),
  ..fila-giro("12", "giro-negocio-elaborador", "ELABORADOR Y ENVASADOR"), separador,
  ..fila-giro("62", "giro-expendio-publico-botilleria-supermercado", "BOTILLERIA, SUPERMERCADO"),
  ..fila-giro("13", "giro-negocio-distribuidor", "DISTRIBUIDOR"), separador,
  ..fila-giro("63", "giro-expendio-publico-fuente-de-soda-cerveceria", "FUENTE DE SODA, CERVECERIA"),
  // 5
  titulo-izq[VINO PIPEÑO], separador, titulo-der[COMERCIO EXTERIOR DE BEBIDAS ALCOHOLICAS (EXCLUIDO ALCOHOL)],
  // 6-7
  ..fila-giro("15", "giro-vino-pipeno-elaboradorvp", "ENVASADOR"), separador,
  ..fila-giro("71", "giro-comercio-exterior-importador-ext", "IMPORTADOR"),
  ..fila-giro("16", "giro-vino-pipeno-elaboradorye", "ELABORADOR Y ENVASADOR"), separador,
  ..fila-giro("72", "giro-comercio-exterior-exportador-ext", "EXPORTADOR"),
  // 8
  titulo-izq[CHICHA], separador, titulo-der[ALCOHOLES (UNICAMENTE ETILICOS)],
  // 9-10
  ..fila-giro("21", "giro-chicha-chicha-cruda", "PRODUCTOR CHICHA CRUDA"), separador,
  ..fila-giro("81", "giro-alcoholes-alcoholes-importador", "IMPORTADOR"),
  ..fila-giro("22", "giro-chicha-chicha-cocida", "PRODUCTOR CHICHA COCIDA"), separador,
  ..fila-giro("82", "giro-alcoholes-alcoholes-estilatorio", "DESTILATORIO"),
  // 11-12 (el título de la izquierda abarca dos filas, como en v2)
  titulo-izq(rowspan: 2)[CHICHA DE MANZANA], separador,
  ..fila-giro("83", "giro-alcoholes-alcoholes-distribuidor", "DISTRIBUIDOR"),
  separador,
  ..fila-giro("84", "giro-alcoholes-alcoholes-distribuidor-envasador", "DISTRIBUIDOR Y ENVASADOR"),
  // 13
  ..fila-giro("31", "giro-chicha-manzana", "PRODUCTOR"), separador, titulo-der[USUARIOS],
  // 14
  titulo-izq[FABRICAS], separador, ..fila-giro("91", "giro-usuarios-farmacia", "FARMACIA"),
  // 15-20
  ..fila-giro("41", "giro-fabricas-vino-espumante-o-espumozo", "DE VINO ESPUMANTE O VINO ESPUMOSO"), separador,
  ..fila-giro("92", "giro-usuarios-ferreteria", "FERRETERIA"),
  ..fila-giro("42", "giro-fabricas-vino-gasificado", "DE VINO GASIFICADO"), separador,
  ..fila-giro("93", "giro-usuarios-hospital", "HOSPITAL, CLINICA, C.MEDICO, C.VETER, ETC."),
  ..fila-giro("43", "giro-fabricas-sidra", "DE SIDRA"), separador,
  ..fila-giro("94", "giro-usuarios-industria-desnaturalizada", "INDUSTRIA (DESNATURALIZADO)"),
  ..fila-giro("44", "giro-fabricas-vinagre", "DE VINAGRE"), separador,
  ..fila-giro("95", "giro-usuarios-laboratorio", "LABORATORIO, DROGUERIA, F.COLONIA, ETC."),
  ..fila-giro("45", "giro-fabricas-cerveza", "DE CERVEZA"), separador,
  ..fila-giro("96", "giro-usuarios-expendio", "EXPENDIO ALCOHOL ENVASADO"),
  ..fila-giro("46", "giro-fabricas-licores", "DE LICORES"), separador,
  ..fila-giro("97", "giro-usuarios-industria-naturalizada", "INDUSTRIA (SIN DESNATURALIZAR)"),
  // 21: fila vacía
  table.cell(colspan: 7, stroke: (left: b, right: b), v(7pt)),
  // 22
  ..fila-giro("51", "giro-fabricas-bebidas-alcoholicas-vinagres", "DISTRIBUIDORES DE BEBIDAS ALCOHOLICAS Y VINAGRES"), separador,
  ..fila-giro("98", "giro-usuarios-otros-giros-no-incluidos", "OTROS GIROS NO INCLUIDOS EN LA NOMINA PRECEDENTE SEÑALARLO EN OBSERVACIONES"),
)

#tabla((100%), [UNICAMENTE PARA BODEGAS PRODUCTORAS, ELABORADORAS, ENVASADORAS DE VINO Y CHICHAS:])

// Capacidad de la bodega
#let derecha(t) = align(right, t)
#table(
  columns: (233pt, 71pt, 118pt, 45pt, 44.5pt, 1fr),
  stroke: b, inset: inset-celda, align: left + top,
  derecha[CAPACIDAD DE LA BODEGA:], centro[MADERA], centro[BARRICA], centro[CEMENTO], centro[ACERO], centro[OTROS],
  derecha[HECTOLITROS:], derecha(val("capacidad-litros-madera")), derecha(val("capacidad-litros-barrica")),
  derecha(val("capacidad-litros-cemento")), derecha(val("capacidad-litros-acero")), derecha(val("capacidad-litros-otros")),
  table.cell(colspan: 3)[], table.cell(colspan: 2)[TOTAL HECTOLITROS BODEGA:], derecha(val("capacidad-total-litros")),
)

// Observaciones / fecha y RUT / CORE (sin líneas verticales internas)
#table(
  columns: (1fr, 90pt),
  rows: (23pt, 20pt),
  inset: inset-celda,
  stroke: none,
  table.cell(stroke: (left: b, top: b, bottom: b), align: left + top)[OBSERVACIONES: #val("giro-usuarios-descripcion-giros-no-incluidos")],
  table.cell(stroke: (right: b, top: b, bottom: b), align: left + bottom)[FECHA:#val("fecha-emision-documento")],
  table.cell(stroke: (left: b, bottom: b), align: left + bottom)[RUT: #val("firmante-rut")],
  table.cell(stroke: (right: b, bottom: b), align: left + bottom)[CORE:#val("core")],
)

// ---------- Timbre / firmante (#timbre: 400px de ancho, 240px a la derecha del centro) ----------
#v(140pt)
#pad(left: 317pt, block(width: 300pt, align(center, {
  set text(size: 15pt, weight: "bold")
  set par(leading: 6pt)
  [#val("firmante-nombre") \ #val("firmante-cargo") \ #val("firmante-region")]
})))
