// Plantilla "CZE Brasil" (Certificado Zoosanitario de Exportación, modelo de certificado
// veterinario internacional para caninos y felinos a los Estados Partes del Mercosur,
// bilingüe portugués/español) migrada desde v2 (mascotas/cze-brasil.html + JavaScript)
// a Typst.
//
// Recibe en `sys.inputs.datos` el mismo `DatosCredencial` que arma
// sag.tramite.api.mascotas (CertificadoService.formatearDatosCZE) para "Datos CZE":
//   pais_destino, nombre_solicitante, direccion_exportador, region_firmante,
//   mascotas (arreglo o string JSON) = [{ nombre, especieId, razaId, sexo, colorPelaje,
//     fechaNacimiento (ISO), microchip: { numeroMicroChip, fecha, parteDelCuerpo },
//     veterinario: { nombres, apellidos, rut }, vacunas: [{ fechaVacuna, fechaVigencia,
//     nombre, laboratorio, numeroSerie }], desparasitaciones: [{ tipo "interno"|"externo",
//     fechaAplicacion, laboratorio, nombre, componente }] }]
//   y, si el consumidor los envía, fechaExamen, detallesCrias, nombreVeterinario,
//   registroVeterinario (v2 dejaba el placeholder literal cuando faltaban; aquí van vacíos).
//
// Lo que en HTML hacía el JavaScript lo hace la plantilla:
//   - la autoridad veterinaria emisora se toma del veterinario de la primera mascota;
//   - la fecha de emisión (día / mes / año) se toma en el momento del render, igual que
//     el `new Date()` del script de v2. OJO: typst-as-lib entrega `datetime.today()` en
//     UTC, por eso se pasa `offset: -4` (hora oficial de Chile); durante el horario de
//     verano (UTC-3) la fecha cambia una hora más tarde que en Santiago.
//
// Diferencias deliberadas con v2 (defectos del script original):
//   - v2 sólo listaba las vacunas y desparasitaciones de la PRIMERA mascota y numeraba
//     las filas con el índice de la vacuna. Aquí se listan las de todas las mascotas y
//     el "Nº de orden" es el de la mascota en la tabla I, como pide el formulario.
//   - v2 comparaba `tipo === 'interno' | 'externo'` (exacto); aquí se acepta cualquier
//     capitalización y también "Interna"/"Externa".

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
#let campo(obj, k) = if obj == none or type(obj) != dictionary { "" } else { como-texto(obj.at(k, default: "")) }
// "2025-10-01T00:00:00.000Z" -> "01/10/2025" (mismo comportamiento que el script original)
#let fmt-fecha(x) = {
  let t = como-texto(x)
  if t == "" { "" } else { t.split("T").at(0).split("-").rev().join("/") }
}
#let lista(obj, k) = {
  let l = if obj == none or type(obj) != dictionary { none } else { obj.at(k, default: none) }
  if type(l) == array { l } else { () }
}
#let mascotas = {
  let m = d.at("mascotas", default: ())
  if m == none { () }
  else if type(m) == str { if m.trim() == "" { () } else { json(bytes(m)) } }
  else if type(m) == array { m }
  else { () }
}
#let vet-info = if mascotas.len() > 0 and type(mascotas.at(0)) == dictionary {
  let v = mascotas.at(0).at("veterinario", default: none)
  if v == none { "" } else { campo(v, "nombres") + " " + campo(v, "apellidos") + " (" + campo(v, "rut") + ")" }
} else { "" }
#let hoy = datetime.today(offset: -4)

// ---------- Medidas (px CSS × 0.75) ----------
#let sans = ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans")
#let gris-th = rgb("#f0f0f0")
#let gris-titulo = rgb("#e0e0e0")

// body: max-width 210mm, margin 20px auto, padding 20px; sin @page → carta con márgenes 0.
// Chromium deja el contenido pegado al borde en las páginas siguientes; aquí se usa un
// margen superior/inferior moderado en todas.
#set page(paper: "us-letter", margin: (x: 15pt, top: 29pt, bottom: 18pt))
// Cajas de línea como en CSS con line-height 1.4: ascendente 0.905em + descendente 0.212em
// más medio interlineado (0.14em) arriba y abajo; así cada línea mide 1.4em, también en
// bloques de una sola línea y en celdas de tabla.
#set text(font: sans, size: 12pt, top-edge: 1.045em, bottom-edge: -0.352em)
#set par(leading: 0em, spacing: 12pt, justify: false)

// ---------- Componentes ----------
// Párrafo .bilingual (10px) o de otro tamaño, con márgenes 1em como <p>
#let parrafo(t, tam: 7.5pt, ..args) = block(above: tam, below: tam, text(size: tam, ..args, t))
// .section: borde 1px, padding 10px, margin-bottom 20px
#let seccion(body) = block(width: 100%, stroke: 0.75pt + black, inset: 7.5pt, above: 0pt, below: 15pt, breakable: true, body)
// .section-title: fondo #e0e0e0, negrita 11px, padding 5px, pegado a los bordes laterales
// (margin -10px) y, si es el primer hijo, también al borde superior.
#let titulo-seccion(t, primero: false) = pad(x: -7.5pt, top: if primero { -7.5pt } else { 0pt },
  block(width: 100%, fill: gris-titulo, inset: 3.75pt, above: 0pt, below: 7.5pt,
    text(size: 8.25pt, weight: "bold", t)))
// table: 100%, 9px, bordes 1px, padding 5px, margin 10px 0; th con fondo #f0f0f0
#let tabla(columnas, ..celdas) = block(above: 7.5pt, below: 7.5pt, width: 100%, table(
  columns: columnas, stroke: 0.75pt + black, inset: 3.75pt, align: left + horizon,
  ..celdas.pos().map(c => text(size: 6.75pt, c)),
))
#let th(t) = table.cell(fill: gris-th, strong(t))
#let etiqueta(t) = strong(t)
// <br/> entre bloques dentro de una sección (medido en el PDF de v2)
#let salto = v(9pt)
// input[type=checkbox] checked: cuadrado azul con visto blanco
#let casilla = box(width: 9.5pt, height: 9.5pt, baseline: 2pt, fill: rgb("#1a73e8"), radius: 1.5pt,
  place(top + left, curve(stroke: 1.4pt + white,
    curve.move((2.2pt, 5pt)), curve.line((4pt, 6.9pt)), curve.line((7.5pt, 2.8pt)))))
#let item-check(body) = block(above: 6pt, below: 6pt, {
  set text(size: 7.5pt, top-edge: 1.145em, bottom-edge: -0.452em)   // line-height 1.6
  casilla
  h(4pt)
  body
})

// ---------- Encabezado ----------
// .logo absoluto; Chromium lo pinta a ~57px del borde izquierdo (medido en el PDF de v2)
#place(top + left, dx: 27pt, dy: 4pt, image("assets/logo-sag-minagri.png", width: 75pt, height: 75pt))
#v(22.5pt)
#let h1(t) = block(above: 3.75pt, below: 3.75pt, text(size: 10.5pt, weight: "bold", t))
#let h2(t) = block(above: 2.25pt, below: 2.25pt, text(size: 9pt, weight: "bold", t))
#align(center)[
  #h1[MINISTERIO DE AGRICULTURA]
  #h2[MINISTÉRIO DE AGRICULTURA]
  #h1[SERVICIO AGRÍCOLA Y GANADERO]
  #h2[SERVIÇO AGRÍCOLA E PECUÁRIO]
  #h1[CERTIFICADO ZOOSANITARIO DE EXPORTACIÓN]
  #h2[CERTIFICADO ZOOSSANITÁRIO DE EXPORTAÇÃO]
  #v(16.8pt)
  #h2[MODELO DE CERTIFICADO VETERINARIO INTERNACIONAL PARA EL INGRESO Y CIRCULACIÓN DE CANINOS Y FELINOS
    DOMÉSTICOS A LOS ESTADOS PARTES / MODELO DE CERTIFICADO VETERINÁRIO INTERNACIONAL PARA O
    INGRESSO E CIRCULAÇÃO DE CANINOS E FELINOS DOMÉSTICOS AOS ESTADOS PARTES]
]
#v(22.5pt)

#tabla((68fr, 32fr),
  etiqueta[País de origem / País de origen:], [Chile],
  etiqueta[País(es) de trânsito (caso corresponda) / País(es) de tránsito (si corresponde):], [],
  etiqueta[País de destino / País de destino:], val("pais_destino"),
  etiqueta[Nome da autoridade veterinária emissora / Nombre de la autoridad veterinaria emisora:], vet-info,
)
#v(16.8pt)

// ---------- Secciones I a III ----------
#seccion[
  #parrafo(tam: 6pt)[Pestaña “Mascotas”, Subpestaña “Antecedentes”, esto es por cada mascota.]
  #salto
  #titulo-seccion[I. Identificação do animal / Identificación del animal]
  #tabla((7.4fr, 10.5fr, 6.9fr, 7.7fr, 5.6fr, 8.4fr, 12.5fr, 25.3fr, 15.6fr),
    th[Nº de ordem / Nº de orden], th[Nome do Animal / Nombre del animal], th[Espécie / Especie],
    th[Raça / Raza], th[Sexo], th[Pelagem / Pelaje], th[Data de Nascimento / Fecha de nacimiento],
    th[Número do transponder (microchip) e data de aplicação / Número transpondedor (microchip) y fecha de aplicación \*],
    th[Localização do transponder / Ubicación transpondedor \*],
    ..mascotas.enumerate().map(((i, m)) => {
      let chip = m.at("microchip", default: none)
      let numero = campo(chip, "numeroMicroChip")
      let fecha = if chip == none { "" } else { fmt-fecha(chip.at("fecha", default: "")) }
      (
        str(i + 1), campo(m, "nombre"), campo(m, "especieId"), campo(m, "razaId"), campo(m, "sexo"),
        campo(m, "colorPelaje"), fmt-fecha(m.at("fechaNacimiento", default: "")),
        numero + (if numero != "" and fecha != "" { " / " } else { " " }) + fecha,
        campo(chip, "parteDelCuerpo"),
      )
    }).flatten(),
  )
  #parrafo(tam: 6pt)[\*Caso corresponda / Si corresponde \
    Em caso de não corresponder, escrever "xxxxx" (cinco xis) / En caso de no corresponder, escribir "xxxxx" (cinco equis)]
  #salto
  #titulo-seccion[II. Informação de origem / Información de origen]
  #tabla((49fr, 51fr),
    etiqueta[Nome do proprietário ou responsável / Nombre del propietario o responsable:], val("nombre_solicitante"),
    etiqueta[Endereço / Dirección:], val("direccion_exportador"),
    etiqueta[Cidade/País / Ciudad/País:], val("region_firmante") + " - Chile",
  )
  #salto
  #titulo-seccion[III. Informação de destino / Información de destino]
  #tabla((80.6fr, 19.4fr),
    etiqueta[Nome do proprietário ou responsável / Nombre del propietario o responsable:], val("nombre_solicitante"),
    etiqueta[Endereço / Dirección:], [],
    etiqueta[Cidade/País / Ciudad/País:], val("pais_destino"),
  )
]
// Etiqueta suelta que v2 deja entre las secciones III y IV (sin valor); se conserva.
// Es un <span> en una línea del body (16px × 1.4).
#block(above: 15pt, below: 0pt, height: 16.8pt, align(horizon, text(size: 6pt, weight: "bold")[Data de emissão / Fecha de emisión]))

// ---------- Sección IV ----------
#let filas-vacunas = mascotas.enumerate().map(((i, m)) => lista(m, "vacunas").map(vac => (
  str(i + 1), fmt-fecha(vac.at("fechaVacuna", default: "")), fmt-fecha(vac.at("fechaVigencia", default: "")),
  campo(vac, "nombre"), campo(vac, "laboratorio"), campo(vac, "numeroSerie"),
))).flatten()
#let filas-despar(tipo) = mascotas.enumerate().map(((i, m)) => lista(m, "desparasitaciones")
  .filter(dp => lower(campo(dp, "tipo")).starts-with(tipo))
  .map(dp => (
    str(i + 1), fmt-fecha(dp.at("fechaAplicacion", default: "")),
    campo(dp, "laboratorio"), campo(dp, "nombre"), campo(dp, "componente"),
  ))).flatten()
#let tabla-despar(tipo) = tabla((14.3fr, 26.6fr, 14.3fr, 27.1fr, 17.6fr),
  th[Nº de ordem / Nº de orden\*], th[Data de administração / Fecha de administración],
  th[Laboratório / Laboratorio], th[Marca/Nome comercial / Marca/Nombre comercial],
  th[Princípio ativo / Principio activo],
  ..filas-despar(tipo),
)

#seccion[
  #titulo-seccion(primero: true)[IV. Informações zoossanitárias / Información zoosanitaria]
  #parrafo[*O veterinário oficial abaixo assinado certifica que / El médico veterinario oficial abajo firmante certifica que:*]
  #block(above: 11.25pt, below: 11.25pt)[
    #parrafo[*a. O(s) animal(is) foi(ram) vacinado(s) contra a raiva\* / El/los animal(es) fue(ron) vacunado(s) contra la rabia\**]
    #parrafo[*1. Dados da vacinação antirrábica / Datos de la vacunación antirrábica*]
    #tabla((9.2fr, 17.6fr, 26.5fr, 22.1fr, 10.7fr, 13.8fr),
      th[Nº de ordem\* / Nº de orden], th[Data de vacinação (día/mes/año) / Fecha de vacunación],
      th[Data de vencimento da vacinação (día/mes/año)\*\* / Fecha de vencimiento de la vacunación\*\*],
      th[Marca/Nome comercial da vacina / Marca/Nombre comercial de la Vacuna], th[Laboratório / Laboratorio],
      th[Número do Serie/lote / Número de serie/lote],
      ..filas-vacunas,
    )
    #parrafo(tam: 6pt)[Manter o mesmo número de ordem da tabela I. Identificação do animal / \*Mantener el mismo número de orden de la tabla I. Identificación del animal]
    #item-check[*Para os animais primovacinados, foram transcorridos vinte e um (21) dias desde a aplicação da referida vacina. Aqueles animais que não tenham sido revacinados antes do vencimento da vacina vigente foram considerados primovacinados.\*\* / Para los animales primovacunados, han transcurrido veintiún (21) días desde la aplicación de la vacuna mencionada. Aquellos animales que no hayan sido revacunados antes del vencimiento de la vacuna vigente se consideran primovacunados.*]
  ]
  #item-check[*b. O(s) animal(is) é(são) menor(es) de noventa (90) dias de idade no momento da emissão do presente certificado, não foi(foram) vacinado(s) contra a raiva e não esteve(estiveram) em contato com nenhum caso de raiva urbana nos últimos noventa (90) dias\* / El/los animal(es) es/son menor(es) de noventa (90) días de edad al momento de la emisión del presente certificado, no fue(ron) vacunado(s) contra la rabia y no estuvo/estuvieron en contacto con ningún caso de rabia urbana en los últimos noventa (90) días\**]
  #block(above: 11.25pt, below: 11.25pt)[
    #parrafo[*2. Dados do tratamento antiparasitário / Tratamiento antiparasitario*]
    #parrafo(tam: 6.75pt)[O(s) animal(is) foi(foram) submetido(s), dentro dos quinze (15) dias anteriores à emissão do presente certificado, a tratamento de amplo espectro contra parasitos internos e externos, com produtos autorizados pela autoridade veterinária ou competente e seguindo as recomendações do fabricante. / El/los animal(es) fue(ron) sometido(s), dentro de los quince (15) días previos a la emisión del presente certificado, a tratamiento de amplio espectro contra parásitos internos y externos, con productos autorizados por la autoridad veterinaria competente y conforme a las recomendaciones del fabricante.]
    #parrafo[*2.1. Tratamento antiparasitário interno / Tratamiento antiparasitario interno:*]
    #tabla-despar("intern")
    #parrafo[*2.2. Tratamento antiparasitário externo / Tratamiento antiparasitario externo:*]
    #tabla-despar("extern")
    #parrafo(tam: 6pt)[\* Manter o mesmo número de ordem da tabela "I. Identificação do animal / Mantener el mismo número de orden de la tabla I. Identificación del animal]
  ]
  #block(above: 11.25pt, below: 11.25pt)[
    #parrafo[*3. Informação adicional / Información adicional*]
    #item-check[Declaro que o(s) animal(is) foi(foram) examinado(s) no dia #val("fechaExamen"), não manifesta(m) evidências de sinais clínicos de doenças infectocontagiosas nem parasitárias e está(ão) apto(s) para transporte / Declaro que el/los animal(es) fue(ron) examinado(s) el día #val("fechaExamen"), no presenta(n) signos clínicos de enfermedades infectocontagiosas ni parasitarias y se encuentra(n) apto(s) para el transporte.]
    #item-check[A fêmea, se transportada com sua(s) cria(s) menor(es) de noventa (90) dias de idade (detalhar a data de nascimento, quantidade e sexo): / La hembra, si es transportada con su(s) cría(s) menor(es) de noventa (90) días de edad (detallar la fecha de nacimiento, cantidad y sexo): #val("detallesCrias")]
  ]
]

// ---------- Pie legal (.footer: 9px italic; los <p class="bilingual"> quedan en 10px) ----------
#block(above: 22.5pt, below: 0pt, {
  set text(style: "italic", weight: "bold")
  parrafo[Este Certificado Veterinário Internacional é válido por sessenta (60) dias, a partir da data de sua emissão, para o ingresso, trânsito, circulação ou retorno aos Estados Partes, desde que a vacinação antirrábica se encontre vigente e, se aplicável, se cumpram com os requisitos adicionais exigidos por cada Estado Parte / Este certificado zoosanitario de exportación es válido por sesenta (60) días, a partir de la fecha de su emisión, para el ingreso, tránsito, circulación o retorno a los Estados Partes, siempre que la vacunación antirrábica se encuentre vigente y, de ser aplicable, se cumplan los requisitos adicionales exigidos por cada Estado Parte.]
  parrafo[Este exemplar original de Certificado Veterinário Internacional não deve ser retido e deve continuar de posse do proprietário ou responsável / El presente certificado zoosanitario de exportación (CZE) no debe ser retenido y debe quedar en poder del/de la propietario/a o responsable del animal.]
})

// ---------- Fecha de emisión y firma ----------
#let campo-fecha(t) = box(width: 22.5pt, stroke: (bottom: 0.75pt + black), outset: (bottom: 1pt), align(center, t))
#block(above: 30pt, below: 0pt, {
  parrafo[*Fecha de emisión / Data de emissão:* #campo-fecha(hoy.display("[day]")) / #campo-fecha(hoy.display("[month]")) / #campo-fecha(hoy.display("[year]"))]
  block(above: 45pt, below: 0pt, {
    parrafo(tam: 12pt, "_" * 41)
    parrafo[*Firma y sello del médico veterinario oficial / Assinatura e carimbo do veterinário oficial*]
    parrafo[Nombre: #val("nombreVeterinario")]
    parrafo[Registro: #val("registroVeterinario")]
  })
})
