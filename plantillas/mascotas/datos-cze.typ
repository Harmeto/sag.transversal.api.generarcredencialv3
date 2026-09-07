// Plantilla "Datos CZE" (solicitud de exportación de perros y gatos) migrada desde
// v2 (mascotas/datos-cze.html + JavaScript) a Typst.
//
// Recibe en `sys.inputs.datos` el mismo objeto `DatosCredencial` que v2. Las
// mascotas llegan como arreglo (o como string JSON, por compatibilidad con el
// consumidor actual, que hoy las envía serializadas para el script de la
// plantilla HTML). Lo que en HTML hacía el JavaScript (clonar la página 2 por
// mascota, elegir la tabla de identificación según `microchip.tipo`, formatear
// fechas ISO a dd-mm-aaaa) aquí lo hace el propio lenguaje de la plantilla.

#import sys: inputs
#let d = inputs.at("datos", default: (:))

// ---------- Paleta (misma que el CSS original) ----------
#let gris-oscuro = rgb("#2c3e50")
#let gris = rgb("#7f8c8d")
#let gris-texto = rgb("#666666")
#let borde = rgb("#bdc3c7")
#let fondo = rgb("#f8f9fa")
#let azul = rgb("#3498db")
#let verde = rgb("#27ae60")
#let texto = rgb("#333333")
#let color-basico = rgb("#e74c3c")
#let color-microchip = rgb("#f39c12")
#let color-externo = rgb("#e67e22")
#let color-vet = rgb("#9b59b6")
#let color-vacuna = rgb("#1abc9c")
#let color-despar = rgb("#34495e")

// ---------- Utilidades ----------
#let como-texto(x) = {
  if x == none { "" }
  else if type(x) == str { x }
  else if type(x) == bool { if x { "Sí" } else { "No" } }
  else if type(x) == int or type(x) == float { str(x) }
  else { repr(x) }
}
#let val(k) = como-texto(d.at(k, default: ""))
#let campo(obj, k) = if obj == none or type(obj) != dictionary { "" } else { como-texto(obj.at(k, default: "")) }
// "2025-10-01T00:00:00.000Z" -> "01-10-2025" (mismo comportamiento que el script original)
#let fmt-fecha(x) = {
  let t = como-texto(x)
  if t == "" { "" } else { t.split("T").at(0).split("-").rev().join("-") }
}
#let mascotas = {
  let m = d.at("mascotas", default: ())
  if m == none { () }
  else if type(m) == str { if m.trim() == "" { () } else { json(bytes(m)) } }
  else if type(m) == array { m }
  else { () }
}

// ---------- Encabezado y pie (se repiten en cada página) ----------
#let encabezado = context [
  #grid(
    columns: (60%, 40%),
    align: (left + top, left + top),
    [
      #text(size: 13pt, weight: "bold", fill: gris-oscuro)[Servicio Agrícola y Ganadero] \
      #text(size: 9pt, fill: gris)[https://www.sag.gob.cl]
    ],
    [
      #line(length: 100%, stroke: 2pt + black)
      #v(-2pt)
      #text(size: 9pt, weight: "bold")[N° Solicitud: #val("numero_solicitud")] \
      #text(size: 9pt, weight: "bold")[Fecha Solicitud: #val("fecha_solicitud")]
      #v(4pt)
      #align(right, pad(right: 8pt, text(size: 9pt, fill: gris-texto)[Página #counter(page).display() de #counter(page).final().first()]))
    ],
  )
  #line(length: 100%, stroke: 2pt + black)
  #v(2pt)
  #align(center, text(size: 13pt, weight: "bold", fill: gris-oscuro)[
    Solicitud para certificar la exportación temporal o definitiva de perros y gatos domésticos fuera de Chile
  ])
  #v(2pt)
  #line(length: 100%, stroke: 1pt + black)
]

#let etiqueta(t) = text(weight: "bold", fill: gris-oscuro, t)

#let pie = [
  #line(length: 100%, stroke: 1pt + rgb("#34495e"))
  #v(2pt)
  #grid(
    columns: (1fr, 1fr),
    align: (left + top, right + top),
    text(size: 8pt)[Fecha de Emisión: #etiqueta(val("fecha_emision"))],
    text(size: 8pt)[
      #etiqueta("Generador:") #val("nombre_generador") \
      #etiqueta("Cargo:") #val("cargo_generador") \
      #etiqueta("Región:") #val("region_generador")
    ],
  )
  #v(2pt)
  #line(length: 100%, stroke: 0.5pt + gray)
]

#set page(
  paper: "us-letter",
  margin: (x: 0.55in, top: 1.75in, bottom: 1.2in),
  header: encabezado,
  header-ascent: 8pt,
  footer: pie,
  footer-descent: 14pt,
)
#set text(font: ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans"), size: 10pt, fill: texto)
#set par(leading: 0.3em, spacing: 0.6em)

// ---------- Componentes ----------
#let titulo-seccion(t) = block(
  width: 100%, inset: (x: 8pt, y: 4pt), stroke: 1pt + gris-oscuro, fill: rgb("#ecf0f1"),
  above: 6pt, below: 4pt,
  align(center, text(size: 11pt, weight: "bold", fill: gris-oscuro, t)),
)

#let fila-info(t) = block(
  width: 100%, inset: (x: 8pt, y: 4pt), stroke: 1pt + borde, fill: fondo, radius: 2pt,
  above: 4pt, below: 4pt,
  text(size: 9pt, weight: "bold", fill: gris-oscuro, t),
)

// Bloque con borde izquierdo azul. En el HTML cada etiqueta tiene 10px de padding
// vertical, de ahí el interlineado amplio.
#let bloque-datos(..lineas) = block(
  width: 100%, inset: 4pt, fill: fondo, stroke: (left: 3pt + azul),
  radius: (top-right: 2pt, bottom-right: 2pt), above: 6pt, below: 6pt, breakable: false,
  {
    set text(size: 8pt)
    set par(leading: 19pt)
    v(5pt)
    lineas.pos().map(l => [#etiqueta(l.at(0)) #l.at(1)]).join(linebreak())
    v(5pt)
  },
)

#let subtitulo(t) = block(
  width: 100%, inset: (x: 6pt, top: 4pt, bottom: 2pt), fill: rgb("#ecf8ff"),
  stroke: (bottom: 2pt + azul), radius: (top-left: 3pt, top-right: 3pt), above: 8pt, below: 6pt,
  text(size: 9pt, weight: "bold", fill: gris-oscuro, t),
)

// Tabla con encabezado de color y filas alternadas, como .pet-table
#let tabla(color, columnas, encabezados, filas) = table(
  columns: columnas,
  stroke: 0.5pt + borde,
  inset: (x: 4pt, y: 5pt),
  align: center + horizon,
  fill: (x, y) => if y == 0 { color } else if calc.even(y) { fondo } else { white },
  table.header(..encabezados.map(h => text(fill: white, weight: "bold", size: 7pt, h))),
  ..filas.flatten().map(c => text(size: 7pt, c)),
)

#let sin-registros = text(size: 7pt, fill: gris, style: "italic")[Sin registros]
#let separador = { v(6pt); line(length: 100%, stroke: (paint: borde, thickness: 0.7pt, dash: "dashed")); v(4pt) }

#let seccion-mascota(i, m) = block(
  width: 100%, inset: 8pt, stroke: 2pt + verde, radius: 6pt, fill: rgb("#f6fcf7"),
  above: 6pt, below: 12pt, breakable: true,
  {
    block(
      width: 100%, inset: 6pt, fill: verde, radius: 4pt, below: 8pt,
      align(center, text(size: 11pt, weight: "bold", fill: white)[MASCOTA \##str(i + 1): #campo(m, "nombre")]),
    )

    subtitulo("Información Básica")
    tabla(color-basico, (18%, 12%, 20%, 10%, 20%, 20%),
      ("NOMBRE", "ESPECIE", "RAZA", "SEXO", "COLOR PELAJE", "FECHA NAC."),
      ((campo(m, "nombre"), campo(m, "especieId"), campo(m, "raza"), campo(m, "sexo"),
        campo(m, "colorPelaje"), fmt-fecha(m.at("fechaNacimiento", default: ""))),),
    )

    separador
    let ident = m.at("microchip", default: none)
    let tipo = if ident == none { "microchip" } else { como-texto(ident.at("tipo", default: "microchip")) }
    if tipo == "identificacionExterna" {
      subtitulo("Identificación Externa")
      tabla(color-externo, (1fr, 1fr),
        ("N° REGISTRO", "ESTADO REGISTRO"),
        ((campo(ident, "numeroRegistro"), campo(ident, "estadoRegistroMunicipal")),),
      )
    } else {
      subtitulo("Microchip")
      tabla(color-microchip, (40%, 25%, 35%),
        ("N° MICROCHIP", "FECHA", "UBICACIÓN"),
        ((campo(ident, "numeroMicroChip"),
          if ident == none { "" } else { fmt-fecha(ident.at("fecha", default: "")) },
          campo(ident, "parteDelCuerpo")),),
      )
    }

    separador
    let vet = m.at("veterinario", default: none)
    subtitulo("Veterinario Responsable")
    tabla(color-vet, (25%, 37.5%, 37.5%),
      ("RUT", "NOMBRES", "APELLIDOS"),
      ((campo(vet, "rut"), campo(vet, "nombres"), campo(vet, "apellidos")),),
    )

    separador
    let vacunas = m.at("vacunas", default: ())
    if vacunas == none { vacunas = () }
    subtitulo("Vacunas")
    if vacunas.len() == 0 { sin-registros } else {
      tabla(color-vacuna, (20%, 15%, 15%, 20%, 15%, 15%),
        ("NOMBRE", "TIPO", "FECHA VAC.", "LABORATORIO", "N° SERIE", "VIGENCIA"),
        vacunas.map(vac => (
          campo(vac, "nombre"), campo(vac, "tipo"), fmt-fecha(vac.at("fechaVacuna", default: "")),
          campo(vac, "laboratorio"), campo(vac, "numeroSerie"), fmt-fecha(vac.at("fechaVigencia", default: "")),
        )),
      )
    }

    separador
    let despar = m.at("desparasitaciones", default: ())
    if despar == none { despar = () }
    subtitulo("Desparasitaciones")
    if despar.len() == 0 { sin-registros } else {
      tabla(color-despar, (18%, 12%, 15%, 18%, 15%, 12%, 10%),
        ("NOMBRE", "TIPO", "FECHA APLIC.", "LABORATORIO", "COMPONENTE", "N° SERIE", "PRÓX. APLIC."),
        despar.map(dp => (
          campo(dp, "nombre"), campo(dp, "tipo"), fmt-fecha(dp.at("fechaAplicacion", default: "")),
          campo(dp, "laboratorio"), campo(dp, "componente"), campo(dp, "numeroSerie"),
          fmt-fecha(dp.at("fechaProximaAplicacion", default: "")),
        )),
      )
    }
    separador
  },
)

// ---------- Página 1: antecedentes generales ----------
#titulo-seccion("1. ANTECEDENTES GENERALES")

#fila-info("Datos Solicitante")
#bloque-datos(
  ("RUT SOLICITANTE:", val("rut_solicitante")),
  ("NOMBRE SOLICITANTE:", val("nombre_solicitante")),
  ("CORREO SOLICITANTE:", val("correo_solicitante")),
  ("¿ES SOLICITANTE QUIEN VIAJA CON LA MASCOTA?:", val("viaja_con_mascota")),
)

#fila-info("Datos Exportador")
#bloque-datos(
  ("RUT EXPORTADOR:", val("rut_exportador")),
  ("NOMBRE Y DIRECCIÓN DE EXPORTADOR:", val("nombre_exportador") + " - " + val("direccion_exportador")),
  ("CORREO EXPORTADOR:", val("correo_exportador")),
  ("TELÉFONO CELULAR EXPORTADOR:", val("telefono_celular_exportador")),
  ("TELÉFONO FIJO EXPORTADOR:", val("telefono_fijo_exportador")),
)

#fila-info("Datos Viaje")
#bloque-datos(
  ("FECHA DE SALIDA:", val("fecha_salida")),
  ("REGIÓN DE SALIDA:", val("region_salida")),
  ("MEDIO DE TRANSPORTE (AVIÓN-VEHÍCULO-BARCO):", val("medio_transporte")),
  ("AEROLÍNEA (SI CORRESPONDE):", val("aerolinea")),
  ("PAÍS DE DESTINO:", val("pais_destino")),
  ("CODIGO POSTAL (SI CORRESPONDE):", val("codigo_postal")),
  ("CONTROL FRONTERIZO DE SALIDA DE CHILE:", val("control_fronterizo_salida_chile")),
  ("CONTROL FRONTERIZO DE DESTINO:", val("control_fronterizo_destino")),
  ("¿ES UNA SALIDA TEMPORAL?:", val("salida_temporal")),
)

// ---------- Páginas siguientes: una por mascota ----------
#if mascotas.len() == 0 [
  #pagebreak()
  #titulo-seccion("2. INFORMACIÓN DE LAS MASCOTAS")
  #text(size: 9pt, fill: gris, style: "italic")[Sin mascotas informadas]
] else [
  #for (i, m) in mascotas.enumerate() [
    #pagebreak()
    #titulo-seccion("2. INFORMACIÓN DE LAS MASCOTAS")
    #seccion-mascota(i, m)
  ]
]
